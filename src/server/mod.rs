use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;

use crate::git::GitModule;
use crate::types::{DiffResponse, ReviewMetadata};

pub mod middleware;
pub mod routes;
pub mod shutdown;

pub use shutdown::Shutdown;

pub struct ServerState {
    pub metadata: ReviewMetadata,
    pub diff_data: DiffResponse,
    pub repo_root: String,
    pub csrf_token: String,
    pub output_path: String,
    pub git: Arc<Mutex<GitModule>>,
}

pub async fn start_server(
    state: ServerState,
    port: u16,
) -> anyhow::Result<(u16, Arc<Shutdown>)> {
    let shutdown = Arc::new(Shutdown::new());
    let shutdown_clone = shutdown.clone();

    let app_state = routes::AppState {
        metadata: state.metadata,
        diff_data: state.diff_data,
        repo_root: state.repo_root.clone(),
        csrf_token: state.csrf_token.clone(),
        output_path: state.output_path,
        git: state.git,
        shutdown: shutdown_clone,
    };

    let api = routes::create_api_router(app_state);

    let frontend_dir = std::env::current_exe()?
        .parent()
        .map(|p| p.join("frontend").join("dist"))
        .unwrap_or_else(|| std::env::current_dir().unwrap().join("frontend").join("dist"));
    eprintln!("Serving frontend from: {}", frontend_dir.display());

    let app = Router::new()
        .nest("/", api)
        .fallback_service(ServeDir::new(&frontend_dir));

    let actual_port = bind_port(app, port).await?;
    Ok((actual_port, shutdown))
}

async fn bind_port(app: Router, start_port: u16) -> anyhow::Result<u16> {
    let max_retries = 3;
    let mut port = start_port;

    for attempt in 0..=max_retries {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        match tokio::net::TcpListener::bind(addr).await {
            Ok(listener) => {
                if attempt > 0 {
                    eprintln!(
                        "Server started on http://127.0.0.1:{} (port {} was in use)",
                        port, start_port
                    );
                } else {
                    eprintln!("Server started on http://127.0.0.1:{}", port);
                }
                tokio::spawn(async move {
                    let _ = axum::serve(listener, app).await;
                });
                return Ok(port);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse && attempt < max_retries => {
                eprintln!("Port {} in use, trying {}...", port, port + 1);
                port += 1;
            }
            Err(e) => return Err(e.into()),
        }
    }

    Err(anyhow::anyhow!("Could not bind to any port"))
}
