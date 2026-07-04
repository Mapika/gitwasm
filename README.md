# gitwasm

**Repos that carry their own behavior.** Git hooks, merge drivers, and (eventually)
CI steps as WebAssembly modules **committed into the repository itself**, executed
in a capability-scoped sandbox on every collaborator's machine — any OS, zero install,
safe to auto-run even from a repo you just cloned from a stranger.

## The problem

Git is the world's most deployed database with no safe way to ship code that governs it.
All of git's extension points — hooks, merge drivers, diff drivers, filters — require
every collaborator to manually install platform-specific tooling. Hooks can't be
committed *by design*, because auto-running arbitrary code from a clone would be a
security disaster. So in practice nobody uses these features, and we all suffer
lockfile conflicts, unenforced conventions, and leaked secrets.

## Why wasm dissolves this

1. **Trust** — a wasm module is sandboxed. The host hands it exactly one directory,
   its argv, and stdout/stderr. No network, no env, no filesystem beyond the mount.
   Auto-running committed code becomes safe *by construction*.
2. **Portability** — one `.wasm` blob runs identically on Windows, macOS, Linux, CI.
   The behavior is versioned with the code it governs: check out a 2-year-old commit
   and you get the merge semantics that commit was written under.

## Layout

```
crates/gitwasm/          host CLI (wasmtime embed): install / hook / merge / run
modules/lockfile-merge/  structural 3-way JSON merge — lockfiles never conflict again
modules/secret-scan/     pre-commit scanner over the staged tree snapshot
demo/run-demo.ps1        end-to-end demo (builds, creates a playground repo, proves both)
```

A consuming repo commits:

```
.gitwasm/
  manifest.toml          maps hooks + merge patterns to modules
  secret-scan.wasm
  lockfile-merge.wasm
.gitattributes           e.g. "package-lock.json merge=gitwasm"
```

and each clone activates once with `gitwasm install` (pure `git config`:
`core.hooksPath` + `merge.gitwasm.driver`).

## Quickstart

```powershell
rustup target add wasm32-wasip1
.\demo\run-demo.ps1
```

The demo proves two things end to end:

- Two branches each add a dependency to `package-lock.json` (adjacent lines —
  guaranteed textual conflict; the control run with `git merge-file` shows it).
  The wasm merge driver merges them cleanly by 3-way merging the JSON structure.
- Committing a file containing an AWS key is blocked by the sandboxed pre-commit
  scanner, which sees only a read-only snapshot of the staged tree.

## How execution works

`gitwasm hook pre-commit` materializes the *staged* tree (what's actually about to be
committed, not the working tree) into a temp dir and mounts it **read-only** as the
module's entire world. `gitwasm merge` mounts a temp dir containing exactly
`base`/`ours`/`theirs`; the module writes `result`; exit 0 means merged, nonzero
means a real conflict is left for the human.

## Roadmap — where this gets big

- **More drivers**: `Cargo.lock`, `yarn.lock`, `poetry.lock`; tree-sitter-based
  semantic merge for source files (tree-sitter already compiles to wasm).
- **Signed manifests**: trust policy for who may change `.gitwasm/` (defense against
  a malicious module swap in a PR).
- **Deterministic, memoized checks**: wasm is deterministic, so every hook run is a
  pure function `hash(module) + hash(staged tree) → verdict`. Results are cacheable
  and *trustlessly shareable* — CI that never re-runs anything anyone has already run,
  a global content-addressed compute cache built on git's own object model.
- **Component-model interface** (`wit`) instead of argv/files, so modules can be
  written in any language with richer typed I/O.
- **Upstream**: the long-term goal is not this tool — it is `.gitwasm/` as an open
  convention that git hosts understand and, eventually, native sandboxed-module
  support in git itself. This repo is the reference implementation.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your
option. Contributions are welcome under the same terms.
