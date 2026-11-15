# Concepts

lasius's vocabulary is about *the workspace as a unit* — one root
directory that owns every clone of an org.

## Workspace entities

- **Workspace root** — the directory containing `lasius.toml`;
  everything lasius manages hangs off it. The workspace repo
  itself is a normal git repo tracking config and docs only.
- **`repos_dir`** — where clones live (`repos/` by default,
  gitignored). Independent clones — no submodules, each keeps its
  own history and remotes.
- **Clone** — a local checkout of one org repo under `repos_dir`.
  `status`/`update`/`code-workspace generate` iterate *local
  clones*, not the provider's repo list — a manually added clone
  is managed too.
- **Provider** — how org repos are discovered
  (`provider = "github"` → `gh repo list --json`). Archived repos
  are excluded; private ones opt in.
- **`lasius.toml`** — the whole config: `org`, `repos_dir`,
  `workspace_file`, `provider`, `exclude` globs,
  `include_private`.
- **Workspace file** — the generated `*.code-workspace` multi-root
  file editors open; gitignored, regenerated on demand.

## Filters and safety

- **`exclude`** — glob patterns matched against repo names;
  matched repos are never cloned.
- **`include_private`** — opt-in flag; private repos join
  discovery (requires authenticated `gh`).
- **Safety checks** — `rm` refuses dirty trees, unpushed commits,
  stashes, linked worktrees, nested `.git` dirs, and extra remotes
  unless `--force`. Repo names from `gh` are validated before
  they become path segments.

## Ports

- **`lasius-vcs`** — VCS port (history/worktree/tag/sync +
  agent-mutation ops) shared by tarsi/mimus/bupha/kuda/sturn
  consumers (P-019); `lasius-vcs-git` shells out to `git`.
- **`lasius-forge`** — forge port (PRs, merges, checks, commit
  statuses); `lasius-forge-gh` shells out to `gh`.

## Surfaces

`lasius` CLI for humans and scripts; `lasius serve` speaks ndjson
JSON-RPC read-only ops (`ping`, `list`, `status`, `doctor`) for
embedding — the org wire convention shared with tamias, sepia,
cuora, and myco; `lasius-driver` exposes the same ops to pavo.
