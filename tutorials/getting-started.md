# Getting started

## Install

The `lasius` command ships as a [tamias](https://github.com/9aia/tamias)
pack — a generated `bins[]` shim over `tamias-driver-lasius`. With
`tamias` installed:

```bash
tamias install 9aia:lasius@0.3.0    # org registry (needs TAMIAS_REGISTRY_TOKEN)
```

Or from a checkout:

```bash
cargo xtask pack                                  # stage the driver payload
tamias install --allow-drivers ./libraries/lasius # deploy the `lasius` shim
```

(Or build the driver yourself — `cargo xtask install` / `cargo install
--path crates/lasius-driver` — then run the same `tamias install`.)

Requires `git` and the [GitHub CLI](https://cli.github.com/) (`gh`),
authenticated (`gh auth login`).

## Bootstrap a workspace

Turn an empty directory into a managed workspace for a GitHub org:

```bash
mkdir myorg-workspace && cd myorg-workspace
git init && git remote add origin git@github.com:MYORG/workspace.git
lasius init --org MYORG
```

Without `--org`, lasius infers the owner from the `origin` remote —
so add the remote first if you want that.

`lasius init` scaffolds the whole workspace repo — `lasius.toml`
(`org`, `repos_dir`, `workspace_file`, `exclude`, `include_private`),
`.gitignore` entries (`/repos/` + the generated workspace file),
README/AGENTS stubs — then clones every org repo into `repos/` and
generates the editor workspace file. It is idempotent and never
overwrites existing files.

Review what was created, then commit the config and push:

```bash
git add -A && git commit -m "chore: lasius workspace" && git push -u origin main
lasius doctor   # all checks ok, zero drift
```

## Day to day

```bash
lasius update     # git pull --ff-only everywhere
lasius status     # branch / dirty / ahead-behind table (--json for machines)
lasius doctor     # env + drift diagnostics
lasius open       # open the generated workspace file
```
