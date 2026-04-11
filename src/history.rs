use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

use crate::resolve::{collapse_tilde, git_show_prefix, git_toplevel_for, main_worktree};

const HALF_LIFE_HOURS: f64 = 168.0; // 1 week

fn db_path() -> PathBuf {
    dirs::home_dir()
        .expect("could not determine home directory")
        .join(".config")
        .join("tp")
        .join("history.db")
}

fn open_db() -> rusqlite::Result<Connection> {
    let path = db_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let conn = Connection::open(&path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS visits (
            path TEXT NOT NULL,
            repo TEXT,
            score REAL NOT NULL DEFAULT 1.0,
            last_visited INTEGER NOT NULL,
            PRIMARY KEY (path, repo)
        );"
    )?;
    Ok(conn)
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_secs() as i64
}

/// Calculate the effective score with half-life decay applied.
fn effective_score(raw_score: f64, last_visited: i64) -> f64 {
    let elapsed_hours = (now_secs() - last_visited) as f64 / 3600.0;
    raw_score * (0.5_f64).powf(elapsed_hours / HALF_LIFE_HOURS)
}

/// Insert or update a visit record. Increments score by 1.0 on revisit.
///
/// SQLite does not consider two NULL values equal in a PRIMARY KEY conflict check,
/// so we use separate INSERT/UPDATE logic when repo is None.
fn upsert_visit(conn: &Connection, path: &str, repo: Option<&str>) -> rusqlite::Result<()> {
    let now = now_secs();
    match repo {
        Some(repo_val) => {
            conn.execute(
                "INSERT INTO visits (path, repo, score, last_visited)
                 VALUES (?1, ?2, 1.0, ?3)
                 ON CONFLICT (path, repo) DO UPDATE SET
                     score = score + 1.0,
                     last_visited = ?3",
                rusqlite::params![path, repo_val, now],
            )?;
        }
        None => {
            let updated = conn.execute(
                "UPDATE visits SET score = score + 1.0, last_visited = ?1
                 WHERE path = ?2 AND repo IS NULL",
                rusqlite::params![now, path],
            )?;
            if updated == 0 {
                conn.execute(
                    "INSERT INTO visits (path, repo, score, last_visited) VALUES (?1, NULL, 1.0, ?2)",
                    rusqlite::params![path, now],
                )?;
            }
        }
    }
    Ok(())
}

/// A frecent directory entry with its effective score.
pub struct FrecencyEntry {
    pub path: String,
    pub repo: Option<String>,
    pub effective_score: f64,
}

