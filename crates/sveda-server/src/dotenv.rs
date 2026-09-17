use std::path::{Path, PathBuf};

pub fn load_runtime_env() {
    for path in env_file_candidates() {
        if path.is_file() {
            apply_env_file(&path);
            return;
        }
    }
}

fn env_file_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(explicit) = std::env::var("SVEDA_ENV_FILE") {
        let trimmed = explicit.trim();
        if !trimmed.is_empty() {
            paths.push(PathBuf::from(trimmed));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd.join(".env"));
        let mut dir = cwd;
        for _ in 0..8 {
            paths.push(dir.join("apps/runtime/.env"));
            paths.push(dir.join("sveda/apps/runtime/.env"));
            if !dir.pop() {
                break;
            }
        }
    }
    paths
}

pub fn parse_env_file(contents: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for raw in contents.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line).trim();
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        pairs.push((key.to_string(), unquote(value.trim())));
    }
    pairs
}

pub fn apply_env_pairs(pairs: &[(String, String)]) {
    for (key, value) in pairs {
        if std::env::var_os(key).is_none() {
            std::env::set_var(key, value);
        }
    }
}

fn apply_env_file(path: &Path) {
    let Ok(contents) = std::fs::read_to_string(path) else {
        return;
    };
    apply_env_pairs(&parse_env_file(&contents));
}

fn unquote(value: &str) -> String {
    let bytes = value.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return value[1..value.len() - 1].to_string();
        }
    }
    value.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_env_file_reads_export_quotes_and_skips_comments() {
        let pairs = parse_env_file(
            "\n# comment\nexport FOO=bar\nBAZ=\"quoted value\"\nQUX='single'\nSKIP\n=novalue\n",
        );
        assert_eq!(
            pairs,
            vec![
                ("FOO".into(), "bar".into()),
                ("BAZ".into(), "quoted value".into()),
                ("QUX".into(), "single".into()),
            ]
        );
    }

    #[test]
    fn apply_env_pairs_does_not_override_existing_values() {
        let key = format!("SVEDA_DOTENV_TEST_{}", std::process::id());
        std::env::remove_var(&key);
        apply_env_pairs(&[(key.clone(), "one".into())]);
        assert_eq!(std::env::var(&key).unwrap(), "one");
        apply_env_pairs(&[(key.clone(), "two".into())]);
        assert_eq!(std::env::var(&key).unwrap(), "one");
        std::env::remove_var(&key);
    }
}
