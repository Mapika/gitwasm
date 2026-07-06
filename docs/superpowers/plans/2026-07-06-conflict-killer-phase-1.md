# Conflict Killer Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Phase 1 adoption wedge from the approved roadmap: `gitwasm init lockfiles`, `pnpm-lock.yaml` merge support, lockfile-first docs/demo flow, CI guidance, and honest verdict-cache wording.

**Architecture:** Keep the `.gitwasm/manifest.toml` convention unchanged. Add product-facing init profiles in the CLI and stock-module scaffolding layer, add one preview1 pnpm merge module, then update docs and demos around the lockfile conflict story. Treat verdict replay as local cache behavior and audit as the proof operation.

**Tech Stack:** Rust 2021 workspace, Wasmtime host, WASI preview1 stock modules built by `crates/gitwasm/build.rs`, TOML/JSON/YAML parsers, shell and PowerShell demos, GitHub Actions documentation.

---

## Scope Check

The approved roadmap covers three product phases. This plan implements the first independently shippable slice: **Phase 1: Conflict Killer**.

Phase 2 Safe Repo Policy and Phase 3 Verdict Distribution should get separate implementation plans after this slice is green. This plan includes only the Phase 3-adjacent wording and local checks required to avoid overstating verdict trust during Phase 1.

## File Map

- Modify `Cargo.toml`
  - Add `modules/pnpm-lock-merge` to the workspace.
- Create `modules/pnpm-lock-merge/Cargo.toml`
  - Defines the preview1 pnpm merge module package.
- Create `modules/pnpm-lock-merge/src/main.rs`
  - Parses pnpm YAML lockfiles, 3-way merges known map sections, emits diagnostics, and refuses malformed or ambiguous input.
- Modify `crates/gitwasm/build.rs`
  - Builds and embeds `pnpm-lock-merge.wasm` with the other preview1 stock modules.
- Modify `crates/gitwasm/src/stock.rs`
  - Introduces init profiles and profile-filtered manifest/gitattributes rendering.
  - Registers `pnpm-lock-merge.wasm` for `pnpm-lock.yaml`.
- Modify `crates/gitwasm/src/commands.rs`
  - Parses `gitwasm init [all|lockfiles|hooks]`.
  - Scaffolds only the profile-selected modules.
  - Avoids configuring `core.hooksPath` when the manifest has no hooks.
- Modify `crates/gitwasm/src/main.rs`
  - Updates usage and routes optional init profile arguments.
- Modify `README.md`
  - Reorders the front-door quickstart around `gitwasm init lockfiles`.
- Modify `SPEC.md`
  - Clarifies that init profiles are CLI scaffolding affordances, not manifest format changes.
- Modify `SECURITY.md`
  - Clarifies verdict replay versus audit proof.
- Modify `demo/run-demo.sh`
  - Adds lockfile-profile setup and a pnpm merge scenario.
- Modify `demo/run-demo.ps1`
  - Mirrors the Unix demo changes.
- Create `docs/ci.md`
  - Adds a copyable CI recipe for verification and audit.
- Update committed `.gitwasm/` stock content after building:
  - Add `.gitwasm/pnpm-lock-merge.wasm`.
  - Update `.gitwasm/manifest.toml`.
  - Update `.gitattributes`.
  - Refresh `.gitwasm/signatures.toml` with `GITWASM_KEY_PATH=demo/demo-signing-key`.

---

### Task 1: Add Profile-Aware Stock Scaffolding

**Files:**
- Modify: `crates/gitwasm/src/stock.rs`

- [ ] **Step 1: Write failing stock profile tests**

Append these tests inside the existing `#[cfg(test)] mod tests` block. If `stock.rs` has no test module yet, add this module at the end of the file:

```rust
#[cfg(test)]
mod tests {
    use super::{default_manifest_for, gitattributes_lines_for, modules_for, InitProfile};

    #[test]
    fn lockfiles_profile_contains_merge_modules_but_no_hooks() {
        let files: Vec<&str> = modules_for(InitProfile::Lockfiles)
            .into_iter()
            .map(|module| module.file)
            .collect();

        assert!(files.contains(&"lockfile-merge.wasm"));
        assert!(files.contains(&"cargo-lock-merge.wasm"));
        assert!(files.contains(&"yarn-lock-merge.wasm"));
        assert!(files.contains(&"poetry-lock-merge.wasm"));
        assert!(files.contains(&"lineset-merge.wasm"));
        assert!(files.contains(&"pnpm-lock-merge.wasm"));
        assert!(!files.contains(&"secret-scan.wasm"));
        assert!(!files.contains(&"commit-lint.wasm"));

        let manifest = default_manifest_for(InitProfile::Lockfiles);
        assert!(manifest.contains("pattern = \"package-lock.json\""));
        assert!(manifest.contains("pattern = \"pnpm-lock.yaml\""));
        assert!(!manifest.contains("pre-commit"));
        assert!(!manifest.contains("commit-msg"));
    }

    #[test]
    fn hooks_profile_contains_hooks_but_no_merge_rules() {
        let files: Vec<&str> = modules_for(InitProfile::Hooks)
            .into_iter()
            .map(|module| module.file)
            .collect();

        assert!(files.contains(&"secret-scan.wasm"));
        assert!(files.contains(&"commit-lint.wasm"));
        assert!(!files.contains(&"lockfile-merge.wasm"));
        assert!(!files.contains(&"pnpm-lock-merge.wasm"));

        let manifest = default_manifest_for(InitProfile::Hooks);
        assert!(manifest.contains("pre-commit = \"secret-scan.wasm\""));
        assert!(manifest.contains("# commit-msg = \"commit-lint.wasm\""));
        assert!(!manifest.contains("[[merge]]"));
    }

    #[test]
    fn lockfiles_gitattributes_do_not_enable_hooks() {
        let lines = gitattributes_lines_for(InitProfile::Lockfiles);

        assert_eq!(lines[0], ".gitwasm/** -text");
        assert!(lines.contains(&"package-lock.json merge=gitwasm".to_string()));
        assert!(lines.contains(&"pnpm-lock.yaml merge=gitwasm".to_string()));
        assert!(!lines.iter().any(|line| line.contains("secret-scan")));
    }
}
```

