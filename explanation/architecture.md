# Architecture

Rust workspace, ports-and-adapters. `lasius-core` is pure domain (no
I/O); `lasius-cli` is the composition root.

| Crate | Role |
| --- | --- |
| `crates/lasius-core` | `lasius.toml` model, `gh repo list` JSON decoding, repo-name safety + exclude globs, `git status --porcelain=v2` parse/format, `*.code-workspace` generation — pure, unit-tested |
| `crates/lasius-cli` | the `lasius` binary — clap commands, `std::process` for `git`/`gh`/`sh`, `std::fs` for the workspace, `std::thread::scope` parallelism |
| `xtask` | `cargo xtask check` / `test` / `coverage` / `fmt` |

## Data flow

`gh repo list --json` → decode + filter (archived, private, exclude
globs, unsafe names) → `git clone` into `repos_dir` → scan local clones
for status/update/workspace-file generation.

`status`/`update`/`code-workspace generate` iterate the local clones,
not the `gh` list — a manually added clone is managed too, and a
repo deleted upstream still gets pulled until removed.

## Safety

- Repo names from `gh` are validated before they become path segments.
- `rm` refuses dirty trees, unpushed commits, stashes, linked
  worktrees, nested `.git` dirs, and extra remotes unless `--force`.
- Generated files (`*.code-workspace`) and `repos/` are gitignored —
  the workspace repo tracks config and docs only.
