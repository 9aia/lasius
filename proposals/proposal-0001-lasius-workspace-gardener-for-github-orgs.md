+++
id = "P-001"
title = "lasius: workspace gardener for GitHub orgs"
status = "in-progress"
created = "2026-11-09"
owner = "devin"
priority = "high"
labels = ["tool"]
+++

## Context
Org metarepo pattern needed a generic tool: manage every repo of a
GitHub org under one root (clone/sync/status/editor workspace files).
Born as ktbw (ktbhealth, TS/Effect), generalized to Rust as `lasius`
(*Lasius niger*, the black garden ant).

## Scope
- `lasius-core` — pure domain: lasius.toml, gh decode, name safety,
  git status parse, workspace-file gen.
- `lasius-cli` — init/clone/update/status/code-workspace generate/
  doctor/open/rm/exec/add/serve/completions.
- `serve` — ndjson JSON-RPC (ping/list/status/doctor), same wire as
  tamias/sepia/cuora/myco serve.
- Workspace repos: ktbhealth/workspace + 9aia/workspace are
  config-only (`lasius.toml`); clones live in gitignored `repos/`.

## Status
Shipped: github.com/9aia/lasius, both workspace repos live.
Pending: crates.io publish, docs site (deferred).