- [ ] **Step 2: Run tests and verify they fail**

Run:

```bash
cargo test -p gitwasm stock::tests -- --nocapture
```

Expected:

```text
error[E0432]: unresolved imports `super::default_manifest_for`, `super::gitattributes_lines_for`, `super::modules_for`, `super::InitProfile`
```

- [ ] **Step 3: Add `InitProfile` and filtered stock helpers**

In `crates/gitwasm/src/stock.rs`, add this enum after `StockModule`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitProfile {
    All,
    Lockfiles,
    Hooks,
}

impl InitProfile {
    pub fn name(self) -> &'static str {
        match self {
            InitProfile::All => "all",
            InitProfile::Lockfiles => "lockfiles",
            InitProfile::Hooks => "hooks",
        }
    }
}
```

Add this stock module entry before `secret-scan.wasm` in `STOCK`:

```rust
    StockModule {
        file: "pnpm-lock-merge.wasm",
        bytes: include_bytes!(concat!(env!("OUT_DIR"), "/pnpm-lock-merge.wasm")),
        hook: None,
        merge_patterns: &["pnpm-lock.yaml"],
        default_on: true,
        summary: "structural 3-way merge for pnpm-lock.yaml",
    },
```

Replace `default_manifest()` and `gitattributes_lines()` with these functions:

```rust
fn module_in_profile(module: &StockModule, profile: InitProfile) -> bool {
    match profile {
        InitProfile::All => true,
        InitProfile::Lockfiles => !module.merge_patterns.is_empty(),
        InitProfile::Hooks => module.hook.is_some(),
    }
}

fn module_enabled_in_manifest(module: &StockModule, profile: InitProfile) -> bool {
    match profile {
        InitProfile::All | InitProfile::Hooks => module.default_on,
        InitProfile::Lockfiles => !module.merge_patterns.is_empty(),
    }
}

pub fn modules_for(profile: InitProfile) -> Vec<&'static StockModule> {
    STOCK
        .iter()
        .filter(|module| module_in_profile(module, profile))
        .collect()
}

/// Render the manifest.toml for `gitwasm init`.
pub fn default_manifest_for(profile: InitProfile) -> String {
    let mut hooks = String::new();
    let mut merges = String::new();

    for module in modules_for(profile) {
        if let Some(hook) = module.hook {
            let prefix = if module_enabled_in_manifest(module, profile) {
                ""
            } else {
                "# "
            };
            hooks.push_str(&format!("{prefix}{hook} = \"{}\"\n", module.file));
        }

        for pattern in module.merge_patterns {
            merges.push_str(&format!(
                "\n[[merge]]\npattern = \"{pattern}\"\nmodule = \"{}\"\n",
                module.file
            ));
        }
    }

    format!(
        "# gitwasm manifest — maps git extension points to sandboxed wasm modules\n\
         # stored in this directory. Profile: {}\n\
         # See https://github.com/gitwasm/gitwasm\n\
         \n[hooks]\n{hooks}{merges}",
        profile.name()
    )
}

/// Backwards-compatible default manifest for the full stock profile.
pub fn default_manifest() -> String {
    default_manifest_for(InitProfile::All)
}

/// The .gitattributes lines the selected manifest needs. The `-text` line is
/// load-bearing: git EOL conversion would silently change file hashes across
/// platforms and break signature verification.
pub fn gitattributes_lines_for(profile: InitProfile) -> Vec<String> {
    let mut lines = vec![".gitwasm/** -text".to_string()];
    lines.extend(
        modules_for(profile)
            .into_iter()
            .flat_map(|module| module.merge_patterns.iter())
            .map(|pattern| format!("{pattern} merge=gitwasm")),
    );
    lines
}

/// Backwards-compatible gitattributes for the full stock profile.
pub fn gitattributes_lines() -> Vec<String> {
    gitattributes_lines_for(InitProfile::All)
}
```

- [ ] **Step 4: Run stock tests and verify they pass**

Run:

```bash
cargo test -p gitwasm stock::tests -- --nocapture
```

Expected:

```text
test stock::tests::hooks_profile_contains_hooks_but_no_merge_rules ... ok
test stock::tests::lockfiles_gitattributes_do_not_enable_hooks ... ok
test stock::tests::lockfiles_profile_contains_merge_modules_but_no_hooks ... ok
```

- [ ] **Step 5: Commit**

```bash
git add crates/gitwasm/src/stock.rs
git commit -m "feat: add stock init profiles"
```

---

### Task 2: Route `gitwasm init [all|lockfiles|hooks]`

**Files:**
- Modify: `crates/gitwasm/src/commands.rs`
- Modify: `crates/gitwasm/src/main.rs`

- [ ] **Step 1: Write failing profile parser tests**

Append these tests inside `#[cfg(test)] mod tests` in `crates/gitwasm/src/commands.rs`:

```rust
    #[test]
    fn parse_init_profile_defaults_to_all() {
        assert_eq!(parse_init_profile(None).unwrap(), stock::InitProfile::All);
    }

    #[test]
    fn parse_init_profile_accepts_named_profiles() {
        assert_eq!(
            parse_init_profile(Some("lockfiles")).unwrap(),
            stock::InitProfile::Lockfiles
        );
        assert_eq!(
            parse_init_profile(Some("hooks")).unwrap(),
            stock::InitProfile::Hooks
        );
        assert_eq!(
            parse_init_profile(Some("all")).unwrap(),
            stock::InitProfile::All
        );
    }

    #[test]
    fn parse_init_profile_rejects_unknown_profile() {
        let err = parse_init_profile(Some("everything")).unwrap_err();
        assert!(format!("{err:#}").contains("unknown init profile"));
        assert!(format!("{err:#}").contains("lockfiles"));
    }
```

