# serve wire protocol

`lasius serve` — ndjson JSON-RPC over stdio, same wire as
`tamias serve` / `sepia serve` / `cuora serve` / `myco serve`.

## Framing

- Requests: one JSON object per line on stdin: `{"op":"…", …}`
- Responses: one JSON object per line on stdout:
  `{"ok":true,"result":…}` or `{"ok":false,"error":"…"}`
- Empty lines are ignored; the loop ends when stdin closes.

## Ops

| op | result |
| --- | --- |
| `ping` | `{"pong": true}` |
| `list` | `{"repos": [names…], "count": n}` — local clones under `repos_dir` |
| `status` | `{"repos": [{name, branch, upstream, ahead, behind, dirty}…]}` |
| `doctor` | `{"checks": [{name, ok, detail}…]}` |

Unknown ops return `ok:false` with the expected-op list. Read-only by
design — `clone`/`update`/`rm` print progress and would corrupt the
protocol, so they stay CLI-side.
