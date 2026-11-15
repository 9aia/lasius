# Security Policy

Report vulnerabilities privately to the maintainers via GitHub:
https://github.com/9aia/lasius/security/advisories/new

Do not open public issues for security reports.

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
