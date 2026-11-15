---
layout: home
hero:
  name: lasius
  text: Workspace gardener for GitHub orgs
  tagline: Clone, sync, inspect, and generate editor workspace files for every repo of an org under one root.
  actions:
    - theme: brand
      text: Get started
      link: /getting-started
    - theme: alt
      text: CLI reference
      link: /reference/cli
features:
  - title: One root, every repo
    details: Independent git clones under repos/ — no submodules, each keeps its own history and remotes.
  - title: Automatic discovery
    details: Provider-driven repo discovery finds org repos; archived repos are excluded, private ones opt-in.
  - title: Editor-ready
    details: Generates a *.code-workspace multi-root file for VS Code and friends.
  - title: Embeddable
    details: lasius serve speaks ndjson JSON-RPC — the org wire convention shared with tamias, sepia, cuora, and myco.
---

lasius is the org's **workspace gardener** — it clones, syncs,
inspects, and generates editor workspace files for every repo of a
GitHub org under one root. It owns git & workspace expertise and
nothing else: the work ledger is myco's, code maps are kuda's,
coordination is sturn's, per-repo tooling stays per-repo.

## Which quadrant answers which goal

- **Learn it** → [Tutorial: bootstrap a workspace](tutorials/bootstrap.md)
- **Do a task** → [How-to](/how-to/private-repos) — private repos,
  exclude repos, run everywhere, embed via serve
- **Understand why** → [Architecture](/explanation/architecture),
  [concepts](/explanation/concepts),
  [decisions](/explanation/decisions)
- **Look something up** → [CLI](/reference/cli),
  [lasius.toml](/reference/config),
  [serve wire](/reference/serve)
