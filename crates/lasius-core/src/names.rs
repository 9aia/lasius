//! Repo-name safety and glob matching.

/// Whether `name` is safe to use as a single path segment under `repos_dir`.
///
/// Mirrors the rule: ASCII alphanumerics plus `.` `_` `-`, with at least one
/// alphanumeric — rejects `..`, `.`, slashes, whitespace, and empty strings
/// while allowing `.github`.
pub fn is_safe_repo_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        && name.chars().any(|c| c.is_ascii_alphanumeric())
}

/// Glob match where `*` matches any run of characters and `?` one character.
pub fn wildcard_match(pattern: &str, text: &str) -> bool {
    wildcard_match_inner(pattern.as_bytes(), text.as_bytes())
}

fn wildcard_match_inner(pat: &[u8], text: &[u8]) -> bool {
    match pat.first() {
        None => text.is_empty(),
        Some(b'*') => {
            // collapse consecutive stars
            let mut rest = pat;
            while rest.first() == Some(&b'*') {
                rest = &rest[1..];
            }
            if rest.is_empty() {
                return true;
            }
            (0..=text.len()).any(|i| wildcard_match_inner(rest, &text[i..]))
        }
        Some(b'?') => !text.is_empty() && wildcard_match_inner(&pat[1..], &text[1..]),
        Some(&c) => !text.is_empty() && text[0] == c && wildcard_match_inner(&pat[1..], &text[1..]),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn safe_names() {
        for name in ["ui", "product", ".github", "my-repo_2.0", "a"] {
            assert!(is_safe_repo_name(name), "{name} should be safe");
        }
    }

    #[test]
    fn unsafe_names() {
        for name in ["", ".", "..", "a/b", "has space", "a\tb", "../x", "-"] {
            // "-" has no alphanumeric
            assert!(!is_safe_repo_name(name), "{name:?} should be unsafe");
        }
    }

    #[test]
    fn wildcard_basics() {
        assert!(wildcard_match("*", "anything"));
        assert!(wildcard_match("secret-*", "secret-plans"));
        assert!(!wildcard_match("secret-*", "public"));
        assert!(wildcard_match("a?c", "abc"));
        assert!(!wildcard_match("a?c", "ac"));
        assert!(wildcard_match("ui", "ui"));
        assert!(!wildcard_match("ui", "ui2"));
        assert!(wildcard_match("**/x", "a/b/x"));
        assert!(wildcard_match("a**b", "axyzb"));
    }
}
