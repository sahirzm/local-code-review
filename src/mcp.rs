use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router, ErrorData, ServerHandler, ServiceExt};
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::sync::{oneshot, Mutex};

use crate::config::Config;
use crate::git::GitModule;
use crate::review::{self, ReviewInputs};
use crate::server::Shutdown;
use crate::types::CliOptions;

/// Review-target selection for `start_review`. All fields optional; an empty
/// object reviews the default range (last pushed commit..HEAD), matching the
/// bare `local-review` CLI invocation.
#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct StartReviewArgs {
    /// What to review: "staged", "unstaged", "working", "all", or "commits".
    /// Omit for the default (last pushed commit..HEAD).
    #[serde(default)]
    pub mode: Option<String>,
    /// Base ref to diff against (alternative to a positional commit).
    #[serde(default)]
    pub base: Option<String>,
    /// Explicit base commit (requires mode "commits").
    #[serde(default)]
    pub commit1: Option<String>,
    /// Explicit head commit (with mode "commits"; defaults to HEAD).
    #[serde(default)]
    pub commit2: Option<String>,
    /// Unified diff context lines (like `git -U<n>`). Defaults to the config value.
    #[serde(default)]
    pub context: Option<u32>,
    /// Run `git fetch` before resolving the range.
    #[serde(default)]
    pub fetch: Option<bool>,
    /// Include untracked files (only meaningful with mode "all").
    #[serde(default)]
    pub include_untracked: Option<bool>,
    /// Port to bind the review server (1-65535). Auto-increments if in use.
    /// Defaults to 8989.
    #[serde(default)]
    pub port: Option<u16>,
    /// Do not open a browser window automatically. Defaults to false (opens).
    #[serde(default)]
    pub no_open: Option<bool>,
    /// Path to also write the review markdown to. The markdown is always
    /// returned to the caller regardless.
    #[serde(default)]
    pub output: Option<String>,
    /// Serve the frontend from this directory instead of the embedded assets
    /// (dev override).
    #[serde(default)]
    pub frontend_dir: Option<String>,
}

/// Default review server port, matching the CLI's `--port` default. The server
/// auto-increments if the port is busy, so concurrent reviews still bind.
const DEFAULT_PORT: u16 = 8989;

const MODE_STAGED: &str = "staged";
const MODE_UNSTAGED: &str = "unstaged";
const MODE_WORKING: &str = "working";
const MODE_ALL: &str = "all";
const MODE_COMMITS: &str = "commits";

impl StartReviewArgs {
    /// Translate the tool arguments into `CliOptions`, applying the same
    /// mutual-exclusion rules as the CLI parser.
    fn into_options(self, default_context: u32) -> anyhow::Result<CliOptions> {
        let port = match self.port {
            Some(p) => crate::cli::validate_port(p as usize).map_err(|e| anyhow::anyhow!(e))?,
            None => DEFAULT_PORT,
        };
        let mut opts = CliOptions {
            port,
            context: self.context.unwrap_or(default_context),
            base: self.base,
            fetch: self.fetch.unwrap_or(false),
            no_open: self.no_open.unwrap_or(false),
            output: self.output,
            frontend_dir: self.frontend_dir,
            ..Default::default()
        };

        match self.mode.as_deref() {
            None => {}
            Some(MODE_STAGED) => opts.staged = true,
            Some(MODE_UNSTAGED) => opts.unstaged = true,
            Some(MODE_WORKING) => opts.working = true,
            Some(MODE_ALL) => opts.all = true,
            Some(MODE_COMMITS) => {
                let c1 = self
                    .commit1
                    .ok_or_else(|| anyhow::anyhow!("mode \"commits\" requires commit1"))?;
                let c2 = self.commit2.unwrap_or_else(|| "HEAD".to_string());
                opts.commits = Some([c1, c2]);
            }
            Some(other) => anyhow::bail!(
                "unknown mode \"{}\"; expected staged|unstaged|working|all|commits",
                other
            ),
        }

        Ok(opts)
    }
}

