//! Format inventory — GEN-0 of the format-hygiene plan (`cargo xtask inventory`).
//!
//! Scans an environment root (default: parent of this repo) and classifies
//! every file against the GSV allowed-format canon (`.rs/.md/.mdc/.json/.js/
//! .wasm`), reporting per repo: allowed · violating · tracked · exempt ·
//! deferred · actionable · untracked. Exemption/deferral reasons mirror
//! `docs/gsv/FORMAT_EXEMPTIONS.md` (CNV-0/EXM-1 of the plan).
//!
//! Skip rules match the baseline shell sweep exactly:
//! - environment level (direct children of root): hidden entries skipped
//!   (shell `*/` glob), non-dirs skipped (`find` per-dir baseline);
//! - inside a repo, at any depth: dirs `.git`, `node_modules`, `target`
//!   skipped (shell `grep -v '/.git/' … '/target/'`); other hidden entries
//!   (`.cargo`, `.agents`, `.gitignore`) ARE counted;
//! - symlinks are never counted and never descended into (`find -type f`).
//!
//! Tracked counts come from `git ls-files -z` per direct-child git repo, so
//! `tracked` = violating ∩ index (the `9 287` baseline), merged into the full
//! walk. Reasons are pure functions of the repo-relative path — unit-tested.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::Command;

use serde::Serialize;

/// Allowed source formats (GSV AGENTS.md canon), full suffixes (grep `$` parity).
pub const ALLOWED_SUFFIXES: &[&str] = &[".rs", ".md", ".mdc", ".json", ".js", ".wasm"];

/// Directory names skipped at any depth inside a repo (VCS/deps/build output).
pub const SKIP_DIRS: &[&str] = &[".git", "node_modules", "target"];

/// Classification of one file relative to the format canon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Class {
    /// Ends with an allowed suffix — no action.
    Allowed,
    /// Documented exception (registry `FORMAT_EXEMPTIONS.md`).
    Exempt(&'static str),
    /// Scheduled/owner-gated work stream (CLN/CNV/GEN tickets).
    Deferred(&'static str),
    /// Real work: convert to an allowed format or remove.
    Actionable,
}

impl Class {
    pub fn reason(&self) -> Option<&'static str> {
        match self {
            Class::Allowed => None,
            Class::Exempt(r) | Class::Deferred(r) => Some(r),
            Class::Actionable => None,
        }
    }
    pub fn is_allowed(&self) -> bool {
        matches!(self, Class::Allowed)
    }
}

/// True when `rel` (repo-relative, `/`-separated) ends with an allowed suffix.
pub fn is_allowed(rel: &str) -> bool {
    ALLOWED_SUFFIXES.iter().any(|s| rel.ends_with(s))
}

/// True for dir names skipped at any depth inside a repo.
pub fn skip_dir(name: &str) -> bool {
    SKIP_DIRS.contains(&name)
}

fn basename(rel: &str) -> &str {
    rel.rsplit('/').next().unwrap_or(rel)
}

/// Extension after the last dot of the basename; dotfiles without a second
/// dot (`.gitignore`, `.env`) and extension-less names yield `""`.
fn extension(rel: &str) -> &str {
    let b = basename(rel);
    match b.rsplit_once('.') {
        Some((head, e)) if !head.is_empty() => e,
        _ => "",
    }
}

/// Tracked secret-shaped names are never exempt (SEC visibility).
fn is_secret_basename(base: &str) -> bool {
    base == ".env"
        || base.starts_with(".env.")
        || base.ends_with(".pem")
        || base.ends_with(".key")
        || base.ends_with(".token")
}

