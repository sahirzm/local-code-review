use clap::{ArgAction, Parser};
use crate::types::CliOptions;

fn port_in_range(s: &str) -> Result<u16, String> {
    let port: usize = s
        .parse()
        .map_err(|_| format!("`{}` isn't a port number", s))?;
    if (1..=65535).contains(&port) {
        Ok(port as u16)
    } else {
        Err(format!("port not in range 1-65535, got {}", port))
    }
}

#[derive(Parser)]
#[command(name = "local-review")]
#[command(about = "Local code review tool", long_about = None)]
#[command(version)]
pub struct Cli {
    /// Base commit/ref (defaults to last pushed commit)
    #[arg(value_name = "COMMIT")]
    pub commit1: Option<String>,

    /// Head commit/ref (defaults to HEAD)
    #[arg(value_name = "COMMIT")]
    pub commit2: Option<String>,

    /// Port to bind the review server (auto-increments if in use)
    #[arg(short = 'p', long = "port", value_name = "PORT", default_value = "8989", value_parser = port_in_range)]
    pub port: u16,

    /// Base ref to diff against (alternative to positional COMMIT)
    #[arg(short = 'b', long = "base", value_name = "REF")]
    pub base: Option<String>,

    /// Do not open a browser window automatically
    #[arg(long = "no-open", action = ArgAction::SetFalse)]
    pub open: bool,

    /// Path to write the review markdown output
    #[arg(short = 'o', long = "output", value_name = "PATH")]
    pub output: Option<String>,

    /// Review staged changes (index vs HEAD)
    #[arg(long = "staged", default_value_t = false)]
    pub staged: bool,

    /// Review unstaged changes (working tree vs index)
    #[arg(long = "unstaged", default_value_t = false)]
    pub unstaged: bool,

    /// Review all working tree changes (staged + unstaged vs HEAD)
    #[arg(long = "working", default_value_t = false)]
    pub working: bool,

    /// Run `git fetch` before resolving the diff range
    #[arg(long = "fetch", default_value_t = false)]
    pub fetch: bool,

    /// Run in terminal UI mode instead of launching the web server
    #[arg(long = "tui", default_value_t = false)]
    pub tui: bool,

    /// Unified diff context lines (like `git -U<n>`); overrides the config file
    #[arg(short = 'U', long = "context", value_name = "N")]
    pub context: Option<u32>,

    /// Review everything since last pushed commit, including staged, unstaged, and (optionally) untracked files
    #[arg(long = "all", default_value_t = false)]
    pub all: bool,

    /// Serve frontend from this directory instead of the embedded assets (dev override)
    #[arg(long = "frontend-dir", value_name = "DIR", hide = true)]
    pub frontend_dir: Option<String>,
}

/// Parse argv into `CliOptions`. `default_context` (from the shared config)
/// is used only when `-U/--context` is absent, so precedence is
/// CLI flag > config file > built-in default.
pub fn parse_args(default_context: u32) -> anyhow::Result<CliOptions> {
    cli_to_options(Cli::parse(), default_context)
}

