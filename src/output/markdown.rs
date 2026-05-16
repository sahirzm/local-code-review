use std::collections::HashMap;

use crate::types::{Comment, CommentCategory, CommentType, DiffResponse, ReviewMetadata};

const EXT_LANG: &[(&str, &str)] = &[
    (".ts", "typescript"),
    (".tsx", "typescript"),
    (".js", "javascript"),
    (".jsx", "javascript"),
    (".py", "python"),
    (".go", "go"),
    (".rs", "rust"),
    (".java", "java"),
    (".rb", "ruby"),
    (".css", "css"),
    (".html", "html"),
    (".json", "json"),
    (".md", "markdown"),
    (".sh", "bash"),
    (".yml", "yaml"),
    (".yaml", "yaml"),
];

pub fn detect_language(file_path: &str) -> &'static str {
    if let Some(dot) = file_path.rfind('.') {
        let ext = &file_path[dot..];
        for &(e, lang) in EXT_LANG {
            if e == ext {
                return lang;
            }
        }
    }
    ""
}

fn extract_code_context(
    file_path: &str,
    start_line: u32,
    end_line: u32,
    side: Option<&str>,
    diff_data: &DiffResponse,
) -> Option<String> {
    let file = diff_data
        .files
        .iter()
        .find(|f| f.new_path == file_path || f.old_path == file_path)?;
    let use_side = side.unwrap_or("new");
    let mut lines: Vec<&str> = Vec::new();

    for hunk in &file.hunks {
        for change in &hunk.changes {
            let line_num = match use_side {
                "new" => change.new_line_number,
                "old" => change.old_line_number,
                _ => change.new_line_number,
            };
            let Some(line_num) = line_num else { continue };
            if line_num >= start_line && line_num <= end_line {
                if use_side == "new" && change.change_type == crate::types::ChangeType::Delete {
                    continue;
                }
                if use_side == "old" && change.change_type == crate::types::ChangeType::Insert {
                    continue;
                }
                lines.push(&change.content);
            }
        }
    }

    if lines.is_empty() {
        return None;
    }
    let lang = detect_language(file_path);
    Some(format!("```{}\n{}\n```", lang, lines.join("\n")))
}

fn format_comment(c: &Comment) -> String {
    format!("- [{}] {}", match c.category {
        crate::types::CommentCategory::Fix => "fix",
        crate::types::CommentCategory::Question => "question",
        crate::types::CommentCategory::Suggestion => "suggestion",
        crate::types::CommentCategory::Nit => "nit",
    }, c.text)
}

pub struct MarkdownInput {
    pub comments: Vec<Comment>,
    pub diff_data: DiffResponse,
    pub metadata: ReviewMetadata,
}

