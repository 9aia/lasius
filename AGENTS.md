# lasius

Workspace gardener for GitHub orgs — clone, sync, inspect, and generate
editor workspace files for every repo of an org under one root. Named
for *Lasius niger*, the black garden ant — it tends the garden.

Rust workspace, ports-and-adapters. `lasius-core` is pure domain (no
I/O); `lasius-cli` is the composition root.

## Layout

| Crate | Role |
| --- | --- |
| `crates/lasius-core` | `lasius.toml` model (incl. `provider`), `gh repo list` JSON decoding, failure classification, repo-name safety + exclude globs, `git status --porcelain=v2` parse/format, `*.code-workspace` generation — pure, unit-tested |
| `crates/lasius-vcs` | VCS port — union trait (history/worktree/tag/sync ops) shared by tarsi/mimus/bupha/kuda/sturn consumers (P-019) |
| `crates/lasius-vcs-git` | git impl of `lasius-vcs` — shells out to the `git` binary |
| `crates/lasius-cli` | the `lasius` binary — clap derive, `std::process` for `git`/`gh`/`sh`, `std::fs` for the workspace, `std::thread::scope` parallelism, `providers.rs` `RepoProvider` seam. `init` scaffolds the workspace repo (`lasius.toml`, `.gitignore`, doc stubs) then clones + generates. E2e tests spawn the real binary against fake `gh` + `file://` git repos |
| `xtask` (`lasius-xtask`) | `check` (fmt + clippy -D warnings) / `test` (nextest or cargo test) / `coverage` (llvm-cov) / `fmt` |

## Commands

```bash
cargo xtask check          # fmt --check + clippy -D warnings
cargo xtask test           # whole suite (nextest if present)
cargo xtask coverage       # llvm-cov summary; extra args pass through
cargo xtask fmt            # cargo fmt --all
```

Git hooks live in `.hooks/` (`core.hooksPath` = `.hooks`):
pre-commit runs `cargo fmt --all -- --check`, pre-push runs
`cargo xtask check`. `./scripts/setup.sh` wires them on fresh clones.

## Boundary

Lasius owns **git & workspace expertise** — repo discovery,
clone/pull, status, editor workspace files, safe removal, plus the
shared `lasius-vcs` port other repos consume (P-019). It does not own:

- **myco** — the org work ledger; `project/` worktrees are managed by myco verbs, never by lasius.
- **kuda** — code maps; per-repo `.kuda/` cards are that repo's concern.
- **sturn** — ephemeral coordination; lasius doesn't claim files.
- **per-repo tooling** — each clone's own build/test/dev flow is its own `AGENTS.md`'s business. `lasius exec` is the escape hatch, not a task runner.

## Rules

- Workspace lints are shared (`[workspace.lints]`): `unsafe_code`
  forbidden, `unwrap_used` warned — `-D warnings` in `xtask check` means
  no warnings tolerated. Tests `#![allow]` liberally.
- No `unsafe`. Edition 2021.
- Keep the binary dependency-light and sync; parallelism via scoped
  threads, no async runtime.
- `lasius-core` stays pure — no `std::process`/`std::fs`/`std::net`.
  IO lives in `lasius-cli`.
- Repo names from `gh` are validated (`names::is_safe_repo_name`)
  before they become path segments — never bypass it.
- English everywhere (code, comments, docs, commit messages).
- Commits: conventional prefixes (`feat:`/`fix:`/`refactor:`/`docs:`/`chore:`).

## Docs

`README.md` is the user-facing quickstart. This file is the agent/dev
map. Far more detail than either belongs in crate-level `//!` docs —
keep `lib.rs`/`main.rs` module docs accurate; they feed generated
reference docs.

<!-- myco:agent-awareness -->

## Agent awareness

- Session start / any work discussion: run `myco status --prompt`
  before proposing next steps — know the board first.
- User describes new work → offer to capture it (`myco todo add`,
  `issue new`, `proposal new`). Don't wait to be asked.
- Picking up a task: `myco resume <ID>` is the briefing.
- Wrapping up: `myco handoff <ID>` anything resumable; keep statuses
  honest (claimed → in-progress → blocked/done).

<!-- /myco:agent-awareness -->

<!-- sturn:agent-awareness -->

## Sturn awareness

- **No PRs.** The pushed branch/commit is the deliverable; review
  streams continuously (sturn WAL), merge is a human/sturn call.
- sturn-wired repo (`sturn.toml` present): `sturn claim "<glob>"
  --ws <ws>` *before* edits — claims surface conflicts in ms.
- Ops via `sturn write|rename|delete`; `sturn snap` folds the ws
  into a real commit; `sturn push` publishes heads + refs/sturn/*.
- `sturn leave` on exit — drops the actor's claims.

<!-- /sturn:agent-awareness -->

<!-- kuda:agent-awareness -->

## Kuda awareness

- `.kuda/` cards are committed artifacts — read them before
  re-exploring (`kuda map`, `kuda card <mod>`); `kuda status`
  reports per-card drift vs HEAD.
- Code changes that invalidate a card → regenerate it (`kuda
  sync`) in the same change; cards ride the diff they describe.
- `.kuda/index.sqlite` is gitignored + rebuildable; the `*.yaml`
  cards are the checked-in source of truth.

<!-- /kuda:agent-awareness -->
