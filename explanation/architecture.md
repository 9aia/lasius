# Architecture

Rust workspace, ports-and-adapters. `lasius-core` is pure domain (no
I/O); `lasius-cli` is the composition root.

| Crate | Role |
| --- | --- |
| `crates/lasius-core` | `lasius.toml` model (incl. `provider`), provider kind, `gh repo list` JSON decoding, failure classification (`access-denied:`/`not-found:`), repo-name safety + exclude globs, `git status --porcelain=v2` parse/format, `*.code-workspace` generation — pure, unit-tested |
| `crates/lasius-cli` | the `lasius` engine lib — clap commands, `RepoProvider` seam (`providers.rs`, github via `gh` today), `std::process` for `git`/`gh`/`sh`, `std::fs` for the workspace, `std::thread::scope` parallelism. **No bin** — the `lasius` command is the tamias pack's generated `bins[]` shim → `tamias-driver-lasius cli` → `lasius::run_from` (T-012) |
| `crates/lasius-driver` | `tamias-driver-lasius` — pavo driver: engine ops as `cli.<verb>` wire queries + `run`/`cli` engine passthroughs, `serve` ndjson loop, feature-gated `mcp` |
| `libraries/lasius` | the tamias pack — `tamias.yaml` (`type: files`, `bins: [{name: lasius, shim: cli, driver: lasius}]`); `cargo xtask pack` stages `drivers/bin/<triple>/tamias-driver-lasius` (gitignored) |
| `xtask` | `cargo xtask check` / `test` / `coverage` / `pack` / `install` / `fmt` |

## Data flow

Provider discovery (`provider = "github"` → `gh repo list --json`) →
decode + filter (archived, private, exclude globs, unsafe names) →
`git clone` into `repos_dir` → scan local clones for
status/update/workspace-file generation.

`status`/`update`/`code-workspace generate` iterate the local clones,
not the provider's repo list — a manually added clone is managed too,
and a repo deleted upstream still gets pulled until removed.

Per-repo failures never abort a batch: `clone`/`update`/`status`
collect them and report at the end, with stderr classified
(`access-denied:`, `not-found:`) so credential and permission problems
are obvious. `check` adds `remote-reachable`, which probes each
clone's `origin` via `git ls-remote` with credential prompts disabled
and a 20s timeout, so a hung network can't stall the run.

## Safety

- Repo names from `gh` are validated before they become path segments.
- `rm` refuses dirty trees, unpushed commits, stashes, linked
  worktrees, nested `.git` dirs, and extra remotes unless `--force`.
- Generated files (`*.code-workspace`) and `repos/` are gitignored —
  the workspace repo tracks config and docs only.
