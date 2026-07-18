use std::str;

use git2::{DiffFormat, DiffOptions, Repository};

use crate::types::{FileChange, FileStatus};

pub mod diff_parser;
pub mod resolve_range;

pub struct GitModule {
    pub repo_path: String,
    pub repo: Option<Repository>,
}

impl GitModule {
    pub fn new(repo_path: &str) -> anyhow::Result<Self> {
        let repo = Repository::open(repo_path)?;
        Ok(GitModule {
            repo_path: repo_path.to_string(),
            repo: Some(repo),
        })
    }

    fn repo(&self) -> &Repository {
        self.repo.as_ref().expect("GitModule not initialized with a repo")
    }

    pub fn is_git_repo(path: &str) -> bool {
        Repository::open(path).is_ok()
    }

    pub fn resolve_ref(&self, r: &str) -> anyhow::Result<String> {
        let obj = self.repo().revparse_single(r)?;
        Ok(obj.id().to_string())
    }

    pub fn get_remote_tracking_branch(&self) -> Option<String> {
        self.repo().revparse_single("@{upstream}").ok().map(|o| o.id().to_string())
    }

    pub fn get_last_pushed_commit(&self) -> anyhow::Result<String> {
        if let Ok(obj) = self.repo().revparse_single("@{push}") {
            return Ok(obj.id().to_string());
        }

        if let Ok(upstream) = self.repo().revparse_single("@{upstream}") {
            if let Ok(merge_base) = self
                .repo()
                .merge_base(
                    self.repo().head()?.peel_to_commit()?.id(),
                    upstream.id(),
                )
            {
                return Ok(merge_base.to_string());
            }
        }

        let branch_name = match self.repo().head() {
            Ok(head) => head.shorthand().map(|s| s.to_string()).unwrap_or_else(|| "HEAD".to_string()),
            Err(_) => "HEAD".to_string(),
        };

        if branch_name != "HEAD" {
            let refname = format!("origin/{}", branch_name);
            if let Ok(remote_ref) = self.repo().revparse_single(&refname) {
                if let Ok(merge_base) = self
                    .repo()
                    .merge_base(
                        self.repo().head()?.peel_to_commit()?.id(),
                        remote_ref.id(),
                    )
                {
                    return Ok(merge_base.to_string());
                }
            }
        }

        if let Ok(origin_main) = self.repo().revparse_single("origin/main") {
            if let Ok(merge_base) = self
                .repo()
                .merge_base(
                    self.repo().head()?.peel_to_commit()?.id(),
                    origin_main.id(),
                )
            {
                return Ok(merge_base.to_string());
            }
        }

        if let Ok(origin_master) = self.repo().revparse_single("origin/master") {
            if let Ok(merge_base) = self
                .repo()
                .merge_base(
                    self.repo().head()?.peel_to_commit()?.id(),
                    origin_master.id(),
                )
            {
                return Ok(merge_base.to_string());
            }
        }

        Err(anyhow::anyhow!(
            "Could not determine last pushed commit. Use --base <ref> to specify manually."
        ))
    }