- [ ] **Step 2: Run tests and verify they fail**

Run:

```bash
cargo test -p gitwasm commands::tests::parse_init_profile -- --nocapture
```

Expected:

```text
error[E0425]: cannot find function `parse_init_profile` in this scope
```

- [ ] **Step 3: Add profile parsing and profile-aware init**

In `crates/gitwasm/src/commands.rs`, add this function near the top-level constants:

```rust
fn parse_init_profile(arg: Option<&str>) -> Result<stock::InitProfile> {
    match arg.unwrap_or("all") {
        "all" => Ok(stock::InitProfile::All),
        "lockfiles" => Ok(stock::InitProfile::Lockfiles),
        "hooks" => Ok(stock::InitProfile::Hooks),
        other => bail!(
            "unknown init profile '{other}' — expected one of: all, lockfiles, hooks"
        ),
    }
}
```

Change the init signature and the first half of its body from:

```rust
pub fn init() -> Result<i32> {
    let root = repo_root()?;
```

to:

```rust
pub fn init(profile_arg: Option<&str>) -> Result<i32> {
    let profile = parse_init_profile(profile_arg)?;
    let root = repo_root()?;
```

In the same function, replace:

```rust
    for module in stock::STOCK {
        fs::write(dir.join(module.file), module.bytes)?;
        let state = if module.default_on { "on " } else { "off" };
        println!("gitwasm: [{state}] {: <22} {}", module.file, module.summary);
    }
    fs::write(dir.join(MANIFEST_FILE), stock::default_manifest())?;
```

with:

```rust
    println!("gitwasm: init profile '{}'", profile.name());
    for module in stock::modules_for(profile) {
        fs::write(dir.join(module.file), module.bytes)?;
        let state = if profile == stock::InitProfile::Lockfiles || module.default_on {
            "on "
        } else {
            "off"
        };
        println!("gitwasm: [{state}] {: <22} {}", module.file, module.summary);
    }
    fs::write(
        dir.join(MANIFEST_FILE),
        stock::default_manifest_for(profile),
    )?;
```

Replace the gitattributes loop:

```rust
    for line in stock::gitattributes_lines() {
```

with:

```rust
    for line in stock::gitattributes_lines_for(profile) {
```

In `install()`, wrap hook shim creation and `core.hooksPath` configuration so lockfile-only manifests do not override hooks:

```rust
    if !manifest.hooks.is_empty() {
        let hooks_dir = root.join(GITWASM_DIR).join("hooks");
        fs::create_dir_all(&hooks_dir)?;
        for hook_name in manifest.hooks.keys() {
            let shim = hooks_dir.join(hook_name);
            fs::write(
                &shim,
                format!(
                    "#!/bin/sh\n\
                     if command -v gitwasm >/dev/null 2>&1; then\n\
                     \x20 exec gitwasm hook {hook_name} \"$@\"\n\
                     fi\n\
                     echo \"gitwasm: not on PATH; skipping {hook_name} hook (see .gitwasm/)\" >&2\n"
                ),
            )?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&shim, fs::Permissions::from_mode(0o755))?;
            }
            println!(
                "gitwasm: hook shim  {hook_name} -> {}",
                manifest.hooks[hook_name]
            );
        }

        let hooks_path = hooks_dir.to_string_lossy().replace('\\', "/");
        git_string(&root, &["config", "core.hooksPath", &hooks_path])?;
    } else {
        println!("gitwasm: no hooks enabled by this manifest");
    }
```

Keep the merge driver config immediately after this block.

- [ ] **Step 4: Update CLI usage and routing**

In `crates/gitwasm/src/main.rs`, replace the init usage line with:

```rust
  gitwasm init [all|lockfiles|hooks]       scaffold .gitwasm/ stock modules + activate
```

Replace the init match arm:

```rust
        Some("init") => commands::init(),
```

with:

```rust
        Some("init") => commands::init(args.get(1).map(String::as_str)),
```

- [ ] **Step 5: Run parser tests and CLI build**

Run:

```bash
cargo test -p gitwasm commands::tests::parse_init_profile -- --nocapture
cargo check -p gitwasm
```

Expected:

```text
test commands::tests::parse_init_profile_accepts_named_profiles ... ok
test commands::tests::parse_init_profile_defaults_to_all ... ok
test commands::tests::parse_init_profile_rejects_unknown_profile ... ok
Finished `dev` profile
```

- [ ] **Step 6: Commit**

```bash
git add crates/gitwasm/src/commands.rs crates/gitwasm/src/main.rs
git commit -m "feat: add init profiles"
```

---

### Task 3: Add `pnpm-lock.yaml` Merge Module

**Files:**
- Modify: `Cargo.toml`
- Create: `modules/pnpm-lock-merge/Cargo.toml`
- Create: `modules/pnpm-lock-merge/src/main.rs`

- [ ] **Step 1: Add workspace package and module manifest**

In the root `Cargo.toml`, add `modules/pnpm-lock-merge` in the workspace members list after `modules/poetry-lock-merge`:

```toml
    "modules/poetry-lock-merge",
    "modules/pnpm-lock-merge",
```

Create `modules/pnpm-lock-merge/Cargo.toml`:

```toml
[package]
name = "pnpm-lock-merge"
version = "0.1.0"
edition = "2021"
description = "Structural 3-way merge for pnpm-lock.yaml — runs sandboxed inside gitwasm"

[dependencies]
serde_yaml = "0.9"
```

- [ ] **Step 2: Write failing pnpm merge tests**

Create `modules/pnpm-lock-merge/src/main.rs` with tests first and minimal stubs:

