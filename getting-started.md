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

```bash
mkdir myorg-workspace && cd myorg-workspace
git init && git remote add origin git@github.com:MYORG/workspace.git
lasius init --org MYORG
```

`lasius init` scaffolds the whole workspace repo — `lasius.toml`,
`.gitignore` entries, README/AGENTS stubs — then clones every org repo
into `repos/` and generates the editor workspace file. It is idempotent
and never overwrites existing files.

## Day to day

```bash
lasius update     # git pull --ff-only everywhere
lasius status     # branch / dirty / ahead-behind table (--json for machines)
lasius doctor     # env + drift diagnostics
lasius open       # open the generated workspace file
```
