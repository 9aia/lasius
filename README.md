# lasius

Workspace gardener for GitHub orgs — clone, sync, inspect, and generate
editor workspace files for **every repo of an org under one root**.

Named for *Lasius niger*, the black garden ant: it tends the garden of
repos so you don't have to. Generalizes the `ktbw` metarepo CLI (ktbhealth
workspace) into a generic, config-driven Rust tool.

A *workspace* is just a directory with a `lasius.toml`:

```toml
org = "ktbhealth"                        # GitHub org or user
provider = "github"                      # optional, default "github" (only provider today)
repos_dir = "repos"                      # optional, default "repos"
workspace_file = "ktbhealth.code-workspace"  # optional, default "{org}.code-workspace"
exclude = ["secret-*"]                   # optional name globs
include_private = true                   # optional, default false
```

`repos/` holds independent clones — no submodules, each repo keeps its
own history and remotes. `*.code-workspace` is generated, never committed.

## Install

The `lasius` command ships as a [tamias](https://github.com/9aia/tamias)
pack — a generated `bins[]` shim over `tamias-driver-lasius`, no
hand-rolled binary. With `tamias` installed:

```bash
tamias install 9aia:lasius@0.4.0    # org registry (needs TAMIAS_REGISTRY_TOKEN)
```

From a checkout:

```bash
cargo xtask pack                                        # stage the driver payload
cargo install --path crates/lasius-driver               # or: cargo xtask install
tamias install --allow-drivers ./libraries/lasius       # deploy the `lasius` shim
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
lasius vcs worktree add <repo> --branch <b> --rev <sha>  # agent worktree
lasius vcs commit <repo> -m <msg> [--body <text>] [--allow-empty]
lasius vcs push <repo> [--remote origin] [--branch <b>]
lasius vcs fetch <repo> [--remote origin] [--refspec <spec>]...
lasius forge pr create <repo> --title <t> [--body <b>] [--base main] [--head <b>] [--draft]
lasius forge pr merge <repo> <n> [--method squash|merge|rebase] [--delete-branch]
lasius forge pr view|checks|list <repo> [<n>] [--json]  # PR + CI status
lasius serve                   # ndjson JSON-RPC on stdio (ping/list/status/doctor)
lasius completions <shell>     # print a shell completion script
```

All commands accept `--root <dir>`; without it, `lasius.toml` is
discovered upward from the current directory.

`lasius init` scaffolds the whole workspace repo — `lasius.toml`,
`.gitignore` entries for `repos/` and the workspace file, README/AGENTS
stubs — then clones and generates. Everything is idempotent and never
overwrites existing files.

Repo discovery is provider-driven (`provider = "github"` today): `gh repo
list <org> --json name,sshUrl,isArchived,isPrivate`
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
| `crates/lasius-cli` | the `lasius` engine lib — clap commands, process/fs layer (no bin — the command is the tamias pack's `bins[]` shim) |
| `crates/lasius-driver` | `tamias-driver-lasius` — pavo driver: engine ops as `cli.<verb>` wire queries + `run`/`cli` passthroughs (`cli` is the bins-shim engine passthrough; `mcp` feature-gated) |
| `libraries/lasius` | the tamias pack — `tamias.yaml` declares `bins: [{name: lasius, shim: cli, driver: lasius}]`; `cargo xtask pack` stages `drivers/bin/<triple>/tamias-driver-lasius` (gitignored) |
| `xtask` | `cargo xtask check` / `test` / `coverage` / `pack` / `install` / `fmt` |

## Development

```bash
./scripts/setup.sh     # once: git hooks + toolchain check
cargo xtask check      # fmt --check + clippy -D warnings
cargo xtask test       # nextest (or cargo test)
cargo xtask pack       # stage libraries/lasius driver payload
cargo xtask install    # release-build tamias-driver-lasius → ~/.local/bin
cargo xtask coverage   # llvm-cov summary; --fail-under-lines N to gate
```

Git hooks live in `.hooks/` (`core.hooksPath` = `.hooks`): pre-commit
runs `cargo fmt --check`, pre-push runs `cargo xtask check`.

## Conventions

See [AGENTS.md](AGENTS.md) and [CONTRIBUTING.md](CONTRIBUTING.md).
