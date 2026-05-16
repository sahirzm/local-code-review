use crate::types::{Comment, CommentCategory, CommentType, FileChange, FileStatus, ParsedFileDiff};

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Split,
    Unified,
    CommentInput,
}

impl App {
    pub fn new(
        files: Vec<FileChange>,
        parsed_diffs: Vec<ParsedFileDiff>,
        head_ref: String,
    ) -> Self {
        let combined: Vec<_> = files
            .into_iter()
            .map(|fc| {
                let diff = parsed_diffs
                    .iter()
                    .find(|pd| pd.new_path == fc.path || pd.old_path.as_str() == fc.path.as_str())
                    .cloned();
                (fc, diff)
            })
            .collect();

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
        }
    }

    pub fn is_input_active(&self) -> bool {
        self.view_mode == ViewMode::CommentInput
    }

    pub fn push_input_char(&mut self, c: char) {
        if self.input_buffer.len() < 2000 {
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
                ca.start_line
                    .unwrap_or(0)
                    .cmp(&cb.start_line.unwrap_or(0))
            })
        });
        indices
    }

    pub fn add_comment(&mut self, file_path: Option<String>, start_line: Option<u32>, side: Option<String>) {
        let now = chrono::Utc::now().to_rfc3339();
        let comment_type = if file_path.is_none() {
            CommentType::Overall
        } else {
            CommentType::Line
        };
        let comment = Comment {
            id: uuid::Uuid::new_v4().to_string(),
            comment_type,
            category: self.input_category.clone(),
            text: self.input_buffer.clone(),
            file_path,
            start_line,
            end_line: start_line,
            side,
            created_at: now.clone(),
            updated_at: now,
        };
        self.comments.push(comment);
        self.input_buffer.clear();
        self.view_mode = ViewMode::Split;
    }
}
