# lasius.toml

The workspace repo's source of truth. Created by `lasius init`.

```toml
org = "acme"                    # required — GitHub org/user to manage
include_private = false         # optional — clone private repos too (default false)
exclude = ["workspace"]         # optional — glob patterns to skip
repos_dir = "repos"             # optional — clone directory (default "repos")
workspace_file = "acme.code-workspace"  # optional — generated file name
```

Everything except `org` has a default; `init` writes only the fields it
needs and never rewrites an existing file.
