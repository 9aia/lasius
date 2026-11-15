# Decisions

ADR-lite — the decisions that shape lasius, the alternatives
rejected, and the consequence each carries.

## Independent clones, not submodules

**Decision:** every org repo is a full git clone under `repos/` —
no submodules, no monorepo merge.

**Alternatives:** git submodules (each clone carries `.git` file
pointer complexity, `git pull` doesn't recurse cleanly, editors
see nested repos); a single merged repo (loses per-repo history,
remotes, and CI boundaries).

**Consequence:** each repo stays independent — its own branches,
hooks, and workflows — while lasius provides the *management*
layer on top. The cost: N clones of shared history and `repos/`
must stay gitignored.

## Local clones are the source of truth

**Decision:** `status`, `update`, and `code-workspace generate`
iterate the clones on disk, not the provider's repo list.

**Alternatives:** drive everything from `gh repo list` (a repo
deleted upstream vanishes from status while its clone rots;
manually added clones become invisible); a manifest file listing
clones (drift between manifest and disk).

**Consequence:** `lasius add <name>` and hand-cloned dirs are
first-class; a repo deleted upstream keeps getting pulled until
`lasius rm` removes it — visible, not silent.

## `rm` is paranoid by default

**Decision:** removal runs a checklist — clean tree, no unpushed
commits, no stash, no linked worktrees, no nested `.git`, no extra
remotes — and refuses unless `--force`.

**Alternatives:** `rm -rf` semantics (one bad call loses
uncommitted work across an org); confirm-prompt-only (a tired
human still nukes a dirty tree).

**Consequence:** the destructive verb is safe to expose to agents
and scripts; `--force` exists for the cases the checklist can't
see, and every refusal names what it found.

## VCS and forge are shared ports, not lasius internals

**Decision:** `lasius-vcs` and `lasius-forge` are trait crates
consumed by other repos (tarsi, mimus, bupha, kuda, sturn) for
their agentic-CI surfaces (P-019); lasius-cli just happens to be
the first consumer.

**Alternatives:** per-repo git/gh wrappers (N implementations of
`worktree_add`/`create_status` drifting apart); a lasius-specific
API other repos can't use.

**Consequence:** workspace expertise is reusable — any engine can
shell out to git/gh through the same tested port. The crates are
dependency-light and sync on purpose.

## The command ships as a tamias pack

**Decision:** there is no cargo-installed `lasius` binary — the
command is a generated `bins[]` shim over `tamias-driver-lasius`
via `libraries/lasius`.

**Alternatives:** a `[[bin]]` in lasius-cli (a second distribution
path to keep in sync with the driver); per-repo install scripts
(drift).

**Consequence:** one artifact serves both humans (`lasius` shim)
and agents (`cli.<verb>` wire queries, `run`/`cli` passthroughs)
— install, upgrade, and rollback are tamias's problem.
