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
    let Some(file_path) = path_param.strip_prefix(prefix) else {
        return Ok(next.run(request).await);
    };

    if file_path.contains('\0') {
        return Err(StatusCode::FORBIDDEN);
    }

    let resolved_root = Path::new(&pg.repo_root)
        .canonicalize()
        .unwrap_or_else(|_| Path::new(&pg.repo_root).to_path_buf());
    let resolved = resolved_root.join(file_path);
    let resolved_full = resolved.canonicalize().unwrap_or(resolved);

    if !resolved_full.starts_with(&resolved_root) {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(request).await)
}
