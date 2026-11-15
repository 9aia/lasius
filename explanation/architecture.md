# Architecture

Rust workspace, ports-and-adapters. `lasius-core` is pure domain (no
I/O); `lasius-cli` is the composition root.

<!-- crate-map -->
| crate | layer | purpose |
|---|---|---|
| `lasius-core` | domain | Pure domain logic for lasius — config model, gh decoding, git status, workspace files |
| `lasius-forge` | port | Forge port — pull requests, merges, and check status, generic over the backend. |
| `lasius-vcs` | port | VCS port — history/worktree/tag/sync ops, generic over the backend (P-019). |
| `lasius-forge-gh` | adapter | GitHub impl of lasius-forge — shells out to the gh CLI. |
| `lasius-vcs-git` | adapter | Git impl of lasius-vcs — shells out to the git binary. |
| `lasius` | engine | lasius — the engine as a library. Everything that used to live in the binary lives here: `Cli`/`Cmd`, workspace resolution, verb dispatch. |
| `lasius-driver` | driver | tamias-driver-lasius — pavo driver exposing a lasius workspace's engine ops (no views) |
| `lasius-xtask` | tooling | lasius-xtask — workspace automation: check / test / coverage / pack / install / fmt. |

The `lasius` command itself ships as the tamias pack in
`libraries/lasius/` (`tamias.yaml` declares
`bins: [{name: lasius, shim: cli, driver: lasius}]`;
`cargo xtask pack` stages the driver payload, gitignored).

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
