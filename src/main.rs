use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let options = cli::parse_args()?;
    let cwd = std::env::current_dir()?;
    let cwd_str = cwd.to_string_lossy().to_string();

    if !git::GitModule::is_git_repo(&cwd_str) {
        eprintln!("Error: not a git repository");
        std::process::exit(1);
    }

    let git = git::GitModule::new(&cwd_str)?;

    if options.fetch {
        eprintln!("Fetching...");
        if let Err(e) = git.fetch() {
            eprintln!("Warning: fetch failed ({}), continuing with local data", e);
        }
    }

    let range = git::resolve_range::resolve_range(&options, &git).await?;
    eprintln!("Resolved range: mode={} args={:?}", range.mode, range.args);

    let raw_diff = match range.mode.as_str() {
        "staged" => git.get_staged_diff()?,
        "unstaged" => git.get_unstaged_diff()?,
        "working" => git.get_working_diff()?,
        "commits" => git.get_diff(&range.args[0], &range.args[1])?,
        _ => anyhow::bail!("Unknown range mode: {}", range.mode),
    };

    let files = git::diff_parser::parse_diff(&raw_diff);

    let file_list = if range.mode == "commits" {
        git.get_file_list(&range.args[0], &range.args[1]).unwrap_or_default()
    } else {
        files.iter().map(|f| types::FileChange {
            path: f.new_path.clone(),
            old_path: if f.old_path != f.new_path { Some(f.old_path.clone()) } else { None },
            status: f.status.clone(),
            additions: f.additions,
            deletions: f.deletions,
        }).collect()
    };

    let repo_name = cwd
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let csrf_token = uuid::Uuid::new_v4().to_string();
    let base_ref = range.args.first().cloned().unwrap_or_else(|| range.mode.clone());
    let head_ref = range.args.get(1).cloned().unwrap_or_else(|| "HEAD".to_string());

    let output_path = options.output.clone().unwrap_or_else(|| {
        output::file_writer::get_default_output_path()
    });

    let repo_name_for_meta = repo_name.clone();
    let base_ref_for_meta = base_ref.clone();
    let head_ref_for_meta = head_ref.clone();

    let metadata = types::ReviewMetadata {
        repo_name: repo_name_for_meta,
        commit_range: format!("{}..{}", base_ref_for_meta, head_ref_for_meta),
        base_ref: base_ref_for_meta,
        head_ref: head_ref_for_meta,
        files: file_list.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        csrf_token: csrf_token.clone(),
    };

    let diff_data = types::DiffResponse { files: files.clone() };

    if options.tui {
        let _ = tui::run_tui(
            file_list,
            files,
            head_ref,
            repo_name,
            base_ref,
        );
        return Ok(());
    }

    let server_state = server::ServerState {
        metadata,
        diff_data,
        repo_root: cwd_str.clone(),
        csrf_token,
        output_path,
        git: Arc::new(Mutex::new(git)),
    };

    let (actual_port, shutdown) = server::start_server(server_state, options.port).await?;
    let url = format!("http://127.0.0.1:{}", actual_port);

    for _ in 0..50 {
        if reqwest::get(format!("{}/api/v1/health", &url)).await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    eprintln!("Review UI: {}", url);
    if !options.no_open {
        if let Err(e) = open::that(&url) {
            eprintln!("Open {} in your browser ({})", url, e);
        }
    }

    shutdown.wait_for_shutdown().await;
    Ok(())
}

pub mod cli;
pub mod git;
pub mod output;
pub mod server;
pub mod session;
pub mod types;

pub mod tui;
