# Real Fork Demo Repo Design

Date: 2026-07-07

## Summary

Create a public example repository under the `gitwasm` GitHub organization
that demonstrates gitwasm on a real, useful codebase instead of a synthetic
sample. The demo should prove the Phase 1 adoption wedge:

> A normal team app can commit `.gitwasm/`, install once, and stop fighting
> generated lockfile conflicts.

The recommended upstream is `LinMoQC/Magic-Resume`, a MIT-licensed Next.js
and pnpm application that can run fully in the browser without accounts,
servers, databases, or secrets. It is substantial enough to look like a real
project, but its self-hosted mode keeps the demo reproducible for local users,
CI, and conference-style walkthroughs.

Target repository:

```text
gitwasm/magic-resume-gitwasm-demo
```

The repository should preserve upstream attribution and license terms, add a
small `GITWASM.md` walkthrough, commit gitwasm's generated merge-driver assets,
and include a script that creates a real `pnpm-lock.yaml` conflict and shows
gitwasm resolving it cleanly.

## Goals

- Publish a real fork/adaptation under the `gitwasm` team.
- Keep the app runnable without external services or secrets.
- Commit `.gitwasm/`, `.gitattributes`, and any required gitwasm metadata.
- Document the normal clone path: install gitwasm, run `gitwasm install`, use
  the app normally.
- Add a conflict demo that creates two dependency-change branches, produces a
  `pnpm-lock.yaml` conflict, and resolves it through gitwasm.
- Add CI that verifies the app, verifies gitwasm metadata, and runs the
  conflict demo.
- Make the repo a credible artifact to link from the gitwasm README and launch
  material.

## Non-Goals

- Do not build a new application UI.
- Do not change Magic Resume's product behavior except for minimal demo
  documentation or local-only scripts.
- Do not require hosted services, API keys, databases, or account setup.
- Do not use this repo to introduce gitwasm hooks or policy modules.
- Do not market verdict sharing as part of this demo. Local verification and
  audit are enough.

## Upstream Selection

Use `LinMoQC/Magic-Resume` as the first demo candidate.

Reasons:

- It is a real end-user app rather than a starter template.
- It uses pnpm and has a `pnpm-lock.yaml`, which directly exercises the
  Phase 1 pnpm lockfile work.
- It is self-hostable and does not require a backend for the core demo.
- It is MIT-licensed, which is compatible with a public demo fork as long as
  license and attribution are preserved.
- The tech stack is familiar to the target audience: Next.js, TypeScript,
  Tailwind CSS, pnpm, and Turborepo.

Fallbacks if the upstream proves unsuitable during implementation:

- `theodorusclarence/ts-nextjs-tailwind-starter`: smaller and reliable, but
  less compelling because it is intentionally a starter.
- `andrechandra/next-tailwind-starter`: simple pnpm/Next.js app, but also more
  starter-like than product-like.
- `josedab/prflow`: real pnpm/Turborepo app, but likely heavier and less
  directly understandable as a local demo.

## Repository Strategy

The implementation should create or fork:

```text
gitwasm/magic-resume-gitwasm-demo
```

Prefer a GitHub fork if organization permissions make that straightforward.
If a normal fork is blocked by GitHub organization settings, create a fresh
repository that imports the upstream history or includes clear upstream
attribution in the README and license files. In either case:

- Preserve the original `LICENSE`.
- Preserve upstream author attribution in `README.md` or a dedicated
  `UPSTREAM.md`.
- Keep the upstream remote configured locally during implementation so future
  syncs are possible.
- Keep gitwasm changes small and reviewable in one or two commits on top of
  the imported code.

## Gitwasm Integration

The demo repository should contain the repo-committed behavior that a team
would actually review:

```text
.gitattributes
.gitwasm/
GITWASM.md
scripts/gitwasm-conflict-demo.sh
.github/workflows/gitwasm-demo.yml
```

Expected setup:

- Run the current gitwasm CLI from the latest `origin/main` or a local release
  build.
- Initialize only lockfile-oriented merge behavior.
- Ensure `.gitattributes` maps `pnpm-lock.yaml` to `merge=gitwasm`.
- Commit the generated WASM module, manifest, and signatures.
- Run `gitwasm verify` before committing.

