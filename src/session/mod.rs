use std::path::Path;

use sha2::{Digest, Sha256};

use crate::types::ReviewSession;

const SESSION_PREFIX: &str = "local-review:";
const EXPIRY_MS: i64 = 14 * 24 * 60 * 60 * 1000;

pub fn hash_repo_path(repo_path: &str) -> String {
    let mut hash: i32 = 0;
    for &b in repo_path.as_bytes() {
        hash = ((hash << 5).wrapping_sub(hash)).wrapping_add(b as i32);
    }
    format!("{:08x}", hash as u32)
}

pub fn get_session_key(repo_path_hash: &str, commit_range: &str) -> String {
    format!("{}{}:{}", SESSION_PREFIX, repo_path_hash, commit_range)
}

fn session_dir() -> std::path::PathBuf {
    Path::new(".local-review").to_path_buf()
}

fn session_path(key: &str) -> std::path::PathBuf {
    let hash = Sha256::digest(key.as_bytes());
    let hash_hex = hex::encode(&hash[..6]);
    session_dir().join(format!(".session-{}.json", hash_hex))
}

pub fn save_session(key: &str, session: &ReviewSession) -> anyhow::Result<()> {
    let dir = session_dir();
    std::fs::create_dir_all(&dir)?;
    let path = session_path(key);
    let json = serde_json::to_string_pretty(session)?;
    std::fs::write(&path, json)?;
    Ok(())
}

pub fn load_session(key: &str) -> Option<ReviewSession> {
    let path = session_path(key);
    let raw = std::fs::read_to_string(&path).ok()?;
    let session: ReviewSession = serde_json::from_str(&raw).ok()?;

    let last_accessed = chrono::DateTime::parse_from_rfc3339(&session.last_accessed_at)
        .ok()?
        .with_timezone(&chrono::Utc);
    let elapsed = chrono::Utc::now().signed_duration_since(last_accessed);
    if elapsed.num_milliseconds() > EXPIRY_MS {
        let _ = std::fs::remove_file(&path);
        return None;
    }

    Some(session)
}

pub fn clear_session(key: &str) -> anyhow::Result<()> {
    let path = session_path(key);
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

pub fn clean_expired_sessions() -> anyhow::Result<()> {
    let dir = session_dir();
    if !dir.exists() {
        return Ok(());
    }
    let now = chrono::Utc::now();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        if !name.starts_with(".session-") || !name.ends_with(".json") {
            continue;
        }
        let raw = match std::fs::read_to_string(entry.path()) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let session: Result<ReviewSession, _> = serde_json::from_str(&raw);
        let Ok(session) = session else { continue };
        let last_accessed = match chrono::DateTime::parse_from_rfc3339(&session.last_accessed_at) {
            Ok(dt) => dt.with_timezone(&chrono::Utc),
            Err(_) => continue,
        };
        if now.signed_duration_since(last_accessed).num_milliseconds() > EXPIRY_MS {
            let _ = std::fs::remove_file(entry.path());
        }
    }
    Ok(())
}