```rust
//! Structural 3-way merge for pnpm-lock.yaml.

use serde_yaml::{Mapping, Value};
use std::process::exit;

#[derive(Clone, Debug, PartialEq)]
struct Lock {
    doc: Mapping,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 5 {
        eprintln!("usage: pnpm-lock-merge <base> <ours> <theirs> <result> [path]");
        exit(2);
    }
    let base = read_lock(&args[1]);
    let ours = read_lock(&args[2]);
    let theirs = read_lock(&args[3]);

    match merge3(&base, &ours, &theirs) {
        Ok((merged, notes)) => {
            for note in notes {
                eprintln!("pnpm-lock-merge: {note}");
            }
            std::fs::write(&args[4], render(&merged)).expect("write result");
            eprintln!("pnpm-lock-merge: clean structural merge");
        }
        Err(reason) => {
            eprintln!("pnpm-lock-merge: real conflict: {reason}");
            exit(1);
        }
    }
}

fn read_lock(path: &str) -> Lock {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    parse(&text).unwrap_or_else(|err| {
        eprintln!("pnpm-lock-merge: {path} is not a supported pnpm lockfile ({err}) — refusing");
        exit(1);
    })
}

fn parse(_text: &str) -> Result<Lock, String> {
    Err("parser not implemented".into())
}

fn render(_lock: &Lock) -> String {
    String::new()
}

fn merge3(_base: &Lock, _ours: &Lock, _theirs: &Lock) -> Result<(Lock, Vec<String>), String> {
    Err("merge not implemented".into())
}

#[cfg(test)]
mod tests {
    use super::{merge3, parse, render};

    const BASE: &str = r#"
lockfileVersion: '9.0'

importers:
  .:
    dependencies:
      express:
        specifier: ^4.19.0
        version: 4.19.2

packages:
  express@4.19.2:
    resolution: {integrity: sha512-express}

snapshots:
  express@4.19.2: {}
"#;

    const OURS: &str = r#"
lockfileVersion: '9.0'

importers:
  .:
    dependencies:
      express:
        specifier: ^4.19.0
        version: 4.19.2
      left-pad:
        specifier: ^1.3.0
        version: 1.3.0

packages:
  express@4.19.2:
    resolution: {integrity: sha512-express}
  left-pad@1.3.0:
    resolution: {integrity: sha512-left}

snapshots:
  express@4.19.2: {}
  left-pad@1.3.0: {}
"#;

    const THEIRS: &str = r#"
lockfileVersion: '9.0'

importers:
  .:
    dependencies:
      express:
        specifier: ^4.19.0
        version: 4.19.2
      right-pad:
        specifier: ^1.0.1
        version: 1.0.1

packages:
  express@4.19.2:
    resolution: {integrity: sha512-express}
  right-pad@1.0.1:
    resolution: {integrity: sha512-right}

snapshots:
  express@4.19.2: {}
  right-pad@1.0.1: {}
"#;

    #[test]
    fn disjoint_dependency_additions_merge_cleanly() {
        let base = parse(BASE).unwrap();
        let ours = parse(OURS).unwrap();
        let theirs = parse(THEIRS).unwrap();

        let (merged, notes) = merge3(&base, &ours, &theirs).unwrap();
        let rendered = render(&merged);

        assert!(notes.is_empty());
        assert!(rendered.contains("left-pad"));
        assert!(rendered.contains("right-pad"));
        assert!(rendered.contains("express@4.19.2"));
    }

    #[test]
    fn same_package_changed_differently_conflicts() {
        let base = parse(BASE).unwrap();
        let ours = parse(&OURS.replace("sha512-left", "sha512-one")).unwrap();
        let theirs = parse(&OURS.replace("sha512-left", "sha512-two")).unwrap();

        let err = merge3(&base, &ours, &theirs).unwrap_err();

        assert!(err.contains("packages.left-pad@1.3.0"));
    }

    #[test]
    fn malformed_yaml_is_refused() {
        let err = parse("lockfileVersion: [").unwrap_err();

        assert!(err.contains("YAML"));
    }

    #[test]
    fn unsupported_top_level_shape_is_refused() {
        let err = parse("- not\n- a\n- map\n").unwrap_err();

        assert!(err.contains("top-level mapping"));
    }

    #[test]
    fn missing_lockfile_version_is_refused() {
        let err = parse("packages: {}\n").unwrap_err();

        assert!(err.contains("lockfileVersion"));
    }
}
```

- [ ] **Step 3: Run pnpm tests and verify they fail for missing implementation**

Run:

```bash
cargo test -p pnpm-lock-merge -- --nocapture
```

Expected:

```text
test tests::disjoint_dependency_additions_merge_cleanly ... FAILED
test tests::same_package_changed_differently_conflicts ... FAILED
```

- [ ] **Step 4: Implement parser, renderer, and structural merge**

Replace the stub `parse`, `render`, and `merge3` functions with:

```rust
fn parse(text: &str) -> Result<Lock, String> {
    if text.trim().is_empty() {
        return Ok(Lock {
            doc: Mapping::new(),
        });
    }

    let value: Value =
        serde_yaml::from_str(text).map_err(|err| format!("YAML parse error: {err}"))?;
    let doc = match value {
        Value::Mapping(map) => map,
        _ => return Err("expected top-level mapping".into()),
    };

    if !doc.contains_key(Value::String("lockfileVersion".into())) {
        return Err("missing lockfileVersion".into());
    }

    Ok(Lock { doc })
}

fn render(lock: &Lock) -> String {
    let value = Value::Mapping(lock.doc.clone());
    let mut rendered = serde_yaml::to_string(&value).expect("serialize merged pnpm lockfile");
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    rendered
}

fn merge3(base: &Lock, ours: &Lock, theirs: &Lock) -> Result<(Lock, Vec<String>), String> {
    let mut notes = Vec::new();
    let merged = merge_mapping("", &base.doc, &ours.doc, &theirs.doc, &mut notes)?;
    Ok((Lock { doc: merged }, notes))
}
```

