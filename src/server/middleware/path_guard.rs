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
