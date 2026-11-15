# Boundaries

Lasius owns **git & workspace expertise** — repo discovery,
clone/pull, status, editor workspace files, safe removal, plus the
shared `lasius-vcs`/`lasius-forge` ports. It does not own:

- **myco** — the org work ledger; `project/` worktrees are managed
  by myco verbs, never by lasius.
- **kuda** — code maps; per-repo `.kuda/` cards are that repo's
  concern.
- **sturn** — ephemeral coordination; lasius doesn't claim files.
- **Per-repo tooling** — each clone's own build/test/dev flow is
  its own `AGENTS.md`'s business. `lasius exec` is the escape
  hatch, not a task runner.
- **Secrets** — `gh`/`git` credentials are the user's own; lasius
  shells out and inherits the environment.