fn cli_to_options(cli: Cli, default_context: u32) -> anyhow::Result<CliOptions> {
    let mode_flags = [cli.staged, cli.unstaged, cli.working, cli.all]
        .iter()
        .filter(|&&x| x)
        .count();

    if mode_flags > 1 {
        anyhow::bail!("--staged, --unstaged, --working, and --all are mutually exclusive");
    }

    if mode_flags > 0 && (cli.commit1.is_some() || cli.commit2.is_some()) {
        anyhow::bail!(
            "--staged, --unstaged, --working, and --all cannot be combined with positional commits"
        );
    }

    let commits = match (cli.commit1, cli.commit2) {
        (Some(a), Some(b)) => Some([a, b]),
        (Some(a), None) => Some([a, "HEAD".to_string()]),
        _ => None,
    };

    Ok(CliOptions {
        port: cli.port,
        base: cli.base,
        no_open: !cli.open,
        output: cli.output,
        commits,
        staged: cli.staged,
        unstaged: cli.unstaged,
        working: cli.working,
        fetch: cli.fetch,
        tui: cli.tui,
        all: cli.all,
        context: cli.context.unwrap_or(default_context),
        frontend_dir: cli.frontend_dir,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_port_default() {
        assert_eq!(port_in_range("8989").unwrap(), 8989);
    }

    #[test]
    fn parses_port_custom() {
        assert_eq!(port_in_range("8080").unwrap(), 8080);
    }

    #[test]
    fn validates_port_range() {
        assert!(port_in_range("0").is_err());
        assert!(port_in_range("65536").is_err());
        assert!(port_in_range("abc").is_err());
    }

    fn parse(args: &[&str]) -> anyhow::Result<CliOptions> {
        let cli = Cli::try_parse_from(std::iter::once(&"local-review").chain(args.iter()))?;
        cli_to_options(cli, 3)
    }

    #[test]
    fn defaults_no_mode_flags() {
        let opts = parse(&[]).unwrap();
        assert!(!opts.staged && !opts.unstaged && !opts.working && !opts.all);
        assert!(opts.commits.is_none());
    }

    #[test]
    fn parses_all_flag() {
        let opts = parse(&["--all"]).unwrap();
        assert!(opts.all);
    }

    #[test]
    fn rejects_all_with_other_mode_flag() {
        let err = parse(&["--all", "--staged"]).unwrap_err();
        assert!(err.to_string().contains("mutually exclusive"));
    }

    #[test]
    fn rejects_all_with_working() {
        let err = parse(&["--all", "--working"]).unwrap_err();
        assert!(err.to_string().contains("mutually exclusive"));
    }

    #[test]
    fn rejects_all_with_positional_commit() {
        let err = parse(&["--all", "HEAD~1"]).unwrap_err();
        assert!(err.to_string().contains("cannot be combined"));
    }

    #[test]
    fn second_positional_defaults_head_when_only_one_given() {
        let opts = parse(&["abc123"]).unwrap();
        let commits = opts.commits.unwrap();
        assert_eq!(commits[0], "abc123");
        assert_eq!(commits[1], "HEAD");
    }

    #[test]
    fn no_open_inverts_open_flag() {
        let with = parse(&["--no-open"]).unwrap();
        let without = parse(&[]).unwrap();
        assert!(with.no_open);
        assert!(!without.no_open);
    }

    #[test]
    fn parses_defaults_correctly() {
        let opts = parse(&[]).unwrap();
        assert_eq!(opts.port, 8989);
        assert!(!opts.no_open);
        assert!(!opts.staged);
        assert!(!opts.unstaged);
        assert!(!opts.working);
        assert!(!opts.fetch);
        assert!(!opts.all);
        assert!(opts.commits.is_none());
        assert!(opts.base.is_none());
        assert!(opts.output.is_none());
    }

    #[test]
    fn parses_port_with_custom_value() {
        let opts = parse(&["--port", "8080"]).unwrap();
        assert_eq!(opts.port, 8080);
    }

    #[test]
    fn parses_short_p_flag_for_port() {
        let opts = parse(&["-p", "4000"]).unwrap();
        assert_eq!(opts.port, 4000);
    }

    #[test]
    fn parses_staged_flag() {
        assert!(parse(&["--staged"]).unwrap().staged);
    }

    #[test]
    fn parses_unstaged_flag() {
        assert!(parse(&["--unstaged"]).unwrap().unstaged);
    }

    #[test]
    fn parses_working_flag() {
        assert!(parse(&["--working"]).unwrap().working);
    }

    #[test]
    fn parses_fetch_flag() {
        assert!(parse(&["--fetch"]).unwrap().fetch);
    }

    #[test]
    fn parses_base_option() {
        assert_eq!(parse(&["--base", "main"]).unwrap().base.as_deref(), Some("main"));
    }

    #[test]
    fn parses_output_option() {
        assert_eq!(
            parse(&["--output", "review.md"]).unwrap().output.as_deref(),
            Some("review.md")
        );
    }

    #[test]
    fn parses_two_positional_commits() {
        let opts = parse(&["abc123", "def456"]).unwrap();
        assert_eq!(opts.commits.unwrap(), ["abc123".to_string(), "def456".to_string()]);
    }

    #[test]
    fn rejects_staged_with_unstaged() {
        let err = parse(&["--staged", "--unstaged"]).unwrap_err();
        assert!(err.to_string().contains("mutually exclusive"));
    }

    #[test]
    fn rejects_staged_with_working() {
        let err = parse(&["--staged", "--working"]).unwrap_err();
        assert!(err.to_string().contains("mutually exclusive"));
    }

    #[test]
    fn rejects_staged_with_positional_commit() {
        let err = parse(&["--staged", "abc123"]).unwrap_err();
        assert!(err.to_string().contains("cannot be combined"));
    }

    #[test]
    fn parses_fetch_and_port_together() {
        let opts = parse(&["--fetch", "--port", "3000"]).unwrap();
        assert!(opts.fetch);
        assert_eq!(opts.port, 3000);
    }

    #[test]
    fn context_defaults_to_provided_fallback() {
        // No -U flag → uses the config-provided default.
        let cli = Cli::try_parse_from(["local-review"]).unwrap();
        assert_eq!(cli_to_options(cli, 5).unwrap().context, 5);
    }

    #[test]
    fn context_flag_overrides_fallback() {
        let cli = Cli::try_parse_from(["local-review", "-U", "8"]).unwrap();
        assert_eq!(cli_to_options(cli, 3).unwrap().context, 8);
        let cli = Cli::try_parse_from(["local-review", "--context", "0"]).unwrap();
        assert_eq!(cli_to_options(cli, 3).unwrap().context, 0);
    }

    #[test]
    fn version_is_set() {
        // clap auto-generates --version from Cargo.toml; -V should produce a
        // DisplayVersion error rather than parsing into a Cli value.
        let result = Cli::try_parse_from(["local-review", "-V"]);
        let err = match result {
            Ok(_) => panic!("expected -V to short-circuit"),
            Err(e) => e,
        };
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion);
    }
}
