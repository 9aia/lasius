//! VS Code `*.code-workspace` file generation.

use serde_json::{json, Map, Value};

/// Search-exclude settings shared by every generated workspace file.
fn search_exclude() -> Value {
    let mut map = Map::new();
    for pat in [
        "**/node_modules",
        "**/dist",
        "**/coverage",
        "**/.alchemy",
        "**/.output",
        "**/.vite",
        "**/target",
    ] {
        map.insert(pat.to_string(), Value::Bool(true));
    }
    Value::Object(map)
}

/// Build the contents of `<org>.code-workspace`: the workspace root plus one
/// folder per clone under `repos_dir`.
///
/// `root_name` is the display name of the root folder (the org name).
pub fn build_workspace_file(org: &str, repos_dir: &str, repo_names: &[String]) -> String {
    let mut names: Vec<&String> = repo_names.iter().collect();
    names.sort();

    let mut folders = vec![json!({ "name": org, "path": "." })];
    for name in names {
        folders.push(json!({
            "name": name,
            "path": format!("{repos_dir}/{name}"),
        }));
    }

    let doc = json!({
        "folders": folders,
        "settings": { "search.exclude": search_exclude() },
    });

    // serde_json::to_string_pretty uses 2 spaces; VS Code accepts any JSON,
    // but tabs match the rest of the 9aia toolchain style.
    let pretty = serde_json::to_string_pretty(&doc).unwrap_or_else(|_| doc.to_string());
    pretty.replace("  ", "\t") + "\n"
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn includes_root_and_sorted_repos() {
        let names = vec!["beta".to_string(), "alpha".to_string()];
        let out = build_workspace_file("ktbhealth", "repos", &names);
        let doc: Value = serde_json::from_str(&out).unwrap();
        let folders = doc["folders"].as_array().unwrap();
        let got: Vec<(&str, &str)> = folders
            .iter()
            .map(|f| (f["name"].as_str().unwrap(), f["path"].as_str().unwrap()))
            .collect();
        assert_eq!(
            got,
            vec![
                ("ktbhealth", "."),
                ("alpha", "repos/alpha"),
                ("beta", "repos/beta"),
            ]
        );
    }

    #[test]
    fn root_only_when_no_repos() {
        let out = build_workspace_file("o", "repos", &[]);
        let doc: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(doc["folders"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn excludes_node_modules_and_target() {
        let out = build_workspace_file("o", "repos", &[]);
        let doc: Value = serde_json::from_str(&out).unwrap();
        let ex = &doc["settings"]["search.exclude"];
        assert_eq!(ex["**/node_modules"], Value::Bool(true));
        assert_eq!(ex["**/target"], Value::Bool(true));
    }
}