Add these helper functions below `merge3`:

```rust
fn merge_mapping(
    path: &str,
    base: &Mapping,
    ours: &Mapping,
    theirs: &Mapping,
    notes: &mut Vec<String>,
) -> Result<Mapping, String> {
    let mut keys: Vec<Value> = base
        .keys()
        .chain(ours.keys())
        .chain(theirs.keys())
        .cloned()
        .collect();
    keys.sort_by_key(key_name);
    keys.dedup();

    let mut out = Mapping::new();
    for key in keys {
        let key_path = join_path(path, &key);
        let b = base.get(&key);
        let o = ours.get(&key);
        let t = theirs.get(&key);
        let winner = merge_value(&key_path, b, o, t, notes)?;
        if let Some(value) = winner {
            out.insert(key, value);
        }
    }
    Ok(out)
}

fn merge_value(
    path: &str,
    base: Option<&Value>,
    ours: Option<&Value>,
    theirs: Option<&Value>,
    notes: &mut Vec<String>,
) -> Result<Option<Value>, String> {
    if ours == theirs {
        return Ok(ours.cloned());
    }
    if ours == base {
        return Ok(theirs.cloned());
    }
    if theirs == base {
        return Ok(ours.cloned());
    }

    match (base, ours, theirs) {
        (Some(Value::Mapping(b)), Some(Value::Mapping(o)), Some(Value::Mapping(t))) => {
            Ok(Some(Value::Mapping(merge_mapping(path, b, o, t, notes)?)))
        }
        (None, Some(Value::Mapping(o)), Some(Value::Mapping(t))) => {
            Ok(Some(Value::Mapping(merge_mapping(path, &Mapping::new(), o, t, notes)?)))
        }
        (Some(_), None, Some(_)) | (Some(_), Some(_), None) => {
            Err(format!("{path}: deleted on one side and modified on the other"))
        }
        (None, Some(_), Some(_)) => Err(format!("{path}: added differently on both sides")),
        _ => Err(format!("{path}: changed differently on both sides")),
    }
}

fn key_name(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => serde_yaml::to_string(other)
            .unwrap_or_else(|_| format!("{other:?}"))
            .replace('\n', ""),
    }
}

fn join_path(prefix: &str, key: &Value) -> String {
    let key = key_name(key);
    if prefix.is_empty() {
        key
    } else {
        format!("{prefix}.{key}")
    }
}
```

- [ ] **Step 5: Run pnpm tests and format**

Run:

```bash
cargo test -p pnpm-lock-merge -- --nocapture
cargo fmt --check
```

Expected:

```text
test tests::disjoint_dependency_additions_merge_cleanly ... ok
test tests::malformed_yaml_is_refused ... ok
test tests::missing_lockfile_version_is_refused ... ok
test tests::same_package_changed_differently_conflicts ... ok
test tests::unsupported_top_level_shape_is_refused ... ok
```

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml modules/pnpm-lock-merge/Cargo.toml modules/pnpm-lock-merge/src/main.rs
git commit -m "feat: add pnpm lockfile merge module"
```

---

### Task 4: Embed pnpm as a Stock Module

**Files:**
- Modify: `crates/gitwasm/build.rs`
- Modify: `crates/gitwasm/src/stock.rs`
- Update generated/committed stock artifacts after build:
  - `.gitwasm/pnpm-lock-merge.wasm`
  - `.gitwasm/manifest.toml`
  - `.gitattributes`
  - `.gitwasm/signatures.toml`

- [ ] **Step 1: Write failing stock registration test**

Add this test inside `crates/gitwasm/src/stock.rs` test module:

```rust
    #[test]
    fn pnpm_is_registered_as_stock_lockfile_merge_driver() {
        let pnpm = STOCK
            .iter()
            .find(|module| module.file == "pnpm-lock-merge.wasm")
            .expect("pnpm stock module is registered");

        assert_eq!(pnpm.merge_patterns, &["pnpm-lock.yaml"]);
        assert!(pnpm.default_on);

        let attrs = gitattributes_lines_for(InitProfile::Lockfiles);
        assert!(attrs.contains(&"pnpm-lock.yaml merge=gitwasm".to_string()));
    }
```

- [ ] **Step 2: Run test and verify it fails before build registration**

Run:

```bash
cargo test -p gitwasm stock::tests::pnpm_is_registered_as_stock_lockfile_merge_driver -- --nocapture
```

Expected:

```text
thread 'stock::tests::pnpm_is_registered_as_stock_lockfile_merge_driver' panicked at 'pnpm stock module is registered'
```

- [ ] **Step 3: Register pnpm in `build.rs`**

In `crates/gitwasm/build.rs`, add `pnpm-lock-merge` to `PREVIEW1_MODULES`:

```rust
const PREVIEW1_MODULES: &[&str] = &[
    "lockfile-merge",
    "cargo-lock-merge",
    "yarn-lock-merge",
    "poetry-lock-merge",
    "pnpm-lock-merge",
    "secret-scan",
    "commit-lint",
];
```

- [ ] **Step 4: Register pnpm in stock list**

If Task 1 did not already add this entry, add it before `secret-scan.wasm` in `crates/gitwasm/src/stock.rs`:

```rust
    StockModule {
        file: "pnpm-lock-merge.wasm",
        bytes: include_bytes!(concat!(env!("OUT_DIR"), "/pnpm-lock-merge.wasm")),
        hook: None,
        merge_patterns: &["pnpm-lock.yaml"],
        default_on: true,
        summary: "structural 3-way merge for pnpm-lock.yaml",
    },
