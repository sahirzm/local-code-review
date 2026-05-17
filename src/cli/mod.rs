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
    #[arg(value_name = "COMMIT")]
    pub commit1: Option<String>,

    #[arg(value_name = "COMMIT")]
    pub commit2: Option<String>,

    #[arg(short = 'p', long = "port", value_name = "PORT", default_value = "8989", value_parser = port_in_range)]
    pub port: u16,

    #[arg(short = 'b', long = "base", value_name = "REF")]
    pub base: Option<String>,

    #[arg(long = "no-open", action = ArgAction::SetFalse)]
    pub open: bool,

    #[arg(short = 'o', long = "output", value_name = "PATH")]
    pub output: Option<String>,

    #[arg(long = "staged", default_value_t = false)]
    pub staged: bool,

    #[arg(long = "unstaged", default_value_t = false)]
    pub unstaged: bool,

    #[arg(long = "working", default_value_t = false)]
    pub working: bool,

    #[arg(long = "fetch", default_value_t = false)]
    pub fetch: bool,

    #[arg(long = "tui", default_value_t = false)]
    pub tui: bool,

    /// Serve frontend from this directory instead of the embedded assets (dev override).
    #[arg(long = "frontend-dir", value_name = "DIR", hide = true)]
    pub frontend_dir: Option<String>,
}

pub fn parse_args() -> anyhow::Result<CliOptions> {
    let cli = Cli::parse();

    let mode_flags = [cli.staged, cli.unstaged, cli.working]
        .iter()
        .filter(|&&x| x)
        .count();

    if mode_flags > 1 {
        anyhow::bail!("--staged, --unstaged, and --working are mutually exclusive");
    }

    if mode_flags > 0 && (cli.commit1.is_some() || cli.commit2.is_some()) {
        anyhow::bail!(
            "--staged, --unstaged, and --working cannot be combined with positional commits"
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
}
