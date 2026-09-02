use std::path::Path;
use std::sync::Arc;

use axum::{
    extract::{Path as AxumPath, Query, State},
    response::IntoResponse,
    Json, Router,
};
use axum::routing::{get, post};
use serde::Deserialize;
use tokio::sync::Mutex;

use crate::git::{diff_parser, DiffSource, GitModule};
use crate::output::file_writer::{get_default_output_path, write_review_output};
use crate::output::markdown::{generate_markdown, MarkdownInput};
use crate::session;
use crate::types::*;

use super::Shutdown;

#[derive(Clone)]
pub struct AppState {
    /// Mutable diff state (metadata, parsed diff, and the diff source) so the
    /// UI can switch diff mode/base at runtime without a restart. Guarded by a
    /// mutex because `AppState` is `Clone` and shared across requests.
    pub diff: Arc<Mutex<DiffRuntime>>,
    pub repo_root: String,
    pub csrf_token: String,
    pub output_path: String,
    pub git: Arc<Mutex<GitModule>>,
    pub shutdown: Arc<Shutdown>,
    pub config: crate::config::Config,
    /// Present in MCP mode: `post_finish` sends the generated markdown here
    /// instead of printing it to stdout (stdout is reserved for JSON-RPC).
    pub finish_tx: Option<super::FinishTx>,
}

/// The diff-related state that can change at runtime via the diff-mode switch.
pub struct DiffRuntime {
    pub metadata: ReviewMetadata,
    /// Diff at the startup default context; served when no `context` override
    /// is requested and reused as the fallback if regeneration fails.
    pub diff_data: DiffResponse,
    pub diff_source: DiffSource,
    pub default_context: u32,
}

/// git2's `DiffOptions::context_lines` takes a u32; this stands in for "Full"
/// (whole-file context) since no real file approaches this many context lines.
pub const FULL_CONTEXT_LINES: u32 = 1_000_000;

#[derive(Deserialize)]
struct DiffQuery {
    /// Requested context lines: a number, or `full` for whole-file context.
    /// Absent → startup default.
    context: Option<String>,
}

#[derive(Deserialize)]
struct SaveSessionBody {
    session: ReviewSession,
    _csrf: String,
}

/// Body of the runtime diff-mode switch. Mirrors the CLI's diff selection so
/// the UI can re-target the review without a restart. CSRF is enforced by the
/// shared middleware (x-csrf-token header + origin check), matching the other
/// mutating routes, so no token field is needed here.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiffModeBody {
    /// One of: staged | unstaged | working | all | commits | last_pushed.
    mode: String,
    /// Base ref for `all`; ignored otherwise.
    #[serde(default)]
    base: Option<String>,
    /// Explicit commit range for `commits` mode.
    #[serde(default)]
    commit1: Option<String>,
    #[serde(default)]
    commit2: Option<String>,
    #[serde(default)]
    include_untracked: bool,
}

pub fn create_api_router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/config", get(get_config))
        .route("/api/v1/metadata", get(get_metadata))
        .route("/api/v1/diff", get(get_diff))
        .route("/api/v1/diff-mode", post(post_diff_mode))
        .route("/api/v1/finish", post(post_finish))
        .route("/api/v1/save-session", post(post_save_session))
        .route("/api/v1/shutdown", post(post_shutdown))
        .route("/api/v1/file/*path", get(get_file))
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

/// Read-only view of the shared config so the web UI can seed theme/icon prefs
/// from the same file the TUI uses. Serialized camelCase to match the frontend.
async fn get_config(State(state): State<AppState>) -> Json<crate::config::Config> {
    Json(state.config)
}

async fn get_metadata(State(state): State<AppState>) -> Json<ReviewMetadata> {
    Json(state.diff.lock().await.metadata.clone())
}

async fn get_diff(
    State(state): State<AppState>,
    Query(query): Query<DiffQuery>,
) -> Json<DiffResponse> {
    let diff = state.diff.lock().await;
    let context = match query.context.as_deref() {
        None => return Json(diff.diff_data.clone()),
        Some("full") => FULL_CONTEXT_LINES,
        Some(n) => match n.parse::<u32>() {
            Ok(parsed) => parsed,
            Err(_) => return Json(diff.diff_data.clone()),
        },
    };

    if context == diff.default_context {
        return Json(diff.diff_data.clone());
    }

    let git = state.git.lock().await;
    match git.diff_for_source(&diff.diff_source, context) {
        Ok(raw) => Json(DiffResponse {
            files: diff_parser::parse_diff(&raw),
        }),
        Err(_) => Json(diff.diff_data.clone()),
    }
}

