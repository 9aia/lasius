# Include private repos

**Goal:** make `lasius` discover and clone the org's private repos
alongside public ones.

**Prerequisites:** a `lasius.toml` (see
[Getting started](../tutorials/getting-started.md)); `gh`
authenticated with access to the private repos
(`gh auth status`).

## Steps

Private repos are excluded from discovery by default. Opt in with
one line in `lasius.toml`:

```toml
org = "acme"
include_private = true
```

Then re-run discovery:

```bash
lasius clone    # clones any missing repos, private included
```

## Verify

```bash
lasius status   # private clones appear in the table
```

## Deeper

- [Concepts — provider and filters](../explanation/concepts.md)
- [Exclude repos](exclude-repos.md) — the inverse knob
