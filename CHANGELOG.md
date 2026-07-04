# Changelog

## 0.3.0 — 2026-07-05

### Added

- **`yarn-lock-merge` module** (yarn.lock v1, stock pattern): atomic-block
  3-way merge keyed by descriptor line; higher version wins on concurrent
  bumps; refuses yarn berry (v2+) files rather than corrupting them;
  parse→serialize round-trips byte-identically.
- **`poetry-lock-merge` module** (poetry.lock, stock pattern): `name@version`
  entry-set merge; merge-introduced duplicates collapse to the higher
  version; conflicting `metadata.content-hash` takes theirs with a
  `poetry lock --no-update` warning; marker-based multi-version package sets
  survive intact.

### Known papercuts

- `gitwasm <cmd> | head`-style early pipe closure makes the CLI panic on
  broken pipe instead of exiting quietly.

## 0.2.0 — 2026-07-05

The trust release.

### Added

- **Signed manifests**: `gitwasm keygen` / `sign` / `verify` / `trust`.
  ed25519 signatures over every file in `.gitwasm/` — including the hook
  shims git executes natively. `install` pins signers per clone (TOFU);
  every subsequent hook/merge run verifies **fail-closed**. (SPEC.md §6)
- **Wall-clock deadline** on module runs (`limits.wall_ms`, default 60s) —
  catches stalls that fuel metering can't.
- **Output sanitization**: module stdout/stderr is captured and stripped of
  control/escape bytes before reaching the terminal.
- **`lineset-merge` module**: set-algebra 3-way merge for line-set files;
  `go.sum` is a stock pattern.
- **`package.json` is now a stock merge pattern** for `lockfile-merge` —
  validated on a real repo where npm accepted the merged result with zero
  rewrites.
- `.gitwasm/** -text` gitattributes line, so EOL conversion can never break
  signature hashes across platforms.
- Demo scenario 5: tamper with a signed module blob → execution refused.

## 0.1.0 — 2026-07-05

Initial release: `gitwasm init/install/list/hook/merge/run`; sandboxed
(wasmtime) WASI modules committed in-repo; fuel + memory limits; stock
modules `lockfile-merge`, `cargo-lock-merge`, `secret-scan`, `commit-lint`;
SPEC/SECURITY/CONTRIBUTING; 3-OS CI with end-to-end demos.
