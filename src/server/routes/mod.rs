use std::path::Path;
use std::sync::Arc;

use axum::{
    extract::{Path as AxumPath, State},
    response::IntoResponse,
    Json, Router,
};
use axum::routing::{get, post};
use serde::Deserialize;
use tokio::sync::Mutex;

use crate::git::GitModule;
use crate::output::file_writer::{get_default_output_path, write_review_output};
use crate::output::markdown::{generate_markdown, MarkdownInput};
use crate::session;
use crate::types::*;

use super::Shutdown;

#[derive(Clone)]
pub struct AppState {
    pub metadata: ReviewMetadata,
    pub diff_data: DiffResponse,
    pub repo_root: String,
    pub csrf_token: String,
    pub output_path: String,
    pub git: Arc<Mutex<GitModule>>,
    pub shutdown: Arc<Shutdown>,
    pub config: crate::config::Config,
}

#[derive(Deserialize)]
struct SaveSessionBody {
    session: ReviewSession,
    _csrf: String,
}

pub fn create_api_router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/config", get(get_config))
        .route("/api/v1/metadata", get(get_metadata))
        .route("/api/v1/diff", get(get_diff))
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
    Json(state.metadata)
}

async fn get_diff(State(state): State<AppState>) -> Json<DiffResponse> {
    Json(state.diff_data)
}

async fn get_file(
    State(state): State<AppState>,
    AxumPath(path): AxumPath<String>,
) -> impl IntoResponse {
    let git = state.git.lock().await;
    let head_ref = state.metadata.head_ref.clone();

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
    let markdown_input = MarkdownInput {
        comments: body.comments,
        diff_data: state.diff_data,
        metadata: state.metadata,
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

    print!("{}", markdown);

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
    let hash = session::hash_repo_path(&state.metadata.commit_range);
    let key = session::get_session_key(&hash, &state.metadata.commit_range);
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
            metadata: metadata(),
            diff_data: diff(),
            repo_root,
            csrf_token: CSRF.into(),
            output_path: String::new(),
            git: Arc::new(Mutex::new(git)),
            shutdown: Arc::new(Shutdown::new()),
            config: crate::config::Config::default(),
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
    async fn get_file_falls_back_to_filesystem_when_not_in_git() {
        let (state, tmp) = test_state();
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
