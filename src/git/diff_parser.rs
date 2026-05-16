use regex::Regex;
use std::sync::LazyLock;

use crate::types::{Change, ChangeType, FileStatus, Hunk, ParsedFileDiff};

static DIFF_HEADER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^diff --git a/(.*) b/(.*)$").unwrap());
static HUNK_HEADER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@(.*)$").unwrap());
static RENAME_FROM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^rename from (.+)$").unwrap());
static RENAME_TO: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^rename to (.+)$").unwrap());

fn detect_status(header_lines: &[&str], old_path: &str, new_path: &str) -> FileStatus {
    for line in header_lines {
        if line.starts_with("new file mode") {
            return FileStatus::Added;
        }
        if line.starts_with("deleted file mode") {
            return FileStatus::Deleted;
        }
        if line.starts_with("rename from") {
            return FileStatus::Renamed;
        }
        if line.starts_with("copy from") {
            return FileStatus::Copied;
        }
    }
    if old_path == "/dev/null" {
        return FileStatus::Added;
    }
    if new_path == "/dev/null" {
        return FileStatus::Deleted;
    }
    FileStatus::Modified
}

fn is_binary_diff(lines: &[&str]) -> bool {
    for line in lines {
        if line.starts_with("Binary files") || line.starts_with("GIT binary patch") {
            return true;
        }
    }
    false
}