/// Switch the diff mode/base at runtime. Recomputes the diff and file list,
/// updates the shared metadata, and returns the fresh metadata + diff so the
/// UI can re-render without a restart.
async fn post_diff_mode(
    State(state): State<AppState>,
    Json(body): Json<DiffModeBody>,
) -> impl IntoResponse {
    let git = state.git.lock().await;

    // Resolve the requested mode into the (mode, args) pair compute_diff wants.
    let resolved: anyhow::Result<(String, Vec<String>)> = (|| match body.mode.as_str() {
        "staged" | "unstaged" | "working" => Ok((body.mode.clone(), vec![])),
        "last_pushed" => {
            let base = git.get_last_pushed_commit()?;
            Ok(("commits".to_string(), vec![base, "HEAD".to_string()]))
        }
        "all" => {
            let base = match body.base.as_deref() {
                Some(b) => git.resolve_ref(b)?,
                None => git.get_last_pushed_commit()?,
            };
            Ok(("all".to_string(), vec![base]))
        }
        "commits" => {
            let a = body.commit1.as_deref().ok_or_else(|| anyhow::anyhow!("commit1 required"))?;
            let b = body.commit2.as_deref().unwrap_or("HEAD");
            Ok(("commits".to_string(), vec![git.resolve_ref(a)?, git.resolve_ref(b)?]))
        }
        other => anyhow::bail!("Unknown diff mode: {}", other),
    })();

    let (mode, args) = match resolved {
        Ok(v) => v,
        Err(e) => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e.to_string(), "code": "BAD_MODE"})),
            );
        }
    };

    let mut diff = state.diff.lock().await;
    let computed = match crate::review::compute_diff(
        &git,
        &mode,
        &args,
        body.include_untracked,
        diff.default_context,
    ) {
        Ok(c) => c,
        Err(e) => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e.to_string(), "code": "DIFF_FAILED"})),
            );
        }
    };

    diff.metadata.commit_range = format!("{}..{}", computed.base_ref, computed.head_ref);
    diff.metadata.base_ref = computed.base_ref;
    diff.metadata.head_ref = computed.head_ref;
    diff.metadata.files = computed.file_list;
    diff.diff_data = DiffResponse { files: computed.files };
    diff.diff_source = DiffSource { mode, args, include_untracked: body.include_untracked };

    (
        axum::http::StatusCode::OK,
        Json(serde_json::json!({
            "metadata": diff.metadata,
            "diff": diff.diff_data,
        })),
    )
}

