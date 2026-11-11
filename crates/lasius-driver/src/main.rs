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
//!   output never touches the serve stream) and the streaming
//!   surface for `surfaces` verbs.
//! - `serve` — ndjson JSON-RPC on stdio: `{"op":"manifest"}`,
//!   `{"op":"views"}`, `{"op":"view","view":…,"params":{…}}`,
//!   `{"op":"query","query":…,"params":{…}}` →
//!   `{"ok":true,"result":…}` / `{"ok":false,"error":…}`.
//!   `query:"ops"` answers the ops manifest; `query:"cli.<verb>"`
//!   runs the engine op.
//!
//! Optional surfaces: features `cli`/`mcp` embed
//! pavo-driver-kit's surface layer — `cli <op> [args]` is a
//! generated CLI over the ops manifest (`serve`/`open`/`exec` ride
//! as streaming subcommands), `mcp` serves the ops as MCP tools on
//! stdio. Both are equivalent to the standalone
//! `tamias-driver-cli`/`tamias-driver-mcp` surface drivers, minus
//! the PATH/discovery hop.
//!
//! Which workspace: ops inherit the `serve` process's cwd — the
//! `root` param maps to the CLI's `--root` global, and absent it
//! `lasius`'s own upward `lasius.toml` discovery applies, identical
//! to running the `lasius` verb by hand. This driver ships no
//! views — it is the engine-ops surface for lasius workspaces;
//! view/query names outside `ops`/`cli.*` answer `ok:false`.

mod ops;
#[cfg(any(feature = "cli", feature = "mcp"))]
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
        // queries (ops::run_verb re-execs this same verb) and the
        // streaming surface for `surfaces` verbs (`run serve`,
        // `run open`, `run exec -- …`).
        Some("run") => {
            let argv = std::iter::once("lasius".to_string()).chain(args.iter().skip(1).cloned());
            std::process::exit(lasius::run_from(argv));
        }
        // Embedded surface verbs (feature-gated, optional): the
        // pavo-driver-kit surface layer renders this driver's own
        // ops manifest — `cli` = generated clap CLI (op →
        // subcommand), `mcp` = ops → MCP tools over stdio. The
        // standalone tamias-driver-cli/-mcp binaries do the same
        // over process dispatch; these verbs need nothing on PATH.
        #[cfg(feature = "cli")]
        Some("cli") => {
            let m = kit::surface::OpsManifest::from_value(&ops::manifest()).unwrap_or_default();
            let argv: Vec<String> = std::iter::once("lasius".to_string())
                .chain(args.iter().skip(1).cloned())
                .collect();
            kit::surface::cli::run("lasius", &m, &argv, &mut surface::Engine)
        }
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
                "usage: tamias-driver-lasius manifest|capabilities|ops|run <argv>|serve\n       (feature `cli` adds `cli <op> [args]`; feature `mcp` adds `mcp`)"
            );
            1
        }
    };
    std::process::exit(code);
}
