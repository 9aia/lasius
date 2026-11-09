# Include private repos

Private repos are excluded from discovery by default. Opt in with one
line in `lasius.toml`:

```toml
org = "acme"
include_private = true
```

`lasius clone` / `lasius init` then clone private repos too — requires
`gh` to be authenticated with access (`gh auth status`).
