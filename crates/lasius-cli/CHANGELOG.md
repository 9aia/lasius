# Changelog

## [0.5.0] - 2026-11-24

### Added

- auto-exclude the workspace repo itself (`fd4fcd0`)
- include_private config flag (default false) (`e0e048d`)
- lasius — workspace gardener for GitHub orgs (`d293a80`)
- serve (ndjson JSON-RPC) + shell completions (`cc02937`)
- lasius conform — org docs-contract scaffolding for clones (`c254f8f`)
- **serve:** manifest op — P-020 contract alignment (`bec32cf`)
- sync alias + --json on update/doctor/code-workspace generate (`bc7738e`)
- lasius check — conformance engine (`96edd32`)
- **lasius-driver:** cli verb is an engine passthrough — the tamias bins[] shim contract (`674fb14`)
- add vcs + forge verb groups for agentic PR workflows (`668c36d`)
- **forge:** add commit-status create verb — lasius forge status create (`1cdd6f6`)
- init scaffolds the whole workspace repo (`0e6e7a1`)
- access-denied diagnostics and provider seam (`0ad5e1f`)

### Changed

- **lasius-cli:** lib-ify the engine — Cli/Cmd, parse/parse_from/command/dispatch/run_from (`f7c816e`)

### Fixed

- bound remote-reachable probe with a 20s timeout (`6e8cb94`)
