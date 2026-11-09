//! `lasius serve` — ndjson JSON-RPC over stdio, same wire as
//! `tamias serve` / `sepia serve` / `cuora serve` / `myco serve`.
//!
//! Requests are `{"op":"…", …}`; responses are
//! `{"ok":true,"result":…}` or `{"ok":false,"error":"…"}`.
//!
//! Read-only ops only — mutating commands print progress to stdout and
//! would corrupt the protocol, so `clone`/`update`/`rm` stay CLI-side.

use std::io::{BufRead, Write};

use anyhow::Result;
use serde_json::{json, Value};

use crate::support::{self, Workspace};

/// Ops exposed over the wire.
const OPS: &[&str] = &["ping", "list", "status", "doctor"];

/// Run the serve loop until stdin closes.
pub fn serve(ws: &Workspace) -> Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let response = handle(ws, &line);
        writeln!(stdout, "{response}")?;
        stdout.flush()?;
    }
    Ok(())
}

fn handle(ws: &Workspace, line: &str) -> String {
    match dispatch(ws, line) {
        Ok(result) => json!({"ok": true, "result": result}).to_string(),
        Err(e) => json!({"ok": false, "error": format!("{e:#}")}).to_string(),
    }
}

fn dispatch(ws: &Workspace, line: &str) -> Result<Value> {
    let req: Value = serde_json::from_str(line)?;
    let op = req
        .get("op")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing 'op'"))?;
    match op {
        "ping" => Ok(json!({"pong": true})),
        "list" => {
            let names = support::local_repos(ws)?;
            Ok(json!({"repos": names, "count": names.len()}))
        }
        "status" => crate::commands::status_rows(ws).map(|rows| {
            let items: Vec<Value> = rows
                .iter()
                .map(|r| {
                    json!({
                        "name": r.name,
                        "branch": r.status.branch,
                        "upstream": r.status.upstream,
                        "ahead": r.status.ahead,
                        "behind": r.status.behind,
                        "dirty": r.status.dirty,
                    })
                })
                .collect();
            json!({"repos": items})
        }),
        "doctor" => crate::commands::doctor_checks(ws).map(|checks| {
            json!({
                "checks": checks.iter().map(|(name, ok, detail)| {
                    json!({"name": name, "ok": ok, "detail": detail})
                }).collect::<Vec<_>>(),
            })
        }),
        other => anyhow::bail!("unknown op {other:?} — expected one of: {}", OPS.join(", ")),
    }
}