/// Query the top N frecent entries, scored with half-life decay.
fn query_frecent_from(conn: &Connection, limit: usize) -> rusqlite::Result<Vec<FrecencyEntry>> {
    let mut stmt = conn.prepare(
        "SELECT path, repo, score, last_visited FROM visits"
    )?;

    let mut entries: Vec<FrecencyEntry> = stmt
        .query_map([], |row| {
            let path: String = row.get(0)?;
            let repo: Option<String> = row.get(1)?;
            let score: f64 = row.get(2)?;
            let last_visited: i64 = row.get(3)?;
            Ok(FrecencyEntry {
                path,
                repo,
                effective_score: effective_score(score, last_visited),
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    entries.sort_by(|a, b| b.effective_score.total_cmp(&a.effective_score));
    entries.truncate(limit);
    Ok(entries)
}

/// Query entries where all tokens appear as substrings in left-to-right order.
/// Tokens are matched against the full path (for non-repo) or "repo/path" (for repo entries).
fn query_by_substring_from(
    conn: &Connection,
    tokens: &[&str],
) -> rusqlite::Result<Vec<FrecencyEntry>> {
    let mut stmt = conn.prepare(
        "SELECT path, repo, score, last_visited FROM visits"
    )?;

    let mut entries: Vec<FrecencyEntry> = stmt
        .query_map([], |row| {
            let path: String = row.get(0)?;
            let repo: Option<String> = row.get(1)?;
            let score: f64 = row.get(2)?;
            let last_visited: i64 = row.get(3)?;
            Ok((path, repo, score, last_visited))
        })?
        .filter_map(|r| r.ok())
        .filter(|(path, repo, _, _)| {
            let search_str = match repo {
                Some(r) => format!("{}/{}", r, path),
                None => path.clone(),
            };
            tokens_match_in_order(&search_str, tokens)
        })
        .map(|(path, repo, score, last_visited)| FrecencyEntry {
            path,
            repo,
            effective_score: effective_score(score, last_visited),
        })
        .collect();

    entries.sort_by(|a, b| b.effective_score.total_cmp(&a.effective_score));
    Ok(entries)
}

/// Check if all tokens appear as case-insensitive substrings in left-to-right order.
fn tokens_match_in_order(haystack: &str, tokens: &[&str]) -> bool {
    let haystack_lower = haystack.to_lowercase();
    let mut search_from = 0;
    for token in tokens {
        let token_lower = token.to_lowercase();
        match haystack_lower[search_from..].find(&token_lower) {
            Some(pos) => search_from += pos + token_lower.len(),
            None => return false,
        }
    }
    true
}

/// Record a directory visit. Called by the chpwd hook via `warp-core log`.
/// Normalizes repo paths to main-worktree-relative form.
/// Silently returns Ok(()) on any error to never break cd.
pub fn record_visit(abs_path: &str) -> rusqlite::Result<()> {
    if is_excluded(abs_path) {
        return Ok(());
    }

    let conn = open_db()?;
    let abs = std::path::Path::new(abs_path);

    // Check if we're inside a git repo
    let toplevel = git_toplevel_for(abs);

    match toplevel {
        Some(toplevel) => {
            let main_wt = main_worktree(&toplevel);
            let repo_str = collapse_tilde(&main_wt);
            let rel_path = git_show_prefix(abs).unwrap_or_default();

            // At repo root, store empty string as path
            upsert_visit(&conn, &rel_path, Some(&repo_str))
        }
        None => {
            let collapsed = collapse_tilde(abs);
            upsert_visit(&conn, &collapsed, None)
        }
    }
}

/// Get the top N frecent entries. Returns empty vec on DB errors.
pub fn top_frecent(limit: usize) -> Vec<FrecencyEntry> {
    let conn = match open_db() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    query_frecent_from(&conn, limit).unwrap_or_default()
}

/// Find frecent entries matching all tokens as substrings in order.
/// Returns empty vec on DB errors.
pub fn find_by_substring(tokens: &[&str]) -> Vec<FrecencyEntry> {
    let conn = match open_db() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    query_by_substring_from(&conn, tokens).unwrap_or_default()
}

/// Remove an entry from history (used for stale entry cleanup).
pub fn remove_entry(path: &str, repo: Option<&str>) -> rusqlite::Result<()> {
    let conn = open_db()?;
    conn.execute(
        "DELETE FROM visits WHERE path = ?1 AND repo IS ?2",
        rusqlite::params![path, repo],
    )?;
    Ok(())
}

/// Check if a path should be excluded from history tracking.
fn is_excluded(path: &str) -> bool {
    let home = dirs::home_dir()
        .expect("could not determine home directory")
        .display()
        .to_string();

    // Home directory itself (with or without trailing slash)
    if path == home || path == format!("{}/", home) {
        return true;
    }

    path == "/tmp"
        || path.starts_with("/tmp/")
        || path == "/private/tmp"
        || path.starts_with("/private/tmp/")
        || path.starts_with("/nix/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE visits (
                path TEXT NOT NULL,
                repo TEXT,
                score REAL NOT NULL DEFAULT 1.0,
                last_visited INTEGER NOT NULL,
                PRIMARY KEY (path, repo)
            );"
        ).unwrap();
        conn
    }

    #[test]
    fn record_non_repo_path() {
        let conn = test_db();
        upsert_visit(&conn, "/Users/me/Documents", None).unwrap();

        let (score, repo): (f64, Option<String>) = conn
            .query_row(
                "SELECT score, repo FROM visits WHERE path = ?1",
                ["/Users/me/Documents"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert!((score - 1.0).abs() < f64::EPSILON);
        assert!(repo.is_none());
    }

    #[test]
    fn record_increments_score() {
        let conn = test_db();
        upsert_visit(&conn, "/Users/me/Documents", None).unwrap();
        upsert_visit(&conn, "/Users/me/Documents", None).unwrap();
        upsert_visit(&conn, "/Users/me/Documents", None).unwrap();

        let score: f64 = conn
            .query_row(
                "SELECT score FROM visits WHERE path = ?1",
                ["/Users/me/Documents"],
                |row| row.get(0),
            )
            .unwrap();

        assert!(score > 2.9 && score < 3.1);
    }

    #[test]
    fn record_repo_relative_path() {
        let conn = test_db();
        upsert_visit(&conn, "src/components", Some("~/r/app")).unwrap();

        let (path, repo): (String, String) = conn
            .query_row(
                "SELECT path, repo FROM visits WHERE repo IS NOT NULL",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert_eq!(path, "src/components");
        assert_eq!(repo, "~/r/app");
    }

    #[test]
    fn excluded_paths_are_skipped() {
        assert!(is_excluded("/tmp"));
        assert!(is_excluded("/private/tmp"));
        assert!(is_excluded("/nix/store/something"));
    }

    #[test]
    fn normal_paths_not_excluded() {
        assert!(!is_excluded("/Users/me/code/project"));
        assert!(!is_excluded("/Users/me/Documents"));
    }

    #[test]
    fn query_frecent_returns_top_entries() {
        let conn = test_db();
        let now = now_secs();

        conn.execute(
            "INSERT INTO visits (path, repo, score, last_visited) VALUES (?1, NULL, ?2, ?3)",
            rusqlite::params!["~/Documents", 10.0, now],
        ).unwrap();
        conn.execute(
            "INSERT INTO visits (path, repo, score, last_visited) VALUES (?1, NULL, ?2, ?3)",
            rusqlite::params!["~/Downloads", 5.0, now],
        ).unwrap();
        conn.execute(
            "INSERT INTO visits (path, repo, score, last_visited) VALUES (?1, NULL, ?2, ?3)",
            rusqlite::params!["~/code", 20.0, now],
        ).unwrap();

        let results = query_frecent_from(&conn, 2).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].path, "~/code");
        assert_eq!(results[1].path, "~/Documents");
    }

    #[test]
    fn query_substring_matches_in_order() {
        let conn = test_db();
        let now = now_secs();

        conn.execute(
            "INSERT INTO visits (path, repo, score, last_visited) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params!["src/components", "~/r/app", 10.0, now],
        ).unwrap();
        conn.execute(
            "INSERT INTO visits (path, repo, score, last_visited) VALUES (?1, NULL, ?2, ?3)",
            rusqlite::params!["~/components/src", 5.0, now],
        ).unwrap();

        // "src comp" should match "src/components" but not "components/src"
        let results = query_by_substring_from(&conn, &["src", "comp"]).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "src/components");
    }

    #[test]
    fn query_substring_single_token() {
        let conn = test_db();
        let now = now_secs();

        conn.execute(
            "INSERT INTO visits (path, repo, score, last_visited) VALUES (?1, NULL, ?2, ?3)",
            rusqlite::params!["~/Documents/notes", 10.0, now],
        ).unwrap();
        conn.execute(
            "INSERT INTO visits (path, repo, score, last_visited) VALUES (?1, NULL, ?2, ?3)",
            rusqlite::params!["~/Downloads", 5.0, now],
        ).unwrap();

        let results = query_by_substring_from(&conn, &["doc"]).unwrap();
        // "doc" matches "Documents" (case-insensitive) but not "Downloads"
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "~/Documents/notes");
    }

    #[test]
    fn old_entries_have_lower_effective_score() {
        let now = now_secs();
        let one_week_ago = now - (168 * 3600);

        let recent = effective_score(10.0, now);
        let old = effective_score(10.0, one_week_ago);

        // After one half-life, score should be ~half
        assert!(recent > old * 1.8); // roughly 2x difference
        assert!(old > 0.0); // but not zero
    }
}
