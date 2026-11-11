//! lasius-xtask — workspace automation: check / test / coverage /
//! pack / install / fmt.
//!
//! ```text
//! cargo xtask check      # fmt --check + clippy -D warnings
//! cargo xtask test       # cargo nextest run (falls back to cargo test)
//! cargo xtask coverage   # llvm-cov report (extra args pass through)
//! cargo xtask pack       # stage libraries/lasius driver payload
//!                        # (release tamias-driver-lasius → drivers/bin/<triple>/)
//! cargo xtask install    # release-build tamias-driver-lasius → ~/.local/bin
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

/// Captured stdout of a command — for probes like `rustc -vV`.
fn run_out(args: &[&str]) -> Option<String> {
    let (prog, rest) = args.split_first()?;
    let out = Command::new(prog).args(rest).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The effective target dir — `cargo metadata` reports it, which keeps
/// `pack`/`install` correct under a shared CARGO_TARGET_DIR
/// (`target/release` relative to the workspace is stale).
fn target_dir() -> anyhow::Result<std::path::PathBuf> {
    let out = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()?;
    let meta: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    meta.get("target_directory")
        .and_then(|v| v.as_str())
        .map(std::path::PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("cargo metadata: no target_directory"))
}

/// The host target triple — the `drivers/bin/<triple>/` selector
/// tamias uses for compiled driver payloads.
fn host_triple() -> String {
    run_out(&["rustc", "-vV"])
        .and_then(|v| {
            v.lines()
                .find_map(|l| l.strip_prefix("host: ").map(str::to_string))
        })
        .unwrap_or_else(|| "x86_64-unknown-linux-gnu".to_string())
}

/// `cargo xtask pack` — stage the tamias pack payload (T-012):
/// release-build `tamias-driver-lasius` and drop it under
/// `libraries/lasius/drivers/bin/<host-triple>/` — the
/// bundled-driver dir `tamias install` registers into
/// `<tamias-home>/drivers/`. Gitignored on purpose: the binary is a
/// build artifact; the committed pack is just `tamias.yaml` + this
/// staging verb.
fn pack() -> anyhow::Result<()> {
    anyhow::ensure!(
        run("cargo", &["build", "--release", "-p", "lasius-driver"]),
        "cargo build --release -p lasius-driver failed"
    );
    let triple = host_triple();
    let dest_dir = std::path::PathBuf::from("libraries/lasius/drivers/bin").join(&triple);
    std::fs::create_dir_all(&dest_dir)?;
    let src = target_dir()?.join("release/tamias-driver-lasius");
    let dest = dest_dir.join("tamias-driver-lasius");
    std::fs::copy(&src, &dest)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755));
    }
    println!("pack staged: {}", dest.display());
    println!();
    println!("install the lasius command (generated bins[] shim):");
    println!("  tamias install --allow-drivers ./libraries/lasius");
    println!("  # or a pinned tarball: tamias bundle pack libraries/lasius");
    Ok(())
}

/// `cargo xtask install` — release-build `tamias-driver-lasius` and
/// copy it to `~/.local/bin` (idempotent, PATH-friendly name). There
/// is no `lasius` cargo bin — the command ships as the tamias pack
/// (`cargo xtask pack` + `tamias install ./libraries/lasius`); this
/// installs the driver the shim resolves.
fn install() -> anyhow::Result<()> {
    anyhow::ensure!(
        run("cargo", &["build", "--release", "-p", "lasius-driver"]),
        "cargo build --release -p lasius-driver failed"
    );
    let home = std::env::var_os("HOME")
        .map_or_else(|| std::path::PathBuf::from("/"), std::path::PathBuf::from);
    let out = home.join(".local/bin");
    std::fs::create_dir_all(&out)?;
    let bin = "tamias-driver-lasius";
    let dst = out.join(bin);
    // Atomic replace — a running driver holds the old inode busy
    // ("text file busy" on plain copy).
    std::fs::copy(
        target_dir()?.join("release").join(bin),
        out.join(format!(".{bin}.new")),
    )?;
    std::fs::rename(out.join(format!(".{bin}.new")), &dst)?;
    println!("installed {bin} to {}", out.display());
    println!();
    println!("install the lasius command (generated bins[] shim):");
    println!("  cargo xtask pack && tamias install --allow-drivers ./libraries/lasius");
    Ok(())
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
        Some("pack") => match pack() {
            Ok(()) => true,
            Err(e) => {
                eprintln!("xtask: pack failed: {e:#}");
                false
            }
        },
        Some("install") => match install() {
            Ok(()) => true,
            Err(e) => {
                eprintln!("xtask: install failed: {e:#}");
                false
            }
        },
        Some("fmt") => run("cargo", &["fmt", "--all"]),
        _ => {
            eprintln!("usage: cargo xtask <check|test|coverage|pack|install|fmt>");
            return ExitCode::from(2);
        }
    };
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
