use serde::Deserialize;

use crate::error::CoreError;
use crate::names::is_safe_repo_name;

/// A non-archived repo discovered from a forge. `clone_url` is whatever the
/// provider reports — `git@…` ssh for GitHub today, but https URLs are fine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repo {
    pub name: String,
    pub clone_url: String,
}

#[derive(Debug, Deserialize)]
struct GhRepo {
    name: String,
    #[serde(rename = "sshUrl")]
    clone_url: String,
    #[serde(rename = "isArchived", default)]
    is_archived: bool,
    #[serde(rename = "isPrivate", default)]
    is_private: bool,
}

/// Decode `gh repo list` JSON into sorted, non-archived, name-filtered repos.
///
/// Private repos are dropped unless `include_private` is set. Fails on
/// malformed JSON or unsafe names (checked before filtering, matching the
/// reference implementation).
pub fn decode_repo_list(
    json: &str,
    exclude: &[String],
    include_private: bool,
) -> Result<Vec<Repo>, CoreError> {
    let items: Vec<GhRepo> = serde_json::from_str(json)?;

    if let Some(bad) = items.iter().find(|r| !is_safe_repo_name(&r.name)) {
        return Err(CoreError::UnsafeName(bad.name.clone()));
    }

    let mut repos: Vec<Repo> = items
        .into_iter()
        .filter(|r| !r.is_archived)
        .filter(|r| include_private || !r.is_private)
        .filter(|r| {
            !exclude
                .iter()
                .any(|p| crate::names::wildcard_match(p, &r.name))
        })
        .map(|r| Repo {
            name: r.name,
            clone_url: r.clone_url,
        })
        .collect();
    repos.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(repos)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn json(items: &[(&str, &str, bool)]) -> String {
        let body: Vec<String> = items
            .iter()
            .map(|(n, u, a)| format!("{{\"name\":\"{n}\",\"sshUrl\":\"{u}\",\"isArchived\":{a}}}"))
            .collect();
        format!("[{}]", body.join(","))
    }

    #[test]
    fn decodes_sorts_and_filters_archived() {
        let out = decode_repo_list(
            &json(&[
                ("beta", "git@x:beta.git", false),
                ("old", "git@x:old.git", true),
                ("alpha", "git@x:alpha.git", false),
            ]),
            &[],
            false,
        )
        .unwrap();
        assert_eq!(
            out,
            vec![
                Repo {
                    name: "alpha".into(),
                    clone_url: "git@x:alpha.git".into()
                },
                Repo {
                    name: "beta".into(),
                    clone_url: "git@x:beta.git".into()
                },
            ]
        );
    }

    #[test]
    fn applies_exclude_globs() {
        let out = decode_repo_list(
            &json(&[("secret-x", "u", false), ("ui", "u", false)]),
            &["secret-*".into()],
            false,
        )
        .unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "ui");
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(matches!(
            decode_repo_list("garbage", &[], false),
            Err(CoreError::GhJson(_))
        ));
    }

    #[test]
    fn rejects_wrong_field_types() {
        assert!(decode_repo_list(
            r#"[{"name":"ui","sshUrl":42,"isArchived":false}]"#,
            &[],
            false
        )
        .is_err());
    }

    #[test]
    fn rejects_unsafe_names_even_when_archived() {
        for name in [".", "..", "a/b", "has space", ""] {
            let bad = format!("[{{\"name\":\"{name}\",\"sshUrl\":\"u\",\"isArchived\":true}}]");
            assert!(
                matches!(
                    decode_repo_list(&bad, &[], false),
                    Err(CoreError::UnsafeName(_))
                ),
                "{name:?} should be rejected"
            );
        }
    }

    #[test]
    fn filters_private_unless_included() {
        let payload = r#"[
            {"name":"pub","sshUrl":"u","isArchived":false,"isPrivate":false},
            {"name":"priv","sshUrl":"u","isArchived":false,"isPrivate":true}
        ]"#;
        let out = decode_repo_list(payload, &[], false).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "pub");

        let out = decode_repo_list(payload, &[], true).unwrap();
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn allows_dotfile_repos() {
        let out = decode_repo_list(&json(&[(".github", "u", false)]), &[], false).unwrap();
        assert_eq!(out[0].name, ".github");
    }
}
