# Exclude repos

`exclude` takes glob patterns matched against repo names:

```toml
org = "acme"
exclude = ["workspace", "*-archive", "experiments-*"]
```

Use it for repos that should never be cloned locally. The workspace
repo itself needs no entry — lasius detects it from the root's `origin`
remote and never clones it. Alternatively, archive a repo on GitHub to
drop it from discovery entirely.