pub fn generate_markdown(input: &MarkdownInput) -> String {
    if input.comments.is_empty() {
        return "# Code Review Comments\n\nNo comments.\n".to_string();
    }

    let mut parts: Vec<String> = vec!["# Code Review Comments".to_string(), String::new()];

    let overall: Vec<&Comment> = input
        .comments
        .iter()
        .filter(|c| c.comment_type == crate::types::CommentType::Overall)
        .collect();

    let mut by_file: HashMap<String, Vec<&Comment>> = HashMap::new();
    for c in &input.comments {
        if c.comment_type == crate::types::CommentType::Overall {
            continue;
        }
        let key = c.file_path.clone().unwrap_or_default();
        by_file.entry(key).or_default().push(c);
    }

    if !overall.is_empty() {
        parts.push("## Overall".to_string());
        parts.push(String::new());
        for c in &overall {
            parts.push(format_comment(c));
        }
        parts.push(String::new());
    }

    let mut file_keys: Vec<String> = by_file.keys().cloned().collect();
    file_keys.sort();

    for file_path in &file_keys {
        let file_comments = &by_file[file_path];
        parts.push(format!("## {}", file_path));
        parts.push(String::new());

        let file_level: Vec<_> = file_comments
            .iter()
            .filter(|c| c.comment_type == crate::types::CommentType::File)
            .collect();

        let mut line_comments: Vec<_> = file_comments
            .iter()
            .filter(|c| {
                c.comment_type == crate::types::CommentType::Line
                    || c.comment_type == crate::types::CommentType::Range
            })
            .collect();
        line_comments.sort_by_key(|c| c.start_line.unwrap_or(0));

        if !file_level.is_empty() {
            parts.push("### File-level".to_string());
            parts.push(String::new());
            for c in &file_level {
                parts.push(format_comment(c));
            }
            parts.push(String::new());
        }

        for c in &line_comments {
            let start = c.start_line.unwrap_or(0);
            let end = c.end_line.unwrap_or(start);

            if c.comment_type == crate::types::CommentType::Range && end > start {
                parts.push(format!("### Lines {}-{}", start, end));
            } else {
                parts.push(format!("### Line {}", start));
            }
            parts.push(String::new());

            if let Some(code_block) = extract_code_context(
                file_path,
                start,
                end,
                c.side.as_deref(),
                &input.diff_data,
            ) {
                parts.push(code_block);
            }

            parts.push(format_comment(c));
            parts.push(String::new());
        }
    }

    let result = parts.join("\n");
    let result = result.replace("\n\n\n", "\n\n");
    let result = result.trim_end().to_string();
    result + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    fn make_comment(overrides: Comment) -> Comment {
        Comment {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: "2026-05-01T12:00:00Z".into(),
            updated_at: "2026-05-01T12:00:00Z".into(),
            ..overrides
        }
    }

    fn base_meta() -> ReviewMetadata {
        ReviewMetadata {
            repo_name: "test-repo".into(),
            commit_range: "abc..def".into(),
            base_ref: "abc".into(),
            head_ref: "def".into(),
            files: vec![],
            timestamp: "2026-05-01T12:00:00Z".into(),
            csrf_token: "tok".into(),
        }
    }

    fn empty_diff() -> DiffResponse {
        DiffResponse { files: vec![] }
    }

    fn diff_with_files() -> DiffResponse {
        DiffResponse {
            files: vec![
                ParsedFileDiff {
                    old_path: "src/server.ts".into(),
                    new_path: "src/server.ts".into(),
                    status: FileStatus::Modified,
                    additions: 4,
                    deletions: 2,
                    is_binary: false,
                    is_large: false,
                    hunks: vec![
                        Hunk {
                            old_start: 40,
                            old_lines: 10,
                            new_start: 40,
                            new_lines: 12,
                            content: "@@ -40,10 +40,12 @@".into(),
                            changes: vec![
                                Change { change_type: ChangeType::Normal, old_line_number: Some(40), new_line_number: Some(40), content: "import express from \"express\";".into() },
                                Change { change_type: ChangeType::Normal, old_line_number: Some(41), new_line_number: Some(41), content: "".into() },
                                Change { change_type: ChangeType::Delete, old_line_number: Some(42), new_line_number: None, content: "const result = fetch(url);".into() },
                                Change { change_type: ChangeType::Delete, old_line_number: Some(43), new_line_number: None, content: "const data = result.json();".into() },
                                Change { change_type: ChangeType::Insert, old_line_number: None, new_line_number: Some(42), content: "const result = await fetch(url);".into() },
                                Change { change_type: ChangeType::Insert, old_line_number: None, new_line_number: Some(43), content: "const data = result.json();".into() },
                                Change { change_type: ChangeType::Insert, old_line_number: None, new_line_number: Some(44), content: "const parsed = await data;".into() },
                                Change { change_type: ChangeType::Insert, old_line_number: None, new_line_number: Some(45), content: "return parsed;".into() },
                                Change { change_type: ChangeType::Normal, old_line_number: Some(44), new_line_number: Some(46), content: "}".into() },
                            ],
                        },
                        Hunk {
                            old_start: 75,
                            old_lines: 5,
                            new_start: 77,
                            new_lines: 5,
                            content: "@@ -75,5 +77,5 @@".into(),
                            changes: vec![
                                Change { change_type: ChangeType::Normal, old_line_number: Some(75), new_line_number: Some(77), content: "const a = 1;".into() },
                                Change { change_type: ChangeType::Delete, old_line_number: Some(76), new_line_number: None, content: "import { unused } from \"mod\";".into() },
                                Change { change_type: ChangeType::Insert, old_line_number: None, new_line_number: Some(78), content: "// cleaned up".into() },
                                Change { change_type: ChangeType::Normal, old_line_number: Some(77), new_line_number: Some(79), content: "const b = 2;".into() },
                            ],
                        },
                    ],
                },
                ParsedFileDiff {
                    old_path: "src/utils.py".into(),
                    new_path: "src/utils.py".into(),
                    status: FileStatus::Modified,
                    additions: 1,
                    deletions: 1,
                    is_binary: false,
                    is_large: false,
                    hunks: vec![
                        Hunk {
                            old_start: 1,
                            old_lines: 3,
                            new_start: 1,
                            new_lines: 3,
                            content: "@@ -1,3 +1,3 @@".into(),
                            changes: vec![
                                Change { change_type: ChangeType::Normal, old_line_number: Some(1), new_line_number: Some(1), content: "def hello():".into() },
                                Change { change_type: ChangeType::Delete, old_line_number: Some(2), new_line_number: None, content: "    return \"hi\"".into() },
                                Change { change_type: ChangeType::Insert, old_line_number: None, new_line_number: Some(2), content: "    return \"hello\"".into() },
                                Change { change_type: ChangeType::Normal, old_line_number: Some(3), new_line_number: Some(3), content: "".into() },
                            ],
                        },
                    ],
                },
            ],
        }
    }

    #[test]
    fn empty_comments_returns_no_comments_message() {
        let input = MarkdownInput {
            comments: vec![],
            diff_data: empty_diff(),
            metadata: base_meta(),
        };
        assert_eq!(generate_markdown(&input), "# Code Review Comments\n\nNo comments.\n");
    }

    #[test]
    fn formats_overall_comments() {
        let input = MarkdownInput {
            comments: vec![
                make_comment(Comment { comment_type: CommentType::Overall, category: CommentCategory::Fix, text: "Add error handling".into(), ..Default::default() }),
                make_comment(Comment { comment_type: CommentType::Overall, category: CommentCategory::Suggestion, text: "Consider caching".into(), ..Default::default() }),
            ],
            diff_data: empty_diff(),
            metadata: base_meta(),
        };
        let result = generate_markdown(&input);
        assert!(result.contains("## Overall"));
        assert!(result.contains("- [fix] Add error handling"));
        assert!(result.contains("- [suggestion] Consider caching"));
    }

    #[test]
    fn formats_file_level_comments() {
        let input = MarkdownInput {
            comments: vec![
                make_comment(Comment { comment_type: CommentType::File, category: CommentCategory::Suggestion, text: "File is too large".into(), file_path: Some("src/server.ts".into()), ..Default::default() }),
            ],
            diff_data: empty_diff(),
            metadata: base_meta(),
        };
        let result = generate_markdown(&input);
        assert!(result.contains("## src/server.ts"));
        assert!(result.contains("### File-level"));
        assert!(result.contains("- [suggestion] File is too large"));
    }

    #[test]
    fn formats_line_comment_with_code_context() {
        let input = MarkdownInput {
            comments: vec![
                make_comment(Comment { comment_type: CommentType::Line, category: CommentCategory::Nit, text: "Unused import".into(), file_path: Some("src/server.ts".into()), start_line: Some(78), side: Some("new".into()), ..Default::default() }),
            ],
            diff_data: diff_with_files(),
            metadata: base_meta(),
        };
        let result = generate_markdown(&input);
        assert!(result.contains("### Line 78"));
        assert!(result.contains("```typescript"));
        assert!(result.contains("// cleaned up"));
        assert!(result.contains("- [nit] Unused import"));
    }

    #[test]
    fn formats_range_comment_with_code_context() {
        let input = MarkdownInput {
            comments: vec![
                make_comment(Comment { comment_type: CommentType::Range, category: CommentCategory::Fix, text: "Missing await".into(), file_path: Some("src/server.ts".into()), start_line: Some(42), end_line: Some(45), side: Some("new".into()), ..Default::default() }),
            ],
            diff_data: diff_with_files(),
            metadata: base_meta(),
        };
        let result = generate_markdown(&input);
        assert!(result.contains("### Lines 42-45"));
        assert!(result.contains("```typescript"));
        assert!(result.contains("const result = await fetch(url);"));
        assert!(result.contains("const data = result.json();"));
        assert!(result.contains("return parsed;"));
        assert!(result.contains("- [fix] Missing await"));
    }

    #[test]
    fn groups_multiple_files_correctly() {
        let input = MarkdownInput {
            comments: vec![
                make_comment(Comment { comment_type: CommentType::Line, category: CommentCategory::Fix, text: "Fix A".into(), file_path: Some("src/server.ts".into()), start_line: Some(78), side: Some("new".into()), ..Default::default() }),
                make_comment(Comment { comment_type: CommentType::Line, category: CommentCategory::Nit, text: "Fix B".into(), file_path: Some("src/utils.py".into()), start_line: Some(2), side: Some("new".into()), ..Default::default() }),
            ],
            diff_data: diff_with_files(),
            metadata: base_meta(),
        };
        let result = generate_markdown(&input);
        assert!(result.contains("## src/server.ts"));
        assert!(result.contains("## src/utils.py"));
        assert!(result.contains("```python"));
        assert!(result.contains("return \"hello\""));
    }

    #[test]
    fn handles_mixed_comment_types_in_order() {
        let input = MarkdownInput {
            comments: vec![
                make_comment(Comment { comment_type: CommentType::Overall, category: CommentCategory::Suggestion, text: "Overall note".into(), ..Default::default() }),
                make_comment(Comment { comment_type: CommentType::File, category: CommentCategory::Nit, text: "File note".into(), file_path: Some("src/server.ts".into()), ..Default::default() }),
                make_comment(Comment { comment_type: CommentType::Range, category: CommentCategory::Fix, text: "Range note".into(), file_path: Some("src/server.ts".into()), start_line: Some(42), end_line: Some(45), side: Some("new".into()), ..Default::default() }),
                make_comment(Comment { comment_type: CommentType::Line, category: CommentCategory::Question, text: "Line note".into(), file_path: Some("src/server.ts".into()), start_line: Some(78), side: Some("new".into()), ..Default::default() }),
            ],
            diff_data: diff_with_files(),
            metadata: base_meta(),
        };
        let result = generate_markdown(&input);

        let overall_idx = result.find("## Overall").unwrap();
        let file_idx = result.find("## src/server.ts").unwrap();
        let file_level_idx = result.find("### File-level").unwrap();
        let range_idx = result.find("### Lines 42-45").unwrap();
        let line_idx = result.find("### Line 78").unwrap();

        assert!(overall_idx < file_idx);
        assert!(file_idx < file_level_idx);
        assert!(file_level_idx < range_idx);
        assert!(range_idx < line_idx);
    }

    #[test]
    fn skips_code_context_when_line_not_found() {
        let input = MarkdownInput {
            comments: vec![
                make_comment(Comment { comment_type: CommentType::Line, category: CommentCategory::Nit, text: "Not in diff".into(), file_path: Some("src/server.ts".into()), start_line: Some(999), side: Some("new".into()), ..Default::default() }),
            ],
            diff_data: diff_with_files(),
            metadata: base_meta(),
        };
        let result = generate_markdown(&input);
        assert!(result.contains("### Line 999"));
        assert!(result.contains("- [nit] Not in diff"));
        assert!(!result.contains("```typescript"));
    }

    #[test]
    fn skips_code_context_when_file_not_found() {
        let input = MarkdownInput {
            comments: vec![
                make_comment(Comment { comment_type: CommentType::Line, category: CommentCategory::Nit, text: "Unknown file".into(), file_path: Some("unknown.ts".into()), start_line: Some(1), side: Some("new".into()), ..Default::default() }),
            ],
            diff_data: diff_with_files(),
            metadata: base_meta(),
        };
        let result = generate_markdown(&input);
        assert!(result.contains("### Line 1"));
        assert!(result.contains("- [nit] Unknown file"));
        assert!(!result.contains("```"));
    }

    #[test]
    fn includes_all_four_comment_types() {
        let input = MarkdownInput {
            comments: vec![
                make_comment(Comment { comment_type: CommentType::Overall, category: CommentCategory::Fix, text: "Overall fix".into(), ..Default::default() }),
                make_comment(Comment { comment_type: CommentType::File, category: CommentCategory::Suggestion, text: "File suggestion".into(), file_path: Some("src/server.ts".into()), ..Default::default() }),
                make_comment(Comment { comment_type: CommentType::Line, category: CommentCategory::Question, text: "Line question".into(), file_path: Some("src/server.ts".into()), start_line: Some(42), side: Some("new".into()), ..Default::default() }),
                make_comment(Comment { comment_type: CommentType::Range, category: CommentCategory::Nit, text: "Range nit".into(), file_path: Some("src/server.ts".into()), start_line: Some(42), end_line: Some(45), side: Some("new".into()), ..Default::default() }),
            ],
            diff_data: diff_with_files(),
            metadata: base_meta(),
        };
        let result = generate_markdown(&input);
        assert!(result.contains("- [fix] Overall fix"));
        assert!(result.contains("- [suggestion] File suggestion"));
        assert!(result.contains("- [question] Line question"));
        assert!(result.contains("- [nit] Range nit"));
    }

    #[test]
    fn range_comment_single_line_outputs_line_header() {
        let input = MarkdownInput {
            comments: vec![
                make_comment(Comment { comment_type: CommentType::Range, category: CommentCategory::Fix, text: "Single line range".into(), file_path: Some("src/server.ts".into()), start_line: Some(40), end_line: Some(40), side: Some("new".into()), ..Default::default() }),
            ],
            diff_data: diff_with_files(),
            metadata: base_meta(),
        };
        let result = generate_markdown(&input);
        assert!(result.contains("### Line 40"));
        assert!(!result.contains("### Lines 40-40"));
    }

    #[test]
    fn handles_range_outside_diff_hunks() {
        let input = MarkdownInput {
            comments: vec![
                make_comment(Comment { comment_type: CommentType::Range, category: CommentCategory::Fix, text: "Out of range".into(), file_path: Some("src/server.ts".into()), start_line: Some(200), end_line: Some(210), side: Some("new".into()), ..Default::default() }),
            ],
            diff_data: diff_with_files(),
            metadata: base_meta(),
        };
        let result = generate_markdown(&input);
        assert!(result.contains("### Lines 200-210"));
        assert!(result.contains("- [fix] Out of range"));
        assert!(!result.contains("```"));
    }

    #[test]
    fn detect_language_maps_extensions() {
        assert_eq!(detect_language("file.ts"), "typescript");
        assert_eq!(detect_language("file.js"), "javascript");
        assert_eq!(detect_language("file.py"), "python");
        assert_eq!(detect_language("file.go"), "go");
        assert_eq!(detect_language("file.rs"), "rust");
        assert_eq!(detect_language("file.java"), "java");
        assert_eq!(detect_language("file.rb"), "ruby");
        assert_eq!(detect_language("file.css"), "css");
        assert_eq!(detect_language("file.html"), "html");
        assert_eq!(detect_language("file.json"), "json");
        assert_eq!(detect_language("file.md"), "markdown");
        assert_eq!(detect_language("file.sh"), "bash");
        assert_eq!(detect_language("file.yml"), "yaml");
        assert_eq!(detect_language("file.yaml"), "yaml");
    }

    #[test]
    fn detect_language_returns_empty_for_unknown() {
        assert_eq!(detect_language("file.xyz"), "");
        assert_eq!(detect_language("Makefile"), "");
    }

    #[test]
    fn detect_language_handles_tsx_jsx() {
        assert_eq!(detect_language("file.tsx"), "typescript");
        assert_eq!(detect_language("file.jsx"), "javascript");
    }
}

impl Default for Comment {
    fn default() -> Self {
        Comment {
            id: String::new(),
            comment_type: CommentType::Line,
            category: CommentCategory::Fix,
            text: String::new(),
            file_path: None,
            start_line: None,
            end_line: None,
            side: None,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }
}