/// Deferral streams (plan tickets CLN-1..4, GEN-4, CNV-1, ECO-8.2, owner fences).
fn defer_reason(rel: &str) -> Option<&'static str> {
    if rel.starts_with("omniroute/.build/") {
        return Some("CLN-1 next-build output (untracked)");
    }
    if rel.starts_with("llama-rs/.cmake/") {
        return Some("CLN-2 vendored cmake distro (owner)");
    }
    if rel.starts_with("llama-rs/llama.cpp-master/") {
        return Some("vendored llama.cpp (owner)");
    }
    if rel.starts_with("llama-rs/patches/") {
        return Some("ECO-8.2 llama-cpp patch-set (owner)");
    }
    if rel.starts_with("rebook/") {
        return Some("owner: book-repo untouchable");
    }
    if rel.starts_with("mesh-node/") {
        return Some("GEN-4 mesh-node scripts/images (owner)");
    }
    if rel.starts_with("rusty-engine/data/models/") {
        return Some("model weights/source artifacts");
    }
    if rel.starts_with("poolAI/.tmp") {
        return Some("CLN-3 test artifacts (untracked)");
    }
    if rel.starts_with("omniroute/")
        && (rel.ends_with(".ts")
            || rel.ends_with(".tsx")
            || rel.ends_with(".mjs")
            || rel.ends_with(".jsx")
            || rel.ends_with(".cjs"))
    {
        return Some("CNV-1 omniroute TS→Rust owner epic");
    }
    None
}

/// Documented exemptions (registry `docs/gsv/FORMAT_EXEMPTIONS.md`).
fn exempt_reason(rel: &str) -> Option<&'static str> {
    let base = basename(rel);
    if matches!(
        base,
        ".gitignore"
            | ".gitattributes"
            | ".dockerignore"
            | ".cbmignore"
            | ".gitkeep"
            | ".npmrc"
            | ".nvmrc"
            | ".editorconfig"
            | ".prettierrc"
            | ".prettierignore"
            | ".eslintrc"
            | "LICENSE"
            | "LICENSE-MIT"
            | "LICENSE-APACHE"
            | "gsv.code-workspace"
            | "Makefile"
            | "makefile"
            | "CMakeLists.txt"
            | "Dockerfile"
    ) {
        return Some("repo-infra");
    }
    if is_secret_basename(base) {
        return Some("local-secret (untracked)");
    }
    match extension(rel) {
        "toml" | "lock" | "yml" | "yaml" => Some("rust-ci-infra"),
        "html" | "css" => Some("ui-glue"),
        "svg" | "png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" => Some("docs-assets"),
        "jsonl" => Some("metadata-ring"),
        "sql" => Some("db-migrations"),
        "vbs" | "iss" | "service" => Some("os-deploy-launcher"),
        "http" => Some("ide-http-client"),
        "csv" => Some("data-export"),
        "exe" | "dll" => Some("release-binary"),
        "bin" | "gguf" | "safetensors" | "img" | "xz" | "pth" | "onnx" | "mp4" | "mov" | "webm"
        | "pdf" => Some("binary-artifact"),
        "log" | "pid" => Some("runtime-output"),
        "patch" => Some("patch-set"),
        _ => None,
    }
    .or_else(|| {
        if rel.starts_with("poolAI/e2e/") {
            Some("playwright-e2e-js")
        } else if rel.contains("/tests/data/") {
            Some("test-fixture")
        } else {
            None
        }
    })
}

/// Classify one repo-relative path. `tracked` = present in `git ls-files`.
pub fn classify(rel: &str, tracked: bool) -> Class {
    if is_allowed(rel) {
        return Class::Allowed;
    }
    let base = basename(rel);
    if base == ".env.example" || base == ".env.sample" || base == ".env.template" {
        return Class::Exempt("env-template");
    }
    if tracked && is_secret_basename(base) {
        return Class::Actionable;
    }
    if let Some(r) = defer_reason(rel) {
        return Class::Deferred(r);
    }
    if let Some(r) = exempt_reason(rel) {
        return Class::Exempt(r);
    }
    Class::Actionable
}

/// Per-repo counters.
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct RepoInv {
    pub repo: String,
    pub allowed: u64,
    /// All files ending outside the allowed canon (baseline parity).
    pub violating: u64,
    /// violating ∩ git index (baseline `9 287` parity).
    pub tracked: u64,
    pub exempt: u64,
    pub deferred: u64,
    pub actionable: u64,
    pub untracked: u64,
}

/// Whole-environment inventory.
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct Inventory {
    pub root: String,
    pub repos: Vec<RepoInv>,
    pub totals: RepoInv,
    /// reason → count (exempt + deferred, from the same walk).
    pub by_reason: BTreeMap<String, u64>,
    /// Problems that made the scan approximate (git failures, read errors).
    #[serde(default)]
    pub notes: Vec<String>,
}

