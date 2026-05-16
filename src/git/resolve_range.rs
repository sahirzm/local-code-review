use crate::git::GitModule;
use crate::types::CliOptions;

#[derive(Debug, Clone)]
pub struct RangeResult {
    pub mode: String,
    pub args: Vec<String>,
}

pub async fn resolve_range(options: &CliOptions, git: &GitModule) -> anyhow::Result<RangeResult> {
    if let Some(ref commits) = options.commits {
        return Ok(RangeResult {
            mode: "commits".into(),
            args: vec![commits[0].clone(), commits[1].clone()],
        });
    }
    if options.staged {
        return Ok(RangeResult {
            mode: "staged".into(),
            args: vec![],
        });
    }
    if options.unstaged {
        return Ok(RangeResult {
            mode: "unstaged".into(),
            args: vec![],
        });
    }
    if options.working {
        return Ok(RangeResult {
            mode: "working".into(),
            args: vec![],
        });
    }
    if let Some(ref base) = options.base {
        let resolved = git.resolve_ref(base)?;
        return Ok(RangeResult {
            mode: "commits".into(),
            args: vec![resolved, "HEAD".into()],
        });
    }
    let last_pushed = git.get_last_pushed_commit()?;
    Ok(RangeResult {
        mode: "commits".into(),
        args: vec![last_pushed, "HEAD".into()],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_repo() -> (GitModule, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        let git = GitModule::new(dir.path().to_str().unwrap()).unwrap();
        (git, dir)
    }

    #[tokio::test]
    async fn returns_commits_mode_for_explicit_commits() {
        let (git, _dir) = test_repo();
        let options = CliOptions {
            commits: Some(["aaa".into(), "bbb".into()]),
            ..Default::default()
        };
        let result = resolve_range(&options, &git).await.unwrap();
        assert_eq!(result.mode, "commits");
        assert_eq!(result.args, vec!["aaa", "bbb"]);
    }

    #[tokio::test]
    async fn returns_staged_mode() {
        let (git, _dir) = test_repo();
        let options = CliOptions {
            staged: true,
            ..Default::default()
        };
        let result = resolve_range(&options, &git).await.unwrap();
        assert_eq!(result.mode, "staged");
        assert!(result.args.is_empty());
    }

    #[tokio::test]
    async fn returns_unstaged_mode() {
        let (git, _dir) = test_repo();
        let options = CliOptions {
            unstaged: true,
            ..Default::default()
        };
        let result = resolve_range(&options, &git).await.unwrap();
        assert_eq!(result.mode, "unstaged");
        assert!(result.args.is_empty());
    }

    #[tokio::test]
    async fn returns_working_mode() {
        let (git, _dir) = test_repo();
        let options = CliOptions {
            working: true,
            ..Default::default()
        };
        let result = resolve_range(&options, &git).await.unwrap();
        assert_eq!(result.mode, "working");
        assert!(result.args.is_empty());
    }
}