async fn get_file(
    State(state): State<AppState>,
    AxumPath(path): AxumPath<String>,
) -> impl IntoResponse {
    let git = state.git.lock().await;
    let head_ref = state.diff.lock().await.metadata.head_ref.clone();

    if let Ok(content) = git.get_file_content(&head_ref, &path) {
        return (
            [(axum::http::header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            content,
        );
    }

    let full_path = Path::new(&state.repo_root).join(&path);
    match tokio::fs::read_to_string(&full_path).await {
        Ok(content) => (
            [(axum::http::header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            content,
        ),
        Err(_) => (
            [(axum::http::header::CONTENT_TYPE, "application/json")],
            serde_json::json!({"error": "File not found", "code": "NOT_FOUND"}).to_string(),
        ),
    }
}

async fn post_finish(
    State(state): State<AppState>,
    Json(body): Json<FinishRequest>,
) -> impl IntoResponse {
    let (diff_data, metadata) = {
        let diff = state.diff.lock().await;
        (diff.diff_data.clone(), diff.metadata.clone())
    };
    let markdown_input = MarkdownInput {
        comments: body.comments,
        diff_data,
        metadata,
    };
    let markdown = generate_markdown(&markdown_input);

    let out_path = if state.output_path.is_empty() {
        get_default_output_path()
    } else {
        state.output_path.clone()
    };
    let abs_path = write_review_output(&markdown, &out_path).await.unwrap_or_else(|e| {
        eprintln!("Failed to write output: {}", e);
        out_path
    });

    // MCP mode: hand the markdown to the blocked `start_review` tool call via the
    // channel and keep stdout clean for JSON-RPC. CLI/TUI mode: print to stdout.
    match &state.finish_tx {
        Some(tx) => {
            if let Some(sender) = tx.lock().await.take() {
                let _ = sender.send(markdown.clone());
            }
        }
        None => print!("{}", markdown),
    }

    Json(serde_json::json!({
        "success": true,
        "outputPath": abs_path,
        "markdown": markdown,
    }))
}

async fn post_save_session(
    State(state): State<AppState>,
    Json(body): Json<SaveSessionBody>,
) -> Json<serde_json::Value> {
    let commit_range = state.diff.lock().await.metadata.commit_range.clone();
    let hash = session::hash_repo_path(&commit_range);
    let key = session::get_session_key(&hash, &commit_range);
    match session::save_session(&key, &body.session) {
        Ok(()) => Json(serde_json::json!({"success": true})),
        Err(_) => Json(serde_json::json!({"error": "Failed to save session", "code": "SAVE_ERROR"})),
    }
}

async fn post_shutdown(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    state.shutdown.signal_shutdown();
    Json(serde_json::json!({"success": true}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    const CSRF: &str = "test-csrf-token";

    fn metadata() -> ReviewMetadata {
        ReviewMetadata {
            repo_name: "demo".into(),
            commit_range: "main..feature".into(),
            base_ref: "main".into(),
            head_ref: "feature".into(),
            files: vec![FileChange {
                path: "src/lib.rs".into(),
                old_path: None,
                status: FileStatus::Modified,
                additions: 1,
                deletions: 0,
            }],
            timestamp: "2026-01-01T00:00:00Z".into(),
            csrf_token: CSRF.into(),
        }
    }

    fn diff() -> DiffResponse {
        DiffResponse {
            files: vec![ParsedFileDiff {
                old_path: "src/lib.rs".into(),
                new_path: "src/lib.rs".into(),
                hunks: vec![],
                status: FileStatus::Modified,
                additions: 1,
                deletions: 0,
                is_binary: false,
                is_large: false,
                raw_patch: String::new(),
            }],
        }
    }

    /// Builds an AppState backed by a fresh temp git repo, and returns the
    /// tempdir so it outlives the test.
    fn test_state() -> (AppState, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        git2::Repository::init(tmp.path()).unwrap();
        let repo_root = tmp.path().to_string_lossy().to_string();
        let git = GitModule::new(&repo_root).unwrap();
        let state = AppState {
            diff: Arc::new(Mutex::new(DiffRuntime {
                metadata: metadata(),
                diff_data: diff(),
                diff_source: DiffSource {
                    mode: "working".into(),
                    args: vec![],
                    include_untracked: false,
                },
                default_context: 3,
            })),
            repo_root,
            csrf_token: CSRF.into(),
            output_path: String::new(),
            git: Arc::new(Mutex::new(git)),
            shutdown: Arc::new(Shutdown::new()),
            config: crate::config::Config::default(),
            finish_tx: None,
        };
        (state, tmp)
    }

    async fn body_json(res: axum::response::Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    async fn body_text(res: axum::response::Response) -> String {
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn health_returns_ok_status() {
        let (state, _tmp) = test_state();
        let res = create_api_router(state)
            .oneshot(Request::builder().uri("/api/v1/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status().as_u16(), 200);
        assert_eq!(body_json(res).await["status"], "ok");
    }

    #[tokio::test]
    async fn config_endpoint_returns_shared_prefs() {
        let (state, _tmp) = test_state();
        let res = create_api_router(state)
            .oneshot(Request::builder().uri("/api/v1/config").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status().as_u16(), 200);
        let json = body_json(res).await;
        assert_eq!(json["theme"], "default-dark");
        assert_eq!(json["iconMode"], "nerdfont");
        assert_eq!(json["diffContextLines"], 3);
    }

    #[tokio::test]
    async fn metadata_endpoint_returns_repo_metadata() {
        let (state, _tmp) = test_state();
        let res = create_api_router(state)
            .oneshot(Request::builder().uri("/api/v1/metadata").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status().as_u16(), 200);
        let json = body_json(res).await;
        assert_eq!(json["repoName"], "demo");
        assert_eq!(json["commitRange"], "main..feature");
        assert_eq!(json["csrfToken"], CSRF);
    }

    #[tokio::test]
    async fn diff_endpoint_returns_files() {
        let (state, _tmp) = test_state();
        let res = create_api_router(state)
            .oneshot(Request::builder().uri("/api/v1/diff").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status().as_u16(), 200);
        let json = body_json(res).await;
        assert_eq!(json["files"][0]["newPath"], "src/lib.rs");
    }

    #[tokio::test]
    async fn diff_endpoint_absent_context_returns_default() {
        let (state, _tmp) = test_state();
        let res = create_api_router(state)
            .oneshot(Request::builder().uri("/api/v1/diff").body(Body::empty()).unwrap())
            .await
            .unwrap();
        // No context override: the static startup diff is returned verbatim.
        assert_eq!(body_json(res).await["files"][0]["newPath"], "src/lib.rs");
    }

    #[tokio::test]
    async fn diff_endpoint_invalid_context_falls_back_to_default() {
        let (state, _tmp) = test_state();
        let res = create_api_router(state)
            .oneshot(
                Request::builder()
                    .uri("/api/v1/diff?context=notanumber")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status().as_u16(), 200);
        assert_eq!(body_json(res).await["files"][0]["newPath"], "src/lib.rs");
    }

    #[tokio::test]
    async fn diff_endpoint_regenerates_at_requested_context() {
        // A real repo with one commit + a working-tree change spanning enough
        // lines that context=1 and context=full produce different line counts.
        let tmp = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(tmp.path()).unwrap();
        let repo_root = tmp.path().to_string_lossy().to_string();
        let file = tmp.path().join("f.txt");
        let base: String = (1..=20).map(|n| format!("line {n}\n")).collect();
        std::fs::write(&file, &base).unwrap();
        {
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("f.txt")).unwrap();
            index.write().unwrap();
            let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
            let sig = git2::Signature::now("t", "t@t").unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[]).unwrap();
        }
        // Modify a single middle line so context around it is what varies.
        let edited = base.replace("line 10\n", "line 10 changed\n");
        std::fs::write(&file, edited).unwrap();

        let git = GitModule::new(&repo_root).unwrap();
        let state = AppState {
            diff: Arc::new(Mutex::new(DiffRuntime {
                metadata: metadata(),
                // Empty startup diff forces every request to regenerate from the
                // git repo at the requested context rather than the default cache.
                diff_data: DiffResponse { files: vec![] },
                diff_source: DiffSource {
                    mode: "working".into(),
                    args: vec![],
                    include_untracked: false,
                },
                default_context: 3,
            })),
            repo_root,
            csrf_token: CSRF.into(),
            output_path: String::new(),
            git: Arc::new(Mutex::new(git)),
            shutdown: Arc::new(Shutdown::new()),
            config: crate::config::Config::default(),
            finish_tx: None,
        };

        let count_lines = |v: &serde_json::Value| -> usize {
            v["files"][0]["hunks"][0]["changes"].as_array().map_or(0, |a| a.len())
        };

        let router = create_api_router(state);
        let tight = router
            .clone()
            .oneshot(Request::builder().uri("/api/v1/diff?context=1").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let tight_json = body_json(tight).await;

        let full = router
            .oneshot(Request::builder().uri("/api/v1/diff?context=full").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let full_json = body_json(full).await;

        // The change is a line modification (1 delete + 1 insert). context=1
        // adds 1 unchanged line either side → 1 + 1 + 1 + 1 = 4 change rows.
        assert_eq!(count_lines(&tight_json), 4);
        // Full context keeps every line: 19 unchanged + 1 delete + 1 insert.
        assert_eq!(count_lines(&full_json), 21);
        // The regenerated patch is carried through for the frontend adapter.
        assert!(full_json["files"][0]["rawPatch"].as_str().unwrap().contains("line 10 changed"));
    }

    #[tokio::test]
    async fn diff_mode_switch_recomputes_and_updates_metadata() {
        // Repo with a committed file plus a staged-only change, so `working`
        // and `staged` diffs differ and switching modes is observable.
        let tmp = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(tmp.path()).unwrap();
        let repo_root = tmp.path().to_string_lossy().to_string();
        let file = tmp.path().join("f.txt");
        std::fs::write(&file, "a\nb\nc\n").unwrap();
        {
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("f.txt")).unwrap();
            index.write().unwrap();
            let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
            let sig = git2::Signature::now("t", "t@t").unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[]).unwrap();
        }
        // Stage a change (present in `staged`, and also in `working`).
        std::fs::write(&file, "a\nB\nc\n").unwrap();
        {
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("f.txt")).unwrap();
            index.write().unwrap();
        }

        let git = GitModule::new(&repo_root).unwrap();
        let state = AppState {
            diff: Arc::new(Mutex::new(DiffRuntime {
                metadata: metadata(),
                diff_data: DiffResponse { files: vec![] },
                diff_source: DiffSource {
                    mode: "working".into(),
                    args: vec![],
                    include_untracked: false,
                },
                default_context: 3,
            })),
            repo_root,
            csrf_token: CSRF.into(),
            output_path: String::new(),
            git: Arc::new(Mutex::new(git)),
            shutdown: Arc::new(Shutdown::new()),
            config: crate::config::Config::default(),
            finish_tx: None,
        };

        let res = create_api_router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/diff-mode")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"mode":"staged"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status().as_u16(), 200);
        let json = body_json(res).await;
        // The response carries fresh metadata (mode reflected in base_ref) and a
        // recomputed diff for the staged file.
        assert_eq!(json["metadata"]["baseRef"], "staged");
        assert_eq!(json["diff"]["files"][0]["newPath"], "f.txt");
    }

    #[tokio::test]
    async fn diff_mode_switch_rejects_unknown_mode() {
        let (state, _tmp) = test_state();
        let res = create_api_router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/diff-mode")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"mode":"bogus"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status().as_u16(), 400);
    }

    #[tokio::test]
    async fn get_file_falls_back_to_filesystem_when_not_in_git() {        let (state, tmp) = test_state();
        std::fs::write(tmp.path().join("notes.txt"), "hello from disk").unwrap();
        let res = create_api_router(state)
            .oneshot(Request::builder().uri("/api/v1/file/notes.txt").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status().as_u16(), 200);
        assert_eq!(body_text(res).await, "hello from disk");
    }

    #[tokio::test]
    async fn get_file_reports_not_found_for_missing_file() {
        let (state, _tmp) = test_state();
        let res = create_api_router(state)
            .oneshot(Request::builder().uri("/api/v1/file/does-not-exist.txt").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status().as_u16(), 200);
        assert_eq!(body_json(res).await["code"], "NOT_FOUND");
    }

    #[tokio::test]
    async fn finish_writes_output_and_returns_markdown() {
        let (mut state, tmp) = test_state();
        let out = tmp.path().join("review-out.md");
        state.output_path = out.to_string_lossy().to_string();

        let payload = serde_json::json!({
            "comments": [],
            "reviewedFiles": [],
            "metadata": {"commitRange": "main..feature", "timestamp": "2026-01-01T00:00:00Z"},
            "_csrf": CSRF,
        });
        let res = create_api_router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/finish")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status().as_u16(), 200);
        let json = body_json(res).await;
        assert_eq!(json["success"], true);
        let markdown = json["markdown"].as_str().unwrap();
        assert!(markdown.contains("Code Review Comments"));
        assert_eq!(json["outputPath"], out.to_string_lossy().as_ref());
        // The markdown must have actually been written to the output path.
        assert_eq!(std::fs::read_to_string(&out).unwrap(), markdown);
    }

    #[tokio::test]
    async fn finish_sends_markdown_through_channel_in_mcp_mode() {
        let (mut state, tmp) = test_state();
        let out = tmp.path().join("review-out.md");
        state.output_path = out.to_string_lossy().to_string();

        let (tx, rx) = tokio::sync::oneshot::channel::<String>();
        state.finish_tx = Some(std::sync::Arc::new(tokio::sync::Mutex::new(Some(tx))));

        let payload = serde_json::json!({
            "comments": [],
            "reviewedFiles": [],
            "metadata": {"commitRange": "main..feature", "timestamp": "2026-01-01T00:00:00Z"},
            "_csrf": CSRF,
        });
        let res = create_api_router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/finish")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status().as_u16(), 200);
        let json = body_json(res).await;

        // The markdown must arrive on the channel and match the response body.
        let from_channel = rx.await.expect("finish_tx should receive markdown");
        assert!(from_channel.contains("Code Review Comments"));
        assert_eq!(from_channel, json["markdown"].as_str().unwrap());
    }

    #[tokio::test]
    async fn shutdown_endpoint_signals_shutdown() {
        let (state, _tmp) = test_state();
        let shutdown = state.shutdown.clone();
        let res = create_api_router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/shutdown")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status().as_u16(), 200);
        assert_eq!(body_json(res).await["success"], true);
        // The endpoint must actually trip the shutdown flag.
        shutdown.wait_for_shutdown().await;
    }
}
