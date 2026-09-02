use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use tokio::sync::Mutex;
use tower_http::services::{ServeDir, ServeFile};

use tokio::sync::oneshot;

use crate::config::Config;
use crate::git::{DiffSource, GitModule};
use crate::types::{DiffResponse, ReviewMetadata};

pub mod frontend;
pub mod middleware;
pub mod routes;
pub mod shutdown;

pub use shutdown::Shutdown;

/// Channel used to hand the generated review markdown from `post_finish` back to
/// a blocked MCP `start_review` tool call. `None` in CLI/TUI mode, where the
/// markdown is printed to stdout instead. Wrapped in `Arc<Mutex<Option<..>>>`
/// because `AppState` is `Clone` while a `oneshot::Sender` can only fire once:
/// `post_finish` locks and `take()`s the sender to send exactly one message.
pub type FinishTx = Arc<Mutex<Option<oneshot::Sender<String>>>>;

pub struct ServerState {
    pub metadata: ReviewMetadata,
    pub diff_data: DiffResponse,
    pub repo_root: String,
    pub csrf_token: String,
    pub output_path: String,
    pub git: Arc<Mutex<GitModule>>,
    pub frontend_dir: Option<PathBuf>,
    pub config: Config,
    pub diff_source: DiffSource,
    pub default_context: u32,
    pub finish_tx: Option<FinishTx>,
}

pub async fn start_server(
    state: ServerState,
    port: u16,
) -> anyhow::Result<(u16, Arc<Shutdown>, tokio::task::JoinHandle<()>)> {
    start_server_with_shutdown(state, port, Shutdown::new()).await
}

/// Like `start_server` but with a caller-supplied `Shutdown`, so MCP mode can
/// disable the idle timeout (`Shutdown::with_timeout(None)`).
pub async fn start_server_with_shutdown(
    state: ServerState,
    port: u16,
    shutdown: Shutdown,
) -> anyhow::Result<(u16, Arc<Shutdown>, tokio::task::JoinHandle<()>)> {
    let shutdown = Arc::new(shutdown);
    let shutdown_clone = shutdown.clone();

    let app_state = routes::AppState {
        diff: Arc::new(Mutex::new(routes::DiffRuntime {
            metadata: state.metadata,
            diff_data: state.diff_data,
            diff_source: state.diff_source,
            default_context: state.default_context,
        })),
        repo_root: state.repo_root.clone(),
        csrf_token: state.csrf_token.clone(),
        output_path: state.output_path,
        git: state.git,
        shutdown: shutdown_clone,
        config: state.config,
        finish_tx: state.finish_tx,
    };

    let api = routes::create_api_router(app_state);

    let app = Router::new().nest("/", api);
    let app = match state.frontend_dir {
        Some(dir) => {
            eprintln!("Serving frontend from disk: {}", dir.display());
            let index = dir.join("index.html");
            app.fallback_service(ServeDir::new(&dir).fallback(ServeFile::new(index)))
        }
        None => app.fallback(frontend::serve_embedded),
    };

    let (actual_port, handle) = bind_port(app, port, shutdown.clone()).await?;
    Ok((actual_port, shutdown, handle))
}

async fn bind_port(
    app: Router,
    start_port: u16,
    shutdown: Arc<Shutdown>,
) -> anyhow::Result<(u16, tokio::task::JoinHandle<()>)> {
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
                // Graceful shutdown: signalling the `Shutdown` stops `axum::serve`
                // and frees the port, so a subsequent review can rebind it.
                let handle = tokio::spawn(async move {
                    let _ = axum::serve(listener, app)
                        .with_graceful_shutdown(async move {
                            shutdown.wait_for_shutdown().await;
                        })
                        .await;
                });
                return Ok((port, handle));
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
