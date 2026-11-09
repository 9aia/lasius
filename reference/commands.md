# Commands

All commands accept `--root <dir>`; without it, `lasius.toml` is
discovered upward from the current directory.

| Command | Description |
| --- | --- |
| `lasius init [--org <org>]` | Scaffold the workspace repo, clone all org repos, generate the workspace file |
| `lasius clone` | Clone all missing org repos into `repos_dir` |
| `lasius update [--json]` (`sync` alias) | `git pull --ff-only` every clone |
| `lasius status [--json]` | Branch / dirty / ahead-behind table per clone |
| `lasius code-workspace generate [--json]` | Regenerate the `*.code-workspace` file |
| `lasius open [--editor <cmd>]` | Open the workspace file (`$VISUAL`/`$EDITOR`/`code`/`zed`/`cursor`/`vi`) |
| `lasius doctor [--json]` | Environment + drift diagnostics; exit 1 on any failure |
| `lasius check [--domain d] [--only ids] [--json]` | Conformance findings — structure/clones/generated/policy; exit 1 on errors |
| `lasius exec [--filter <substr>] -- <cmd>` | Run a command in every (matching) clone |
| `lasius add <repo>` | Clone a single org repo by name |
| `lasius rm <repo> [--yes] [--force]` | Remove a clone after safety checks |
| `lasius serve` | ndjson JSON-RPC on stdio (see [wire reference](/reference/serve)) |
| `lasius completions <shell>` | Print a shell completion script |

`check` reads `[checks]` (disabled ids, severity overrides) and `[policy]`
(require_worktrees, require_files, remote_must_match_org) from `lasius.toml` —
see [config reference](/reference/config).
