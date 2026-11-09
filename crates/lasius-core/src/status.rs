//! `git status --porcelain=v2 --branch` parsing and table formatting.

/// Parsed git status for one clone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitStatus {
    pub branch: String,
    pub upstream: bool,
    pub ahead: u32,
    pub behind: u32,
    pub dirty: u32,
}

const BRANCH_HEAD: &str = "# branch.head ";
const BRANCH_AB: &str = "# branch.ab ";

/// Parse porcelain v2 output into a [`GitStatus`].
pub fn parse_git_status(porcelain: &str) -> GitStatus {
    let mut branch = "(detached)".to_string();
    let mut upstream = false;
    let mut ahead = 0;
    let mut behind = 0;
    let mut dirty = 0u32;

    for line in porcelain.lines() {
        if let Some(rest) = line.strip_prefix(BRANCH_HEAD) {
            branch = rest.to_string();
        } else if let Some(rest) = line.strip_prefix(BRANCH_AB) {
            upstream = true;
            // "# branch.ab +A -B"
            for tok in rest.split_whitespace() {
                if let Some(n) = tok.strip_prefix('+') {
                    ahead = n.parse().unwrap_or(0);
                } else if let Some(n) = tok.strip_prefix('-') {
                    behind = n.parse().unwrap_or(0);
                }
            }
        } else if !line.is_empty() && !line.starts_with('#') {
            dirty += 1;
        }
    }
    GitStatus {
        branch,
        upstream,
        ahead,
        behind,
        dirty,
    }
}

/// One row of the status table.
#[derive(Debug, Clone)]
pub struct StatusRow {
    pub name: String,
    pub status: GitStatus,
}

fn pad(s: &str, width: usize) -> String {
    format!("{s:<width$}", width = width)
}

/// Format one row: `name  branch  dirty  sync`.
pub fn format_status_row(name: &str, status: &GitStatus) -> String {
    let sync = if status.upstream {
        format!("+{}/-{}", status.ahead, status.behind)
    } else {
        "no upstream".to_string()
    };
    format!(
        "{} {} {:>6}  {}",
        pad(name, 22),
        pad(&status.branch, 18),
        status.dirty,
        sync
    )
}

/// Column header matching [`format_status_row`].
pub fn format_status_header() -> String {
    format!(
        "{} {} {:>6}  {}",
        pad("NAME", 22),
        pad("BRANCH", 18),
        "DIRTY",
        "SYNC"
    )
}

/// Full table: header + one line per row.
pub fn format_status_table(rows: &[StatusRow]) -> String {
    let mut out = format_status_header();
    for row in rows {
        out.push('\n');
        out.push_str(&format_status_row(&row.name, &row.status));
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn parses_clean_branch_with_upstream() {
        let s = parse_git_status("# branch.oid abc\n# branch.head main\n# branch.upstream origin/main\n# branch.ab +2 -1\n");
        assert_eq!(s.branch, "main");
        assert!(s.upstream);
        assert_eq!(s.ahead, 2);
        assert_eq!(s.behind, 1);
        assert_eq!(s.dirty, 0);
    }

    #[test]
    fn parses_detached_and_dirty() {
        let s = parse_git_status(
            "# branch.head (detached)\n1 .M N... 100644 100644 100644 abc file.rs\n? notes.txt\n",
        );
        assert_eq!(s.branch, "(detached)");
        assert!(!s.upstream);
        assert_eq!(s.dirty, 2);
    }

    #[test]
    fn empty_input_is_detached() {
        let s = parse_git_status("");
        assert_eq!(s.branch, "(detached)");
        assert_eq!(s.dirty, 0);
    }

    #[test]
    fn table_format() {
        let rows = vec![StatusRow {
            name: "alpha".into(),
            status: GitStatus {
                branch: "main".into(),
                upstream: true,
                ahead: 1,
                behind: 0,
                dirty: 3,
            },
        }];
        let table = format_status_table(&rows);
        let lines: Vec<&str> = table.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("NAME"));
        assert!(lines[1].contains("alpha"));
        assert!(lines[1].contains("+1/-0"));
    }
}
