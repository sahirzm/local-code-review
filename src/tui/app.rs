use crate::config::Config;
use crate::tui::icons::IconSet;
use crate::tui::theme::{self, Theme};
use crate::types::{Comment, CommentCategory, CommentType, FileChange, FileStatus, ParsedFileDiff};

const MAX_COMMENT_CHARS: usize = 2000;

#[derive(Debug, Clone)]
pub struct App {
    pub files: Vec<(FileChange, Option<ParsedFileDiff>)>,
    pub comments: Vec<Comment>,
    pub reviewed_files: Vec<String>,
    pub current_file_idx: usize,
    pub current_comment_idx: usize,
    pub view_mode: ViewMode,
    pub input_buffer: String,
    pub input_category: CommentCategory,
    pub show_help: bool,
    pub sidebar_collapsed: bool,
    pub search_query: String,
    pub filter_status: Option<FileStatus>,
    pub head_ref: String,

    // Preferences (persisted to the shared config).
    pub theme_id: String,
    pub icons: IconSet,
    pub context_lines: u32,

    // Diff viewport state for the currently-selected file.
    pub scroll_offset: usize,
    pub diff_cursor: usize,

    // In-flight comment creation/edit intent, set before entering CommentInput.
    pub pending_comment: Option<PendingComment>,
    // Anchor for a range selection (line + side), set by the range key.
    pub range_anchor: Option<(u32, Side)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Split,
    Unified,
    CommentInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Old,
    New,
}

impl Side {
    pub fn as_str(&self) -> &'static str {
        match self {
            Side::Old => "old",
            Side::New => "new",
        }
    }
}

/// What the comment form will produce when submitted.
#[derive(Debug, Clone)]
pub struct PendingComment {
    pub kind: CommentType,
    pub file_path: Option<String>,
    pub start_line: Option<u32>,
    pub end_line: Option<u32>,
    pub side: Option<String>,
    /// Set when editing an existing comment; None for a fresh comment.
    pub editing_id: Option<String>,
}

/// A single rendered row of the diff for the selected file. The cursor only
/// lands on `Change` rows, which carry the line/side needed to target comments.
#[derive(Debug, Clone)]
pub enum DiffRow {
    HunkHeader(String),
    Change {
        old: Option<u32>,
        new: Option<u32>,
        side: Side,
        change_type: crate::types::ChangeType,
        content: String,
    },
    Blank,
}

impl DiffRow {
    pub fn is_change(&self) -> bool {
        matches!(self, DiffRow::Change { .. })
    }
}

impl App {
    pub fn new(
        files: Vec<FileChange>,
        parsed_diffs: Vec<ParsedFileDiff>,
        head_ref: String,
        config: &Config,
    ) -> Self {
        let combined = combine(files, parsed_diffs);
        App {
            files: combined,
            comments: Vec::new(),
            reviewed_files: Vec::new(),
            current_file_idx: 0,
            current_comment_idx: 0,
            view_mode: ViewMode::Split,
            input_buffer: String::new(),
            input_category: CommentCategory::Fix,
            show_help: false,
            sidebar_collapsed: false,
            search_query: String::new(),
            filter_status: None,
            head_ref,
            theme_id: config.theme.clone(),
            icons: IconSet::new(config.icon_mode),
            context_lines: config.diff_context_lines,
            scroll_offset: 0,
            diff_cursor: 0,
            pending_comment: None,
            range_anchor: None,
        }
    }

    /// The resolved theme for the current `theme_id`.
    pub fn theme(&self) -> Theme {
        theme::by_id(&self.theme_id)
    }

    /// Cycle to the next/previous theme in the registry order and persist.
    pub fn cycle_theme(&mut self, forward: bool) {
        let order = theme::ORDER;
        let cur = order.iter().position(|&id| id == self.theme_id).unwrap_or(0);
        let next = if forward {
            (cur + 1) % order.len()
        } else {
            (cur + order.len() - 1) % order.len()
        };
        self.theme_id = order[next].to_string();
    }

