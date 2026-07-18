use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub id: String,
    #[serde(rename = "type")]
    pub comment_type: CommentType,
    pub category: CommentCategory,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CommentType {
    Line,
    Range,
    File,
    Overall,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CommentCategory {
    Fix,
    Question,
    Suggestion,
    Nit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct CliOptions {
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    pub no_open: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    pub commits: Option<[String; 2]>,
    pub staged: bool,
    pub unstaged: bool,
    pub working: bool,
    pub fetch: bool,
    pub tui: bool,
    pub all: bool,
    /// Resolved unified diff context lines (CLI `-U` > config > default 3).
    pub context: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frontend_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_path: Option<String>,
    pub status: FileStatus,
    pub additions: u32,
    pub deletions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewMetadata {
    pub repo_name: String,
    pub commit_range: String,
    pub base_ref: String,
    pub head_ref: String,
    pub files: Vec<FileChange>,
    pub timestamp: String,
    pub csrf_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffResponse {
    pub files: Vec<ParsedFileDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ParsedFileDiff {
    pub old_path: String,
    pub new_path: String,
    pub hunks: Vec<Hunk>,
    pub status: FileStatus,
    pub additions: u32,
    pub deletions: u32,
    pub is_binary: bool,
    pub is_large: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Hunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub content: String,
    pub changes: Vec<Change>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    #[serde(rename = "type")]
    pub change_type: ChangeType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_line_number: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_line_number: Option<u32>,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChangeType {
    Insert,
    Delete,
    Normal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinishRequest {
    pub comments: Vec<Comment>,
    pub reviewed_files: Vec<String>,
    pub metadata: FinishMetadata,
    #[serde(rename = "_csrf")]
    pub _csrf: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinishMetadata {
    pub commit_range: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinishResponse {
    pub success: bool,
    pub output_path: String,
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionBackup {
    pub session: ReviewSession,
    #[serde(rename = "_csrf")]
    pub _csrf: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewSession {
    pub version: u32,
    pub commit_range: String,
    pub repo_path: String,
    pub repo_path_hash: String,
    pub comments: Vec<Comment>,
    pub reviewed_files: Vec<String>,
    pub view_mode: String,
    pub created_at: String,
    pub last_accessed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPreferences {
    pub theme: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileTreeNode {
    pub name: String,
    pub path: String,
    #[serde(rename = "type")]
    pub node_type: TreeNodeType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<FileTreeNode>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<FileStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additions: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deletions: Option<u32>,
    pub is_reviewed: bool,
    pub comment_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TreeNodeType {
    File,
    Directory,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finish_request_deserializes_underscore_csrf_from_frontend_payload() {
        // Frontend sends `_csrf` literally; rename_all = "camelCase" would otherwise
        // turn it into `csrf` and break /api/v1/finish with a 422.
        let body = r#"{
            "comments": [],
            "reviewedFiles": [],
            "metadata": {"commitRange": "HEAD~1..HEAD", "timestamp": "2026-05-17T00:00:00Z"},
            "_csrf": "token-abc"
        }"#;
        let parsed: FinishRequest = serde_json::from_str(body).unwrap();
        assert_eq!(parsed._csrf, "token-abc");
        assert_eq!(parsed.metadata.commit_range, "HEAD~1..HEAD");
    }

    #[test]
    fn session_backup_deserializes_underscore_csrf_from_frontend_payload() {
        let body = r#"{
            "session": {
                "version": 2,
                "commitRange": "HEAD~1..HEAD",
                "repoPath": "/repo",
                "repoPathHash": "abc123",
                "comments": [],
                "reviewedFiles": [],
                "viewMode": "split",
                "createdAt": "2026-05-17T00:00:00Z",
                "lastAccessedAt": "2026-05-17T00:00:00Z"
            },
            "_csrf": "token-abc"
        }"#;
        let parsed: SessionBackup = serde_json::from_str(body).unwrap();
        assert_eq!(parsed._csrf, "token-abc");
    }
}
