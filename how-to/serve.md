# Embed via serve

**Goal:** query workspace state from another process using the org's
ndjson JSON-RPC wire convention.

**Prerequisites:** `lasius` installed (see
[Getting started](../tutorials/getting-started.md)).

## Steps

`lasius serve` reads one JSON request per line on stdin and writes
one JSON response per line on stdout — the same wire as
`tamias serve`, `sepia serve`, `cuora serve`, and `myco serve`:

```bash
printf '{"op":"list"}\n{"op":"status"}\n' | lasius serve
```

Ops: `manifest`, `ping`, `list`, `status`, `doctor` — read-only; mutating
commands stay CLI-side so stdout stays protocol-clean.

## Verify

The piped commands each emit one JSON line — `ping` returns a
pong-style ack, `list`/`status`/`doctor` return their payloads.
See [the wire reference](../reference/serve) for exact shapes.

## Deeper

- [serve wire protocol](../reference/serve.md)
- [Concepts — surfaces](../explanation/concepts.md)
