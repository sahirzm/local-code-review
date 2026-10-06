//! Offers untracked files for a working-tree review. Each file is either
//! marked intent-to-add (`git add -N`) so working-tree diffs show it, or
//! recorded in the ignore list so later runs stop offering it.

use std::fs::OpenOptions;
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};

use crate::git::GitModule;
use crate::session::STATE_DIR;
use crate::types::CliOptions;

/// Gitignore-syntax list of untracked files that are never offered.
const IGNORE_FILE_NAME: &str = "ignore";
const GITIGNORE_ROOT_ANCHOR: &str = "/";
const GITIGNORE_ESCAPE: char = '\\';
const GITIGNORE_GLOB_CHARS: [char; 4] = [GITIGNORE_ESCAPE, '*', '?', '['];
const GITIGNORE_LINE_BREAKS: [char; 2] = ['\n', '\r'];
const TRAILING_SPACE: char = ' ';
/// libgit2 reports an untracked directory it does not descend into, such as a
/// nested repository, as one entry ending in `/`.
const DIRECTORY_SUFFIX: char = '/';

const YES: &str = "y";
const YES_WORD: &str = "yes";
const NO: &str = "n";
const NO_WORD: &str = "no";
const ALL: &str = "a";
const ALL_WORD: &str = "all";
const QUIT: &str = "q";
const QUIT_WORD: &str = "quit";

/// Only working-tree modes benefit: they read the index, so an intent-to-add
/// entry is the one thing that makes a new file show up.
pub fn applies_to(options: &CliOptions) -> bool {
    options.unstaged || options.working
}

pub fn offer_untracked_files(git: &GitModule) -> anyhow::Result<()> {
    let ignore_list = IgnoreList::in_repo(git.workdir()?);
    let candidates = review_candidates(git, &ignore_list)?;
    if candidates.is_empty() {
        return Ok(());
    }
    if !io::stdin().is_terminal() {
        eprintln!(
            "{} untracked file(s) not offered for review (stdin is not a TTY)",
            candidates.len()
        );
        return Ok(());
    }

    eprintln!("\n{} untracked file(s) are not part of this review:", candidates.len());
    let outcome = UntrackedTriage::new(io::stdin().lock(), io::stderr()).run(&candidates)?;
    let unrecordable = ignore_list.append(&outcome.ignore)?;
    let skipped = git.mark_intent_to_add(&outcome.include)?;
    report(&outcome, &ignore_list, &unrecordable, &skipped);
    Ok(())
}

/// Untracked files minus local-review's own state directory (never under
/// review, even when the repository does not gitignore it), the ignore list,
/// and directory entries, which cannot be marked intent-to-add.
fn review_candidates(git: &GitModule, ignore_list: &IgnoreList) -> anyhow::Result<Vec<String>> {
    git.add_ignore_rules(&state_dir_rule())?;
    git.add_ignore_rules(&ignore_list.rules()?)?;
    let mut candidates = git.list_untracked()?;
    candidates.retain(|path| !path.ends_with(DIRECTORY_SUFFIX));
    Ok(candidates)
}

fn state_dir_rule() -> String {
    format!("{}{}{}", GITIGNORE_ROOT_ANCHOR, STATE_DIR, DIRECTORY_SUFFIX)
}