    /// The view mode to persist ("split"/"unified"); the transient CommentInput
    /// mode is never stored, so fall back to Split.
    pub fn persistable_view_mode(&self) -> &'static str {
        match self.view_mode {
            ViewMode::Unified => "unified",
            _ => "split",
        }
    }

    /// Apply a persisted view-mode string from a restored session.
    pub fn set_view_mode_from_str(&mut self, s: &str) {
        self.view_mode = match s {
            "unified" => ViewMode::Unified,
            _ => ViewMode::Split,
        };
    }

    pub fn is_input_active(&self) -> bool {
        self.view_mode == ViewMode::CommentInput
    }

    pub fn push_input_char(&mut self, c: char) {
        if self.input_buffer.chars().count() < MAX_COMMENT_CHARS {
            self.input_buffer.push(c);
        }
    }

    pub fn pop_input_char(&mut self) {
        self.input_buffer.pop();
    }

    pub fn filtered_files(&self) -> Vec<usize> {
        self.files
            .iter()
            .enumerate()
            .filter(|(_, (fc, _))| {
                if !self.search_query.is_empty()
                    && !fc.path.to_lowercase().contains(&self.search_query.to_lowercase())
                {
                    return false;
                }
                if let Some(ref status) = self.filter_status {
                    if fc.status != *status {
                        return false;
                    }
                }
                true
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// The global file index currently selected (honoring the active filter).
    pub fn selected_file_index(&self) -> Option<usize> {
        let filtered = self.filtered_files();
        if filtered.is_empty() {
            return None;
        }
        let sidx = self.current_file_idx.min(filtered.len() - 1);
        filtered.get(sidx).copied()
    }

    pub fn selected_file_path(&self) -> Option<String> {
        self.selected_file_index()
            .and_then(|i| self.files.get(i))
            .map(|(fc, _)| fc.path.clone())
    }

    pub fn comment_count(&self, file_path: &str) -> usize {
        self.comments
            .iter()
            .filter(|c| c.file_path.as_deref() == Some(file_path))
            .count()
    }

    pub fn is_reviewed(&self, file_path: &str) -> bool {
        self.reviewed_files.iter().any(|f| f == file_path)
    }

    pub fn toggle_reviewed(&mut self, file_path: &str) {
        if self.is_reviewed(file_path) {
            self.reviewed_files.retain(|f| f != file_path);
        } else {
            self.reviewed_files.push(file_path.to_string());
        }
    }

    /// Build the flat list of rendered rows for the selected file. Rebuilt on
    /// demand (cheap) rather than cached, so it always reflects the diff.
    pub fn diff_rows(&self) -> Vec<DiffRow> {
        let Some(idx) = self.selected_file_index() else {
            return vec![];
        };
        let Some((_, Some(diff))) = self.files.get(idx) else {
            return vec![];
        };
        let mut rows = Vec::new();
        for (h, hunk) in diff.hunks.iter().enumerate() {
            if h > 0 {
                rows.push(DiffRow::Blank);
            }
            rows.push(DiffRow::HunkHeader(format!(
                "@@ -{},{} +{},{} @@{}",
                hunk.old_start, hunk.old_lines, hunk.new_start, hunk.new_lines, hunk.content
            )));
            for change in &hunk.changes {
                let side = match change.change_type {
                    crate::types::ChangeType::Delete => Side::Old,
                    _ => Side::New,
                };
                rows.push(DiffRow::Change {
                    old: change.old_line_number,
                    new: change.new_line_number,
                    side,
                    change_type: change.change_type.clone(),
                    content: change.content.clone(),
                });
            }
        }
        rows
    }

    /// Indices of `diff_rows()` that are `Change` rows (cursor lands only here).
    pub fn change_row_indices(rows: &[DiffRow]) -> Vec<usize> {
        rows.iter()
            .enumerate()
            .filter(|(_, r)| r.is_change())
            .map(|(i, _)| i)
            .collect()
    }

    /// Resolve the current diff cursor to a concrete (line, side) target.
    pub fn cursor_target(&self) -> Option<(u32, Side)> {
        let rows = self.diff_rows();
        match rows.get(self.diff_cursor) {
            Some(DiffRow::Change { old, new, side, .. }) => {
                let line = match side {
                    Side::Old => *old,
                    Side::New => *new,
                }?;
                Some((line, *side))
            }
            _ => None,
        }
    }

    /// Move the diff cursor to the next/previous `Change` row.
    pub fn move_cursor(&mut self, forward: bool) {
        let rows = self.diff_rows();
        let changes = Self::change_row_indices(&rows);
        if changes.is_empty() {
            return;
        }
        // Find where we are among change rows, then step.
        let pos = changes.iter().position(|&i| i >= self.diff_cursor).unwrap_or(0);
        let cur = changes
            .iter()
            .rposition(|&i| i == self.diff_cursor)
            .unwrap_or(pos);
        let next = if forward {
            (cur + 1).min(changes.len() - 1)
        } else {
            cur.saturating_sub(1)
        };
        self.diff_cursor = changes[next];
    }

    /// Reset the diff viewport when switching files.
    pub fn reset_diff_view(&mut self) {
        self.scroll_offset = 0;
        let rows = self.diff_rows();
        self.diff_cursor = Self::change_row_indices(&rows).first().copied().unwrap_or(0);
        self.range_anchor = None;
    }

    pub fn sorted_comment_indices(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = (0..self.comments.len()).collect();
        indices.sort_by(|&a, &b| {
            let ca = &self.comments[a];
            let cb = &self.comments[b];
            if ca.comment_type == CommentType::Overall && cb.comment_type != CommentType::Overall {
                return std::cmp::Ordering::Less;
            }
            if ca.comment_type != CommentType::Overall && cb.comment_type == CommentType::Overall {
                return std::cmp::Ordering::Greater;
            }
            let a_path = ca.file_path.as_deref().unwrap_or("");
            let b_path = cb.file_path.as_deref().unwrap_or("");
            a_path.cmp(b_path).then_with(|| {
                ca.start_line.unwrap_or(0).cmp(&cb.start_line.unwrap_or(0))
            })
        });
        indices
    }

    /// The comment currently selected by comment-navigation (`j`/`k`).
    pub fn selected_comment_id(&self) -> Option<String> {
        let order = self.sorted_comment_indices();
        order
            .get(self.current_comment_idx)
            .and_then(|&i| self.comments.get(i))
            .map(|c| c.id.clone())
    }

    /// Comments attached to a given file line/side (range containment mirrors
    /// the web store's `getCommentsForLine`).
    pub fn get_comments_for_line(&self, file_path: &str, line: u32, side: Side) -> Vec<&Comment> {
        self.comments
            .iter()
            .filter(|c| {
                c.file_path.as_deref() == Some(file_path)
                    && c.side.as_deref() == Some(side.as_str())
                    && matches!(c.comment_type, CommentType::Line | CommentType::Range)
            })
            .filter(|c| {
                let start = c.start_line.unwrap_or(0);
                let end = c.end_line.unwrap_or(start);
                start <= line && line <= end
            })
            .collect()
    }

    /// Create a comment from the pending intent + the current input buffer.
    pub fn commit_pending_comment(&mut self) {
        let text = self.input_buffer.trim().to_string();
        let Some(pending) = self.pending_comment.take() else {
            return;
        };
        if text.is_empty() {
            self.cancel_input();
            return;
        }
        let now = chrono::Utc::now().to_rfc3339();
        if let Some(id) = pending.editing_id {
            if let Some(c) = self.comments.iter_mut().find(|c| c.id == id) {
                c.text = text;
                c.category = self.input_category.clone();
                c.updated_at = now;
            }
        } else {
            self.comments.push(Comment {
                id: uuid::Uuid::new_v4().to_string(),
                comment_type: pending.kind,
                category: self.input_category.clone(),
                text,
                file_path: pending.file_path,
                start_line: pending.start_line,
                end_line: pending.end_line,
                side: pending.side,
                created_at: now.clone(),
                updated_at: now,
            });
        }
        self.input_buffer.clear();
        self.range_anchor = None;
        self.view_mode = ViewMode::Split;
    }

    pub fn delete_comment(&mut self, id: &str) {
        self.comments.retain(|c| c.id != id);
        let order_len = self.comments.len();
        if self.current_comment_idx >= order_len && order_len > 0 {
            self.current_comment_idx = order_len - 1;
        }
    }

    /// Begin editing an existing comment: pre-fill the buffer + category.
    pub fn begin_edit(&mut self, id: &str) {
        if let Some(c) = self.comments.iter().find(|c| c.id == id) {
            self.input_buffer = c.text.clone();
            self.input_category = c.category.clone();
            self.pending_comment = Some(PendingComment {
                kind: c.comment_type.clone(),
                file_path: c.file_path.clone(),
                start_line: c.start_line,
                end_line: c.end_line,
                side: c.side.clone(),
                editing_id: Some(c.id.clone()),
            });
            self.view_mode = ViewMode::CommentInput;
        }
    }

    /// Begin a new comment of `kind` targeting the current cursor/file.
    pub fn begin_comment(&mut self, kind: CommentType) {
        self.input_buffer.clear();
        self.input_category = CommentCategory::Fix;
        let file_path = self.selected_file_path();
        let pending = match kind {
            CommentType::Overall => PendingComment {
                kind,
                file_path: None,
                start_line: None,
                end_line: None,
                side: None,
                editing_id: None,
            },
            CommentType::File => PendingComment {
                kind,
                file_path,
                start_line: None,
                end_line: None,
                side: None,
                editing_id: None,
            },
            CommentType::Range => {
                let Some((cur_line, side)) = self.cursor_target() else {
                    return;
                };
                let (start, end) = match self.range_anchor {
                    Some((anchor, aside)) if aside == side => {
                        (anchor.min(cur_line), anchor.max(cur_line))
                    }
                    _ => (cur_line, cur_line),
                };
                PendingComment {
                    kind: CommentType::Range,
                    file_path,
                    start_line: Some(start),
                    end_line: Some(end),
                    side: Some(side.as_str().to_string()),
                    editing_id: None,
                }
            }
            CommentType::Line => {
                let Some((line, side)) = self.cursor_target() else {
                    return;
                };
                PendingComment {
                    kind: CommentType::Line,
                    file_path,
                    start_line: Some(line),
                    end_line: Some(line),
                    side: Some(side.as_str().to_string()),
                    editing_id: None,
                }
            }
        };
        self.pending_comment = Some(pending);
        self.view_mode = ViewMode::CommentInput;
    }

    /// Set (or clear) the range anchor at the current cursor position.
    pub fn set_range_anchor(&mut self) {
        self.range_anchor = self.cursor_target();
    }

    pub fn cancel_input(&mut self) {
        self.input_buffer.clear();
        self.pending_comment = None;
        self.view_mode = ViewMode::Split;
    }

    /// Replace the diff data (after a context re-diff) while preserving review
    /// state and clamping cursors to the new content.
    pub fn rebuild_diffs(&mut self, files: Vec<FileChange>, parsed: Vec<ParsedFileDiff>) {
        self.files = combine(files, parsed);
        let filtered = self.filtered_files();
        if !filtered.is_empty() {
            self.current_file_idx = self.current_file_idx.min(filtered.len() - 1);
        }
        let rows = self.diff_rows();
        let changes = Self::change_row_indices(&rows);
        if !changes.contains(&self.diff_cursor) {
            self.diff_cursor = changes.first().copied().unwrap_or(0);
        }
        self.scroll_offset = 0;
    }
}

fn combine(
    files: Vec<FileChange>,
    parsed_diffs: Vec<ParsedFileDiff>,
) -> Vec<(FileChange, Option<ParsedFileDiff>)> {
    files
        .into_iter()
        .map(|fc| {
            let diff = parsed_diffs
                .iter()
                .find(|pd| pd.new_path == fc.path || pd.old_path.as_str() == fc.path.as_str())
                .cloned();
            (fc, diff)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Change, ChangeType, Hunk};

    fn cfg() -> Config {
        Config::default()
    }

    fn file(path: &str) -> FileChange {
        FileChange {
            path: path.into(),
            old_path: None,
            status: FileStatus::Modified,
            additions: 1,
            deletions: 1,
        }
    }

    fn diff(path: &str) -> ParsedFileDiff {
        ParsedFileDiff {
            old_path: path.into(),
            new_path: path.into(),
            status: FileStatus::Modified,
            additions: 1,
            deletions: 1,
            is_binary: false,
            is_large: false,
            hunks: vec![Hunk {
                old_start: 1,
                old_lines: 2,
                new_start: 1,
                new_lines: 2,
                content: String::new(),
                changes: vec![
                    Change {
                        change_type: ChangeType::Normal,
                        old_line_number: Some(1),
                        new_line_number: Some(1),
                        content: "ctx".into(),
                    },
                    Change {
                        change_type: ChangeType::Insert,
                        old_line_number: None,
                        new_line_number: Some(2),
                        content: "added".into(),
                    },
                    Change {
                        change_type: ChangeType::Delete,
                        old_line_number: Some(2),
                        new_line_number: None,
                        content: "removed".into(),
                    },
                ],
            }],
        }
    }

    fn app() -> App {
        App::new(vec![file("a.rs")], vec![diff("a.rs")], "HEAD".into(), &cfg())
    }

    #[test]
    fn view_mode_persist_mapping() {
        let mut a = app();
        a.view_mode = ViewMode::Unified;
        assert_eq!(a.persistable_view_mode(), "unified");
        a.view_mode = ViewMode::CommentInput; // transient → never persisted
        assert_eq!(a.persistable_view_mode(), "split");
        a.set_view_mode_from_str("unified");
        assert_eq!(a.view_mode, ViewMode::Unified);
    }

    #[test]
    fn cycle_theme_wraps_both_directions() {
        let mut a = app();
        a.theme_id = "default-dark".into();
        a.cycle_theme(false); // wrap to last
        assert_eq!(a.theme_id, "catppuccin-latte");
        a.cycle_theme(true); // wrap forward to first
        assert_eq!(a.theme_id, "default-dark");
    }

    #[test]
    fn diff_rows_and_cursor_target() {
        let mut a = app();
        a.reset_diff_view();
        let rows = a.diff_rows();
        // 1 hunk header + 3 changes.
        assert_eq!(rows.iter().filter(|r| r.is_change()).count(), 3);
        // Cursor starts on the first change (the Normal ctx line, new side).
        assert_eq!(a.cursor_target(), Some((1, Side::New)));
        a.move_cursor(true); // insert row
        assert_eq!(a.cursor_target(), Some((2, Side::New)));
        a.move_cursor(true); // delete row → old side
        assert_eq!(a.cursor_target(), Some((2, Side::Old)));
    }

    #[test]
    fn line_comment_targets_cursor() {
        let mut a = app();
        a.reset_diff_view();
        a.move_cursor(true); // insert line, new side, line 2
        a.begin_comment(CommentType::Line);
        a.input_buffer = "looks off".into();
        a.commit_pending_comment();
        assert_eq!(a.comments.len(), 1);
        let c = &a.comments[0];
        assert_eq!(c.comment_type, CommentType::Line);
        assert_eq!(c.start_line, Some(2));
        assert_eq!(c.side.as_deref(), Some("new"));
        assert_eq!(c.file_path.as_deref(), Some("a.rs"));
    }

    #[test]
    fn range_comment_spans_anchor_to_cursor() {
        let mut a = app();
        a.reset_diff_view(); // cursor at line 1 new
        a.set_range_anchor();
        a.move_cursor(true); // line 2 new
        a.begin_comment(CommentType::Range);
        a.input_buffer = "range".into();
        a.commit_pending_comment();
        let c = &a.comments[0];
        assert_eq!(c.comment_type, CommentType::Range);
        assert_eq!(c.start_line, Some(1));
        assert_eq!(c.end_line, Some(2));
    }

    #[test]
    fn file_and_overall_comments() {
        let mut a = app();
        a.begin_comment(CommentType::File);
        a.input_buffer = "whole file".into();
        a.commit_pending_comment();
        a.begin_comment(CommentType::Overall);
        a.input_buffer = "overall".into();
        a.commit_pending_comment();
        assert_eq!(a.comments[0].comment_type, CommentType::File);
        assert_eq!(a.comments[0].file_path.as_deref(), Some("a.rs"));
        assert_eq!(a.comments[1].comment_type, CommentType::Overall);
        assert!(a.comments[1].file_path.is_none());
    }

    #[test]
    fn edit_and_delete_comment() {
        let mut a = app();
        a.reset_diff_view();
        a.begin_comment(CommentType::Line);
        a.input_buffer = "first".into();
        a.commit_pending_comment();
        let id = a.comments[0].id.clone();

        a.begin_edit(&id);
        assert_eq!(a.input_buffer, "first");
        a.input_buffer = "second".into();
        a.input_category = CommentCategory::Nit;
        a.commit_pending_comment();
        assert_eq!(a.comments.len(), 1);
        assert_eq!(a.comments[0].text, "second");
        assert_eq!(a.comments[0].category, CommentCategory::Nit);

        a.delete_comment(&id);
        assert!(a.comments.is_empty());
    }

    #[test]
    fn get_comments_for_line_range_containment() {
        let mut a = app();
        a.comments.push(Comment {
            id: "1".into(),
            comment_type: CommentType::Range,
            category: CommentCategory::Fix,
            text: "r".into(),
            file_path: Some("a.rs".into()),
            start_line: Some(2),
            end_line: Some(5),
            side: Some("new".into()),
            created_at: "t".into(),
            updated_at: "t".into(),
        });
        assert_eq!(a.get_comments_for_line("a.rs", 3, Side::New).len(), 1);
        assert_eq!(a.get_comments_for_line("a.rs", 6, Side::New).len(), 0);
        assert_eq!(a.get_comments_for_line("a.rs", 3, Side::Old).len(), 0);
    }
}
