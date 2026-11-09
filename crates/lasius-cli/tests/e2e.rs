#![allow(clippy::unwrap_used)]
//! End-to-end tests: run the real `lasius` binary against a temp workspace
//! with a fake `gh` on PATH and real `file://` git repos.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_lasius")
}

fn run(args: &[&str], cwd: &Path, path_env: &str) -> std::process::Output {
    Command::new(bin())
        .args(args)
        .current_dir(cwd)
        .env("PATH", path_env)
        .output()
        .expect("spawn lasius")
}

fn stdout(o: &std::process::Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &std::process::Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

/// A real git repo with one commit — `file://` cloneable.
fn make_git_repo(path: &Path) {
    fs::create_dir_all(path).unwrap();
    let init = Command::new("git")
        .args(["init", "-b", "main"])
        .arg(path)
        .status()
        .unwrap();
    assert!(init.success());
    git(
        path,
        &[
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "--allow-empty",
            "-m",
            "init",
        ],
    );
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?} failed in {}", dir.display());
}

/// Fake `gh` that prints a fixed repo list; returns PATH prefix.
fn fake_gh(dir: &Path, repos: &[(&str, PathBuf)]) -> String {
    let bin_dir = dir.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let payload = repos
        .iter()
        .map(|(name, path)| {
            format!(
                "{{\"name\":\"{name}\",\"sshUrl\":\"file://{}\",\"isArchived\":false}}",
                path.display()
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let script = format!("#!/bin/sh\necho '[{payload}]'\n");
    fs::write(bin_dir.join("gh"), script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(bin_dir.join("gh"), fs::Permissions::from_mode(0o755)).unwrap();
    }
    format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap())
}

/// Workspace root with lasius.toml + org fixture repos; returns (tmp, root, path_env).
fn setup(names: &[&str]) -> (TempDir, PathBuf, String) {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path();
    let org_dir = base.join("org");
    let mut repos = Vec::new();
    for name in names {
        let src = org_dir.join(name);
        make_git_repo(&src);
        repos.push((*name, src));
    }
    let root = base.join("ws");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("lasius.toml"), "org = \"testorg\"\n").unwrap();
    let path_env = fake_gh(base, &repos);
    (tmp, root, path_env)
}

#[test]
fn init_clones_and_generates_workspace() {
    let (_tmp, root, path) = setup(&["alpha", "beta"]);
    let out = run(&["init"], &root, &path);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(stdout(&out).contains("cloned  alpha"));
    assert!(root.join("repos/alpha/.git").exists());

    let ws = fs::read_to_string(root.join("testorg.code-workspace")).unwrap();
    assert!(ws.contains("\"name\": \"alpha\""));
    assert!(ws.contains("repos/alpha"));
}

#[test]
fn init_creates_config_with_org_flag() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path();
    let org_dir = base.join("org");
    let src = org_dir.join("alpha");
    make_git_repo(&src);
    let root = base.join("ws");
    fs::create_dir_all(&root).unwrap();
    let path_env = fake_gh(base, &[("alpha", src)]);

    let out = run(&["init", "--org", "testorg"], &root, &path_env);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(root.join("lasius.toml").exists());
    assert!(root.join("repos/alpha/.git").exists());
    // scaffold artifacts
    let gitignore = fs::read_to_string(root.join(".gitignore")).unwrap();
    assert!(gitignore.contains("/repos/"));
    assert!(gitignore.contains("testorg.code-workspace"));
    assert!(root.join("README.md").exists());
    assert!(root.join("AGENTS.md").exists());
    // idempotent: second init doesn't duplicate gitignore entries
    let out2 = run(&["init"], &root, &path_env);
    assert!(out2.status.success());
    let gitignore2 = fs::read_to_string(root.join(".gitignore")).unwrap();
    assert_eq!(gitignore, gitignore2);
}

#[test]
fn clone_is_idempotent() {
    let (_tmp, root, path) = setup(&["alpha"]);
    assert!(run(&["clone"], &root, &path).status.success());
    let second = run(&["clone"], &root, &path);
    assert!(second.status.success());
    assert!(stdout(&second).contains("skip    alpha"));
}

#[test]
fn update_pulls_every_clone() {
    let (_tmp, root, path) = setup(&["alpha", "beta"]);
    assert!(run(&["clone"], &root, &path).status.success());
    let out = run(&["update"], &root, &path);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(stdout(&out).contains("ok      alpha"));
}

#[test]
fn status_prints_table() {
    let (_tmp, root, path) = setup(&["alpha"]);
    assert!(run(&["clone"], &root, &path).status.success());
    let out = run(&["status"], &root, &path);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("NAME"));
    assert!(text.contains("alpha"));
    assert!(text.contains("main"));
}

#[test]
fn status_json_output() {
    let (_tmp, root, path) = setup(&["alpha"]);
    assert!(run(&["clone"], &root, &path).status.success());
    let out = run(&["status", "--json"], &root, &path);
    assert!(out.status.success());
    let doc: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(doc[0]["name"], "alpha");
    assert_eq!(doc[0]["branch"], "main");
}

