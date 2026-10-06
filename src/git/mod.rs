use std::collections::HashSet;
use std::path::Path;
use std::str;

use git2::{
    DiffFormat, DiffOptions, FileMode, Index, IndexEntry, IndexEntryExtendedFlag, IndexTime,
    Repository,
};

use crate::types::{FileChange, FileStatus};

pub mod diff_parser;
pub mod resolve_range;

const INDEX_FILE_NAME: &str = "index";
/// Index entry `flags` keep the path length in their low 12 bits.
const INDEX_ENTRY_PATH_LENGTH_MASK: usize = 0x0fff;

pub struct GitModule {
    pub repo_path: String,
    pub repo: Option<Repository>,
}

/// Captures which diff a session is showing so it can be regenerated at a
/// different context-line count on demand (e.g. the web UI's 5/10/20/50/Full
/// selector). Mirrors the `RangeResult` mode/args plus the untracked choice
/// resolved once at startup.
#[derive(Debug, Clone)]
pub struct DiffSource {
    pub mode: String,
    pub args: Vec<String>,
    pub include_untracked: bool,
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

    /// Intent-to-add (`git add -N`) entries are left out, matching
    /// `git diff --cached`: their content is not staged yet.
    pub fn get_staged_diff(&self, context_lines: u32) -> anyhow::Result<String> {
        let head_tree = self.repo().head()?.peel_to_tree().ok();
        let mut opts = DiffOptions::new();
        opts.context_lines(context_lines);
        let intent_to_add = self.intent_to_add_paths()?;
        let index = if intent_to_add.is_empty() {
            None
        } else {
            Some(self.index_without(&intent_to_add)?)
        };
        let diff = self
            .repo()
            .diff_tree_to_index(head_tree.as_ref(), index.as_ref(), Some(&mut opts))?;
        Self::diff_to_string(self.repo(), diff)
    }

    /// Intent-to-add (`git add -N`) entries show as whole-file additions,
    /// matching `git diff`.
    pub fn get_unstaged_diff(&self, context_lines: u32) -> anyhow::Result<String> {
        let intent_to_add = self.intent_to_add_paths()?;
        if intent_to_add.is_empty() {
            let mut opts = DiffOptions::new();
            opts.include_untracked(false);
            opts.context_lines(context_lines);
            let diff = self.repo().diff_index_to_workdir(None, Some(&mut opts))?;
            return Self::diff_to_string(self.repo(), diff);
        }
        self.get_unstaged_diff_with_intent_to_add(&intent_to_add, context_lines)
    }

    /// libgit2 has no intent-to-add awareness and would diff these entries
    /// against the empty blob as plain modifications. Dropping them from a
    /// private index copy turns them into untracked files, whose content
    /// libgit2 renders as new-file additions. That untracked diff is limited to
    /// exactly those paths so other untracked files are never read; tracked
    /// changes come from a separate diff merged into it. An intent-to-add file that is also gitignored is not shown, because
    /// libgit2 never prints content for ignored files.
    fn get_unstaged_diff_with_intent_to_add(
        &self,
        intent_to_add: &HashSet<String>,
        context_lines: u32,
    ) -> anyhow::Result<String> {
        let index = self.index_without(intent_to_add)?;

        let mut intent_to_add_opts = DiffOptions::new();
        intent_to_add_opts
            .include_untracked(true)
            .recurse_untracked_dirs(true)
            .show_untracked_content(true)
            .disable_pathspec_match(true)
            .context_lines(context_lines);
        for path in intent_to_add {
            intent_to_add_opts.pathspec(path);
        }
        let mut diff = self
            .repo()
            .diff_index_to_workdir(Some(&index), Some(&mut intent_to_add_opts))?;

        let mut tracked_opts = DiffOptions::new();
        tracked_opts.include_untracked(false).context_lines(context_lines);
        let tracked = self
            .repo()
            .diff_index_to_workdir(Some(&index), Some(&mut tracked_opts))?;

        // A merged diff prints with the receiving diff's options, so it must be
        // the one that shows untracked content.
        diff.merge(&tracked)?;
        Self::diff_to_string(self.repo(), diff)
    }

    fn intent_to_add_paths(&self) -> anyhow::Result<HashSet<String>> {
        let mut index = self.repo().index()?;
        index.read(false)?;
        Ok(index
            .iter()
            .filter(|entry| {
                IndexEntryExtendedFlag::from_bits_truncate(entry.flags_extended).is_intent_to_add()
            })
            .map(|entry| String::from_utf8_lossy(&entry.path).into_owned())
            .collect())
    }

