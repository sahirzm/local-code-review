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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request, middleware, routing::post, Router};
    use tower::ServiceExt;

    const TOKEN: &str = "test-csrf-token-123";
    const PORT: u16 = 9876;

    fn app() -> Router {
        let state = Arc::new(CsrfState { token: TOKEN.into(), port: PORT });
        Router::new()
            .route("/test", post(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(state, csrf_middleware))
    }

    fn req(headers: &[(&str, &str)]) -> Request<Body> {
        let mut b = Request::builder().method("POST").uri("/test");
        for (k, v) in headers {
            b = b.header(*k, *v);
        }
        b.body(Body::empty()).unwrap()
    }

    #[tokio::test]
    async fn passes_with_valid_token_and_origin_127_0_0_1() {
        let res = app()
            .oneshot(req(&[
                ("origin", "http://127.0.0.1:9876"),
                ("x-csrf-token", TOKEN),
            ]))
            .await
            .unwrap();
        assert_eq!(res.status().as_u16(), 200);
    }

    #[tokio::test]
    async fn passes_with_valid_token_and_origin_localhost() {
        let res = app()
            .oneshot(req(&[
                ("origin", "http://localhost:9876"),
                ("x-csrf-token", TOKEN),
            ]))
            .await
            .unwrap();
        assert_eq!(res.status().as_u16(), 200);
    }

    #[tokio::test]
    async fn rejects_invalid_token() {
        let res = app()
            .oneshot(req(&[
                ("origin", "http://127.0.0.1:9876"),
                ("x-csrf-token", "wrong-token"),
            ]))
            .await
            .unwrap();
        assert_eq!(res.status().as_u16(), 403);
    }

    #[tokio::test]
    async fn rejects_missing_origin() {
        let res = app()
            .oneshot(req(&[("x-csrf-token", TOKEN)]))
            .await
            .unwrap();
        assert_eq!(res.status().as_u16(), 403);
    }

    #[tokio::test]
    async fn rejects_wrong_origin() {
        let res = app()
            .oneshot(req(&[
                ("origin", "http://evil.com"),
                ("x-csrf-token", TOKEN),
            ]))
            .await
            .unwrap();
        assert_eq!(res.status().as_u16(), 403);
    }

    #[tokio::test]
    async fn accepts_referer_when_origin_missing() {
        // Rust impl falls back to referer; TS does not. Document behavior.
        let res = app()
            .oneshot(req(&[
                ("referer", "http://127.0.0.1:9876/some/page"),
                ("x-csrf-token", TOKEN),
            ]))
            .await
            .unwrap();
        assert_eq!(res.status().as_u16(), 200);
    }
}
