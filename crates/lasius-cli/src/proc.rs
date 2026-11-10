//! Thin process layer — spawn `git`/`gh`/`sh`, capture output, run in parallel.

use std::io::{self, Read};
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Captured result of a finished process.
#[derive(Debug)]
pub struct ProcOutput {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
    /// The process was killed for exceeding its timeout.
    pub timed_out: bool,
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
        timed_out: false,
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

/// [`capture_env`] with a deadline: the child is killed and `timed_out`
/// set when it runs past `timeout`. Stdout/stderr drain on reader
/// threads so a chatty child can't deadlock the poll loop.
pub fn capture_env_timeout(
    program: &str,
    args: &[String],
    cwd: Option<&Path>,
    envs: &[(&str, &str)],
    timeout: Duration,
) -> io::Result<ProcOutput> {
    let mut cmd = Command::new(program);
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    for (key, value) in envs {
        cmd.env(key, value);
    }

    let mut child = cmd.spawn()?;
    let (mut out_pipe, mut err_pipe) = match (child.stdout.take(), child.stderr.take()) {
        (Some(o), Some(e)) => (o, e),
        _ => return Err(io::Error::other("failed to capture child pipes")),
    };
    let out_reader = thread::spawn(move || {
        let mut buf = Vec::new();
        out_pipe.read_to_end(&mut buf).map(|_| buf)
    });
    let err_reader = thread::spawn(move || {
        let mut buf = Vec::new();
        err_pipe.read_to_end(&mut buf).map(|_| buf)
    });

    let deadline = Instant::now() + timeout;
    let (status, timed_out) = loop {
        match child.try_wait()? {
            Some(status) => break (status, false),
            None if Instant::now() >= deadline => {
                child.kill()?;
                break (child.wait()?, true);
            }
            None => thread::sleep(Duration::from_millis(50)),
        }
    };

    let stdout = out_reader.join().unwrap_or_else(|_| Ok(Vec::new()))?;
    let stderr = err_reader.join().unwrap_or_else(|_| Ok(Vec::new()))?;
    Ok(ProcOutput {
        code: status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
        timed_out,
    })
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use std::time::Instant;

    fn args(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn capture_returns_exit_code_and_output() {
        let out = capture("sh", &args(&["-c", "echo hi; exit 3"]), None).unwrap();
        assert_eq!(out.code, 3);
        assert_eq!(out.stdout.trim(), "hi");
        assert!(!out.success());
        assert!(!out.timed_out);
    }

    #[test]
    fn timeout_kills_runaway_process() {
        let start = Instant::now();
        let out = capture_env_timeout(
            "sh",
            &args(&["-c", "sleep 30"]),
            None,
            &[],
            Duration::from_millis(200),
        )
        .unwrap();
        assert!(out.timed_out);
        assert!(!out.success());
        assert!(start.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn fast_process_is_not_timed_out() {
        let out = capture_env_timeout(
            "sh",
            &args(&["-c", "exit 0"]),
            None,
            &[],
            Duration::from_secs(5),
        )
        .unwrap();
        assert!(out.success());
        assert!(!out.timed_out);
    }
}
