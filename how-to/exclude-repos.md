# Exclude repos

`exclude` takes glob patterns matched against repo names:

```toml
org = "acme"
exclude = ["workspace", "*-archive", "experiments-*"]
```

Use it for repos that should never be cloned locally — typically the
workspace repo itself. Alternatively, archive a repo on GitHub to drop
it from discovery entirely.
