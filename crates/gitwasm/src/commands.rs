use crate::gitutil::{git_bytes, git_string, repo_root};
use crate::manifest::{Manifest, GITWASM_DIR};
use crate::runner::{run_module, Sandbox};
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;

/// Activate the repo's committed `.gitwasm/` modules in this clone.
/// This is the only per-clone step, and it's pure git config — the
/// behavior itself travels with the repository.
pub fn install() -> Result<i32> {
    let root = repo_root()?;
    let manifest = Manifest::load(&root)?;

    let hooks_dir = root.join(GITWASM_DIR).join("hooks");
    fs::create_dir_all(&hooks_dir)?;
    for hook_name in manifest.hooks.keys() {
        let shim = hooks_dir.join(hook_name);
        // sh shim, LF endings — git for Windows runs hooks through its bundled sh.
        fs::write(&shim, format!("#!/bin/sh\nexec gitwasm hook {hook_name} \"$@\"\n"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&shim, fs::Permissions::from_mode(0o755))?;
        }
        println!("gitwasm: hook shim  {hook_name} -> {}", manifest.hooks[hook_name]);
    }

    let hooks_path = hooks_dir.to_string_lossy().replace('\\', "/");
    git_string(&root, &["config", "core.hooksPath", &hooks_path])?;
    git_string(
        &root,
        &["config", "merge.gitwasm.driver", "gitwasm merge %O %A %B %P"],
    )?;
    git_string(&root, &["config", "merge.gitwasm.name", "gitwasm sandboxed wasm merge driver"])?;

    for rule in &manifest.merge {
        println!("gitwasm: merge rule {} -> {}", rule.pattern, rule.module);
    }
    println!("gitwasm: installed (core.hooksPath + merge.gitwasm.driver set for this clone)");
    if !manifest.merge.is_empty() {
        println!("gitwasm: note: merge rules require matching .gitattributes lines, e.g.:");
        for rule in &manifest.merge {
            println!("  {} merge=gitwasm", rule.pattern);
        }
    }
    Ok(0)
}

/// Dispatch a git hook to its wasm module. The module gets a read-only
/// snapshot of the *staged* tree (what is actually about to be committed),
/// not the working tree.
pub fn hook(name: &str, _hook_args: &[String]) -> Result<i32> {
    let root = repo_root()?;
    let manifest = Manifest::load(&root)?;
    let Some(module_name) = manifest.hooks.get(name) else {
        return Ok(0); // no module registered for this hook — allow
    };
    let module = Manifest::module_path(&root, module_name);

    let tmp = tempfile::tempdir().context("creating staging snapshot dir")?;
    let listing = git_bytes(
        &root,
        &["diff", "--cached", "--name-only", "--diff-filter=ACM", "-z"],
    )?;
    let listing = String::from_utf8_lossy(&listing);
    let mut file_count = 0usize;
    for path in listing.split('\0').filter(|p| !p.is_empty()) {
        let content = git_bytes(&root, &["show", &format!(":{path}")])
            .with_context(|| format!("reading staged blob {path}"))?;
        let dest = tmp.path().join(path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&dest, content)?;
        file_count += 1;
    }
    if file_count == 0 {
        return Ok(0);
    }

    eprintln!("gitwasm: {name} -> {module_name} ({file_count} staged file(s), sandboxed read-only)");
    run_module(
        &module,
        Sandbox {
            dir: tmp.path(),
            writable: false,
            argv: vec![module_name.clone()],
        },
    )
}

/// Git merge driver entry point: `gitwasm merge %O %A %B %P`.
/// %O/%A/%B are temp files (base/ours/theirs), %P is the repo-relative path.
/// On success the merged result must be left in %A.
pub fn merge(base: &str, ours: &str, theirs: &str, path: &str) -> Result<i32> {
    let root = repo_root()?;
    let manifest = Manifest::load(&root)?;
    let Some(module_name) = manifest.merge_module(path) else {
        eprintln!("gitwasm: no merge module matches '{path}' — leaving conflict for git");
        return Ok(1);
    };
    let module = Manifest::module_path(&root, module_name);

    // The module's entire world: one temp dir with exactly these three files.
    let tmp = tempfile::tempdir().context("creating merge sandbox dir")?;
    copy_or_empty(base, &tmp.path().join("base"))?;
    copy_or_empty(ours, &tmp.path().join("ours"))?;
    copy_or_empty(theirs, &tmp.path().join("theirs"))?;

    eprintln!("gitwasm: merging '{path}' with {module_name} (sandboxed)");
    let code = run_module(
        &module,
        Sandbox {
            dir: tmp.path(),
            writable: true,
            argv: vec![
                module_name.to_string(),
                "base".into(),
                "ours".into(),
                "theirs".into(),
                "result".into(),
                path.to_string(),
            ],
        },
    )?;

    let result = tmp.path().join("result");
    if code == 0 && result.exists() {
        fs::copy(&result, ours).context("writing merge result back to %A")?;
        Ok(0)
    } else {
        eprintln!("gitwasm: module reported a real conflict for '{path}'");
        Ok(1)
    }
}

/// Dev utility: run any module with the current directory preopened read-only.
pub fn run_direct(wasm: &str, args: &[String]) -> Result<i32> {
    let wasm_path = Path::new(wasm);
    if !wasm_path.exists() {
        bail!("no such module: {wasm}");
    }
    let mut argv = vec![wasm.to_string()];
    argv.extend(args.iter().cloned());
    let cwd = std::env::current_dir()?;
    run_module(
        wasm_path,
        Sandbox {
            dir: &cwd,
            writable: false,
            argv,
        },
    )
}

fn copy_or_empty(src: &str, dest: &Path) -> Result<()> {
    if Path::new(src).exists() {
        fs::copy(src, dest).with_context(|| format!("copying {src}"))?;
    } else {
        fs::write(dest, b"")?;
    }
    Ok(())
}
