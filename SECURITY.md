# Security Policy

Report vulnerabilities privately by emailing
luisfuturist+9aia@gmail.com with "SECURITY" in the subject — do
not open public issues, do not use GitHub's advisory or
private-reporting features, and do not disclose details publicly;
contact us first and we will coordinate disclosure privately.

## Scope

- lasius is a workspace gardener — it clones, pulls, and inspects
  git checkouts under one root, and generates `*.code-workspace`
  files. It shells out to `git` and `gh`; it holds no secrets
  itself (credentials are whatever `git`/`gh` already have).
- Repo names from `gh` are validated (`names::is_safe_repo_name`)
  before they become path segments — bypassing it is a bug.
- Mutating verbs (`vcs worktree|commit|push|fetch`,
  `forge pr create|merge|status create`) touch real repos — they
  take a clone name or checkout path and refuse unsafe repo names.
- `lasius rm` refuses removal when the clone has unpushed commits,
  a dirty tree, stashes, linked worktrees, or extra remotes.

## Notes

We respond to reports as quickly as we can; expect acknowledgement
within a few days.
