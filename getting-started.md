# Getting started

## Install

```bash
cargo install lasius
```

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
