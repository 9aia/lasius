+++
id = "P-002"
title = "lasius check: conformance engine for workspace repos"
status = "done"
created = "2026-11-09"
owner = "devin"
priority = "high"
labels = ["tool"]
+++

## Context
`lasius doctor` checks environment + drift. Missing: does the workspace
*follow the lasius pattern* — config validity, gitignore coverage,
clone hygiene, worktree health, org conventions. Proposal: a composable
check engine.

## Design

One engine, five domains — `env | structure | clones | generated |
policy`. `lasius doctor` becomes `check --domain env` internally (CLI
compat kept); no duplicated logic.

- `lasius-core/src/checks/` — pure: checks are functions over an
  injected `CheckContext` (config + facts gathered once by cli) returning
  `Vec<Finding>`.
- `Finding { check: &'static str (kebab id), severity, repo: Option,
  message, fix: Option<FixId> }` — deterministic sort (check, repo).
- cli gathers facts once (local repos, parallel git probes via par_map,
  org list) — checks never re-scan; adding a check is O(1) plumbing.
- `--fix` resolves FixIds to safe auto-remediations in cli only
  (gitignore entries, workspace regen). Checks never mutate.

## Config (lasius.toml)

```toml
[checks]
disabled = ["nested-git"]
severity = { "worktree-health" = "warn" }

[policy]
require_worktrees = ["project", "docs"]
require_files = ["AGENTS.md"]
remote_must_match_org = true
```

Policy checks are data-driven — orgs express conventions without code.

## CLI

`lasius check [--domain …] [--only …] [--json] [--fix]` — table default,
non-zero exit on Error severity (CI/pre-push usable). `serve` gains a
`check` op later (read-only wire already exists).

## Explicitly out of scope

- Plugin/external check binaries — YAGNI; serve wire leaves the door open.
- Async runtime — stays sync + scoped threads.

## Phasing

1. MVP: structure + clones + generated checks, table/--json
2. `[policy]` engine + `--fix`
3. `serve check` op