    fn diff_to_string(_repo: &Repository, diff: git2::Diff) -> anyhow::Result<String> {
        let mut output = Vec::new();
        diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
            match line.origin() {
                '+' | '-' | ' ' => output.push(line.origin() as u8),
                _ => {}
            }
            output.extend_from_slice(line.content());
            true
        })?;
        Ok(String::from_utf8(output)?)
    }

    pub fn get_diff(&self, commit1: &str, commit2: &str, context_lines: u32) -> anyhow::Result<String> {
        let t1 = self.repo().revparse_single(commit1)?.peel_to_tree().ok();
        let t2 = self.repo().revparse_single(commit2)?.peel_to_tree().ok();
        let mut opts = DiffOptions::new();
        opts.context_lines(context_lines);
        let diff = self
            .repo()
            .diff_tree_to_tree(t1.as_ref(), t2.as_ref(), Some(&mut opts))?;
        Self::diff_to_string(self.repo(), diff)
    }

    pub fn get_staged_diff(&self, context_lines: u32) -> anyhow::Result<String> {
        let head_tree = self.repo().head()?.peel_to_tree().ok();
        let mut opts = DiffOptions::new();
        opts.context_lines(context_lines);
        let diff = self.repo().diff_tree_to_index(head_tree.as_ref(), None, Some(&mut opts))?;
        Self::diff_to_string(self.repo(), diff)
    }

    pub fn get_unstaged_diff(&self, context_lines: u32) -> anyhow::Result<String> {
        let mut opts = DiffOptions::new();
        opts.include_untracked(false);
        opts.context_lines(context_lines);
        let diff = self.repo().diff_index_to_workdir(None, Some(&mut opts))?;
        Self::diff_to_string(self.repo(), diff)
    }

    pub fn get_working_diff(&self, context_lines: u32) -> anyhow::Result<String> {
        let head_tree = self.repo().head()?.peel_to_tree().ok();
        let mut opts = DiffOptions::new();
        opts.include_untracked(false);
        opts.context_lines(context_lines);
        let diff = self
            .repo()
            .diff_tree_to_workdir_with_index(head_tree.as_ref(), Some(&mut opts))?;
        Self::diff_to_string(self.repo(), diff)
    }

    pub fn list_untracked(&self) -> anyhow::Result<Vec<String>> {
        let mut opts = git2::StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(true)
            .include_ignored(false);
        let statuses = self.repo().statuses(Some(&mut opts))?;
        let mut out = Vec::new();
        for entry in statuses.iter() {
            if entry.status().is_wt_new() {
                if let Some(p) = entry.path() {
                    out.push(p.to_string());
                }
            }
        }
        out.sort();
        Ok(out)
    }

    pub fn get_diff_from_to_workdir(
        &self,
        base: &str,
        include_untracked: bool,
        context_lines: u32,
    ) -> anyhow::Result<String> {
        let base_tree = self
            .repo()
            .revparse_single(base)?
            .peel_to_tree()
            .map_err(|_| anyhow::anyhow!("Could not resolve {} to a tree", base))?;
        let mut opts = DiffOptions::new();
        opts.include_untracked(include_untracked)
            .recurse_untracked_dirs(include_untracked)
            .show_untracked_content(include_untracked)
            .context_lines(context_lines);
        let diff = self
            .repo()
            .diff_tree_to_workdir_with_index(Some(&base_tree), Some(&mut opts))?;
        Self::diff_to_string(self.repo(), diff)
    }

    pub fn get_file_list(
        &self,
        commit1: &str,
        commit2: &str,
    ) -> anyhow::Result<Vec<FileChange>> {
        let t1 = self.repo().revparse_single(commit1)?.peel_to_tree().ok();
        let t2 = self.repo().revparse_single(commit2)?.peel_to_tree().ok();

        let mut opts = DiffOptions::new();
        let mut find_opts = git2::DiffFindOptions::new();
        find_opts.renames(true);
        find_opts.copies(true);

        let mut diff = self
            .repo()
            .diff_tree_to_tree(t1.as_ref(), t2.as_ref(), Some(&mut opts))?;
        diff.find_similar(Some(&mut find_opts))?;

        let mut files = Vec::new();
        for delta in diff.deltas() {
            let status = match delta.status() {
                git2::Delta::Added => FileStatus::Added,
                git2::Delta::Deleted => FileStatus::Deleted,
                git2::Delta::Modified => FileStatus::Modified,
                git2::Delta::Renamed => FileStatus::Renamed,
                git2::Delta::Copied => FileStatus::Copied,
                _ => FileStatus::Modified,
            };

            let path = delta
                .new_file()
                .path()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            let old_path = if status == FileStatus::Renamed || status == FileStatus::Copied {
                delta
                    .old_file()
                    .path()
                    .map(|p| p.to_string_lossy().to_string())
            } else {
                None
            };

            files.push(FileChange {
                path,
                old_path,
                status,
                additions: 0,
                deletions: 0,
            });
        }

        Ok(files)
    }

    pub fn get_file_content(&self, commit: &str, file_path: &str) -> anyhow::Result<String> {
        let obj = self
            .repo()
            .revparse_single(&format!("{}:{}", commit, file_path))?;
        let blob = obj
            .into_blob()
            .map_err(|_| anyhow::anyhow!("Not a blob: {}:{}", commit, file_path))?;
        Ok(String::from_utf8(blob.content().to_vec())?)
    }

    pub fn fetch(&self) -> anyhow::Result<()> {
        let mut origin = self
            .repo()
            .find_remote("origin")
            .map_err(|_| anyhow::anyhow!("No 'origin' remote found"))?;
        let mut fetch_opts = git2::FetchOptions::new();
        origin.fetch(
            &["refs/heads/*:refs/remotes/origin/*"],
            Some(&mut fetch_opts),
            None,
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::diff_parser::parse_diff;
    use crate::types::ChangeType;
    use std::fs;
    use std::path::Path;

    struct Fixture {
        _dir: tempfile::TempDir,
        git: GitModule,
        path: std::path::PathBuf,
        base: String,
    }

    fn sig() -> git2::Signature<'static> {
        git2::Signature::now("t", "t@t.com").unwrap()
    }

    fn commit_all(repo: &git2::Repository, msg: &str) -> git2::Oid {
        let mut idx = repo.index().unwrap();
        idx.add_all(["."].iter(), git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        idx.write().unwrap();
        let tree_id = idx.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let parent = repo
            .head()
            .ok()
            .and_then(|h| h.peel_to_commit().ok());
        let parents: Vec<&git2::Commit> = parent.iter().collect();
        let s = sig();
        repo.commit(Some("HEAD"), &s, &s, msg, &tree, &parents)
            .unwrap()
    }

    fn fixture() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        fs::write(dir.path().join("a.txt"), "one\ntwo\nthree\n").unwrap();
        let oid = commit_all(&repo, "init");
        let git = GitModule::new(dir.path().to_str().unwrap()).unwrap();
        Fixture {
            path: dir.path().to_path_buf(),
            _dir: dir,
            git,
            base: oid.to_string(),
        }
    }

    #[test]
    fn diff_preserves_line_origin_prefix() {
        // Regression test: diff_to_string used to drop the +/- origin char,
        // causing parse_diff to count 0 additions/deletions.
        let fx = fixture();
        fs::write(fx.path.join("a.txt"), "one\nTWO\nthree\nfour\n").unwrap();
        let raw = fx.git.get_working_diff(3).unwrap();
        assert!(
            raw.lines().any(|l| l.starts_with('+')),
            "diff should contain '+' lines, got:\n{}",
            raw
        );
        assert!(
            raw.lines().any(|l| l.starts_with('-')),
            "diff should contain '-' lines, got:\n{}",
            raw
        );
        let parsed = parse_diff(&raw);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].additions, 2);
        assert_eq!(parsed[0].deletions, 1);
    }

    #[test]
    fn list_untracked_finds_new_files_only() {
        let fx = fixture();
        fs::write(fx.path.join("untracked.txt"), "x").unwrap();
        fs::write(fx.path.join("a.txt"), "modified\n").unwrap();
        let untracked = fx.git.list_untracked().unwrap();
        assert_eq!(untracked, vec!["untracked.txt".to_string()]);
    }

    #[test]
    fn list_untracked_empty_when_clean() {
        let fx = fixture();
        assert!(fx.git.list_untracked().unwrap().is_empty());
    }

    #[test]
    fn diff_from_to_workdir_excludes_untracked_by_default() {
        let fx = fixture();
        fs::write(fx.path.join("untracked.txt"), "new file\n").unwrap();
        fs::write(fx.path.join("a.txt"), "one\nTWO\nthree\n").unwrap();
        let raw = fx.git.get_diff_from_to_workdir(&fx.base, false, 3).unwrap();
        let parsed = parse_diff(&raw);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].new_path, "a.txt");
    }

    #[test]
    fn diff_from_to_workdir_includes_untracked_when_requested() {
        let fx = fixture();
        fs::write(fx.path.join("untracked.txt"), "new content\n").unwrap();
        let raw = fx.git.get_diff_from_to_workdir(&fx.base, true, 3).unwrap();
        let parsed = parse_diff(&raw);
        let names: Vec<&str> = parsed.iter().map(|f| f.new_path.as_str()).collect();
        assert!(
            names.contains(&"untracked.txt"),
            "expected untracked.txt in diff, got {:?}",
            names
        );
        let untracked = parsed
            .iter()
            .find(|f| f.new_path == "untracked.txt")
            .unwrap();
        assert_eq!(untracked.additions, 1);
        assert_eq!(untracked.status, FileStatus::Added);
    }

    #[test]
    fn diff_from_to_workdir_combines_committed_and_uncommitted() {
        // Make a second commit, then add an unstaged + a staged change.
        let fx = fixture();
        let repo = git2::Repository::open(&fx.path).unwrap();
        fs::write(fx.path.join("committed.txt"), "c\n").unwrap();
        commit_all(&repo, "second commit");

        // Unstaged edit on existing file.
        fs::write(fx.path.join("a.txt"), "one\ntwo\nthree\nfour\n").unwrap();

        // Staged new file.
        fs::write(fx.path.join("staged.txt"), "s\n").unwrap();
        let mut idx = repo.index().unwrap();
        idx.add_path(Path::new("staged.txt")).unwrap();
        idx.write().unwrap();

        let raw = fx.git.get_diff_from_to_workdir(&fx.base, false, 3).unwrap();
        let parsed = parse_diff(&raw);
        let names: std::collections::HashSet<&str> =
            parsed.iter().map(|f| f.new_path.as_str()).collect();
        assert!(names.contains("a.txt"));
        assert!(names.contains("committed.txt"));
        assert!(names.contains("staged.txt"));
    }

    #[test]
    fn get_staged_diff_counts_additions() {
        let fx = fixture();
        let repo = git2::Repository::open(&fx.path).unwrap();
        fs::write(fx.path.join("a.txt"), "one\ntwo\nthree\nfour\n").unwrap();
        let mut idx = repo.index().unwrap();
        idx.add_path(Path::new("a.txt")).unwrap();
        idx.write().unwrap();

        let raw = fx.git.get_staged_diff(3).unwrap();
        let parsed = parse_diff(&raw);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].additions, 1);
        assert_eq!(parsed[0].deletions, 0);
    }

    #[test]
    fn larger_context_yields_more_normal_lines() {
        // A file with plenty of unchanged lines around a single edit: a wider
        // context window must surface more surrounding (Normal) lines.
        let fx = fixture();
        let repo = git2::Repository::open(&fx.path).unwrap();
        let mut body = String::new();
        for i in 0..40 {
            body.push_str(&format!("line {}\n", i));
        }
        fs::write(fx.path.join("big.txt"), &body).unwrap();
        commit_all(&repo, "add big file");

        // Edit a single line in the middle.
        let edited = body.replace("line 20\n", "line 20 CHANGED\n");
        fs::write(fx.path.join("big.txt"), &edited).unwrap();

        let count_normal = |ctx: u32| -> usize {
            let raw = fx.git.get_working_diff(ctx).unwrap();
            parse_diff(&raw)
                .iter()
                .flat_map(|f| f.hunks.iter())
                .flat_map(|h| h.changes.iter())
                .filter(|c| c.change_type == ChangeType::Normal)
                .count()
        };

        let narrow = count_normal(1);
        let wide = count_normal(6);
        assert!(
            wide > narrow,
            "expected more context lines with -U6 ({}) than -U1 ({})",
            wide,
            narrow
        );
    }
}
