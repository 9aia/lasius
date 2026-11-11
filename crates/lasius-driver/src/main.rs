//! tamias-driver-lasius — the pavo driver for a lasius workspace,
//! plus the engine ops surface.
//!
//! Verbs (the tamias driver contract):
//!
//! - `manifest` — print the [`DriverManifest`] JSON (discovery
//!   probe; never touches workspace state).
//! - `capabilities` — `{"capabilities": ["pavo"], "verbs": […]}`.
//! - `ops` — the engine ops manifest: every bounded `lasius` CLI
//!   verb as a `cli.<verb>` wire query, derived from the engine's
//!   own clap tree ([`lasius::command`]). Unbounded/host-interactive
//!   verbs (`serve`, `open`, `exec`) are `surfaces`, never ops.
//! - `run <argv>` — engine passthrough: `tamias-driver-lasius run
//!   status` ≡ `lasius status`. Also the child side of `cli.*` wire
//!   queries (`ops::run_verb` re-execs this same verb so engine
//!   output never touches the serve stream).
//! - `cli <lasius argv>` — the tamias `bins[]` shim verb: `tamias
//!   bin exec` calls it with the standard `--dir`/`--name` driver
//!   flags ahead of the user's argv; they're stripped and the rest
//!   runs through `lasius::run_from` in-process with inherited
//!   stdio — full engine parity (`serve` streams, globals/`--json`/
//!   `--help` all work).
//! - `serve` — ndjson JSON-RPC on stdio: `{"op":"manifest"}`,
//!   `{"op":"views"}`, `{"op":"view","view":…,"params":{…}}`,
//!   `{"op":"query","query":…,"params":{…}}` →
//!   `{"ok":true,"result":…}` / `{"ok":false,"error":…}`.
//!   `query:"ops"` answers the ops manifest; `query:"cli.<verb>"`
//!   runs the engine op.
//!
//! Optional surface: feature `mcp` embeds pavo-driver-kit's
//! surface layer — `mcp` serves the ops manifest as MCP tools on
//! stdio, equivalent to the standalone `tamias-driver-mcp` surface
//! driver minus the PATH/discovery hop. (`cli` needs no feature:
//! it's the bins-shim passthrough above, not a generated surface —
//! the engine's own argv grammar is the contract.)
//!
//! Which workspace: ops inherit the `serve` process's cwd — the
//! `root` param maps to the CLI's `--root` global, and absent it
//! `lasius`'s own upward `lasius.toml` discovery applies, identical
//! to running the `lasius` verb by hand. This driver ships no
//! views — it is the engine-ops surface for lasius workspaces;
//! view/query names outside `ops`/`cli.*` answer `ok:false`.

mod ops;
#[cfg(feature = "mcp")]
mod surface;

use pavo_driver_kit as kit;
use pavo_protocol::wire::{self, Request, Response};
use pavo_protocol::DriverManifest;
use serde_json::{json, Value};

/// The driver manifest — no views (engine-ops driver); `ops`/`run`
/// are argv verbs, `serve`/`manifest` are the wire verbs `kit`
/// already declares.
fn manifest() -> DriverManifest {
    kit::Manifest::new("lasius", "Lasius")
        .version(env!("CARGO_PKG_VERSION"))
        .icon("🐜")
        .verb("ops")
        .verb("run")
        .verb("capabilities")
        .build()
}

fn capabilities_json() -> Value {
    let m = manifest();
    let mut v = kit::capabilities_value(&m);
    v["views"] = json!(m.views.len());
    v
}

/// Dispatch one parsed request to its response.
fn dispatch(req: &Request) -> Response {
    match req {
        Request::Manifest => Response::ok_typed(&manifest()),
        Request::Views => Response::ok_typed(&Vec::<wire::ViewSummary>::new()),
        Request::View { view, .. } => Response::err(format!(
            "tamias-driver-lasius claims no views (view '{view}')"
        )),
        Request::Render { kind, .. } => Response::err(format!(
            "tamias-driver-lasius claims no renderers (kind '{kind}')"
        )),
        Request::Query { query, params } => {
            // `ops` and `cli.<verb>` are engine ops; there are no
            // view queries — unknown names fail the op.
            if query == "ops" {
                Response::ok_typed(&wire::QueryResult {
                    query: query.clone(),
                    data: ops::manifest(),
                })
            } else if let Some(v) = ops::op_of(query) {
                ops::run_verb(&v, params)
            } else {
                Response::err(format!("unknown query: {query}"))
            }
        }
        // The protocol grew `submit`/`jobs`/`notifications` ops after
        // this driver was written; lasius mutations go through
        // `cli.<verb>`/`run`, never `submit`, so they answer
        // `ok:false` like any unsupported op.
        Request::Submit { .. } | Request::Jobs | Request::Notifications => {
            Response::err("tamias-driver-lasius implements no submit/jobs/notifications ops")
        }
    }
}

