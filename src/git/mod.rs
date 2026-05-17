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

    pub fn get_diff(&self, commit1: &str, commit2: &str) -> anyhow::Result<String> {
        let t1 = self.repo().revparse_single(commit1)?.peel_to_tree().ok();
        let t2 = self.repo().revparse_single(commit2)?.peel_to_tree().ok();
        let diff = self
            .repo()
            .diff_tree_to_tree(t1.as_ref(), t2.as_ref(), None)?;
        Self::diff_to_string(self.repo(), diff)
    }

    pub fn get_staged_diff(&self) -> anyhow::Result<String> {
        let head_tree = self.repo().head()?.peel_to_tree().ok();
        let diff = self.repo().diff_tree_to_index(head_tree.as_ref(), None, None)?;
        Self::diff_to_string(self.repo(), diff)
    }

    pub fn get_unstaged_diff(&self) -> anyhow::Result<String> {
        let mut opts = DiffOptions::new();
        opts.include_untracked(false);
        let diff = self.repo().diff_index_to_workdir(None, Some(&mut opts))?;
        Self::diff_to_string(self.repo(), diff)
    }

    pub fn get_working_diff(&self) -> anyhow::Result<String> {
        let head_tree = self.repo().head()?.peel_to_tree().ok();
        let mut opts = DiffOptions::new();
        opts.include_untracked(false);
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
    ) -> anyhow::Result<String> {
        let base_tree = self
            .repo()
            .revparse_single(base)?
            .peel_to_tree()
            .map_err(|_| anyhow::anyhow!("Could not resolve {} to a tree", base))?;
        let mut opts = DiffOptions::new();
        opts.include_untracked(include_untracked)
            .recurse_untracked_dirs(include_untracked)
            .show_untracked_content(include_untracked);
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
