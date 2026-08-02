use std::io::{IsTerminal, Write};

fn prompt_include_untracked(git: &git::GitModule) -> anyhow::Result<bool> {
    let untracked = git.list_untracked().unwrap_or_default();
    if untracked.is_empty() {
        return Ok(false);
    }

    eprintln!("\nUntracked files ({}):", untracked.len());
    let preview = 50;
    for path in untracked.iter().take(preview) {
        eprintln!("  {}", path);
    }
    if untracked.len() > preview {
        eprintln!("  ... and {} more", untracked.len() - preview);
    }

    if !std::io::stdin().is_terminal() {
        eprintln!("(stdin is not a TTY; skipping untracked files)");
        return Ok(false);
    }

    eprint!("Include untracked files in review? [y/N] ");
    std::io::stderr().flush().ok();

    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer)?;
    let yes = matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes");
    Ok(yes)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Logs go to stderr; in MCP mode stdout is reserved for JSON-RPC framing.
    tracing_subscriber::fmt().with_writer(std::io::stderr).init();

    let cli = cli::parse_cli();

    if let Some(cli::Command::Mcp) = cli.command {
        return mcp::run().await;
    }

    // Load shared config first so the CLI can fall back to its diff-context
    // preference when `-U` is not passed.
    let app_config = config::Config::load();
    let options = cli::cli_to_options(cli, app_config.diff_context_lines)?;
    let cwd = std::env::current_dir()?;
    let cwd_str = cwd.to_string_lossy().to_string();

    if !git::GitModule::is_git_repo(&cwd_str) {
        eprintln!("Error: not a git repository");
        std::process::exit(1);
    }

    if options.tui {
        return run_tui_mode(options, app_config, &cwd, cwd_str).await;
    }

    let no_open = options.no_open;
    let port = options.port;

    // The `all` mode's untracked-file choice may prompt on stdin; resolve it here
    // (outside any terminal takeover) before building the server state.
    let include_untracked = if options.all {
        let git = git::GitModule::new(&cwd_str)?;
        prompt_include_untracked(&git)?
    } else {
        false
    };

    let git = git::GitModule::new(&cwd_str)?;
    let inputs = review::ReviewInputs {
        options,
        config: app_config,
        finish_tx: None,
    };
    let state = review::build_server_state(&inputs, git, &cwd, include_untracked).await?;

    let (_port, shutdown, _handle) =
        review::start_and_open(state, port, no_open, server::Shutdown::new()).await?;

    shutdown.wait_for_shutdown().await;
    Ok(())
}

/// The `--tui` path: resolve the diff in-process and hand it to the terminal UI.
/// Kept separate from the web path because the TUI consumes the git module and
/// parsed diffs directly rather than a `ServerState`.
async fn run_tui_mode(
    options: types::CliOptions,
    app_config: config::Config,
    cwd: &std::path::Path,
    cwd_str: String,
) -> anyhow::Result<()> {
    let git = git::GitModule::new(&cwd_str)?;

    if options.fetch {
        eprintln!("Fetching...");
        if let Err(e) = git.fetch() {
            eprintln!("Warning: fetch failed ({}), continuing with local data", e);
        }
    }

    let range = git::resolve_range::resolve_range(&options, &git)?;
    eprintln!("Resolved range: mode={} args={:?}", range.mode, range.args);

    let mut include_untracked = false;
    let raw_diff = match range.mode.as_str() {
        "staged" => git.get_staged_diff(options.context)?,
        "unstaged" => git.get_unstaged_diff(options.context)?,
        "working" => git.get_working_diff(options.context)?,
        "commits" => git.get_diff(&range.args[0], &range.args[1], options.context)?,
        "all" => {
            let base = &range.args[0];
            include_untracked = prompt_include_untracked(&git)?;
            git.get_diff_from_to_workdir(base, include_untracked, options.context)?
        }
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
    let base_ref = range.args.first().cloned().unwrap_or_else(|| range.mode.clone());
    let head_ref = range.args.get(1).cloned().unwrap_or_else(|| "HEAD".to_string());

    let _ = tui::run_tui(tui::TuiContext {
        files: file_list,
        parsed_diffs: files,
        head_ref,
        base_ref,
        repo_name,
        repo_path: cwd_str,
        git,
        range,
        include_untracked,
        context_lines: options.context,
        config: app_config,
    });
    Ok(())
}

pub mod cli;
pub mod config;
pub mod git;
pub mod mcp;
pub mod output;
pub mod review;
pub mod server;
pub mod session;
pub mod types;

pub mod tui;
