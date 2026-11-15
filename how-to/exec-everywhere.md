# Run commands everywhere

**Goal:** execute one shell command across every clone (or a
filtered subset) from the workspace root.

**Prerequisites:** a bootstrapped workspace with clones under
`repos/` (see [Bootstrap a workspace](../tutorials/bootstrap.md)).

## Steps

`lasius exec` runs a shell command in every clone; `--filter`
restricts to clones whose name contains a substring:

```bash
lasius exec --filter ui -- cargo build
lasius exec --filter web -- bun install
lasius exec -- ls
```

The command and its args go after `--`. Exits non-zero if any clone
fails; per-clone output is prefixed with the repo name.

`lasius exec` is the escape hatch, not a task runner — per-repo
tooling belongs in each clone's own `AGENTS.md` flow.

## Verify

Run something trivial first:

```bash
lasius exec -- pwd    # prints each clone's path, prefixed by repo name
```

## Deeper

- [CLI reference](../reference/cli.md) — all verbs and flags
- [Boundary](../explanation/boundaries.md) — what
  per-repo tooling owns instead
