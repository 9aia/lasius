# Bootstrap a workspace repo

Goal: turn an empty directory into a managed workspace for a GitHub org.

## Steps

1. Create the directory and an empty git repo:

   ```bash
   mkdir acme && cd acme && git init
   ```

2. Run init with the org name:

   ```bash
   lasius init --org acme
   ```

   Without `--org`, lasius infers the owner from the `origin` remote —
   so `git remote add origin git@github.com:acme/workspace.git` first if
   you want that.

3. Review what was created:

   - `lasius.toml` — `org`, `repos_dir`, `workspace_file`, `exclude`,
     `include_private`
   - `.gitignore` — `/repos/` and the generated workspace file
   - `README.md` / `AGENTS.md` — stubs (edit freely; never overwritten)
   - `repos/` — one clone per non-archived org repo
   - `<org>.code-workspace` — generated, gitignored

4. Commit the config and push:

   ```bash
   git add -A && git commit -m "chore: lasius workspace" && git push -u origin main
   ```

## Verify

```bash
lasius doctor   # all checks ok, zero drift
```
