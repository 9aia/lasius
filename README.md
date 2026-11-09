# lasius

Workspace gardener for GitHub orgs — clone, sync, inspect, and generate
editor workspace files for **every repo of an org under one root**.

Named for *Lasius niger*, the black garden ant: it tends the garden of
repos so you don't have to. Generalizes the `ktbw` metarepo CLI (ktbhealth
workspace) into a generic, config-driven Rust tool.

A *workspace* is just a directory with a `lasius.toml`:

```toml
org = "ktbhealth"                        # GitHub org or user
repos_dir = "repos"                      # optional, default "repos"
workspace_file = "ktbhealth.code-workspace"  # optional, default "{org}.code-workspace"
exclude = ["secret-*"]                   # optional name globs
include_private = true                   # optional, default false
```

`repos/` holds independent clones — no submodules, each repo keeps its
own history and remotes. `*.code-workspace` is generated, never committed.

## Install

```bash
cargo install lasius
# or from source:
cargo install --path crates/lasius-cli
```

Requires `git` and `gh` (authenticated) on PATH.

## Usage

```bash
cd my-org-workspace

lasius init                    # scaffold the workspace repo + clone all org repos + generate
lasius clone                   # clone all missing org repos
lasius update [--json]         # git pull --ff-only everywhere (alias: sync)
lasius status                  # branch / dirty / ahead-behind table (--json for machines)
lasius code-workspace generate [--json]  # regenerate the *.code-workspace file
lasius open                    # open the workspace file in your editor
lasius doctor [--json]         # env + drift diagnostics
lasius check [--json] [--only id]  # conformance checks (structure/clones/generated/policy)
lasius exec -- cargo build     # run a command in every clone (--filter <substr>)
lasius add <repo>              # clone a single org repo
lasius rm <repo>               # remove a clone after safety checks
lasius serve                   # ndjson JSON-RPC on stdio (ping/list/status/doctor)
lasius completions <shell>     # print a shell completion script
```

All commands accept `--root <dir>`; without it, `lasius.toml` is
discovered upward from the current directory.

`lasius init` scaffolds the whole workspace repo — `lasius.toml`,
`.gitignore` entries for `repos/` and the workspace file, README/AGENTS
stubs — then clones and generates. Everything is idempotent and never
overwrites existing files.

Repo discovery is automatic: `gh repo list <org> --json name,sshUrl,isArchived,isPrivate`
(non-archived only; private repos are skipped unless `include_private =
true`, `exclude` globs applied, unsafe names rejected before they can
become paths). Archive a repo on GitHub to exclude it. The workspace
repo itself is auto-detected from the root's `origin` remote and never
cloned — no `exclude` entry needed.

`lasius serve` speaks ndjson JSON-RPC on stdio — the same wire as
`tamias serve` / `sepia serve` / `cuora serve` / `myco serve`. Requests
are `{"op":"…"}`; responses are `{"ok":true,"result":…}` or
`{"ok":false,"error":"…"}`. Ops: `ping`, `list`, `status`, `doctor`
(read-only — mutating commands print progress and stay CLI-side).

## Layout

Rust workspace, ports-and-adapters. `lasius-core` is pure domain (no
I/O); `lasius-cli` is the composition root — `std::process` for `git`/`gh`,
`std::fs` for the workspace, scoped threads for parallel repo ops.

| Crate | Role |
| --- | --- |
| `crates/lasius-core` | config model, `gh` JSON decoding, repo-name safety, git status parse/format, workspace-file generation — pure, unit-tested |
| `crates/lasius-cli` | the `lasius` binary — clap commands, process/fs layer, e2e tests |
| `xtask` | `cargo xtask check` / `test` / `coverage` / `fmt` |

## Development

```bash
./scripts/setup.sh     # once: git hooks + toolchain check
cargo xtask check      # fmt --check + clippy -D warnings
cargo xtask test       # nextest (or cargo test)
cargo xtask coverage   # llvm-cov summary; --fail-under-lines N to gate
```

Git hooks live in `.hooks/` (`core.hooksPath` = `.hooks`): pre-commit
runs `cargo fmt --check`, pre-push runs `cargo xtask check`.

## Conventions

See [AGENTS.md](AGENTS.md) and [CONTRIBUTING.md](CONTRIBUTING.md).