/// `cli` verb argv (after the `cli` token) → engine argv (argv[0]
/// included). `tamias bin exec`/`driver exec` prepend the standard
/// driver flags — `--dir <payload>` `--name <pkg>` — which the
/// engine doesn't know; consume the leading run of those two
/// (`--flag v` and `--flag=v` forms) and pass everything else
/// through untouched.
fn engine_argv(args: &[String]) -> Vec<String> {
    let mut out = vec!["lasius".to_string()];
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        if a == "--dir" || a == "--name" {
            // flag + separate value — skip both (a trailing flag
            // with no value just ends the scan).
            i = (i + 2).min(args.len());
        } else if a.starts_with("--dir=") || a.starts_with("--name=") {
            i += 1;
        } else {
            break;
        }
    }
    out.extend(args[i..].iter().cloned());
    out
}

fn serve() -> i32 {
    // The kit owns the ndjson loop — stdin EOF and a dead client
    // both end it quietly, and either way the driver exits 0.
    let _ = kit::serve_stdio(dispatch);
    0
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("manifest") => {
            println!("{}", kit::manifest_json(&manifest()));
            0
        }
        Some("capabilities") => {
            println!(
                "{}",
                serde_json::to_string_pretty(&capabilities_json()).unwrap_or_default()
            );
            0
        }
        Some("ops") => {
            println!("{}", ops::manifest());
            0
        }
        // Engine passthrough — `tamias-driver-lasius run status` is
        // `lasius status`. Also the child side of `cli.*` wire
        // queries (ops::run_verb re-execs this same verb).
        Some("run") => {
            let argv = std::iter::once("lasius".to_string()).chain(args.iter().skip(1).cloned());
            std::process::exit(lasius::run_from(argv));
        }
        // The tamias `bins[]` shim verb (T-012): `tamias bin exec`
        // runs `<driver> cli --dir <payload> --name <pkg> <lasius
        // argv>` for a declared `shim: cli` bin. lasius's engine IS
        // the CLI, so this is a passthrough — strip the two standard
        // driver flags and hand the rest to `lasius::run_from`
        // with inherited stdio. That keeps full engine parity a
        // generated ops→CLI surface can't express: globals before
        // the verb, `--json`, `--help`, the unbounded `serve`
        // ndjson loop, and stdin-bound verbs (`rm`'s confirm,
        // `exec`'s shell) all behave exactly like the retired
        // `lasius` binary.
        Some("cli") => {
            let argv = engine_argv(&args[1..]);
            std::process::exit(lasius::run_from(argv));
        }
        // Optional embedded surface (feature-gated): the
        // pavo-driver-kit surface layer renders this driver's own
        // ops manifest as MCP tools over stdio — the same ops the
        // `cli.*` wire queries and the standalone
        // tamias-driver-mcp binary expose.
        #[cfg(feature = "mcp")]
        Some("mcp") => {
            let m = kit::surface::OpsManifest::from_value(&ops::manifest()).unwrap_or_default();
            match kit::surface::mcp::serve_stdio(&m, &mut surface::Engine) {
                Ok(()) => 0,
                Err(e) => {
                    eprintln!("tamias-driver-lasius mcp: {e}");
                    1
                }
            }
        }
        Some("serve") | None => serve(),
        _ => {
            eprintln!(
                "usage: tamias-driver-lasius manifest|capabilities|ops|run <argv>|cli <lasius argv>|serve\n       (feature `mcp` adds `mcp`)"
            );
            1
        }
    };
    std::process::exit(code);
}