```

- [ ] **Step 5: Build and test embedded module registration**

Run:

```bash
cargo test -p gitwasm stock::tests::pnpm_is_registered_as_stock_lockfile_merge_driver -- --nocapture
cargo build --release -p gitwasm
```

Expected:

```text
test stock::tests::pnpm_is_registered_as_stock_lockfile_merge_driver ... ok
Finished `release` profile
```

- [ ] **Step 6: Refresh this repository's committed `.gitwasm/` stock content**

Run:

```bash
tmp_repo="$(mktemp -d)"
(
  cd "$tmp_repo"
  git init -q
  PATH="/home/markmarosi/projects/gitwasm/target/release:$PATH" gitwasm init all
)
cp "$tmp_repo/.gitwasm/pnpm-lock-merge.wasm" .gitwasm/pnpm-lock-merge.wasm
rm -rf "$tmp_repo"
```

Add this merge rule to `.gitwasm/manifest.toml` with the other merge rules:

```toml
[[merge]]
pattern = "pnpm-lock.yaml"
module = "pnpm-lock-merge.wasm"
```

Add this line to `.gitattributes` with the other merge-driver attributes:

```gitattributes
pnpm-lock.yaml merge=gitwasm
```

Refresh signatures:

```bash
GITWASM_KEY_PATH=demo/demo-signing-key ./target/release/gitwasm sign
```

- [ ] **Step 7: Verify signed stock content**

Run:

```bash
./target/release/gitwasm verify
```

Expected:

```text
gitwasm: valid signature by
```

- [ ] **Step 8: Commit**

```bash
git add crates/gitwasm/build.rs crates/gitwasm/src/stock.rs .gitwasm/pnpm-lock-merge.wasm .gitwasm/manifest.toml .gitattributes .gitwasm/signatures.toml
git commit -m "feat: register pnpm stock merge driver"
```

---

### Task 5: Update Demo Scripts for Lockfile Profile and pnpm

**Files:**
- Modify: `demo/run-demo.sh`
- Modify: `demo/run-demo.ps1`

- [ ] **Step 1: Update Unix demo to use `init lockfiles` for the merge-first setup**

In `demo/run-demo.sh`, change the first init step label:

```bash
step "gitwasm init lockfiles - one command from zero to lockfile conflict protection"
gitwasm init lockfiles
```

Immediately after committing the initial gitwasm adoption commit, add a separate hook activation step for later hook scenarios:

```bash
step "Enable the default hook pack for policy scenarios"
gitwasm init hooks || true
gitwasm install
```

If `gitwasm init hooks` refuses because `.gitwasm/manifest.toml` already exists, replace the above with a manifest edit that enables `pre-commit` in the existing manifest:

```bash
if ! grep -q '^pre-commit = "secret-scan.wasm"' .gitwasm/manifest.toml; then
    sed -i.bak '/^\[hooks\]/a pre-commit = "secret-scan.wasm"' .gitwasm/manifest.toml
    rm .gitwasm/manifest.toml.bak
fi
gitwasm install
```

- [ ] **Step 2: Add Unix pnpm merge scenario**

Add this scenario after the Cargo.lock scenario in `demo/run-demo.sh`:

```bash
step "Scenario 3: pnpm-lock.yaml - both branches add a dependency"
cat > pnpm-lock.yaml <<'EOF'
lockfileVersion: '9.0'

importers:
  .:
    dependencies:
      express:
        specifier: ^4.19.0
        version: 4.19.2

packages:
  express@4.19.2:
    resolution: {integrity: sha512-express}

snapshots:
  express@4.19.2: {}
EOF
git add pnpm-lock.yaml && git commit -q -m "chore: add pnpm lock baseline"
git checkout -q -b pnpm-feature
python3 - <<'PY'
from pathlib import Path
p = Path("pnpm-lock.yaml")
s = p.read_text()
s = s.replace("      express:\n        specifier: ^4.19.0\n        version: 4.19.2\n", "      express:\n        specifier: ^4.19.0\n        version: 4.19.2\n      left-pad:\n        specifier: ^1.3.0\n        version: 1.3.0\n")
s = s.replace("  express@4.19.2:\n    resolution: {integrity: sha512-express}\n", "  express@4.19.2:\n    resolution: {integrity: sha512-express}\n  left-pad@1.3.0:\n    resolution: {integrity: sha512-left}\n")
s = s.replace("  express@4.19.2: {}\n", "  express@4.19.2: {}\n  left-pad@1.3.0: {}\n")
p.write_text(s)
PY
git commit -q -am "feat: add left-pad to pnpm lock"
git checkout -q main
python3 - <<'PY'
from pathlib import Path
p = Path("pnpm-lock.yaml")
s = p.read_text()
s = s.replace("      express:\n        specifier: ^4.19.0\n        version: 4.19.2\n", "      express:\n        specifier: ^4.19.0\n        version: 4.19.2\n      right-pad:\n        specifier: ^1.0.1\n        version: 1.0.1\n")
s = s.replace("  express@4.19.2:\n    resolution: {integrity: sha512-express}\n", "  express@4.19.2:\n    resolution: {integrity: sha512-express}\n  right-pad@1.0.1:\n    resolution: {integrity: sha512-right}\n")
s = s.replace("  express@4.19.2: {}\n", "  express@4.19.2: {}\n  right-pad@1.0.1: {}\n")
p.write_text(s)
PY
git commit -q -am "feat: add right-pad to pnpm lock"
git merge pnpm-feature -m "Merge branch 'pnpm-feature'"
grep -q left-pad pnpm-lock.yaml && grep -q right-pad pnpm-lock.yaml
ok "3. pnpm-lock.yaml merged clean with both dependencies"
```

Renumber later scenarios by incrementing their visible labels by one.

- [ ] **Step 3: Mirror pnpm scenario in PowerShell**

In `demo/run-demo.ps1`, add the same scenario using PowerShell string replacement:

```powershell
    Step "Scenario 3: pnpm-lock.yaml - both branches add a dependency"
    Set-Content pnpm-lock.yaml @'
lockfileVersion: '9.0'

importers:
  .:
    dependencies:
      express:
        specifier: ^4.19.0
        version: 4.19.2