#[derive(Clone)]
pub struct ReviewServer {
    config: Config,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl ReviewServer {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Launch a local human code review in the browser for the current git repo. \
        Opens a diff viewer, blocks until the human finishes the review, then returns the review \
        as markdown. Use when you want a human to review changes before proceeding. Supports the \
        same options as the CLI (except --tui): port, no_open, output, frontend_dir, context, \
        fetch, mode, base, commit1/commit2, and include_untracked."
    )]
    async fn start_review(
        &self,
        Parameters(args): Parameters<StartReviewArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        match self.run_review(args).await {
            Ok(markdown) => Ok(CallToolResult::success(vec![ContentBlock::text(markdown)])),
            Err(e) => Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                "Review failed: {}",
                e
            ))])),
        }
    }

    async fn run_review(&self, args: StartReviewArgs) -> anyhow::Result<String> {
        let cwd = std::env::current_dir()?;
        let cwd_str = cwd.to_string_lossy().to_string();
        if !GitModule::is_git_repo(&cwd_str) {
            anyhow::bail!("not a git repository: {}", cwd_str);
        }

        let include_untracked = args.include_untracked.unwrap_or(false);
        let options = args.into_options(self.config.diff_context_lines)?;
        let port = options.port;
        let no_open = options.no_open;

        let (tx, rx) = oneshot::channel::<String>();
        let inputs = ReviewInputs {
            options,
            config: self.config.clone(),
            finish_tx: Some(Arc::new(Mutex::new(Some(tx)))),
        };

        let git = GitModule::new(&cwd_str)?;
        let state = review::build_server_state(&inputs, git, &cwd, include_untracked).await?;

        // No idle timeout: a human review may take arbitrarily long, and only an
        // explicit finish (or the agent cancelling) should end it.
        let (_port, shutdown, handle) =
            review::start_and_open(state, port, no_open, Shutdown::with_timeout(None)).await?;

        let markdown = rx.await.map_err(|_| {
            anyhow::anyhow!("review ended without a submission (server closed before finish)")
        })?;

        // The finish flow already fires `/api/v1/shutdown`; signal defensively and
        // await teardown so the port is free before the next review.
        shutdown.signal_shutdown();
        let _ = handle.await;

        Ok(markdown)
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for ReviewServer {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.server_info = rmcp::model::Implementation::new(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        );
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.instructions = Some(
            "Exposes `start_review`, which opens a local browser code-review UI and returns \
             the human's review as markdown once they finish."
                .to_string(),
        );
        info
    }
}

/// Run the MCP server over stdio until the client disconnects.
pub async fn run() -> anyhow::Result<()> {
    let config = Config::load();
    let service = ReviewServer::new(config)
        .serve(rmcp::transport::io::stdio())
        .await?;
    service.waiting().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_args_map_to_default_options() {
        let opts = StartReviewArgs::default().into_options(3).unwrap();
        assert!(!opts.staged && !opts.unstaged && !opts.working && !opts.all);
        assert!(opts.commits.is_none());
        assert_eq!(opts.context, 3);
    }

    #[test]
    fn mode_flags_map_correctly() {
        let mk = |mode: &str| StartReviewArgs {
            mode: Some(mode.to_string()),
            ..Default::default()
        };
        assert!(mk("staged").into_options(3).unwrap().staged);
        assert!(mk("unstaged").into_options(3).unwrap().unstaged);
        assert!(mk("working").into_options(3).unwrap().working);
        assert!(mk("all").into_options(3).unwrap().all);
    }

    #[test]
    fn context_override_takes_precedence() {
        let args = StartReviewArgs {
            context: Some(10),
            ..Default::default()
        };
        assert_eq!(args.into_options(3).unwrap().context, 10);
    }

    #[test]
    fn commits_mode_requires_commit1() {
        let args = StartReviewArgs {
            mode: Some("commits".to_string()),
            ..Default::default()
        };
        assert!(args.into_options(3).is_err());
    }

    #[test]
    fn commits_mode_defaults_head() {
        let args = StartReviewArgs {
            mode: Some("commits".to_string()),
            commit1: Some("abc123".to_string()),
            ..Default::default()
        };
        let opts = args.into_options(3).unwrap();
        assert_eq!(opts.commits.unwrap(), ["abc123".to_string(), "HEAD".to_string()]);
    }

    #[test]
    fn unknown_mode_is_rejected() {
        let args = StartReviewArgs {
            mode: Some("bogus".to_string()),
            ..Default::default()
        };
        assert!(args.into_options(3).is_err());
    }

    #[test]
    fn port_defaults_to_8989_when_absent() {
        assert_eq!(StartReviewArgs::default().into_options(3).unwrap().port, DEFAULT_PORT);
    }

    #[test]
    fn custom_port_is_applied() {
        let args = StartReviewArgs {
            port: Some(8080),
            ..Default::default()
        };
        assert_eq!(args.into_options(3).unwrap().port, 8080);
    }

    #[test]
    fn zero_port_is_rejected() {
        let args = StartReviewArgs {
            port: Some(0),
            ..Default::default()
        };
        assert!(args.into_options(3).is_err());
    }

    #[test]
    fn no_open_defaults_false_and_is_forwarded() {
        assert!(!StartReviewArgs::default().into_options(3).unwrap().no_open);
        let args = StartReviewArgs {
            no_open: Some(true),
            ..Default::default()
        };
        assert!(args.into_options(3).unwrap().no_open);
    }

    #[test]
    fn output_and_frontend_dir_are_forwarded() {
        let args = StartReviewArgs {
            output: Some("review.md".to_string()),
            frontend_dir: Some("/tmp/frontend".to_string()),
            ..Default::default()
        };
        let opts = args.into_options(3).unwrap();
        assert_eq!(opts.output.as_deref(), Some("review.md"));
        assert_eq!(opts.frontend_dir.as_deref(), Some("/tmp/frontend"));
    }
}
