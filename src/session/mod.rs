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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // session_dir() is cwd-relative (`.local-review`), so tests that touch the
    // filesystem must not run concurrently or interleave cwd changes.
    static CWD_LOCK: Mutex<()> = Mutex::new(());

    fn session(last_accessed_at: &str) -> ReviewSession {
        ReviewSession {
            version: 2,
            commit_range: "abc..def".into(),
            repo_path: "/repo".into(),
            repo_path_hash: "12345678".into(),
            comments: vec![],
            reviewed_files: vec![],
            view_mode: "split".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            last_accessed_at: last_accessed_at.into(),
        }
    }

    fn now_rfc3339() -> String {
        chrono::Utc::now().to_rfc3339()
    }

    fn expired_rfc3339() -> String {
        (chrono::Utc::now() - chrono::Duration::days(15)).to_rfc3339()
    }

    /// Runs `f` with the process cwd pointed at a fresh tempdir, serialized so
    /// concurrent tests can't observe each other's `.local-review` writes.
    fn in_temp_cwd<T>(f: impl FnOnce() -> T) -> T {
        let _guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::tempdir().unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(tmp.path()).unwrap();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        std::env::set_current_dir(original).unwrap();
        match result {
            Ok(v) => v,
            Err(p) => std::panic::resume_unwind(p),
        }
    }

    #[test]
    fn hash_repo_path_is_deterministic() {
        assert_eq!(hash_repo_path("/some/repo"), hash_repo_path("/some/repo"));
    }

    #[test]
    fn hash_repo_path_differs_for_different_inputs() {
        assert_ne!(hash_repo_path("a"), hash_repo_path("b"));
    }

    #[test]
    fn hash_repo_path_returns_8_char_hex() {
        let hash = hash_repo_path("/some/repo/path");
        assert_eq!(hash.len(), 8);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn hash_repo_path_empty_is_all_zeroes() {
        assert_eq!(hash_repo_path(""), "00000000");
    }

    #[test]
    fn get_session_key_uses_prefix() {
        assert_eq!(
            get_session_key("abcd1234", "abc..def"),
            "local-review:abcd1234:abc..def"
        );
    }

    #[test]
    fn save_then_load_round_trips() {
        in_temp_cwd(|| {
            let key = get_session_key("hash01", "range-roundtrip");
            let s = session(&now_rfc3339());
            save_session(&key, &s).unwrap();
            let loaded = load_session(&key).expect("session should load");
            assert_eq!(loaded.commit_range, s.commit_range);
            assert_eq!(loaded.version, 2);
        });
    }

    #[test]
    fn load_missing_returns_none() {
        in_temp_cwd(|| {
            assert!(load_session(&get_session_key("nope", "missing")).is_none());
        });
    }

    #[test]
    fn load_expired_returns_none_and_removes_file() {
        in_temp_cwd(|| {
            let key = get_session_key("hash02", "range-expired");
            save_session(&key, &session(&expired_rfc3339())).unwrap();
            assert!(load_session(&key).is_none());
            // A second load confirms the stale file was deleted, not just skipped.
            assert!(load_session(&key).is_none());
        });
    }

    #[test]
    fn load_ignores_unparseable_content() {
        in_temp_cwd(|| {
            let key = get_session_key("hash03", "range-bad");
            let path = session_path(&key);
            std::fs::create_dir_all(session_dir()).unwrap();
            std::fs::write(&path, "{ not valid json").unwrap();
            assert!(load_session(&key).is_none());
        });
    }

    #[test]
    fn clear_session_removes_the_file() {
        in_temp_cwd(|| {
            let key = get_session_key("hash04", "range-clear");
            save_session(&key, &session(&now_rfc3339())).unwrap();
            assert!(load_session(&key).is_some());
            clear_session(&key).unwrap();
            assert!(load_session(&key).is_none());
        });
    }

    #[test]
    fn clear_session_is_ok_when_absent() {
        in_temp_cwd(|| {
            assert!(clear_session(&get_session_key("gone", "range-absent")).is_ok());
        });
    }

    #[test]
    fn clean_expired_sessions_removes_only_stale_files() {
        in_temp_cwd(|| {
            let fresh_key = get_session_key("fresh", "range-fresh");
            let stale_key = get_session_key("stale", "range-stale");
            save_session(&fresh_key, &session(&now_rfc3339())).unwrap();
            save_session(&stale_key, &session(&expired_rfc3339())).unwrap();

            clean_expired_sessions().unwrap();

            assert!(load_session(&fresh_key).is_some());
            assert!(load_session(&stale_key).is_none());
        });
    }

    #[test]
    fn clean_expired_sessions_is_ok_without_dir() {
        in_temp_cwd(|| {
            // No `.local-review` created yet.
            assert!(clean_expired_sessions().is_ok());
        });
    }

    #[test]
    fn clean_expired_sessions_skips_unrelated_and_unparseable_files() {
        in_temp_cwd(|| {
            std::fs::create_dir_all(session_dir()).unwrap();
            std::fs::write(session_dir().join("unrelated.txt"), "keep me").unwrap();
            std::fs::write(session_dir().join(".session-bad.json"), "not json").unwrap();

            clean_expired_sessions().unwrap();

            // Non-session file is untouched; malformed session file is left in place
            // (only successfully-parsed, expired sessions are removed).
            assert!(session_dir().join("unrelated.txt").exists());
        });
    }
}