fn report(outcome: &TriageOutcome, ignore_list: &IgnoreList, unrecordable: &[String], skipped: &[String]) {
    let marked = outcome.include.len() - skipped.len();
    if marked > 0 {
        eprintln!("Marked {} file(s) intent-to-add (git add -N)", marked);
    }
    for path in skipped {
        eprintln!("Skipped {}: already in the index or no longer a file", path);
    }
    let ignored = outcome.ignore.len() - unrecordable.len();
    if ignored > 0 {
        eprintln!("Ignored {} file(s) in {}", ignored, ignore_list.path.display());
    }
    for path in unrecordable {
        eprintln!("Not ignored {:?}: a line break cannot be written as an ignore rule", path);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Answer {
    Include,
    Ignore,
    IncludeRemaining,
    Quit,
}

impl Answer {
    fn parse(input: &str) -> Option<Self> {
        match input.trim().to_ascii_lowercase().as_str() {
            YES | YES_WORD => Some(Self::Include),
            NO | NO_WORD => Some(Self::Ignore),
            ALL | ALL_WORD => Some(Self::IncludeRemaining),
            QUIT | QUIT_WORD => Some(Self::Quit),
            _ => None,
        }
    }

    fn choices() -> String {
        format!("[{YES}]es / [{NO}]o, ignore / [{ALL}]ll remaining / [{QUIT}]uit")
    }

    fn retry_hint() -> String {
        format!("Please answer {YES}, {NO}, {ALL} or {QUIT}.")
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct TriageOutcome {
    pub include: Vec<String>,
    pub ignore: Vec<String>,
}

/// Asks about each candidate in turn. Files left unanswered after `quit` or
/// end of input are neither included nor ignored, so the next run asks again.
pub struct UntrackedTriage<R, W> {
    input: R,
    prompt: W,
}

impl<R: BufRead, W: Write> UntrackedTriage<R, W> {
    pub fn new(input: R, prompt: W) -> Self {
        Self { input, prompt }
    }

    pub fn run(&mut self, candidates: &[String]) -> io::Result<TriageOutcome> {
        let mut outcome = TriageOutcome::default();
        for (position, path) in candidates.iter().enumerate() {
            match self.ask(path, position, candidates.len())? {
                Answer::Include => outcome.include.push(path.clone()),
                Answer::Ignore => outcome.ignore.push(path.clone()),
                Answer::IncludeRemaining => {
                    outcome.include.extend_from_slice(&candidates[position..]);
                    break;
                }
                Answer::Quit => break,
            }
        }
        Ok(outcome)
    }

    fn ask(&mut self, path: &str, position: usize, total: usize) -> io::Result<Answer> {
        loop {
            write!(
                self.prompt,
                "[{}/{}] Include {} in the review? {} ",
                position + 1,
                total,
                path,
                Answer::choices()
            )?;
            self.prompt.flush()?;

            let mut line = String::new();
            if self.input.read_line(&mut line)? == 0 {
                writeln!(self.prompt)?;
                return Ok(Answer::Quit);
            }
            if let Some(answer) = Answer::parse(&line) {
                return Ok(answer);
            }
            writeln!(self.prompt, "{}", Answer::retry_hint())?;
        }
    }
}

/// `.local-review/ignore`, in gitignore syntax. Declined files are appended as
/// anchored exact-path rules; hand-written patterns such as `*.log` also work.
pub struct IgnoreList {
    path: PathBuf,
}

impl IgnoreList {
    pub fn in_repo(workdir: &Path) -> Self {
        Self {
            path: workdir.join(STATE_DIR).join(IGNORE_FILE_NAME),
        }
    }

    pub fn rules(&self) -> io::Result<String> {
        match std::fs::read_to_string(&self.path) {
            Ok(rules) => Ok(rules),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(String::new()),
            Err(e) => Err(e),
        }
    }

    /// Returns the paths that were not written: a line break would split the
    /// rule and inject arbitrary patterns.
    pub fn append(&self, paths: &[String]) -> io::Result<Vec<String>> {
        let (unrecordable, recordable): (Vec<String>, Vec<String>) = paths
            .iter()
            .cloned()
            .partition(|path| path.contains(GITIGNORE_LINE_BREAKS));
        if recordable.is_empty() {
            return Ok(unrecordable);
        }
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut file = OpenOptions::new().create(true).append(true).open(&self.path)?;
        for path in &recordable {
            writeln!(file, "{}", exact_path_rule(path))?;
        }
        Ok(unrecordable)
    }
}

/// A gitignore rule matching only `path`: the root anchor ties it to the
/// repository root (which also neutralises a leading `#` or `!`), glob
/// characters are escaped, and trailing spaces are escaped so git keeps them.
fn exact_path_rule(path: &str) -> String {
    let trimmed = path.trim_end_matches(TRAILING_SPACE);
    let trailing_spaces = path.len() - trimmed.len();

    let mut rule = String::from(GITIGNORE_ROOT_ANCHOR);
    for c in trimmed.chars() {
        if GITIGNORE_GLOB_CHARS.contains(&c) {
            rule.push(GITIGNORE_ESCAPE);
        }
        rule.push(c);
    }
    for _ in 0..trailing_spaces {
        rule.push(GITIGNORE_ESCAPE);
        rule.push(TRAILING_SPACE);
    }
    rule
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn triage(answers: &str, candidates: &[&str]) -> TriageOutcome {
        UntrackedTriage::new(answers.as_bytes(), Vec::new())
            .run(&strings(candidates))
            .unwrap()
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| v.to_string()).collect()
    }

    #[test]
    fn yes_includes_and_no_ignores() {
        let outcome = triage("y\nn\nYES\n", &["a", "b", "c"]);
        assert_eq!(outcome.include, strings(&["a", "c"]));
        assert_eq!(outcome.ignore, strings(&["b"]));
    }

    #[test]
    fn all_includes_the_current_and_remaining_files() {
        let outcome = triage("n\na\n", &["a", "b", "c"]);
        assert_eq!(outcome.include, strings(&["b", "c"]));
        assert_eq!(outcome.ignore, strings(&["a"]));
    }

    #[test]
    fn quit_leaves_remaining_files_undecided() {
        let outcome = triage("y\nq\n", &["a", "b", "c"]);
        assert_eq!(outcome.include, strings(&["a"]));
        assert!(outcome.ignore.is_empty());
    }

    #[test]
    fn end_of_input_leaves_remaining_files_undecided() {
        let outcome = triage("n\n", &["a", "b"]);
        assert!(outcome.include.is_empty());
        assert_eq!(outcome.ignore, strings(&["a"]));
    }

    #[test]
    fn unrecognised_answer_asks_again() {
        let mut prompt = Vec::new();
        let outcome = UntrackedTriage::new("maybe\n\ny\n".as_bytes(), &mut prompt)
            .run(&strings(&["a"]))
            .unwrap();
        assert_eq!(outcome.include, strings(&["a"]));
        let prompt = String::from_utf8(prompt).unwrap();
        assert_eq!(prompt.matches("Include a in the review?").count(), 3);
        assert_eq!(prompt.matches(&Answer::retry_hint()).count(), 2);
    }

    #[test]
    fn exact_path_rule_anchors_and_escapes() {
        assert_eq!(exact_path_rule("src/main.rs"), "/src/main.rs");
        assert_eq!(exact_path_rule("#notes"), "/#notes");
        assert_eq!(exact_path_rule("a*b?[c]\\d"), "/a\\*b\\?\\[c]\\\\d");
        assert_eq!(exact_path_rule("trailing  "), "/trailing\\ \\ ");
    }

    #[test]
    fn ignore_list_round_trips_appended_paths() {
        let dir = tempfile::tempdir().unwrap();
        let list = IgnoreList::in_repo(dir.path());
        assert_eq!(list.rules().unwrap(), "");

        list.append(&strings(&["one.txt"])).unwrap();
        list.append(&strings(&["dir/two*.txt"])).unwrap();

        assert_eq!(list.rules().unwrap(), "/one.txt\n/dir/two\\*.txt\n");
    }

    #[test]
    fn ignore_list_refuses_paths_with_line_breaks() {
        let dir = tempfile::tempdir().unwrap();
        let list = IgnoreList::in_repo(dir.path());

        let unrecordable = list.append(&strings(&["x\n*", "y\r", "ok.txt"])).unwrap();

        assert_eq!(unrecordable, strings(&["x\n*", "y\r"]));
        assert_eq!(list.rules().unwrap(), "/ok.txt\n");
    }

    #[test]
    fn review_candidates_exclude_ignored_state_and_directory_entries() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        fs::write(dir.path().join("keep.txt"), "x").unwrap();
        fs::write(dir.path().join("skip*.txt"), "x").unwrap();
        fs::write(dir.path().join("skipped.txt"), "x").unwrap();
        fs::create_dir_all(dir.path().join(STATE_DIR)).unwrap();
        fs::write(dir.path().join(STATE_DIR).join("review.md"), "x").unwrap();
        fs::create_dir_all(dir.path().join("nested")).unwrap();
        git2::Repository::init(dir.path().join("nested")).unwrap();
        fs::write(dir.path().join("nested").join("inner.txt"), "x").unwrap();

        let list = IgnoreList::in_repo(dir.path());
        list.append(&strings(&["skip*.txt"])).unwrap();
        let git = GitModule::new(dir.path().to_str().unwrap()).unwrap();

        assert_eq!(
            review_candidates(&git, &list).unwrap(),
            strings(&["keep.txt", "skipped.txt"])
        );
    }
}
