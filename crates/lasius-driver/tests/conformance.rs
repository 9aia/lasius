//! Driver conformance gate — runs pavo's wire-contract suite
//! ([`pavo_testkit::conformance::check_driver`]) against this crate's
//! own `tamias-driver-lasius serve` binary.
//!
//! The spawned driver inherits this test's cwd
//! (`crates/lasius-driver`, inside the lasius repo). `manifest` /
//! `views` / `query:"ops"` are workspace-free; `cli.*` ops are not
//! exercised by the suite, so no `lasius.toml` is needed — hermetic,
//! no network.

use std::path::Path;

#[test]
fn driver_passes_pavo_conformance() {
    let violations = pavo_testkit::conformance::check_driver(Path::new(env!(
        "CARGO_BIN_EXE_tamias-driver-lasius"
    )));
    assert!(
        violations.is_empty(),
        "pavo conformance violations:\n{}",
        violations
            .iter()
            .map(|v| format!("  - {v}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
