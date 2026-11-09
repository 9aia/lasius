# Embed via serve

`lasius serve` reads one JSON request per line on stdin and writes one
JSON response per line on stdout — the same ndjson wire as
`tamias serve`, `sepia serve`, `cuora serve`, and `myco serve`.

```bash
printf '{"op":"list"}\n{"op":"status"}\n' | lasius serve
```

Ops: `ping`, `list`, `status`, `doctor` — read-only; mutating commands
stay CLI-side so stdout stays protocol-clean. See
[the wire reference](/reference/serve).