    /// A private copy of the on-disk index minus `paths`; the repository's own
    /// index is neither modified nor written.
    fn index_without(&self, paths: &HashSet<String>) -> anyhow::Result<Index> {
        let mut index = Index::open(&self.repo().path().join(INDEX_FILE_NAME))?;
        for path in paths {
            index.remove_path(Path::new(path))?;
        }
        Ok(index)
    }

    /// Record `paths` as intent-to-add, like `git add -N`, so the content is
    /// visible to working-tree diffs without being staged. Returns the paths
    /// left alone: those already in the index and those that are no longer a
    /// regular file or symlink.
    pub fn mark_intent_to_add(&self, paths: &[String]) -> anyhow::Result<Vec<String>> {
        if paths.is_empty() {
            return Ok(Vec::new());
        }
        let workdir = self.workdir()?;
        // Git treats the empty blob as implicitly present; libgit2 needs it in
        // the object database before diffs can read the entry back.
        let empty_blob = self.repo().blob(&[])?;
        let mut index = self.repo().index()?;
        // The cached index predates the interactive prompt; reload it so
        // changes staged meanwhile are neither reverted nor overwritten.
        index.read(false)?;

        let mut skipped = Vec::new();
        for path in paths {
            let mode = Self::worktree_file_mode(&workdir.join(path));
            match mode {
                Some(mode) if index.get_path(Path::new(path), 0).is_none() => {
                    index.add(&Self::intent_to_add_entry(path, mode, empty_blob))?;
                }
                _ => skipped.push(path.clone()),
            }
        }
        index.write()?;
        Ok(skipped)
    }

    fn intent_to_add_entry(path: &str, mode: FileMode, empty_blob: git2::Oid) -> IndexEntry {
        let unset_time = IndexTime::new(0, 0);
        IndexEntry {
            ctime: unset_time,
            mtime: unset_time,
            dev: 0,
            ino: 0,
            mode: u32::from(mode),
            uid: 0,
            gid: 0,
            file_size: 0,
            id: empty_blob,
            flags: path.len().min(INDEX_ENTRY_PATH_LENGTH_MASK) as u16,
            flags_extended: IndexEntryExtendedFlag::INTENT_TO_ADD.bits(),
            path: path.as_bytes().to_vec(),
        }
    }

    /// `None` when `path` is missing or is neither a regular file nor a symlink.
    fn worktree_file_mode(path: &Path) -> Option<FileMode> {
        let metadata = std::fs::symlink_metadata(path).ok()?;
        let file_type = metadata.file_type();
        if file_type.is_symlink() {
            return Some(FileMode::Link);
        }
        if !file_type.is_file() {
            return None;
        }
        Some(if Self::is_executable(&metadata) {
            FileMode::BlobExecutable
        } else {
            FileMode::Blob
        })
    }

    #[cfg(unix)]
    fn is_executable(metadata: &std::fs::Metadata) -> bool {
        use std::os::unix::fs::PermissionsExt;
        const ANY_EXECUTE_BITS: u32 = 0o111;
        metadata.permissions().mode() & ANY_EXECUTE_BITS != 0
    }

    #[cfg(not(unix))]
    fn is_executable(_metadata: &std::fs::Metadata) -> bool {
        false
    }

    pub fn workdir(&self) -> anyhow::Result<&Path> {
        self.repo()
            .workdir()
            .ok_or_else(|| anyhow::anyhow!("repository has no working tree"))
    }