The demo should not enable hooks. The message stays narrow: generated-file
merges first.

## Conflict Demo Script

Add:

```text
scripts/gitwasm-conflict-demo.sh
```

The script should be safe to run from a normal checkout. It should avoid
leaving the user's working tree dirty by operating in a temporary clone or
disposable worktree.

Preferred flow:

1. Create a temporary clone of the current repository.
2. Run `gitwasm install` in that clone.
3. Create a base branch.
4. Create branch `demo-left` and make a legitimate pnpm dependency change.
5. Run `pnpm install --lockfile-only` or `pnpm add --lockfile-only` so
   `pnpm-lock.yaml` changes naturally.
6. Create branch `demo-right` from the same base and make a different
   legitimate dependency change.
7. Attempt to merge `demo-right` into `demo-left`.
8. Show that gitwasm handles `pnpm-lock.yaml` through the committed merge
   driver.
9. Run `pnpm install --frozen-lockfile` or `pnpm install --lockfile-only
   --frozen-lockfile` equivalent validation after the merge.
10. Print the resulting dependency changes and the gitwasm verdict/audit
    command users can inspect.

If real dependency installation makes the demo slow or flaky, the script may
use deterministic package edits plus `pnpm install --lockfile-only`. It should
not hand-edit YAML as the primary path unless pnpm itself is unavailable.

The script should fail loudly when prerequisites are missing:

- `git`
- `gitwasm`
- `corepack`
- `pnpm`

## Documentation

Add `GITWASM.md` with these sections:

- What this repository demonstrates.
- How this differs from normal Git merge drivers.
- How to install gitwasm.
- How to activate repo behavior with `gitwasm install`.
- How to run the pnpm conflict demo.
- How to inspect verification and audit output.
- How to remove local activation if desired.

Update the demo repository `README.md` with a short "gitwasm demo" note near
the top, but keep the original product documentation intact. The README should
link to `GITWASM.md` rather than becoming a gitwasm manual.

## CI

Add `.github/workflows/gitwasm-demo.yml`.

The workflow should prove three things:

- The app still installs and builds in its normal package-manager flow.
- The committed gitwasm assets verify.
- The scripted pnpm conflict demo runs from a clean checkout.

Expected jobs:

```text
app:
  - checkout
  - setup Node.js
  - corepack enable
  - pnpm install --frozen-lockfile
  - pnpm lint or equivalent, if available
  - pnpm build or equivalent, if available

gitwasm:
  - checkout
  - install or build gitwasm
  - gitwasm verify
  - scripts/gitwasm-conflict-demo.sh
```

Implementation can combine these jobs if that makes dependency caching simpler.
The final workflow must remain understandable to someone inspecting the demo
for the first time.

## Verification

Before calling the demo complete:

- Clone the demo repo into a fresh directory.
- Run `corepack enable`.
- Run `pnpm install --frozen-lockfile`.
- Run the app's available lint, test, or build commands.
- Install gitwasm from the selected binary or local build.
- Run `gitwasm install`.
- Run `gitwasm verify`.
- Run `scripts/gitwasm-conflict-demo.sh`.
- Confirm CI passes on GitHub.

## Risks

- GitHub organization permissions may not allow creating a fork directly under
  `gitwasm`. A fresh imported repo with preserved attribution is acceptable.
- Upstream may require a newer Node or pnpm version than the local machine has.
  Use Corepack and pin versions according to upstream metadata.
- The app may have slow or flaky build steps. The demo can focus on install and
  lockfile validation if full production build is impractical, but the reason
  should be documented.
- The conflict script can become too magical. Keep each Git branch and merge
  step visible in the output.
- `pnpm-lock.yaml` behavior must be honest. If gitwasm refuses a genuinely
  ambiguous merge, the script should report that rather than hiding it.

## Success Criteria

- `gitwasm/magic-resume-gitwasm-demo` exists publicly.
- Upstream license and attribution are preserved.
- `.gitwasm/` and `.gitattributes` are committed and reviewable.
- `gitwasm verify` passes in a fresh clone.
- The demo script creates and resolves a real `pnpm-lock.yaml` merge scenario.
- CI passes on the public repo.
- The main gitwasm README can link to the demo as a concrete adoption example.
