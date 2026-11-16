# Command-Line Help for `lasius`

This document contains the help content for the `lasius` command-line program.

lasius keeps a directory of independent git clones in sync with a GitHub org: init bootstraps, clone/update/status maintain, and code-workspace generates the editor workspace file.

**Command Overview:**

* [`lasius`↴](#lasius)
* [`lasius init`↴](#lasius-init)
* [`lasius clone`↴](#lasius-clone)
* [`lasius update`↴](#lasius-update)
* [`lasius status`↴](#lasius-status)
* [`lasius code-workspace`↴](#lasius-code-workspace)
* [`lasius code-workspace generate`↴](#lasius-code-workspace-generate)
* [`lasius doctor`↴](#lasius-doctor)
* [`lasius check`↴](#lasius-check)
* [`lasius conform`↴](#lasius-conform)
* [`lasius open`↴](#lasius-open)
* [`lasius rm`↴](#lasius-rm)
* [`lasius exec`↴](#lasius-exec)
* [`lasius add`↴](#lasius-add)
* [`lasius vcs`↴](#lasius-vcs)
* [`lasius vcs worktree`↴](#lasius-vcs-worktree)
* [`lasius vcs worktree add`↴](#lasius-vcs-worktree-add)
* [`lasius vcs worktree remove`↴](#lasius-vcs-worktree-remove)
* [`lasius vcs commit`↴](#lasius-vcs-commit)
* [`lasius vcs push`↴](#lasius-vcs-push)
* [`lasius vcs fetch`↴](#lasius-vcs-fetch)
* [`lasius forge`↴](#lasius-forge)
* [`lasius forge pr`↴](#lasius-forge-pr)
* [`lasius forge pr create`↴](#lasius-forge-pr-create)
* [`lasius forge pr merge`↴](#lasius-forge-pr-merge)
* [`lasius forge pr view`↴](#lasius-forge-pr-view)
* [`lasius forge pr checks`↴](#lasius-forge-pr-checks)
* [`lasius forge pr list`↴](#lasius-forge-pr-list)
* [`lasius forge status`↴](#lasius-forge-status)
* [`lasius forge status create`↴](#lasius-forge-status-create)
* [`lasius serve`↴](#lasius-serve)
* [`lasius completions`↴](#lasius-completions)

## `lasius`

lasius keeps a directory of independent git clones in sync with a GitHub org: init bootstraps, clone/update/status maintain, and code-workspace generates the editor workspace file.

**Usage:** `lasius [OPTIONS] <COMMAND>`

###### **Commands:**

* `init` — Scaffold the workspace repo (lasius.toml, .gitignore, doc stubs), clone all org repos, generate the workspace file
* `clone` — Clone all missing org repos into repos_dir
* `update` — git pull --ff-only every clone [alias: sync]
* `status` — Show branch / dirty / sync status for every clone
* `code-workspace` — Workspace-file commands
* `doctor` — Environment + drift diagnostics
* `check` — Conformance checks — does the workspace follow the lasius pattern?
* `conform` — Bring clones up to the org docs contract — scaffold missing README/AGENTS/cypera.toml/SECURITY/CONTRIBUTING/docs stubs; never overwrites existing files
* `open` — Open the generated workspace file in an editor
* `rm` — Remove a clone after safety checks (clean tree, no unpushed commits, …)
* `exec` — Run a shell command in every clone
* `add` — Clone a single org repo by name
* `vcs` — VCS verbs — worktrees, commits, fetch/push over the lasius-vcs port
* `forge` — Forge verbs — pull requests, checks, and commit statuses over the lasius-forge port
* `serve` — ndjson JSON-RPC over stdio — read-only ops for embedding (manifest, ping, list, status, doctor); same wire as tamias/sepia/cuora/myco serve
* `completions` — Print shell completion script for SHELL

###### **Options:**

* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `-h, --help` — Print help (see a summary with '-h')
* `-V, --version` — Print version

## `lasius init`

Scaffold the workspace repo (lasius.toml, .gitignore, doc stubs), clone all org repos, generate the workspace file

**Usage:** `lasius init [OPTIONS]`

###### **Options:**

* `--org <ORG>` — GitHub org to manage (required when lasius.toml doesn't exist yet)
* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `-h, --help` — Print help

## `lasius clone`

Clone all missing org repos into repos_dir

**Usage:** `lasius clone [OPTIONS]`

###### **Options:**

* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `-h, --help` — Print help

## `lasius update`

git pull --ff-only every clone

**Usage:** `lasius update [OPTIONS]`

###### **Options:**

* `--json` — Emit JSON instead of the table
* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `-h, --help` — Print help

## `lasius status`

Show branch / dirty / sync status for every clone

**Usage:** `lasius status [OPTIONS]`

###### **Options:**

* `--json` — Emit JSON instead of the table
* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `-h, --help` — Print help

## `lasius code-workspace`

Workspace-file commands

**Usage:** `lasius code-workspace [OPTIONS] <COMMAND>`

###### **Commands:**

* `generate` — Regenerate the *.code-workspace file from local clones

###### **Options:**

* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `-h, --help` — Print help

## `lasius code-workspace generate`

Regenerate the *.code-workspace file from local clones

**Usage:** `lasius code-workspace generate [OPTIONS]`

###### **Options:**

* `--json` — Emit JSON instead of the summary line
* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `-h, --help` — Print help

## `lasius doctor`

Environment + drift diagnostics

**Usage:** `lasius doctor [OPTIONS]`

###### **Options:**

* `--json` — Emit JSON instead of the check table
* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `-h, --help` — Print help

## `lasius check`

Conformance checks — does the workspace follow the lasius pattern?

**Usage:** `lasius check [OPTIONS]`

###### **Options:**

* `--domain <DOMAIN>` — Check domain to run (structure|clones|generated|policy)
* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `--only <ONLY>` — Only run these check ids (repeatable or comma-separated)
* `--json` — Emit JSON findings instead of the table
* `-h, --help` — Print help

## `lasius conform`

Bring clones up to the org docs contract — scaffold missing README/AGENTS/cypera.toml/SECURITY/CONTRIBUTING/docs stubs; never overwrites existing files

**Usage:** `lasius conform [OPTIONS] [NAME]`

###### **Arguments:**

* `[NAME]` — Repo name under repos_dir (default: all clones)

###### **Options:**

* `--dry-run` — Report only — don't write anything
* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `--json` — Emit JSON findings instead of the table
* `-h, --help` — Print help

## `lasius open`

Open the generated workspace file in an editor

**Usage:** `lasius open [OPTIONS]`

###### **Options:**

* `--editor <EDITOR>` — Editor command (default: $VISUAL, $EDITOR, or code/zed/cursor/vi)
* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `-h, --help` — Print help

## `lasius rm`

Remove a clone after safety checks (clean tree, no unpushed commits, …)

**Usage:** `lasius rm [OPTIONS] <NAME>`

###### **Arguments:**

* `<NAME>` — Repo name under repos_dir

###### **Options:**

* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `--yes` — Skip the confirmation prompt
* `--force` — Skip all safety checks
* `-h, --help` — Print help

## `lasius exec`

Run a shell command in every clone

**Usage:** `lasius exec [OPTIONS] <CMD>...`

###### **Arguments:**

* `<CMD>...` — Command + args, after `--`

###### **Options:**

* `--filter <FILTER>` — Only run in clones whose name contains this substring
* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `-h, --help` — Print help

## `lasius add`

Clone a single org repo by name

**Usage:** `lasius add [OPTIONS] <NAME>`

###### **Arguments:**

* `<NAME>` — Repo name

###### **Options:**

* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `-h, --help` — Print help

## `lasius vcs`

VCS verbs — worktrees, commits, fetch/push over the lasius-vcs port

**Usage:** `lasius vcs [OPTIONS] <COMMAND>`

###### **Commands:**

* `worktree` — Worktree commands
* `commit` — Stage all changes and commit in a checkout/worktree
* `push` — Push a branch to a remote (default: current branch → origin)
* `fetch` — Fetch a remote (optionally specific refspecs via --refspec)

###### **Options:**

* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `-h, --help` — Print help

## `lasius vcs worktree`

Worktree commands

**Usage:** `lasius vcs worktree [OPTIONS] <COMMAND>`

###### **Commands:**

* `add` — Create a worktree at --rev (default HEAD), optionally on a new --branch
* `remove` — Remove a worktree (--dir is the worktree path)

###### **Options:**

* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `-h, --help` — Print help

## `lasius vcs worktree add`

Create a worktree at --rev (default HEAD), optionally on a new --branch

**Usage:** `lasius vcs worktree add [OPTIONS] <REPO>`

###### **Arguments:**

* `<REPO>` — Clone name under repos_dir, or a checkout path

###### **Options:**

* `--dir <DIR>` — Worktree path (default: &lt;ws&gt;/worktrees/&lt;repo&gt;/&lt;branch|rev|HEAD&gt;)
* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `--branch <BRANCH>` — New branch to create and check out in the worktree
* `--rev <REV>` — Revision/sha to base the worktree on (default: HEAD)
* `--json` — Emit JSON instead of the summary line
* `-h, --help` — Print help

## `lasius vcs worktree remove`

Remove a worktree (--dir is the worktree path)

**Usage:** `lasius vcs worktree remove [OPTIONS] --dir <DIR> <REPO>`

###### **Arguments:**

* `<REPO>` — Clone name under repos_dir, or the owning checkout path

###### **Options:**

* `--dir <DIR>` — Worktree directory to remove
* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `--json` — Emit JSON instead of the summary line
* `-h, --help` — Print help

## `lasius vcs commit`

Stage all changes and commit in a checkout/worktree

**Usage:** `lasius vcs commit [OPTIONS] --message <MESSAGE> <REPO>`

###### **Arguments:**

* `<REPO>` — Clone name under repos_dir, or a checkout/worktree path

###### **Options:**

* `-m, --message <MESSAGE>` — Commit subject (`-m`)
* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `--body <BODY>` — Commit body — second paragraph (`--body`)
* `--allow-empty` — Allow committing with no staged changes (CI retrigger)
* `--json` — Emit JSON instead of the summary line
* `-h, --help` — Print help

## `lasius vcs push`

Push a branch to a remote (default: current branch → origin)

**Usage:** `lasius vcs push [OPTIONS] <REPO>`

###### **Arguments:**

* `<REPO>` — Clone name under repos_dir, or a checkout/worktree path

###### **Options:**

* `--remote <REMOTE>` — Remote name (default: origin)
* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `--branch <BRANCH>` — Branch to push (default: the branch checked out at repo)
* `--json` — Emit JSON instead of the summary line
* `-h, --help` — Print help

## `lasius vcs fetch`

Fetch a remote (optionally specific refspecs via --refspec)

**Usage:** `lasius vcs fetch [OPTIONS] <REPO>`

###### **Arguments:**

* `<REPO>` — Clone name under repos_dir, or a checkout/worktree path

###### **Options:**

* `--remote <REMOTE>` — Remote name (default: origin)
* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `--refspec <REFSPEC>` — Refspec to fetch (repeatable; empty = remote's configured refspecs)
* `--json` — Emit JSON instead of the summary line
* `-h, --help` — Print help

## `lasius forge`

Forge verbs — pull requests, checks, and commit statuses over the lasius-forge port

**Usage:** `lasius forge [OPTIONS] <COMMAND>`

###### **Commands:**

* `pr` — Pull-request commands
* `status` — Commit-status commands

###### **Options:**

* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `-h, --help` — Print help

## `lasius forge pr`

Pull-request commands

**Usage:** `lasius forge pr [OPTIONS] <COMMAND>`

###### **Commands:**

* `create` — Open a pull request (--title/--body; --base/--head optional)
* `merge` — Merge a pull request (--method, default squash; --delete-branch)
* `view` — Show one pull request (state, refs, mergeability, review)
* `checks` — List CI checks / status contexts on a PR's head
* `list` — List pull requests (--state, --limit)

###### **Options:**

* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `-h, --help` — Print help

## `lasius forge pr create`

Open a pull request (--title/--body; --base/--head optional)

**Usage:** `lasius forge pr create [OPTIONS] --title <TITLE> <REPO>`

###### **Arguments:**

* `<REPO>` — Clone name under repos_dir, or a checkout/worktree path

###### **Options:**

* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `--title <TITLE>` — PR title
* `--body <BODY>` — PR body [default: ""]
* `--base <BASE>` — Base branch (default: repo default)
* `--head <HEAD>` — Head branch/ref (default: the checkout's current branch)
* `--draft` — Open as a draft PR
* `--json` — Emit JSON instead of the summary line
* `-h, --help` — Print help

## `lasius forge pr merge`

Merge a pull request (--method, default squash; --delete-branch)

**Usage:** `lasius forge pr merge [OPTIONS] <REPO> <NUMBER>`

###### **Arguments:**

* `<REPO>` — Clone name under repos_dir, or a checkout/worktree path
* `<NUMBER>` — Pull request number

###### **Options:**

* `--method <METHOD>` — Merge strategy [default: squash] [possible values: squash, merge, rebase]
* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `--delete-branch` — Delete the head branch after merging
* `--json` — Emit JSON instead of the summary line
* `-h, --help` — Print help

## `lasius forge pr view`

Show one pull request (state, refs, mergeability, review)

**Usage:** `lasius forge pr view [OPTIONS] <REPO> <NUMBER>`

###### **Arguments:**

* `<REPO>` — Clone name under repos_dir, or a checkout/worktree path
* `<NUMBER>` — Pull request number

###### **Options:**

* `--json` — Emit JSON instead of the summary
* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `-h, --help` — Print help

## `lasius forge pr checks`

List CI checks / status contexts on a PR's head

**Usage:** `lasius forge pr checks [OPTIONS] <REPO> <NUMBER>`

###### **Arguments:**

* `<REPO>` — Clone name under repos_dir, or a checkout/worktree path
* `<NUMBER>` — Pull request number

###### **Options:**

* `--json` — Emit JSON instead of the table
* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `-h, --help` — Print help

## `lasius forge pr list`

List pull requests (--state, --limit)

**Usage:** `lasius forge pr list [OPTIONS] <REPO>`

###### **Arguments:**

* `<REPO>` — Clone name under repos_dir, or a checkout/worktree path

###### **Options:**

* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `--state <STATE>` — State filter [default: open] [possible values: open, closed, merged, all]
* `--limit <LIMIT>` — Max PRs to list (0 = backend default) [default: 0]
* `--json` — Emit JSON instead of the table
* `-h, --help` — Print help

## `lasius forge status`

Commit-status commands

**Usage:** `lasius forge status [OPTIONS] <COMMAND>`

###### **Commands:**

* `create` — Post a commit status on a sha (CI verdict egress)

###### **Options:**

* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `-h, --help` — Print help

## `lasius forge status create`

Post a commit status on a sha (CI verdict egress)

**Usage:** `lasius forge status create [OPTIONS] --sha <SHA> --state <STATE> --context <CONTEXT> <REPO>`

###### **Arguments:**

* `<REPO>` — Clone name under repos_dir, or a checkout/worktree path

###### **Options:**

* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `--sha <SHA>` — Commit sha the status describes
* `--state <STATE>` — Status state [possible values: pending, success, failure, error]
* `--context <CONTEXT>` — Status context (e.g. docs/deploy, ci/bupha)
* `--description <DESCRIPTION>` — Short description (forge truncates ~140 chars)
* `--target-url <TARGET_URL>` — Link back to the run/logs
* `--json` — Emit JSON instead of the summary line
* `-h, --help` — Print help

## `lasius serve`

ndjson JSON-RPC over stdio — read-only ops for embedding (manifest, ping, list, status, doctor); same wire as tamias/sepia/cuora/myco serve

**Usage:** `lasius serve [OPTIONS]`

###### **Options:**

* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `-h, --help` — Print help

## `lasius completions`

Print shell completion script for SHELL

**Usage:** `lasius completions [OPTIONS] <SHELL>`

###### **Arguments:**

* `<SHELL>` — Shell to generate completions for [possible values: bash, elvish, fish, powershell, zsh]

###### **Options:**

* `--root <ROOT>` — Workspace root containing lasius.toml (default: discovered upward from cwd)
* `-h, --help` — Print help