impl Inventory {
    fn accumulate(&mut self, r: &RepoInv) {
        self.totals.allowed += r.allowed;
        self.totals.violating += r.violating;
        self.totals.tracked += r.tracked;
        self.totals.exempt += r.exempt;
        self.totals.deferred += r.deferred;
        self.totals.actionable += r.actionable;
        self.totals.untracked += r.untracked;
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
    }

    /// Human table + totals + reason breakdown.
    pub fn render(&self) -> String {
        let mut s = format!("format inventory — root: {}\n", self.root);
        s.push_str(
            "repo                allowed  viol  tracked  exempt  defer  action  untracked\n",
        );
        for r in &self.repos {
            s.push_str(&format!(
                "{:<20}{:>8}{:>6}{:>9}{:>8}{:>7}{:>8}{:>11}\n",
                trunc(&r.repo, 20),
                r.allowed,
                r.violating,
                r.tracked,
                r.exempt,
                r.deferred,
                r.actionable,
                r.untracked
            ));
        }
        let t = &self.totals;
        s.push_str(&format!(
            "{:<20}{:>8}{:>6}{:>9}{:>8}{:>7}{:>8}{:>11}\n",
            "TOTAL",
            t.allowed,
            t.violating,
            t.tracked,
            t.exempt,
            t.deferred,
            t.actionable,
            t.untracked
        ));
        if !self.by_reason.is_empty() {
            s.push_str("\nby reason:\n");
            for (k, v) in &self.by_reason {
                s.push_str(&format!("  {v:>7}  {k}\n"));
            }
        }
        if !self.notes.is_empty() {
            s.push_str("\nnotes:\n");
            for n in &self.notes {
                s.push_str(&format!("  - {n}\n"));
            }
        }
        s
    }
}

fn trunc(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let cut: String = s.chars().take(n.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}

/// Parse `git ls-files -z` output into repo-relative paths.
pub fn parse_ls_files(raw: &[u8]) -> Vec<String> {
    raw.split(|&b| b == 0)
        .filter(|p| !p.is_empty())
        .map(|p| String::from_utf8_lossy(p).into_owned())
        .collect()
}

/// `git ls-files -z` for one repo dir; `None` when git is unavailable.
fn git_ls_files(repo: &Path) -> Option<Vec<String>> {
    let out = Command::new("git")
        .current_dir(repo)
        .args(["ls-files", "-z"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(parse_ls_files(&out.stdout))
}

/// One repo's walk + reason histogram. `tracked` = index membership.
/// Hidden entries inside the repo ARE counted (`.cargo`, `.gitignore`, …);
/// only exact `SKIP_DIRS` segments are dropped, at any depth.
fn scan_repo(
    dir: &Path,
    repo: &str,
    tracked: &HashSet<String>,
    notes: &mut Vec<String>,
    reasons: &mut BTreeMap<String, u64>,
) -> RepoInv {
    let mut inv = RepoInv {
        repo: repo.to_string(),
        ..RepoInv::default()
    };
    let mut stack = vec![(dir.to_path_buf(), String::new())];
    while let Some((d, rel)) = stack.pop() {
        let rd = match fs::read_dir(&d) {
            Ok(rd) => rd,
            Err(e) => {
                notes.push(format!("{repo}/{}: read_dir failed: {e}", rel));
                continue;
            }
        };
        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let child_rel = if rel.is_empty() {
                name.clone()
            } else {
                format!("{rel}/{name}")
            };
            let ft = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            if ft.is_symlink() {
                continue; // find -type f parity: neither counted nor followed
            }
            if ft.is_dir() {
                if skip_dir(&name) {
                    continue;
                }
                stack.push((entry.path(), child_rel));
                continue;
            }
            if !ft.is_file() {
                continue;
            }
            let is_tracked = tracked.contains(&child_rel);
            // Classification sees the env-relative path (`<repo>/<rel>`) so
            // repo-keyed rules (omniroute/, poolAI/e2e/, …) match; the git
            // index holds repo-relative paths, hence the two variants.
            let full_rel = if repo.is_empty() {
                child_rel.clone()
            } else {
                format!("{repo}/{child_rel}")
            };
            let class = classify(&full_rel, is_tracked);
            match class {
                Class::Allowed => inv.allowed += 1,
                c => {
                    inv.violating += 1;
                    if is_tracked {
                        inv.tracked += 1;
                    } else {
                        inv.untracked += 1;
                    }
                    match c {
                        Class::Exempt(_) => inv.exempt += 1,
                        Class::Deferred(_) => inv.deferred += 1,
                        _ => inv.actionable += 1,
                    }
                    if let Some(r) = c.reason() {
                        *reasons.entry(r.to_string()).or_insert(0) += 1;
                    }
                }
            }
        }
    }
    inv
}

/// Full scan of `root`'s direct children + per-repo git index merge.
pub fn scan(root: &Path) -> Inventory {
    let mut inv = Inventory {
        root: root.display().to_string(),
        ..Inventory::default()
    };
    let rd = match fs::read_dir(root) {
        Ok(rd) => rd,
        Err(e) => {
            inv.notes.push(format!("root read_dir failed: {e}"));
            return inv;
        }
    };
    let mut children: Vec<(String, std::path::PathBuf)> = Vec::new();
    for entry in rd.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue; // hidden direct children are outside the shell `*/` baseline
        }
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_dir() && !ft.is_symlink() {
            children.push((name, entry.path()));
        }
    }
    children.sort();
    for (name, path) in children {
        let tracked: HashSet<String> = if path.join(".git").is_dir() {
            match git_ls_files(&path) {
                Some(files) => files.into_iter().collect(),
                None => {
                    inv.notes.push(format!("{name}: git ls-files failed"));
                    HashSet::new()
                }
            }
        } else {
            HashSet::new()
        };
        let repo_inv = scan_repo(&path, &name, &tracked, &mut inv.notes, &mut inv.by_reason);
        inv.accumulate(&repo_inv);
        inv.repos.push(repo_inv);
    }
    inv.totals.repo = "TOTAL".to_string();
    inv
}

