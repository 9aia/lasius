# Exclude repos

**Goal:** keep specific org repos out of local discovery and
cloning.

**Prerequisites:** a `lasius.toml` (see
[Getting started](../tutorials/getting-started.md)).

## Steps

`exclude` takes glob patterns matched against repo names:

```toml
org = "acme"
exclude = ["workspace", "*-archive", "experiments-*"]
```

Use it for repos that should never be cloned locally. The
workspace repo itself needs no entry — lasius detects it from the
root's `origin` remote and never clones it. Alternatively, archive
a repo on GitHub to drop it from discovery entirely.

To remove an already-cloned repo, use `lasius rm <name>` — it runs
the safety checklist (clean tree, no unpushed commits, no stash,
no extra remotes) before deleting.

## Verify

```bash
lasius clone    # excluded repos are skipped
lasius status   # they never appear in the table
```

## Deeper

- [Concepts — filters](../explanation/concepts.md)
- [Decisions — local clones are the source of truth](../explanation/decisions.md)