packages:
  express@4.19.2:
    resolution: {integrity: sha512-express}

snapshots:
  express@4.19.2: {}
'@
    git add pnpm-lock.yaml; git commit -q -m "chore: add pnpm lock baseline"
    git checkout -q -b pnpm-feature
    (Get-Content pnpm-lock.yaml -Raw) `
        -replace "      express:`n        specifier: \\^4\\.19\\.0`n        version: 4\\.19\\.2", "      express:`n        specifier: ^4.19.0`n        version: 4.19.2`n      left-pad:`n        specifier: ^1.3.0`n        version: 1.3.0" `
        -replace "  express@4\\.19\\.2:`n    resolution: \\{integrity: sha512-express\\}", "  express@4.19.2:`n    resolution: {integrity: sha512-express}`n  left-pad@1.3.0:`n    resolution: {integrity: sha512-left}" `
        -replace "  express@4\\.19\\.2: \\{\\}", "  express@4.19.2: {}`n  left-pad@1.3.0: {}" `
        | Set-Content pnpm-lock.yaml
    git commit -q -am "feat: add left-pad to pnpm lock"
    git checkout -q main
    (Get-Content pnpm-lock.yaml -Raw) `
        -replace "      express:`n        specifier: \\^4\\.19\\.0`n        version: 4\\.19\\.2", "      express:`n        specifier: ^4.19.0`n        version: 4.19.2`n      right-pad:`n        specifier: ^1.0.1`n        version: 1.0.1" `
        -replace "  express@4\\.19\\.2:`n    resolution: \\{integrity: sha512-express\\}", "  express@4.19.2:`n    resolution: {integrity: sha512-express}`n  right-pad@1.0.1:`n    resolution: {integrity: sha512-right}" `
        -replace "  express@4\\.19\\.2: \\{\\}", "  express@4.19.2: {}`n  right-pad@1.0.1: {}" `
        | Set-Content pnpm-lock.yaml
    git commit -q -am "feat: add right-pad to pnpm lock"
    git merge pnpm-feature -m "Merge branch 'pnpm-feature'"
    if ($LASTEXITCODE -ne 0) { throw "pnpm lockfile merge conflicted" }
    $pnpm = Get-Content pnpm-lock.yaml -Raw
    Assert ($pnpm -match "left-pad" -and $pnpm -match "right-pad") "3. pnpm-lock.yaml merged clean with both dependencies"
```

Renumber later visible scenario labels by one.

- [ ] **Step 4: Run Unix demo**

Run:

```bash
./demo/run-demo.sh
```

Expected:

```text
3. pnpm-lock.yaml merged clean with both dependencies
Demo complete - all eight scenarios passed
```

- [ ] **Step 5: Commit**

```bash
git add demo/run-demo.sh demo/run-demo.ps1
git commit -m "test: add pnpm lockfile demo scenario"
```

---

### Task 6: Update Lockfile-First Documentation and CI Recipe

**Files:**
- Modify: `README.md`
- Modify: `SPEC.md`
- Create: `docs/ci.md`

- [ ] **Step 1: Update README quickstart around the adoption wedge**

In `README.md`, make the first quickstart path lockfile-specific:

```markdown
## Quickstart: end lockfile conflicts

```sh
rustup target add wasm32-wasip1 wasm32-unknown-unknown
cargo build --release -p gitwasm
gitwasm init lockfiles
git add .gitwasm .gitattributes
git commit -m "chore: add gitwasm lockfile merge drivers"
```

Collaborators run once per clone:

```sh
gitwasm install
```

From then on, supported generated files are merged structurally:
`package-lock.json`, `package.json`, `pnpm-lock.yaml`, `Cargo.lock`,
`yarn.lock` v1, `poetry.lock`, and `go.sum`.
```

Keep the broader sandbox/signing/verdict story after this quickstart.

- [ ] **Step 2: Add CLI profile note to SPEC**

In `SPEC.md`, add this paragraph after the repository layout section:

```markdown
The reference CLI exposes setup profiles such as `gitwasm init lockfiles`,
`gitwasm init hooks`, and `gitwasm init all`. These are scaffolding
affordances only. They choose which stock modules and `.gitattributes` lines to
write; they do not change the manifest format described below.
```

- [ ] **Step 3: Add CI recipe**

Create `docs/ci.md`:

```markdown
# CI verification

Use CI to verify the committed `.gitwasm/` state independently of developer
machines.

```yaml
name: gitwasm

on:
  pull_request:
  push:
    branches: [main]

jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-wasip1, wasm32-unknown-unknown
      - uses: Swatinem/rust-cache@v2
      - run: cargo build --release -p gitwasm
      - run: ./target/release/gitwasm verify
      - run: ./target/release/gitwasm list
      - run: ./demo/run-demo.sh
      - run: ./target/release/gitwasm audit
```

`gitwasm verify` checks signatures when `.gitwasm/signatures.toml` exists.
`gitwasm audit` re-derives local verdicts that were recorded during the demo.
Replay is a local cache optimization; audit is the proof operation.
```

- [ ] **Step 4: Run docs sanity checks**

Run:

```bash
rg -n "pnpm-lock.yaml|init lockfiles|Replay is a local cache optimization" README.md SPEC.md docs/ci.md
```

Expected:

```text
README.md
SPEC.md
docs/ci.md
```

- [ ] **Step 5: Commit**

```bash
git add README.md SPEC.md docs/ci.md
git commit -m "docs: lead with lockfile conflict workflow"
```

---

### Task 7: Clarify Verdict Replay Versus Audit Proof

**Files:**
- Modify: `README.md`
- Modify: `SECURITY.md`
- Modify: `SPEC.md`
- Modify: `crates/gitwasm/src/commands.rs`

- [ ] **Step 1: Add a failing test for verdict key mismatch**

In `crates/gitwasm/src/commands.rs`, add this assertion near the existing verdict tamper test:

```rust
        let mut wrong_key = recorded.clone();
        wrong_key.key = "0".repeat(64);
        assert!(
            !verdict_matches_lookup_key(&wrong_key, &key),
            "a verdict whose metadata key differs from its lookup key is unusable"
        );
