//! Failure classification — map stderr/stdout text from `git`/`gh`
//! invocations to a coarse [`FailureKind`] so callers can tag output
//! (`access-denied: …`, `not-found: …`) instead of showing raw stderr only.

/// Coarse classification of a failed `git`/`gh` invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    /// Credentials missing, rejected, or lacking permission — SSH key,
    /// gh auth, or repo ACL problem.
    AccessDenied,
    /// The remote/repo doesn't exist (or isn't visible to this account).
    NotFound,
    /// Anything else — network, local fs, corrupt repo, …
    Other,
}

/// True when `code` (e.g. `"403"`) appears in `text` not glued to digits.
fn has_status_code(lower: &str, code: &str) -> bool {
    lower.match_indices(code).any(|(i, _)| {
        let before = lower[..i].chars().next_back();
        let after = lower[i + code.len()..].chars().next();
        before.is_none_or(|c| !c.is_ascii_digit()) && after.is_none_or(|c| !c.is_ascii_digit())
    })
}

fn matches_any(lower: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| lower.contains(n))
}

/// Classify failure text (stderr preferred, stdout as fallback) from
/// `git clone`/`git pull`/`git ls-remote`/`gh`. Case-insensitive.
pub fn classify(text: &str) -> FailureKind {
    let lower = text.to_lowercase();

    // Auth/permission signals first — GitHub reports "Repository not
    // found" for repos the token can't see, but a literal auth error in
    // the same output is the more actionable signal.
    const ACCESS: &[&str] = &[
        "permission denied",
        "authentication failed",
        "authentication required",
        "could not read username",
        "could not read password",
        "terminal prompts disabled",
        "bad credentials",
        "invalid username or password",
        "access denied",
        "forbidden",
        "not logged in",
        "no oauth token",
        "gh auth login",
        "requires authentication",
        "permission to ", // GitHub: "remote: Permission to org/repo.git denied to user."
    ];
    if matches_any(&lower, ACCESS)
        || has_status_code(&lower, "401")
        || has_status_code(&lower, "403")
    {
        return FailureKind::AccessDenied;
    }

    const NOT_FOUND: &[&str] = &[
        "repository not found",
        "could not resolve to a repository",
        "could not find repository",
        "no such repository",
    ];
    if matches_any(&lower, NOT_FOUND) || has_status_code(&lower, "404") {
        return FailureKind::NotFound;
    }

    FailureKind::Other
}

/// Last `max` bytes of `text` on a char boundary, trimmed — for embedding
/// stderr tails in one-line messages without unbounded output.
pub fn tail(text: &str, max: usize) -> String {
    let t = text.trim();
    if t.len() <= max {
        return t.to_string();
    }
    let mut start = t.len() - max;
    while start < t.len() && !t.is_char_boundary(start) {
        start += 1;
    }
    t[start..].trim_start().to_string()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn classifies_ssh_publickey_denied() {
        let err = "git@github.com: Permission denied (publickey).\r\nfatal: Could not read from remote repository.";
        assert_eq!(classify(err), FailureKind::AccessDenied);
    }

    #[test]
    fn classifies_https_auth_failures() {
        for err in [
            "fatal: could not read Username for 'https://github.com': terminal prompts disabled",
            "fatal: Authentication failed for 'https://github.com/acme/ui.git/'",
            "remote: Invalid username or password.\nfatal: Authentication failed",
            "error: HTTP 401: Requires authentication (https://api.github.com/graphql)",
            "gh: not logged in — run `gh auth login`",
            "GraphQL: FORBIDDEN (repository)",
            "remote: Permission to acme/ui.git denied to luis.",
        ] {
            assert_eq!(classify(err), FailureKind::AccessDenied, "{err}");
        }
    }

    #[test]
    fn classifies_not_found() {
        for err in [
            "remote: Repository not found.\nfatal: repository 'https://github.com/acme/gone.git/' not found",
            "error: Could not resolve to a Repository with the name 'acme/gone'.",
            "HTTP 404: Not Found",
        ] {
            assert_eq!(classify(err), FailureKind::NotFound, "{err}");
        }
    }

    #[test]
    fn status_codes_need_boundaries() {
        // "14033" must not trip the 403/401 matchers.
        assert_eq!(
            classify("fatal: unexpected line count 14033"),
            FailureKind::Other
        );
        assert_eq!(classify("HTTP 403 Forbidden"), FailureKind::AccessDenied);
    }

    #[test]
    fn falls_back_to_other() {
        for err in [
            "fatal: '/tmp/x' does not appear to be a git repository",
            "fatal: unable to access 'https://github.com/': Could not resolve host",
            "",
        ] {
            assert_eq!(classify(err), FailureKind::Other, "{err}");
        }
    }

    #[test]
    fn tail_keeps_last_bytes_on_boundary() {
        assert_eq!(tail("short", 300), "short");
        let long = format!("{}tail-end", "x".repeat(500));
        let t = tail(&long, 20);
        assert!(t.ends_with("tail-end"));
        assert!(t.len() <= 20);
        // multibyte safety
        let uni = format!("{}é", "y".repeat(50));
        assert!(tail(&uni, 10).chars().all(|c| c != '\u{fffd}'));
    }
}
