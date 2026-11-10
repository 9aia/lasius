# lasius.toml

The workspace repo's source of truth. Created by `lasius init`.

```toml
org = "acme"                    # required — GitHub org/user to manage
provider = "github"             # optional — repo provider (default "github"; only provider today)
include_private = false         # optional — clone private repos too (default false)
exclude = ["workspace"]         # optional — glob patterns to skip
repos_dir = "repos"             # optional — clone directory (default "repos")
workspace_file = "acme.code-workspace"  # optional — generated file name
```

Everything except `org` has a default; `init` writes only the fields it
needs and never rewrites an existing file.

## `[checks]` — conformance engine

```toml
[checks]
disabled = ["nested-git"]                          # skip a check id
severity = { "worktree-health" = "warn" }          # downgrade/upgrade
```

Check ids: `gitignore-coverage`, `docs-stubs`, `repos-dir`,
`clone-validity`, `remote-match`, `remote-reachable`, `worktree-health`,
`nested-git`, `workspace-freshness`, `org-drift`, `required-worktrees`,
`required-files`. Severity values: `info`, `warn`, `error`.

`remote-reachable` probes each clone's `origin` with `git ls-remote`
(credential prompts disabled, 20s timeout) and warns when the remote
can't be reached — broken credentials, renamed remotes, lost access,
or an unresponsive host.

## `[policy]` — org conventions

```toml
[policy]
require_worktrees = ["project", "docs"]   # every clone must have these worktrees
require_files = ["AGENTS.md"]             # every clone must contain these files
remote_must_match_org = true              # origin must be org/<name> (error, not warn)
```
