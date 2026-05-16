use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct CsrfState {
    pub token: String,
    pub port: u16,
}

pub async fn csrf_middleware(
    State(csrf): State<Arc<CsrfState>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let allowed_origins = vec![
        format!("http://127.0.0.1:{}", csrf.port),
        format!("http://localhost:{}", csrf.port),
    ];

    let origin = request
        .headers()
        .get("origin")
        .or_else(|| request.headers().get("referer"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if origin.is_empty() || !allowed_origins.iter().any(|o| origin.starts_with(o.as_str())) {
        return Err(StatusCode::FORBIDDEN);
    }

    let header_token = request
        .headers()
        .get("x-csrf-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if header_token != csrf.token {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(request).await)
}