/// Listing of tracked violating files (`inventory --tracked`).
pub fn tracked_listing(root: &Path) -> Vec<String> {
    let mut lines = Vec::new();
    let Ok(rd) = fs::read_dir(root) else {
        return lines;
    };
    let mut children: Vec<(String, std::path::PathBuf)> = Vec::new();
    for entry in rd.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            children.push((name, entry.path()));
        }
    }
    children.sort();
    for (name, path) in children {
        if !path.join(".git").is_dir() {
            continue;
        }
        let Some(files) = git_ls_files(&path) else {
            continue;
        };
        for f in files {
            if f.starts_with("target/") || f.contains("node_modules/") {
                continue;
            }
            let c = classify(&format!("{name}/{f}"), true);
            if !c.is_allowed() {
                let cls = match &c {
                    Class::Exempt(r) => format!("exempt:{r}"),
                    Class::Deferred(r) => format!("deferred:{r}"),
                    _ => "actionable".to_string(),
                };
                lines.push(format!("{name}|{f}|{cls}"));
            }
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fx_root(tag: &str) -> std::path::PathBuf {
        Path::new("target").join(format!("inventory_fixture_{tag}_{}", std::process::id()))
    }

    fn put(root: &Path, rel: &str) {
        let p = root.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&p, b"x").unwrap();
    }

    #[test]
    fn allowed_suffixes_match_canon() {
        assert!(is_allowed("src/main.rs"));
        assert!(is_allowed("docs/a.md"));
        assert!(is_allowed("rules.mdc"));
        assert!(is_allowed("data/x.json"));
        assert!(is_allowed("ui/app.js"));
        assert!(is_allowed("crates/mesh.wasm"));
        // near-misses must stay violating (grep `$` parity)
        assert!(!is_allowed("data/ring.jsonl"));
        assert!(!is_allowed("src/app.ts"));
        assert!(!is_allowed("bin/gen.sh"));
        assert!(!is_allowed("lib.rs.bak"));
        assert!(!is_allowed("x.mjs"));
        assert!(!is_allowed("README"));
        assert!(!is_allowed("A.RS")); // case-sensitive parity
    }

    #[test]
    fn skip_dirs_only_exact_segments() {
        assert!(skip_dir(".git"));
        assert!(skip_dir("node_modules"));
        assert!(skip_dir("target"));
        assert!(!skip_dir("mytarget"));
        assert!(!skip_dir("targets"));
        assert!(!skip_dir(".build"));
        assert!(!skip_dir(".cmake"));
        assert!(!skip_dir(".cargo"));
    }

    #[test]
    fn classify_core_cases() {
        assert_eq!(classify("src/lib.rs", false), Class::Allowed);
        assert_eq!(classify("Cargo.toml", true), Class::Exempt("rust-ci-infra"));
        assert_eq!(classify("scripts/gen.sh", false), Class::Actionable);
        assert_eq!(classify("bin/run.ps1", true), Class::Actionable);
        assert_eq!(
            classify("docs/ring.jsonl", true),
            Class::Exempt("metadata-ring")
        );
        assert_eq!(classify("ui/index.html", true), Class::Exempt("ui-glue"));
        assert_eq!(
            classify("omniroute/.build/next/chunk.ts", false),
            Class::Deferred("CLN-1 next-build output (untracked)")
        );
        assert_eq!(
            classify("omniroute/src/app.ts", false),
            Class::Deferred("CNV-1 omniroute TS→Rust owner epic")
        );
        assert_eq!(
            classify("rebook/scripts/build.sh", true),
            Class::Deferred("owner: book-repo untouchable")
        );
        assert_eq!(
            classify("mesh-node/flash.sh", true),
            Class::Deferred("GEN-4 mesh-node scripts/images (owner)")
        );
        assert_eq!(
            classify("llama-rs/.cmake/share/x.cmake", false),
            Class::Deferred("CLN-2 vendored cmake distro (owner)")
        );
        assert_eq!(
            classify("llama-rs/patches/sys/build.rs.patch", true),
            Class::Deferred("ECO-8.2 llama-cpp patch-set (owner)")
        );
        assert_eq!(
            classify("rusty-engine/data/models/m.gguf", false),
            Class::Deferred("model weights/source artifacts")
        );
    }

    #[test]
    fn classify_secret_rules() {
        // untracked local secret → exempt; tracked → actionable (SEC)
        assert_eq!(
            classify("telenetis/.env", false),
            Class::Exempt("local-secret (untracked)")
        );
        assert_eq!(classify("telenetis/.env", true), Class::Actionable);
        assert_eq!(
            classify("adoreble_wifi/data/adoreble.token", false),
            Class::Exempt("local-secret (untracked)")
        );
        assert_eq!(
            classify("mesh-node/owner_session.key", true),
            Class::Actionable
        );
        // templates are fine even tracked
        assert_eq!(
            classify("omniroute/.env.example", true),
            Class::Exempt("env-template")
        );
        // tracked secret shapes never dodge via other rules
        assert_eq!(classify("poolAI/certs/server.pem", true), Class::Actionable);
    }

    #[test]
    fn classify_path_rules() {
        assert_eq!(
            classify("poolAI/e2e/archive/api-smoke/x.spec.ts", true),
            Class::Exempt("playwright-e2e-js")
        );
        assert_eq!(
            classify("rusty-engine/tests/data/qwen25_golden.txt", true),
            Class::Exempt("test-fixture")
        );
        assert_eq!(classify("poolAI/bin/run.ps1", true), Class::Actionable);
        assert_eq!(
            classify("GSV/.agents/skills/x/find.sh", true),
            Class::Actionable
        );
        assert_eq!(classify("poolAI/docs/x.mmd", true), Class::Actionable);
        assert_eq!(
            classify("ORR_DESKTOP/scripts/build_release.sh", true),
            Class::Actionable
        );
        assert_eq!(
            classify("Tokenomica/usage.csv", true),
            Class::Exempt("data-export")
        );
        assert_eq!(classify("docs/pic.png", true), Class::Exempt("docs-assets"));
        assert_eq!(
            classify("omniroute/tests/manual/api.http", true),
            Class::Exempt("ide-http-client")
        );
        assert_eq!(
            classify("omniroute/.npmrc", true),
            Class::Exempt("repo-infra")
        );
    }

    #[test]
    fn parse_ls_files_null_separated() {
        let raw = b"a/b.rs\0dir name/c.md\0\0weird\nd.txt\0";
        let v = parse_ls_files(raw);
        assert_eq!(v, vec!["a/b.rs", "dir name/c.md", "weird\nd.txt"]);
    }

    #[test]
    fn fixture_scan_counts_and_skips() {
        let root = fx_root("scan");
        let _ = fs::remove_dir_all(&root);
        // allowed
        put(&root, "plainrepo/src/lib.rs");
        put(&root, "plainrepo/docs/index.js");
        // violating
        put(&root, "omniroute/.build/chunk.ts");
        put(&root, "omniroute/src/app.ts");
        put(&root, "rebook/scripts/build.sh");
        put(&root, "plainrepo/scripts/gen.sh");
        put(&root, "plainrepo/Cargo.toml");
        put(&root, "plainrepo/docs/note.jsonl");
        put(&root, "plainrepo/.env");
        // hidden entries INSIDE the repo are counted (.gitignore) …
        put(&root, "plainrepo/.gitignore");
        // … but exact skip segments are dropped at any depth
        put(&root, "plainrepo/target/skipped.rs");
        put(&root, "plainrepo/node_modules/skipped.js");
        put(&root, "plainrepo/.git/skipped.sh");
        // hidden direct child of the ROOT is outside the baseline (shell `*/`)
        put(&root, ".hidden/leak.txt");
        // non-dir direct child of the root is ignored
        fs::write(root.join("loose.txt"), b"x").unwrap();

        let inv = scan(&root);
        let total = &inv.totals;
        // allowed: lib.rs + index.js
        assert_eq!(total.allowed, 2, "allowed: {:?}", inv.repos);
        // violating: chunk.ts, app.ts, build.sh, gen.sh, Cargo.toml, note.jsonl,
        // .env, .gitignore
        assert_eq!(total.violating, 8, "violating: {:?}", inv.repos);
        assert_eq!(total.exempt, 4, "exempt: {:?}", inv.repos); // Cargo, jsonl, .env, .gitignore
        assert_eq!(total.deferred, 3, "deferred: {:?}", inv.repos); // chunk, app, rebook sh
        assert_eq!(total.actionable, 1, "actionable: {:?}", inv.repos); // gen.sh
        assert_eq!(total.untracked, 8, "untracked (no real index here)");
        assert_eq!(total.tracked, 0);
        // reason histogram has the deferrals + exemptions
        assert!(inv
            .by_reason
            .contains_key("CLN-1 next-build output (untracked)"));
        assert!(inv.by_reason.contains_key("owner: book-repo untouchable"));
        assert!(inv.by_reason.contains_key("repo-infra"));
        // hidden direct child not scanned as a repo
        assert!(inv.repos.iter().all(|r| r.repo != ".hidden"));
        // read errors would be a problem; a git note is expected here
        // (fixture .git dirs are not real repos)
        assert!(
            inv.notes.iter().all(|n| !n.contains("read_dir")),
            "{:?}",
            inv.notes
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn tracked_membership_counts() {
        let root = fx_root("tracked");
        let _ = fs::remove_dir_all(&root);
        put(&root, "r/a.sh");
        put(&root, "r/b.sh");
        put(&root, "r/ok.rs");
        // simulate an index containing a.sh (b.sh untracked, ok.rs allowed)
        let mut tracked = HashSet::new();
        tracked.insert("r/a.sh".to_string());
        let mut notes = Vec::new();
        let mut reasons = BTreeMap::new();
        let inv = scan_repo(&root, "r", &tracked, &mut notes, &mut reasons);
        assert_eq!(inv.violating, 2);
        assert_eq!(inv.tracked, 1);
        assert_eq!(inv.untracked, 1);
        assert_eq!(inv.actionable, 2);
        assert_eq!(inv.allowed, 1);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn render_contains_totals_row() {
        let mut inv = Inventory {
            root: "X".into(),
            ..Inventory::default()
        };
        inv.totals.allowed = 10;
        inv.totals.violating = 5;
        inv.totals.tracked = 2;
        inv.totals.exempt = 3;
        inv.totals.deferred = 1;
        inv.totals.actionable = 1;
        inv.totals.untracked = 3;
        let out = inv.render();
        assert!(out.contains("TOTAL"), "{out}");
        assert!(!out.contains("by reason"));
        inv.by_reason.insert("ui-glue".into(), 3);
        assert!(inv.render().contains("ui-glue"));
    }
}
