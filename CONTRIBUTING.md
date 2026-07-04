# Contributing to gitwasm

Thank you — module contributions are the whole point of this project. The
long-term goal is `.gitwasm/` as an open convention (see SPEC.md), and every
new module or second implementation strengthens it.

## Building

```
rustup target add wasm32-wasip1
cargo build --release -p gitwasm     # build.rs compiles + embeds stock modules
cargo test --workspace
./demo/run-demo.sh                   # or demo\run-demo.ps1 on Windows
```

## Writing a module

A module is any WASI preview1 command program (`wasm32-wasip1` target). Read
SPEC.md §4 for the exact contract; `modules/commit-lint` is the smallest
example (~80 lines, zero dependencies). In short:

- **hook module**: scan the mounted staged tree, exit nonzero to block;
- **merge module**: read `base`/`ours`/`theirs`, write `result`, exit 0.

Keep modules deterministic (no clocks, no randomness) and dependency-light —
the blob ships inside user repositories, so size is a feature.

## Ground rules

- Conventional commit messages (`feat:`, `fix:`, ...). Yes, the repo
  dogfoods its own commit-lint module.
- New behavior needs a test or a demo scenario that proves it end-to-end.
- Security-relevant changes (anything touching `runner.rs` or the sandbox
  contract) get extra scrutiny; explain the capability impact in the PR.

## License

Dual MIT/Apache-2.0. By contributing you agree your work is licensed the
same way.
