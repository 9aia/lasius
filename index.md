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
      text: Command reference
      link: /reference/commands
features:
  - title: One root, every repo
    details: Independent git clones under repos/ — no submodules, each keeps its own history and remotes.
  - title: Automatic discovery
    details: gh repo list finds org repos; archived repos are excluded, private ones opt-in.
  - title: Editor-ready
    details: Generates a *.code-workspace multi-root file for VS Code and friends.
  - title: Embeddable
    details: lasius serve speaks ndjson JSON-RPC — the org wire convention shared with tamias, sepia, cuora, and myco.
---
