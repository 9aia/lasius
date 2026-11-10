//! Thin process layer — spawn `git`/`gh`/`sh`, capture output, run in parallel.

use std::io;
use std::path::Path;
use std::process::{Command, Output};
use std::thread;

/// Captured result of a finished process.
#[derive(Debug)]
pub struct ProcOutput {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl ProcOutput {
    pub fn success(&self) -> bool {
        self.code == 0
    }

    /// stdout's last non-empty line (for `git pull` summaries).
    pub fn last_stdout_line(&self) -> &str {
        self.stdout
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("")
    }
}

fn to_proc(output: Output) -> ProcOutput {
    ProcOutput {
        code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// Spawn `program args…` in `cwd`, capturing stdout/stderr. Never panics on
/// non-zero exit — check [`ProcOutput::success`].
pub fn capture(program: &str, args: &[String], cwd: Option<&Path>) -> io::Result<ProcOutput> {
    capture_env(program, args, cwd, &[])
}

/// [`capture`] plus extra environment variables — e.g. `GIT_TERMINAL_PROMPT=0`
/// to keep credential prompts from hanging a probe.
pub fn capture_env(
    program: &str,
    args: &[String],
    cwd: Option<&Path>,
    envs: &[(&str, &str)],
) -> io::Result<ProcOutput> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    for (key, value) in envs {
        cmd.env(key, value);
    }
    cmd.output().map(to_proc)
}

/// `capture` + label: errors carry `"<label>: <io error>"`.
pub fn capture_labeled(
    label: &str,
    program: &str,
    args: &[String],
    cwd: Option<&Path>,
) -> anyhow::Result<ProcOutput> {
    capture(program, args, cwd).map_err(|e| anyhow::anyhow!("{label}: {e}"))
}

/// Run `f` over `items` on scoped threads, returning results in input order.
pub fn par_map<T, R, F>(items: &[T], f: F) -> Vec<R>
where
    T: Sync,
    R: Send,
    F: Fn(&T) -> R + Sync,
{
    thread::scope(|scope| {
        let handles: Vec<_> = items.iter().map(|item| scope.spawn(|| f(item))).collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or_else(|_| panic!("worker panicked")))
            .collect()
    })
}
