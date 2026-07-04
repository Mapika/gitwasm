# Security model

gitwasm's core claim is: **it is safe to run behavior committed to a repo you
do not trust.** That claim deserves precision. This document says exactly
what the sandbox guarantees, what it does not, and what remains your problem.

## What a module can do

A module executes under wasmtime with, at most:

- read (hooks) or read/write (merge drivers) access to **one temporary
  directory** containing copies of the specific inputs for that run — never
  your working tree, never your home directory, never the `.git` directory;
- write access to your terminal via stdout/stderr;
- a bounded amount of CPU (fuel metering) and memory (linear-memory cap),
  configurable in the manifest, so a hostile module cannot even spin your CPU
  or exhaust your RAM.

## What a module cannot do

No network. No environment variables. No filesystem outside the mount. No
spawning processes. No reading your SSH keys, your browser profile, or the
rest of the repo. These are not policies — the capabilities simply do not
exist inside the sandbox (WASI capability model + wasmtime enforcement).

## What the sandbox does NOT protect you from

Honesty matters more than marketing here:

1. **Malicious verdicts.** A hostile merge driver can produce a *wrong merge
   result*; a hostile hook can block your commits or let bad ones pass. The
   sandbox contains the blast radius to the repo's own content — which is
   already fully controlled by whoever writes to the repo. Changes to
   `.gitwasm/` are ordinary committed files: review them in PRs like any code.
2. **Terminal output.** Modules write to your terminal; treat their output as
   untrusted text (the host does not interpret escape sequences for you yet —
   sanitization is planned).
3. **wasmtime bugs.** The sandbox is as strong as wasmtime's isolation, which
   is industry-grade and fuzzed, but not a mathematical guarantee.
4. **Activation is explicit by design.** Nothing runs on `git clone`. Until
   you run `gitwasm install`, a cloned repo's modules are inert bytes.

## Planned hardening (before 1.0)

- **Signed manifests**: a repo-local trust policy over who may change
  `.gitwasm/`, so a drive-by PR swapping a module is cryptographically
  detectable rather than merely diff-visible.
- Escape-sequence sanitization of module stdout/stderr.
- Wall-clock deadline in addition to fuel.

## Reporting

Please report suspected sandbox escapes or contract violations privately to
the maintainers (see repository owners) before public disclosure.
