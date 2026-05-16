use std::path::Path;

pub fn get_default_output_path() -> String {
    let ts = chrono::Utc::now()
        .format("%Y-%m-%dT%H-%M-%S")
        .to_string();
    format!(".local-review/{}.md", ts)
}

pub async fn write_review_output(markdown: &str, output_path: &str) -> anyhow::Result<String> {
    let abs_path = std::path::absolute(Path::new(output_path))?;
    let dir = abs_path.parent().unwrap_or(Path::new("."));

    match tokio::fs::metadata(dir).await {
        Ok(meta) if !meta.is_dir() => {
            return Err(anyhow::anyhow!(
                "\"{}\" exists but is not a directory. Remove it or use --output to specify a different path.",
                dir.display()
            ));
        }
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tokio::fs::create_dir_all(dir).await.map_err(|e| {
                let code = e.kind();
                if code == std::io::ErrorKind::PermissionDenied {
                    anyhow::anyhow!(
                        "Cannot create output directory \"{}\": permission denied. Use --output to specify a writable path.",
                        dir.display()
                    )
                } else {
                    anyhow::anyhow!("Failed to create output directory \"{}\": {}", dir.display(), e)
                }
            })?;
        }
        Err(e) => return Err(e.into()),
    }

    tokio::fs::write(&abs_path, markdown)
        .await
        .map_err(|e| {
            let code = e.kind();
            if code == std::io::ErrorKind::PermissionDenied {
                anyhow::anyhow!(
                    "Cannot write to \"{}\": permission denied. Use --output to specify a writable path.",
                    abs_path.display()
                )
            } else {
                anyhow::anyhow!("Failed to write output: {}", e)
            }
        })?;

    Ok(abs_path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_path_starts_with_local_review() {
        let path = get_default_output_path();
        assert!(path.starts_with(".local-review/"));
    }

    #[test]
    fn default_path_ends_with_md() {
        let path = get_default_output_path();
        assert!(path.ends_with(".md"));
    }

    #[test]
    fn default_path_has_no_colons() {
        let path = get_default_output_path();
        assert!(!path.contains(':'));
    }
}