#[test]
fn code_workspace_generate_regenerates() {
    let (_tmp, root, path) = setup(&["alpha"]);
    assert!(run(&["clone"], &root, &path).status.success());
    fs::remove_file(root.join("testorg.code-workspace")).unwrap();
    let out = run(&["code-workspace", "generate"], &root, &path);
    assert!(out.status.success());
    assert!(root.join("testorg.code-workspace").exists());
}

#[test]
fn doctor_reports_ok_on_healthy_workspace() {
    let (_tmp, root, path) = setup(&["alpha"]);
    assert!(run(&["clone"], &root, &path).status.success());
    let out = run(&["doctor"], &root, &path);
    // gh auth status will fail in the fake env — doctor may exit non-zero,
    // but it must still print the report.
    let text = format!("{}{}", stdout(&out), stderr(&out));
    assert!(text.contains("git"));
    assert!(text.contains("drift"));
}

#[test]
fn rm_refuses_dirty_clone() {
    let (_tmp, root, path) = setup(&["alpha"]);
    assert!(run(&["clone"], &root, &path).status.success());
    fs::write(root.join("repos/alpha/dirty.txt"), "x").unwrap();
    let out = run(&["rm", "alpha", "--yes"], &root, &path);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("dirty"));
    assert!(root.join("repos/alpha").exists());
}

#[test]
fn rm_removes_clean_clone() {
    let (_tmp, root, path) = setup(&["alpha"]);
    assert!(run(&["clone"], &root, &path).status.success());
    // commit the clone's clean state — clone of file:// repo has upstream? no;
    // unpushed check uses --branches --not --remotes: cloned HEAD == remote so ok.
    let out = run(&["rm", "alpha", "--yes"], &root, &path);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(!root.join("repos/alpha").exists());
    // workspace file regenerated without alpha
    let ws = fs::read_to_string(root.join("testorg.code-workspace")).unwrap();
    assert!(!ws.contains("repos/alpha"));
}

#[test]
fn exec_runs_in_each_clone() {
    let (_tmp, root, path) = setup(&["alpha", "beta"]);
    assert!(run(&["clone"], &root, &path).status.success());
    let out = run(&["exec", "--", "pwd"], &root, &path);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("== alpha =="));
    assert!(text.contains("== beta =="));
    assert!(text.contains("repos/alpha"));
}

#[test]
fn exec_filter_selects_subset() {
    let (_tmp, root, path) = setup(&["alpha", "beta"]);
    assert!(run(&["clone"], &root, &path).status.success());
    let out = run(&["exec", "--filter", "alph", "--", "pwd"], &root, &path);
    assert!(out.status.success());
    assert!(stdout(&out).contains("== alpha =="));
    assert!(!stdout(&out).contains("== beta =="));
}

#[test]
fn add_clones_single_repo() {
    let (_tmp, root, path) = setup(&["alpha", "beta"]);
    let out = run(&["add", "beta"], &root, &path);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(root.join("repos/beta/.git").exists());
    assert!(!root.join("repos/alpha/.git").exists());
}

#[test]
fn fails_outside_workspace() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(&["status"], tmp.path(), &std::env::var("PATH").unwrap());
    assert!(!out.status.success());
    assert!(stderr(&out).contains("lasius.toml"));
}

#[test]
fn unsafe_name_rejected_by_core_paths() {
    // gh output with traversal name must fail decode, not write outside repos/
    let tmp = tempfile::tempdir().unwrap();
    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    fs::write(
        bin_dir.join("gh"),
        "#!/bin/sh\necho '[{\"name\":\"..\",\"sshUrl\":\"u\",\"isArchived\":false}]'\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(bin_dir.join("gh"), fs::Permissions::from_mode(0o755)).unwrap();
    }
    let root = tmp.path().join("ws");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("lasius.toml"), "org = \"x\"\n").unwrap();
    let path_env = format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap());
    let out = run(&["clone"], &root, &path_env);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("unsafe repo name"));
}

#[test]
fn serve_answers_jsonrpc_ops() {
    let (_tmp, root, path) = setup(&["alpha"]);
    assert!(run(&["clone"], &root, &path).status.success());

    let mut child = Command::new(bin())
        .args(["serve"])
        .current_dir(&root)
        .env("PATH", &path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    {
        use std::io::Write;
        let mut stdin = child.stdin.take().unwrap();
        writeln!(stdin, "{{\"op\":\"ping\"}}").unwrap();
        writeln!(stdin, "{{\"op\":\"list\"}}").unwrap();
        writeln!(stdin, "{{\"op\":\"status\"}}").unwrap();
        writeln!(stdin, "{{\"op\":\"doctor\"}}").unwrap();
        writeln!(stdin, "{{\"op\":\"bogus\"}}").unwrap();
    }
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let text = stdout(&out);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 5);
    for line in &lines[..4] {
        let v: serde_json::Value = serde_json::from_str(line).unwrap();
        assert_eq!(v["ok"], true, "line: {line}");
    }
    assert!(lines[0].contains("\"pong\":true"));
    assert!(lines[1].contains("\"alpha\""));
    assert!(lines[2].contains("\"branch\""));
    assert!(lines[3].contains("\"checks\""));
    let bad: serde_json::Value = serde_json::from_str(lines[4]).unwrap();
    assert_eq!(bad["ok"], false);
    assert!(bad["error"].as_str().unwrap().contains("unknown op"));
}
