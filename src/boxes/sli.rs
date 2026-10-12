//! SLI console box — command catalog from `src/bin/` (Rust) + `cargo xtask`.
//!
//! Product tests, benches, and scripts are `.rs`. Shell wrappers in `bin/` /
//! `scripts/` are not catalogued.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::boxes::xtask;
use crate::vision;

/// One SLI catalog entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SliEntry {
    /// Command name (file stem).
    pub name: String,
    /// Repo-relative path.
    pub path: String,
    /// Kind: rs | xtask.
    pub kind: String,
    /// One-line description (doc comment / shebang doc).
    pub description: String,
    /// Whether the command appears in recent history (used).
    pub used: bool,
    /// Invocation example.
    pub example: String,
}

/// Full SLI catalog wire.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SliCatalog {
    /// Catalog entries (sorted by name).
    pub entries: Vec<SliEntry>,
    /// Directory roots scanned.
    pub roots: Vec<String>,
    /// Count of used commands.
    pub used_count: usize,
    /// Count of unused scripts (potential new SLI functions).
    pub unused_count: usize,
}

/// `/api/sli` response wire.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliWire {
    pub catalog: SliCatalog,
    pub generated_at: String,
}

impl SliCatalog {
    /// Scan the repo (best effort): read-only, never mutates. The `used` flag is
    /// derived from recent commands gathered **once** from both
    /// `~/.bash_history` and the durable tracker store (`command_labels`) —
    /// the latter covers non-interactive `bash -lc` / MCP `/api/terminal` runs
    /// that never touch `~/.bash_history` (fix for the always-`used_count=0` bug).
    pub fn scan(repo_root: &Path, data_dir: &Path) -> Self {
        let mut recent = crate::tracker::recent_commands(500);
        recent.extend(crate::tracker::command_labels(data_dir, 500));
        let mut entries = Vec::new();
        let dir = repo_root.join("src/bin");
        if let Ok(read) = fs::read_dir(&dir) {
            for entry in read.flatten() {
                let path = entry.path();
                if path.extension().is_none_or(|e| e != "rs") {
                    continue;
                }
                let Some(name) = path.file_stem().and_then(|s| s.to_str()) else {
                    continue;
                };
                let description = first_doc_line(&path);
                entries.push(SliEntry {
                    example: example_for(name),
                    kind: "rs".to_string(),
                    path: format!("src/bin/{name}.rs"),
                    description,
                    name: name.to_string(),
                    used: is_used(name, &recent),
                });
            }
        }
        for (name, desc) in xtask::TASKS {
            entries.push(SliEntry {
                example: format!("cargo xtask {name}"),
                kind: "xtask".to_string(),
                path: "src/boxes/xtask.rs".into(),
                description: (*desc).into(),
                name: (*name).into(),
                used: is_used(name, &recent),
            });
        }
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        let used_count = entries.iter().filter(|e| e.used).count();
        let unused_count = entries.len() - used_count;
        Self {
            entries,
            roots: vec!["src/bin/".into(), "cargo xtask".into()],
            used_count,
            unused_count,
        }
    }
}

/// First doc/comment line for a Rust bin.
fn first_doc_line(path: &Path) -> String {
    let Ok(raw) = fs::read_to_string(path) else {
        return String::new();
    };
    for line in raw.lines() {
        let line = line.trim_start();
        if let Some(desc) = line
            .strip_prefix("//!")
            .or_else(|| line.strip_prefix("///"))
        {
            let desc = desc.trim();
            if !desc.is_empty() {
                return desc.to_string();
            }
        }
    }
    String::new()
}

fn example_for(name: &str) -> String {
    if name == "gsv_xtask" {
        "cargo xtask <task>".into()
    } else if name == "gsv_live" {
        "cargo xtask live".into()
    } else {
        format!("cargo run --bin {} -- …", name.replace('_', "-"))
    }
}

/// Whether the command name appears in the recent-commands haystack. The
/// haystack is gathered once from `~/.bash_history` + the durable tracker
/// `command_labels`, so non-interactive / MCP-terminal runs still mark `used`.
/// `rs` bin stems use `_` (`gsv_ui_probe`) but are invoked with `-`
/// (`cargo run --bin gsv-ui-probe`), so both spellings are matched.
fn is_used(name: &str, recent: &[String]) -> bool {
    let hyphen = name.replace('_', "-");
    recent
        .iter()
        .any(|cmd| cmd.contains(name) || cmd.contains(&hyphen))
}

/// Serve `/api/sli`.
pub fn wire(repo_root: &Path, data_dir: &Path) -> SliWire {
    SliWire {
        catalog: SliCatalog::scan(repo_root, data_dir),
        generated_at: vision::rfc3339_now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_doc_line_rs() {
        let dir = std::env::temp_dir().join(format!("gsv-sli-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let f = dir.join("tool.rs");
        fs::write(&f, "//! A tool doc\n//! second\nfn main() {}").expect("write");
        assert_eq!(first_doc_line(&f), "A tool doc");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn example_for_xtask_bin() {
        assert_eq!(example_for("gsv_xtask"), "cargo xtask <task>");
        assert!(example_for("gsv_server").starts_with("cargo run --bin"));
    }

    #[test]
    fn scan_includes_xtask_products() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let data = std::env::temp_dir().join(format!("gsv-sli-empty-{}", std::process::id()));
        let _ = fs::remove_dir_all(&data);
        let cat = SliCatalog::scan(&root, &data);
        assert!(cat
            .entries
            .iter()
            .any(|e| e.name == "products" && e.kind == "xtask"));
        assert!(cat.entries.iter().any(|e| e.kind == "rs"));
        assert!(!cat.entries.iter().any(|e| e.kind == "sh"));
        assert_eq!(
            cat.used_count + cat.unused_count,
            cat.entries.len(),
            "used+unused must equal entries"
        );
        let _ = fs::remove_dir_all(&data);
    }

    #[test]
    fn is_used_matches_hyphen_invocation_of_underscore_bin() {
        let recent = vec![
            "cargo run --bin gsv-ui-probe -- --json".to_string(),
            "cargo xtask rules-check".to_string(),
        ];
        assert!(
            is_used("gsv_ui_probe", &recent),
            "underscore bin via hyphen cmd"
        );
        assert!(is_used("rules-check", &recent));
        assert!(is_used("gsv_mcp", &["gsv-mcp".to_string()]));
    }

    #[test]
    fn command_labels_reads_durable_tracker_records() {
        let dir = std::env::temp_dir().join(format!("gsv-sli-cmd-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let json = r#"{"records":[
            {"id":"a","kind":"command","label":"cargo xtask rules-check","detail":"ok","status":"ok","at":"2026-01-01T00:00:00Z"},
            {"id":"b","kind":"status","label":"not-a-command","detail":"x","status":"ok","at":"2026-01-01T00:00:00Z"}
        ],"sprints":{"open":[],"closed":[],"next":null,"total":0},"last_saved":null}"#;
        fs::write(dir.join("gsv_tracker.json"), json).unwrap();
        let labels = crate::tracker::command_labels(&dir, 500);
        assert!(labels.iter().any(|l| l == "cargo xtask rules-check"));
        assert!(
            !labels.iter().any(|l| l == "not-a-command"),
            "only kind==command is returned"
        );
        // scan must mark `rules-check` used from the durable store alone.
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let cat = SliCatalog::scan(&root, &dir);
        assert!(
            cat.entries
                .iter()
                .any(|e| e.name == "rules-check" && e.used),
            "tracker command feeds the used flag"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
