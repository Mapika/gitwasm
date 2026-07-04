use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn git_bytes(cwd: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .context("failed to spawn git")?;
    if !out.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(out.stdout)
}

pub fn git_string(cwd: &Path, args: &[&str]) -> Result<String> {
    Ok(String::from_utf8_lossy(&git_bytes(cwd, args)?)
        .trim_end()
        .to_string())
}

pub fn repo_root() -> Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    let root = git_string(&cwd, &["rev-parse", "--show-toplevel"])
        .context("not inside a git repository")?;
    Ok(PathBuf::from(root))
}
