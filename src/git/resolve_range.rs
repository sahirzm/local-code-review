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
    if options.all {
        let base = match options.base.as_ref() {
            Some(b) => git.resolve_ref(b)?,
            None => git.get_last_pushed_commit()?,
        };
        return Ok(RangeResult {
            mode: "all".into(),
            args: vec![base],
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

    fn commit_repo() -> (GitModule, tempfile::TempDir, String) {
        use std::fs;
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let path = dir.path().join("a.txt");
        fs::write(&path, "hi\n").unwrap();
        let mut idx = repo.index().unwrap();
        idx.add_path(std::path::Path::new("a.txt")).unwrap();
        idx.write().unwrap();
        let tree_id = idx.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = git2::Signature::now("t", "t@t.com").unwrap();
        let oid = repo
            .commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();
        let git = GitModule::new(dir.path().to_str().unwrap()).unwrap();
        (git, dir, oid.to_string())
    }

    #[tokio::test]
    async fn returns_all_mode_with_explicit_base() {
        let (git, _dir, oid) = commit_repo();
        let options = CliOptions {
            all: true,
            base: Some(oid.clone()),
            ..Default::default()
        };
        let result = resolve_range(&options, &git).await.unwrap();
        assert_eq!(result.mode, "all");
        assert_eq!(result.args, vec![oid]);
    }

    #[tokio::test]
    async fn all_mode_without_remote_falls_back_to_error() {
        let (git, _dir, _oid) = commit_repo();
        let options = CliOptions {
            all: true,
            ..Default::default()
        };
        // No upstream/origin configured → get_last_pushed_commit should error.
        assert!(resolve_range(&options, &git).await.is_err());
    }

    #[tokio::test]
    async fn resolves_base_flag_to_commits_mode() {
        let (git, _dir, oid) = commit_repo();
        let options = CliOptions {
            base: Some(oid.clone()),
            ..Default::default()
        };
        let result = resolve_range(&options, &git).await.unwrap();
        assert_eq!(result.mode, "commits");
        assert_eq!(result.args[0], oid);
        assert_eq!(result.args[1], "HEAD");
    }

    #[tokio::test]
    async fn defaults_to_last_pushed_commit() {
        // With no remote configured, get_last_pushed_commit fails — verifies the
        // default branch invokes that path.
        let (git, _dir, _oid) = commit_repo();
        let options = CliOptions::default();
        assert!(resolve_range(&options, &git).await.is_err());
    }
}