pub fn parse_diff(raw: &str) -> Vec<ParsedFileDiff> {
    if raw.trim().is_empty() {
        return vec![];
    }

    let lines: Vec<&str> = raw.lines().collect();
    let mut files: Vec<ParsedFileDiff> = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let header_caps = match DIFF_HEADER.captures(lines[i]) {
            Some(c) => c,
            None => {
                i += 1;
                continue;
            }
        };

        let mut old_path = header_caps.get(1).unwrap().as_str().to_string();
        let mut new_path = header_caps.get(2).unwrap().as_str().to_string();
        i += 1;

        let mut header_lines: Vec<&str> = Vec::new();
        while i < lines.len()
            && !DIFF_HEADER.is_match(lines[i])
            && !HUNK_HEADER.is_match(lines[i])
            && !lines[i].starts_with("Binary files")
            && !lines[i].starts_with("GIT binary patch")
        {
            if let Some(caps) = RENAME_FROM.captures(lines[i]) {
                old_path = caps.get(1).unwrap().as_str().to_string();
            }
            if let Some(caps) = RENAME_TO.captures(lines[i]) {
                new_path = caps.get(1).unwrap().as_str().to_string();
            }
            if lines[i].starts_with("--- ") {
                let p = &lines[i][4..];
                if p == "/dev/null" {
                    old_path = "/dev/null".to_string();
                } else if let Some(stripped) = p.strip_prefix("a/") {
                    old_path = stripped.to_string();
                }
            }
            if lines[i].starts_with("+++ ") {
                let p = &lines[i][4..];
                if p == "/dev/null" {
                    new_path = "/dev/null".to_string();
                } else if let Some(stripped) = p.strip_prefix("b/") {
                    new_path = stripped.to_string();
                }
            }
            header_lines.push(lines[i]);
            i += 1;
        }

        let mut body_lines: Vec<&str> = Vec::new();
        if i < lines.len()
            && (lines[i].starts_with("Binary files") || lines[i].starts_with("GIT binary patch"))
        {
            body_lines.push(lines[i]);
            i += 1;
        }

        let status = detect_status(&header_lines, &old_path, &new_path);
        let binary = is_binary_diff(&header_lines)
            || (body_lines.first().map_or(false, |l| {
                l.starts_with("Binary files") || l.starts_with("GIT binary patch")
            }));

        let mut hunks: Vec<Hunk> = Vec::new();
        let mut additions: u32 = 0;
        let mut deletions: u32 = 0;

        while i < lines.len() && !DIFF_HEADER.is_match(lines[i]) {
            let hunk_caps = match HUNK_HEADER.captures(lines[i]) {
                Some(c) => c,
                None => {
                    i += 1;
                    continue;
                }
            };

            let old_start: u32 = hunk_caps.get(1).unwrap().as_str().parse().unwrap();
            let old_lines: u32 = hunk_caps
                .get(2)
                .map_or(1, |m| m.as_str().parse().unwrap());
            let new_start: u32 = hunk_caps.get(3).unwrap().as_str().parse().unwrap();
            let new_lines: u32 = hunk_caps
                .get(4)
                .map_or(1, |m| m.as_str().parse().unwrap());
            let content = hunk_caps
                .get(5)
                .map_or(String::new(), |m| m.as_str().to_string());

            let mut hunk = Hunk {
                old_start,
                old_lines,
                new_start,
                new_lines,
                content,
                changes: Vec::new(),
            };
            i += 1;

            let mut old_line = hunk.old_start;
            let mut new_line = hunk.new_start;

            while i < lines.len()
                && !DIFF_HEADER.is_match(lines[i])
                && !HUNK_HEADER.is_match(lines[i])
            {
                let line = lines[i];

                if line == "\\ No newline at end of file" {
                    i += 1;
                    continue;
                }

                if let Some(rest) = line.strip_prefix('+') {
                    let change = Change {
                        change_type: ChangeType::Insert,
                        old_line_number: None,
                        new_line_number: Some(new_line),
                        content: rest.to_string(),
                    };
                    new_line += 1;
                    hunk.changes.push(change);
                    additions += 1;
                } else if let Some(rest) = line.strip_prefix('-') {
                    let change = Change {
                        change_type: ChangeType::Delete,
                        old_line_number: Some(old_line),
                        new_line_number: None,
                        content: rest.to_string(),
                    };
                    old_line += 1;
                    hunk.changes.push(change);
                    deletions += 1;
                } else if let Some(rest) = line.strip_prefix(' ') {
                    let change = Change {
                        change_type: ChangeType::Normal,
                        old_line_number: Some(old_line),
                        new_line_number: Some(new_line),
                        content: rest.to_string(),
                    };
                    old_line += 1;
                    new_line += 1;
                    hunk.changes.push(change);
                } else {
                    break;
                }
                i += 1;
            }

            hunks.push(hunk);
        }

        let total_changes = additions + deletions;

        let transformed_old = if old_path == "/dev/null" { new_path.clone() } else { old_path.clone() };
        let transformed_new = if new_path == "/dev/null" { old_path } else { new_path };
        files.push(ParsedFileDiff {
            old_path: transformed_old,
            new_path: transformed_new,
            hunks,
            status,
            additions,
            deletions,
            is_binary: binary,
            is_large: total_changes > 10000,
        });
    }

    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_empty_for_empty_input() {
        assert_eq!(parse_diff(""), vec![]);
        assert_eq!(parse_diff("  \n  "), vec![]);
    }

    #[test]
    fn parses_simple_file_modification() {
        let raw = "\
diff --git a/src/app.ts b/src/app.ts
index abc1234..def5678 100644
--- a/src/app.ts
+++ b/src/app.ts
@@ -1,4 +1,4 @@
 import express from 'express';
-const port = 3000;
+const port = 8080;
 const app = express();
";
        let files = parse_diff(raw);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, FileStatus::Modified);
        assert_eq!(files[0].old_path, "src/app.ts");
        assert_eq!(files[0].new_path, "src/app.ts");
        assert_eq!(files[0].additions, 1);
        assert_eq!(files[0].deletions, 1);
        assert!(!files[0].is_binary);
        assert!(!files[0].is_large);
        assert_eq!(files[0].hunks.len(), 1);
        assert_eq!(files[0].hunks[0].old_start, 1);
        assert_eq!(files[0].hunks[0].old_lines, 4);
        assert_eq!(files[0].hunks[0].new_start, 1);
        assert_eq!(files[0].hunks[0].new_lines, 4);
    }

    #[test]
    fn parses_new_file() {
        let raw = "\
diff --git a/newfile.ts b/newfile.ts
new file mode 100644
index 0000000..abc1234
--- /dev/null
+++ b/newfile.ts
@@ -0,0 +1,3 @@
+line 1
+line 2
+line 3
";
        let files = parse_diff(raw);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, FileStatus::Added);
        assert_eq!(files[0].additions, 3);
        assert_eq!(files[0].deletions, 0);
        assert_eq!(files[0].old_path, "newfile.ts");
        assert_eq!(files[0].new_path, "newfile.ts");
    }

    #[test]
    fn parses_deleted_file() {
        let raw = "\
diff --git a/old.ts b/old.ts
deleted file mode 100644
index abc1234..0000000
--- a/old.ts
+++ /dev/null
@@ -1,2 +0,0 @@
-line 1
-line 2
";
        let files = parse_diff(raw);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, FileStatus::Deleted);
        assert_eq!(files[0].additions, 0);
        assert_eq!(files[0].deletions, 2);
        assert_eq!(files[0].old_path, "old.ts");
        assert_eq!(files[0].new_path, "old.ts");
    }

    #[test]
    fn parses_renamed_file_with_similarity() {
        let raw = "\
diff --git a/old-name.ts b/new-name.ts
similarity index 95%
rename from old-name.ts
rename to new-name.ts
index abc1234..def5678 100644
--- a/old-name.ts
+++ b/new-name.ts
@@ -1,3 +1,3 @@
 const a = 1;
-const b = 2;
+const b = 3;
 const c = 3;
";
        let files = parse_diff(raw);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, FileStatus::Renamed);
        assert_eq!(files[0].old_path, "old-name.ts");
        assert_eq!(files[0].new_path, "new-name.ts");
        assert_eq!(files[0].additions, 1);
        assert_eq!(files[0].deletions, 1);
    }

    #[test]
    fn parses_binary_file() {
        let raw = "\
diff --git a/image.png b/image.png
index abc1234..def5678 100644
Binary files a/image.png and b/image.png differ
";
        let files = parse_diff(raw);
        assert_eq!(files.len(), 1);
        assert!(files[0].is_binary);
        assert_eq!(files[0].status, FileStatus::Modified);
        assert_eq!(files[0].hunks.len(), 0);
    }

    #[test]
    fn parses_multiple_files() {
        let raw = "\
diff --git a/file1.ts b/file1.ts
index abc1234..def5678 100644
--- a/file1.ts
+++ b/file1.ts
@@ -1,2 +1,2 @@
-old line
+new line
 context
diff --git a/file2.ts b/file2.ts
new file mode 100644
index 0000000..abc1234
--- /dev/null
+++ b/file2.ts
@@ -0,0 +1,1 @@
+added
";
        let files = parse_diff(raw);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].old_path, "file1.ts");
        assert_eq!(files[0].status, FileStatus::Modified);
        assert_eq!(files[1].new_path, "file2.ts");
        assert_eq!(files[1].status, FileStatus::Added);
    }

    #[test]
    fn flags_large_files() {
        let insert_lines: Vec<String> = (0..10001).map(|i| format!("+line {}", i)).collect();
        let raw = format!(
            "diff --git a/big.ts b/big.ts
new file mode 100644
index 0000000..abc1234
--- /dev/null
+++ b/big.ts
@@ -0,0 +1,10001 @@
{}",
            insert_lines.join("\n")
        );
        let files = parse_diff(&raw);
        assert_eq!(files.len(), 1);
        assert!(files[0].is_large);
        assert_eq!(files[0].additions, 10001);
    }

    #[test]
    fn handles_no_newline_marker() {
        let raw = "\
diff --git a/file.ts b/file.ts
index abc1234..def5678 100644
--- a/file.ts
+++ b/file.ts
@@ -1,2 +1,2 @@
-old content
\\ No newline at end of file
+new content
\\ No newline at end of file
";
        let files = parse_diff(raw);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].hunks[0].changes.len(), 2);
        assert!(files[0].hunks[0]
            .changes
            .iter()
            .all(|c| !c.content.contains("No newline")));
    }

    #[test]
    fn tracks_correct_line_numbers() {
        let raw = "\
diff --git a/file.ts b/file.ts
index abc1234..def5678 100644
--- a/file.ts
+++ b/file.ts
@@ -5,4 +5,5 @@
 context line 5
-deleted line 6
+inserted line 6a
+inserted line 6b
 context line 7
 context line 8
";
        let files = parse_diff(raw);
        let changes = &files[0].hunks[0].changes;

        assert_eq!(changes[0].change_type, ChangeType::Normal);
        assert_eq!(changes[0].old_line_number, Some(5));
        assert_eq!(changes[0].new_line_number, Some(5));
        assert_eq!(changes[0].content, "context line 5");

        assert_eq!(changes[1].change_type, ChangeType::Delete);
        assert_eq!(changes[1].old_line_number, Some(6));
        assert_eq!(changes[1].content, "deleted line 6");

        assert_eq!(changes[2].change_type, ChangeType::Insert);
        assert_eq!(changes[2].new_line_number, Some(6));
        assert_eq!(changes[2].content, "inserted line 6a");

        assert_eq!(changes[3].change_type, ChangeType::Insert);
        assert_eq!(changes[3].new_line_number, Some(7));
        assert_eq!(changes[3].content, "inserted line 6b");

        assert_eq!(changes[4].change_type, ChangeType::Normal);
        assert_eq!(changes[4].old_line_number, Some(7));
        assert_eq!(changes[4].new_line_number, Some(8));
        assert_eq!(changes[4].content, "context line 7");

        assert_eq!(changes[5].change_type, ChangeType::Normal);
        assert_eq!(changes[5].old_line_number, Some(8));
        assert_eq!(changes[5].new_line_number, Some(9));
        assert_eq!(changes[5].content, "context line 8");
    }

    #[test]
    fn handles_git_binary_patch_format() {
        let raw = "\
diff --git a/data.bin b/data.bin
index abc1234..def5678 100644
GIT binary patch
literal 1234
zcmV;@1234abcdef
";
        let files = parse_diff(raw);
        assert_eq!(files.len(), 1);
        assert!(files[0].is_binary);
    }

    #[test]
    fn parses_copied_file() {
        let raw = "\
diff --git a/original.ts b/copy.ts
similarity index 100%
copy from original.ts
copy to copy.ts
";
        let files = parse_diff(raw);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, FileStatus::Copied);
        assert_eq!(files[0].old_path, "original.ts");
        assert_eq!(files[0].new_path, "copy.ts");
    }

    #[test]
    fn parses_file_mode_changes() {
        let raw = "\
diff --git a/script.sh b/script.sh
old mode 100644
new mode 100755
";
        let files = parse_diff(raw);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, FileStatus::Modified);
        assert_eq!(files[0].hunks.len(), 0);
    }

    #[test]
    fn handles_very_long_file_paths() {
        let long_dir = "a/".repeat(50);
        let long_path = format!("{}file.ts", long_dir);
        let raw = format!(
            "diff --git a/{0} b/{0}
index abc1234..def5678 100644
--- a/{0}
+++ b/{0}
@@ -1,1 +1,1 @@
-old
+new
",
            long_path
        );
        let files = parse_diff(&raw);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].new_path, long_path);
    }

    #[test]
    fn handles_special_characters_in_paths() {
        let raw = "\
diff --git a/src/my file (1).ts b/src/my file (1).ts
index abc1234..def5678 100644
--- a/src/my file (1).ts
+++ b/src/my file (1).ts
@@ -1,1 +1,1 @@
-old
+new
";
        let files = parse_diff(raw);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].new_path, "src/my file (1).ts");
    }
}