    /// Add gitignore-syntax `rules` for this handle only; nothing is written to
    /// disk and other handles on the same repository are unaffected.
    pub fn add_ignore_rules(&self, rules: &str) -> anyhow::Result<()> {
        self.repo().add_ignore_rule(rules)?;
        Ok(())
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

    /// Regenerate the raw unified diff for `source` at the requested context.
    /// Shared by startup (`main.rs`) and the on-demand `/api/v1/diff?context=`
    /// route so both honor the exact same range semantics.
    pub fn diff_for_source(&self, source: &DiffSource, context_lines: u32) -> anyhow::Result<String> {
        match source.mode.as_str() {
            "staged" => self.get_staged_diff(context_lines),
            "unstaged" => self.get_unstaged_diff(context_lines),
            "working" => self.get_working_diff(context_lines),
            "commits" => self.get_diff(&source.args[0], &source.args[1], context_lines),
            "all" => self.get_diff_from_to_workdir(&source.args[0], source.include_untracked, context_lines),
            other => anyhow::bail!("Unknown range mode: {}", other),
        }
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
        for (idx, delta) in diff.deltas().enumerate() {
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

            // Per-delta line stats. Without these the sidebar shows "+0 -0"
            // for every file in a commit range, since this list — not the
            // parsed text diff — is what feeds the file tree in that mode.
            // Binary deltas have no patch, which is a legitimate 0/0.
            let (additions, deletions) = match git2::Patch::from_diff(&diff, idx) {
                Ok(Some(patch)) => {
                    let (_context, added, removed) = patch.line_stats()?;
                    (added as u32, removed as u32)
                }
                _ => (0, 0),
            };

            files.push(FileChange {
                path,
                old_path,
                status,
                additions,
                deletions,
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
    fn file_list_carries_per_file_line_counts() {
        // Regression test: get_file_list hardcoded additions/deletions to 0, and
        // it is this list — not the parsed text diff — that feeds the sidebar
        // file tree in `commits` mode, so every file showed "+0 -0".
        let fx = fixture();
        let base = fx.base.clone();

        fs::write(fx.path.join("a.txt"), "one\nTWO\nthree\nfour\n").unwrap();
        fs::write(fx.path.join("b.txt"), "new\nfile\n").unwrap();
        let head = commit_all(&git2::Repository::open(&fx.path).unwrap(), "change").to_string();

        let files = fx.git.get_file_list(&base, &head).unwrap();

        let a = files.iter().find(|f| f.path == "a.txt").unwrap();
        assert_eq!((a.additions, a.deletions), (2, 1), "a.txt line counts");

        let b = files.iter().find(|f| f.path == "b.txt").unwrap();
        assert_eq!((b.additions, b.deletions), (2, 0), "added file counts");
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
    fn intent_to_add_file_is_a_whole_file_addition_in_unstaged_diff() {
        let fx = fixture();
        fs::write(fx.path.join("new.txt"), "alpha\nbeta\n").unwrap();
        fs::write(fx.path.join("other.txt"), "untouched\n").unwrap();
        fs::write(fx.path.join("a.txt"), "one\nTWO\nthree\n").unwrap();

        fx.git.mark_intent_to_add(&["new.txt".to_string()]).unwrap();

        let parsed = parse_diff(&fx.git.get_unstaged_diff(3).unwrap());
        let names: Vec<&str> = parsed.iter().map(|f| f.new_path.as_str()).collect();
        assert_eq!(names, vec!["a.txt", "new.txt"]);
        let added = parsed.iter().find(|f| f.new_path == "new.txt").unwrap();
        assert_eq!(added.status, FileStatus::Added);
        assert_eq!((added.additions, added.deletions), (2, 0));
    }

    fn index_entry(fx: &Fixture, path: &str) -> Option<IndexEntry> {
        git2::Repository::open(&fx.path)
            .unwrap()
            .index()
            .unwrap()
            .get_path(Path::new(path), 0)
    }

    #[cfg(unix)]
    #[test]
    fn intent_to_add_entry_keeps_the_worktree_file_mode() {
        let fx = fixture();
        fs::write(fx.path.join("plain.txt"), "x\n").unwrap();
        fs::write(fx.path.join("run.sh"), "#!/bin/sh\n").unwrap();
        fs::set_permissions(
            fx.path.join("run.sh"),
            std::os::unix::fs::PermissionsExt::from_mode(0o755),
        )
        .unwrap();
        std::os::unix::fs::symlink("plain.txt", fx.path.join("link")).unwrap();

        let paths = vec!["plain.txt".to_string(), "run.sh".to_string(), "link".to_string()];
        assert!(fx.git.mark_intent_to_add(&paths).unwrap().is_empty());

        let mode = |path| index_entry(&fx, path).unwrap().mode;
        assert_eq!(mode("plain.txt"), u32::from(FileMode::Blob));
        assert_eq!(mode("run.sh"), u32::from(FileMode::BlobExecutable));
        assert_eq!(mode("link"), u32::from(FileMode::Link));
    }

    #[test]
    fn mark_intent_to_add_keeps_a_file_staged_after_the_index_was_loaded() {
        let fx = fixture();
        fs::write(fx.path.join("new.txt"), "staged content\n").unwrap();
        assert_eq!(fx.git.list_untracked().unwrap(), vec!["new.txt".to_string()]);

        let other = git2::Repository::open(&fx.path).unwrap();
        let mut other_index = other.index().unwrap();
        other_index.add_path(Path::new("new.txt")).unwrap();
        other_index.write().unwrap();
        let staged_id = index_entry(&fx, "new.txt").unwrap().id;

        let skipped = fx.git.mark_intent_to_add(&["new.txt".to_string()]).unwrap();

        assert_eq!(skipped, vec!["new.txt".to_string()]);
        assert_eq!(index_entry(&fx, "new.txt").unwrap().id, staged_id);
    }

    #[test]
    fn mark_intent_to_add_skips_directories_and_missing_files() {
        let fx = fixture();
        fs::create_dir_all(fx.path.join("nested")).unwrap();
        git2::Repository::init(fx.path.join("nested")).unwrap();
        fs::write(fx.path.join("nested").join("inner.txt"), "x").unwrap();
        assert_eq!(fx.git.list_untracked().unwrap(), vec!["nested/".to_string()]);

        let paths = vec!["nested/".to_string(), "gone.txt".to_string()];
        let skipped = fx.git.mark_intent_to_add(&paths).unwrap();

        assert_eq!(skipped, paths);
        assert!(fx.git.intent_to_add_paths().unwrap().is_empty());
    }

    #[test]
    fn unstaged_diff_leaves_the_index_file_untouched() {
        let fx = fixture();
        fs::write(fx.path.join("new.txt"), "alpha\n").unwrap();
        fx.git.mark_intent_to_add(&["new.txt".to_string()]).unwrap();
        let index_path = fx.path.join(".git").join(INDEX_FILE_NAME);
        let before = fs::read(&index_path).unwrap();

        fx.git.get_unstaged_diff(3).unwrap();
        fx.git.get_staged_diff(3).unwrap();

        assert_eq!(fs::read(&index_path).unwrap(), before);
    }

    #[test]
    fn staged_diff_keeps_real_staged_files_next_to_intent_to_add_ones() {
        let fx = fixture();
        fs::write(fx.path.join("new.txt"), "alpha\n").unwrap();
        fs::write(fx.path.join("staged.txt"), "s\n").unwrap();
        let repo = git2::Repository::open(&fx.path).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("staged.txt")).unwrap();
        index.write().unwrap();
        fx.git.mark_intent_to_add(&["new.txt".to_string()]).unwrap();

        let parsed = parse_diff(&fx.git.get_staged_diff(3).unwrap());

        let names: Vec<&str> = parsed.iter().map(|f| f.new_path.as_str()).collect();
        assert_eq!(names, vec!["staged.txt"]);
    }

    #[test]
    fn intent_to_add_file_is_no_longer_untracked() {
        let fx = fixture();
        fs::write(fx.path.join("new.txt"), "alpha\n").unwrap();
        fx.git.mark_intent_to_add(&["new.txt".to_string()]).unwrap();
        assert!(fx.git.list_untracked().unwrap().is_empty());
    }

    #[test]
    fn intent_to_add_file_is_excluded_from_staged_diff() {
        let fx = fixture();
        fs::write(fx.path.join("new.txt"), "alpha\n").unwrap();
        fx.git.mark_intent_to_add(&["new.txt".to_string()]).unwrap();
        assert!(parse_diff(&fx.git.get_staged_diff(3).unwrap()).is_empty());
    }

    #[test]
    fn intent_to_add_file_is_an_addition_in_working_diff() {
        let fx = fixture();
        fs::write(fx.path.join("new.txt"), "alpha\nbeta\n").unwrap();
        fx.git.mark_intent_to_add(&["new.txt".to_string()]).unwrap();

        let parsed = parse_diff(&fx.git.get_working_diff(3).unwrap());
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].status, FileStatus::Added);
        assert_eq!(parsed[0].additions, 2);
    }

    #[test]
    fn ignore_rules_hide_untracked_files_for_this_handle_only() {
        let fx = fixture();
        fs::write(fx.path.join("keep.txt"), "x").unwrap();
        fs::write(fx.path.join("skip.txt"), "x").unwrap();

        fx.git.add_ignore_rules("/skip.txt").unwrap();

        assert_eq!(fx.git.list_untracked().unwrap(), vec!["keep.txt".to_string()]);
        let fresh = GitModule::new(fx.path.to_str().unwrap()).unwrap();
        assert_eq!(fresh.list_untracked().unwrap().len(), 2);
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
