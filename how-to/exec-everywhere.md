# Run commands everywhere

`lasius exec` runs a shell command in every clone (or a filtered subset):

```bash
lasius exec --filter ui -- cargo build
lasius exec --filter web -- bun install
lasius exec -- ls
```

The command and its args go after `--`. Exits non-zero if any clone
fails; per-clone output is prefixed with the repo name.
