use std::path::Path;

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};

#[derive(Clone)]
pub struct PathGuardState {
    pub repo_root: String,
}

pub async fn path_guard(
    State(pg): State<PathGuardState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let path_param = request.uri().path();
    let prefix = "/api/v1/file/";
    let Some(raw_file_path) = path_param.strip_prefix(prefix) else {
        return Ok(next.run(request).await);
    };

    let decoded = percent_decode(raw_file_path);

    if decoded.contains('\0') {
        return Err(StatusCode::FORBIDDEN);
    }

    let resolved_root = Path::new(&pg.repo_root)
        .canonicalize()
        .unwrap_or_else(|_| Path::new(&pg.repo_root).to_path_buf());
    let resolved = resolved_root.join(&decoded);
    let resolved_full = resolved.canonicalize().unwrap_or(resolved);

    if !resolved_full.starts_with(&resolved_root) {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(request).await)
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (
                hex_digit(bytes[i + 1]),
                hex_digit(bytes[i + 2]),
            ) {
                out.push((hi << 4) | lo);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::Request,
        middleware,
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    fn app(repo_root: &str) -> Router {
        let state = PathGuardState { repo_root: repo_root.to_string() };
        Router::new()
            .route("/api/v1/file/*path", get(|| async { "ok" }))
            .route("/other", get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(state, path_guard))
    }

    async fn status(repo_root: &str, uri: &str) -> u16 {
        let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
        let res = app(repo_root).oneshot(req).await.unwrap();
        res.status().as_u16()
    }

    fn temp_repo() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[tokio::test]
    async fn passes_for_valid_repo_relative_path() {
        let dir = temp_repo();
        let root = dir.path().to_str().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/index.ts"), "x").unwrap();
        assert_eq!(status(root, "/api/v1/file/src/index.ts").await, 200);
    }

    #[tokio::test]
    async fn rejects_parent_traversal() {
        let dir = temp_repo();
        let root = dir.path().to_str().unwrap();
        // Use ../../../etc/passwd which escapes any depth.
        assert_eq!(status(root, "/api/v1/file/../../../etc/passwd").await, 403);
    }

    #[tokio::test]
    async fn rejects_absolute_path_outside_repo() {
        let dir = temp_repo();
        let root = dir.path().to_str().unwrap();
        assert_eq!(status(root, "/api/v1/file//etc/passwd").await, 403);
    }

    #[tokio::test]
    async fn passes_for_nested_path_within_repo() {
        let dir = temp_repo();
        let root = dir.path().to_str().unwrap();
        std::fs::create_dir_all(dir.path().join("deep/nested")).unwrap();
        std::fs::write(dir.path().join("deep/nested/file.txt"), "x").unwrap();
        assert_eq!(status(root, "/api/v1/file/deep/nested/file.txt").await, 200);
    }

    #[tokio::test]
    async fn rejects_paths_with_null_bytes() {
        let dir = temp_repo();
        let root = dir.path().to_str().unwrap();
        // Axum normalizes %00 to a literal NUL in the decoded path.
        assert_eq!(status(root, "/api/v1/file/src%00/etc/passwd").await, 403);
    }

    #[tokio::test]
    async fn skips_guard_for_non_file_routes() {
        let dir = temp_repo();
        let root = dir.path().to_str().unwrap();
        // /other is not under /api/v1/file/ so guard should pass through.
        assert_eq!(status(root, "/other").await, 200);
    }

    #[tokio::test]
    async fn handles_backslash_paths_without_crashing() {
        // On Linux, backslash is a literal filename char, not a separator.
        // Should either pass (treated as literal) or be rejected — not crash.
        let dir = temp_repo();
        let root = dir.path().to_str().unwrap();
        let s = status(root, "/api/v1/file/src%5Cindex.ts").await;
        assert!(s == 200 || s == 403, "unexpected status: {}", s);
    }
}
