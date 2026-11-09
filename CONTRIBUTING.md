# Contributing

Issues and PRs are **disabled** — the project is agent-maintained and
dogfoods its own PM ledger (`project/`). Fork freely under MIT.

## Layout

`crates/` — flat rust workspace. `lasius-core` is pure domain (no I/O);
`lasius-cli` (`crates/lasius-cli`, package `lasius`) is the composition
root; `xtask/` is repo chores. See the crate table in
[AGENTS.md](AGENTS.md#layout).

Fresh clone? Run `./scripts/setup.sh` — it sets `core.hooksPath=.hooks`
(pre-commit `cargo fmt --check`, pre-push `cargo xtask check`) and
verifies the toolchain. `rust-toolchain.toml` pins stable + rustfmt +
clippy.

## Rules of thumb

- New **pure logic** (parsing, formatting, matching, config) →
  `lasius-core` with unit tests. New **IO** (process, fs, network) →
  `lasius-cli`. Never the reverse.
- `cargo xtask check` and `cargo xtask test` must stay green.
- Coverage: `cargo xtask coverage` — llvm-cov summary; gate with
  `--fail-under-lines N`.
- Keep the binary dependency-light and sync; no async runtime.
- Commits: conventional prefixes (`feat:`/`fix:`/`refactor:`/`docs:`/`chore:`).
