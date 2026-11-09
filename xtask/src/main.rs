//! lasius-xtask — workspace automation: check / test / coverage.
//!
//! ```text
//! cargo xtask check      # fmt --check + clippy -D warnings
//! cargo xtask test       # cargo nextest run (falls back to cargo test)
//! cargo xtask coverage   # llvm-cov report (extra args pass through)
//! ```

use std::process::{Command, ExitCode};

/// Run `program args…` inheriting stdio; returns true on success.
fn run(program: &str, args: &[&str]) -> bool {
    match Command::new(program).args(args).status() {
        Ok(status) => status.success(),
        Err(e) => {
            eprintln!("xtask: cannot run {program}: {e}");
            false
        }
    }
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let ok = match args.next().as_deref() {
        Some("check") => {
            run("cargo", &["fmt", "--all", "--", "--check"])
                && run(
                    "cargo",
                    &[
                        "clippy",
                        "--workspace",
                        "--all-targets",
                        "--",
                        "-D",
                        "warnings",
                    ],
                )
        }
        Some("test") => {
            let has_nextest = Command::new("cargo")
                .args(["nextest", "--version"])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            if has_nextest {
                run("cargo", &["nextest", "run", "--workspace"])
            } else {
                run("cargo", &["test", "--workspace"])
            }
        }
        Some("coverage") => {
            let rest: Vec<String> = args.collect();
            let mut argv: Vec<&str> = vec!["llvm-cov", "--workspace", "--summary-only"];
            for a in &rest {
                argv.push(a.as_str());
            }
            run("cargo", &argv)
        }
        Some("fmt") => run("cargo", &["fmt", "--all"]),
        _ => {
            eprintln!("usage: cargo xtask <check|test|coverage|fmt>");
            return ExitCode::from(2);
        }
    };
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
