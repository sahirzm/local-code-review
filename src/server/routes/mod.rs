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
}

#[derive(Deserialize)]
struct SaveSessionBody {
    session: ReviewSession,
    _csrf: String,
}

pub fn create_api_router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
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