```

Also add this positive assertion before mutating the key:

```rust
        assert!(
            verdict_matches_lookup_key(&recorded, &key),
            "a freshly recorded verdict must match its lookup key"
        );
```

- [ ] **Step 2: Run test and verify it fails**

Run:

```bash
cargo test -p gitwasm commands::tests::merge_verdict_records_rederives_and_catches_tampering -- --nocapture
```

Expected:

```text
error[E0425]: cannot find function `verdict_matches_lookup_key` in this scope
```

- [ ] **Step 3: Add lookup-key validation to replay**

In `crates/gitwasm/src/commands.rs`, add this helper near `replay_merge`:

```rust
fn verdict_matches_lookup_key(verdict: &Verdict, lookup_key: &str) -> bool {
    verdict.key == lookup_key
        && verdict.kind == "merge"
        && verdict.engine == verdict::ENGINE_ID
}
```

Change the cache-hit block in `merge()` from:

```rust
        if let Some(recorded) = store.get(&key)? {
            if let Some(code) = replay_merge(store, &recorded, ours, path, module_name)? {
                return Ok(code);
            }
            // A damaged cache entry falls through to a fresh run.
        }
```

to:

```rust
        if let Some(recorded) = store.get(&key)? {
            if verdict_matches_lookup_key(&recorded, &key) {
                if let Some(code) = replay_merge(store, &recorded, ours, path, module_name)? {
                    return Ok(code);
                }
            } else {
                eprintln!(
                    "gitwasm: note: ignoring malformed verdict {}",
                    &key[..12]
                );
            }
            // A damaged or malformed cache entry falls through to a fresh run.
        }
```

- [ ] **Step 4: Update verdict wording in docs**

In `README.md`, replace any wording that implies replay alone proves honesty with:

```markdown
Replay is a local cache optimization. `gitwasm audit` is the proof step: it
re-runs stored module bytes against stored input blobs and checks that the
recorded outcome reproduces. Future shared verdicts must remain unaudited until
the local host re-derives them or the user explicitly trusts their provenance.
```

In `SECURITY.md`, add this subsection under "What the sandbox does NOT protect you from":

```markdown
4. **Local cache tampering.** Verdict replay is a local cache optimization, not
   proof by itself. A verdict becomes evidence only when `gitwasm audit`
   re-derives it from stored module bytes and input blobs. Future imported
   verdicts must not be treated as replay-eligible proof until audited or
   explicitly trusted.
```

In `SPEC.md`, ensure §8 uses this exact distinction:

```markdown
Replay is an optimization over locally recorded state. Audit is the trust
operation: a host re-runs the module with the stored inputs and accepts the
verdict only if the result reproduces exactly.
```

- [ ] **Step 5: Run tests and docs scan**

Run:

```bash
cargo test -p gitwasm commands::tests::merge_verdict_records_rederives_and_catches_tampering -- --nocapture
rg -n "Replay is a local cache optimization|Audit is the trust operation|Local cache tampering" README.md SPEC.md SECURITY.md
```

Expected:

```text
test commands::tests::merge_verdict_records_rederives_and_catches_tampering ... ok
README.md
SPEC.md
SECURITY.md
```

- [ ] **Step 6: Commit**

```bash
git add crates/gitwasm/src/commands.rs README.md SPEC.md SECURITY.md
git commit -m "docs: clarify verdict replay trust semantics"
```

---

### Task 8: Full Verification

**Files:**
- Verification only.

- [ ] **Step 1: Run formatter**

Run:

```bash
cargo fmt --check
```

Expected:

```text
```

No output and exit code 0.

- [ ] **Step 2: Run clippy**

Run:

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

Expected:

```text
Finished `dev` profile
```

- [ ] **Step 3: Run full workspace tests**

Run:

```bash
cargo test --workspace
```

Expected:

```text
test result: ok
```

All crate test binaries must report zero failures.

- [ ] **Step 4: Run end-to-end Unix demo**

Run:

```bash
./demo/run-demo.sh
```

Expected:

```text
Demo complete - all eight scenarios passed
```

- [ ] **Step 5: Verify committed gitwasm signatures**

Run:

```bash
./target/release/gitwasm verify
```

Expected:

```text
gitwasm: valid signature by
```

- [ ] **Step 6: Inspect final worktree**

Run:

```bash
git status --short --branch
```

Expected:

```text
## main...origin/main
```

The branch line may include an ahead count after the task commits. Existing
unrelated worktree changes from before this plan must not be reverted. Verify
that every file changed by this plan is either committed in one of the task
commits or intentionally left uncommitted because a previous step explicitly
said so.

- [ ] **Step 7: Confirm the task commits are present**

Run:

```bash
git log --oneline -8
```

Expected output includes the task commits created by this plan:

```text
feat: add stock init profiles
feat: add init profiles
feat: add pnpm lockfile merge module
feat: register pnpm stock merge driver
test: add pnpm lockfile demo scenario
docs: lead with lockfile conflict workflow
docs: clarify verdict replay trust semantics
```

---

## Plan Self-Review Notes

- Spec coverage: this plan covers Phase 1 CLI profiles, pnpm support, lockfile-first docs, demo coverage, CI guidance, and verdict wording. Phase 2 and Phase 3 product work are intentionally separate follow-up plans.
- Placeholder scan: the plan contains concrete file paths, commands, expected outputs, and code blocks for code changes.
- Type consistency: the profile enum is named `InitProfile` throughout; the pnpm module package is named `pnpm-lock-merge`; the stock wasm file is `pnpm-lock-merge.wasm`; the profile-filtered stock helpers are `modules_for`, `default_manifest_for`, and `gitattributes_lines_for`.
