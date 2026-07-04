//! Conventional-commit linter for the commit-msg hook. The gitwasm host
//! copies the commit message into the sandbox as COMMIT_MSG and passes its
//! name as argv[1]. Exit code != 0 aborts the commit.

use std::process::exit;

const TYPES: &[&str] = &[
    "feat", "fix", "docs", "style", "refactor", "perf", "test", "build", "ci", "chore", "revert",
];
const MAX_HEADER: usize = 100;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).map(String::as_str).unwrap_or("COMMIT_MSG");
    let text = std::fs::read_to_string(path).unwrap_or_default();

    let Some(header) = text
        .lines()
        .find(|l| !l.trim().is_empty() && !l.starts_with('#'))
    else {
        fail("empty commit message");
    };

    // git-generated messages pass as-is.
    for pass in ["Merge ", "Revert ", "fixup! ", "squash! "] {
        if header.starts_with(pass) {
            eprintln!("commit-lint: ok ({})", pass.trim());
            return;
        }
    }

    if header.len() > MAX_HEADER {
        fail(&format!(
            "header is {} chars (max {MAX_HEADER})",
            header.len()
        ));
    }

    let Some((prefix, subject)) = header.split_once(": ") else {
        fail("expected 'type(scope)?: subject' (e.g. 'feat(api): add retry')");
    };
    if subject.trim().is_empty() {
        fail("subject is empty");
    }

    let prefix = prefix.strip_suffix('!').unwrap_or(prefix); // breaking-change marker
    let type_part = match prefix.split_once('(') {
        Some((ty, scope)) if scope.ends_with(')') => ty,
        Some(_) => fail("unclosed scope parenthesis"),
        None => prefix,
    };
    if !TYPES.contains(&type_part) {
        fail(&format!(
            "unknown type '{type_part}' (allowed: {})",
            TYPES.join(", ")
        ));
    }
    eprintln!("commit-lint: ok");
}

fn fail(reason: &str) -> ! {
    eprintln!("commit-lint: {reason}");
    eprintln!("commit-lint: commit message must follow https://www.conventionalcommits.org");
    exit(1);
}
