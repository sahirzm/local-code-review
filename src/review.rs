use std::path::Path;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::config::Config;
use crate::git::{self, DiffSource, GitModule};
use crate::server::{self, FinishTx, Shutdown};
use crate::types::{self, CliOptions};

/// Everything needed to turn resolved CLI options into a running review server.
pub struct ReviewInputs {
    pub options: CliOptions,
    pub config: Config,
    /// `Some` in MCP mode (markdown is returned to the caller via this channel),
    /// `None` in CLI mode (markdown is printed to stdout by `post_finish`).
    pub finish_tx: Option<FinishTx>,
}

/// Build the web `ServerState` from resolved options + an open git repo.
///
/// This is the diff-resolution → metadata → `ServerState` pipeline shared by the
/// CLI (`main`) and the MCP `start_review` tool. The `all` mode's untracked-file
/// choice is passed in via `include_untracked` rather than prompting, so callers
/// without a TTY (MCP) can supply it directly; `main` resolves it beforehand.
pub async fn build_server_state(
    inputs: &ReviewInputs,
    git: GitModule,
    cwd: &Path,
    include_untracked: bool,
) -> anyhow::Result<server::ServerState> {
    let options = &inputs.options;

    if options.fetch {
        eprintln!("Fetching...");
        if let Err(e) = git.fetch() {
            eprintln!("Warning: fetch failed ({}), continuing with local data", e);
        }
    }

    let range = git::resolve_range::resolve_range(options, &git)?;
    eprintln!("Resolved range: mode={} args={:?}", range.mode, range.args);

    let raw_diff = match range.mode.as_str() {
        "staged" => git.get_staged_diff(options.context)?,
        "unstaged" => git.get_unstaged_diff(options.context)?,
        "working" => git.get_working_diff(options.context)?,
        "commits" => git.get_diff(&range.args[0], &range.args[1], options.context)?,
        "all" => {
            let base = &range.args[0];
            git.get_diff_from_to_workdir(base, include_untracked, options.context)?
        }
        _ => anyhow::bail!("Unknown range mode: {}", range.mode),
    };

    let files = git::diff_parser::parse_diff(&raw_diff);

    let file_list = if range.mode == "commits" {
        git.get_file_list(&range.args[0], &range.args[1]).unwrap_or_default()
    } else {
        files
            .iter()
            .map(|f| types::FileChange {
                path: f.new_path.clone(),
                old_path: if f.old_path != f.new_path {
                    Some(f.old_path.clone())
                } else {
                    None
                },
                status: f.status.clone(),
                additions: f.additions,
                deletions: f.deletions,
            })
            .collect()
    };

    let repo_name = cwd
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let csrf_token = uuid::Uuid::new_v4().to_string();
    let base_ref = range.args.first().cloned().unwrap_or_else(|| range.mode.clone());
    let head_ref = range.args.get(1).cloned().unwrap_or_else(|| "HEAD".to_string());

    let output_path = options
        .output
        .clone()
        .unwrap_or_else(crate::output::file_writer::get_default_output_path);

    let metadata = types::ReviewMetadata {
        repo_name,
        commit_range: format!("{}..{}", base_ref, head_ref),
        base_ref: base_ref.clone(),
        head_ref,
        files: file_list,
        timestamp: chrono::Utc::now().to_rfc3339(),
        csrf_token: csrf_token.clone(),
    };

    let diff_data = types::DiffResponse { files };

    let diff_source = DiffSource {
        mode: range.mode.clone(),
        args: range.args.clone(),
        include_untracked,
    };

    Ok(server::ServerState {
        metadata,
        diff_data,
        repo_root: cwd.to_string_lossy().to_string(),
        csrf_token,
        output_path,
        git: Arc::new(Mutex::new(git)),
        frontend_dir: options.frontend_dir.clone().map(std::path::PathBuf::from),
        config: inputs.config.clone(),
        diff_source,
        default_context: options.context,
        finish_tx: inputs.finish_tx.clone(),
    })
}

/// Start the web server, wait for it to answer `/health`, and (unless
/// suppressed) open the browser. Returns the bound port, the `Shutdown` handle,
/// and the serve task's `JoinHandle` so callers can await clean teardown.
pub async fn start_and_open(
    state: server::ServerState,
    port: u16,
    no_open: bool,
    shutdown: Shutdown,
) -> anyhow::Result<(u16, Arc<Shutdown>, tokio::task::JoinHandle<()>)> {
    let (actual_port, shutdown, handle) =
        server::start_server_with_shutdown(state, port, shutdown).await?;
    let url = format!("http://127.0.0.1:{}", actual_port);

    for _ in 0..50 {
        if reqwest::get(format!("{}/api/v1/health", &url)).await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    eprintln!("Review UI: {}", url);
    if !no_open {
        if let Err(e) = open::that(&url) {
            eprintln!("Open {} in your browser ({})", url, e);
        }
    }

    Ok((actual_port, shutdown, handle))
}
