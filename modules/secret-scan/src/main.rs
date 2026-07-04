//! Pre-commit secret scanner. The gitwasm host hands this module a read-only
//! snapshot of the *staged* tree mounted at "." — nothing else is visible,
//! so it is safe to run even from a repo you just cloned from a stranger.
//! Exit code != 0 blocks the commit.

use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::exit;

const MAX_FILE_BYTES: u64 = 1_000_000;

fn patterns() -> Vec<(&'static str, Regex)> {
    [
        ("AWS access key ID", r"\bAKIA[0-9A-Z]{16}\b"),
        ("GitHub token", r"\bgh[pousr]_[A-Za-z0-9]{36,}\b"),
        ("Slack token", r"\bxox[baprs]-[A-Za-z0-9-]{10,}\b"),
        (
            "private key block",
            r"-----BEGIN (RSA |EC |OPENSSH |DSA |PGP )?PRIVATE KEY( BLOCK)?-----",
        ),
        (
            "hardcoded credential",
            r#"(?i)\b(api[_-]?key|secret|password)\b\s*[:=]\s*["'][^"']{12,}["']"#,
        ),
    ]
    .into_iter()
    .map(|(name, re)| (name, Regex::new(re).expect("valid pattern")))
    .collect()
}

fn main() {
    let patterns = patterns();
    let mut files = Vec::new();
    collect_files(Path::new("."), &mut files);

    let mut findings = 0usize;
    for file in &files {
        let Ok(meta) = fs::metadata(file) else { continue };
        if meta.len() > MAX_FILE_BYTES {
            continue;
        }
        let Ok(bytes) = fs::read(file) else { continue };
        if bytes.contains(&0) {
            continue; // binary
        }
        let text = String::from_utf8_lossy(&bytes);
        for (line_no, line) in text.lines().enumerate() {
            for (name, re) in &patterns {
                if re.is_match(line) {
                    eprintln!(
                        "secret-scan: {}:{}: {name}",
                        file.display(),
                        line_no + 1
                    );
                    findings += 1;
                }
            }
        }
    }

    if findings > 0 {
        eprintln!("secret-scan: {findings} finding(s) — commit blocked");
        exit(1);
    }
    eprintln!("secret-scan: {} staged file(s) clean", files.len());
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, out);
        } else {
            out.push(path);
        }
    }
}
