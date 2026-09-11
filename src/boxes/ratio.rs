//! Ratio box — GSV Rust/LOC ratio audit + wire (mirror of poolAI `poolai-loc-audit`).
//!
//! Counts git-tracked product LOC in this GSV repo and reports the Rust share.
//! Rust 95–100% canon → `rust_ratio.json` in `data/`, advisory gate at 0.95.
//!
//! ```text
//! cargo run --bin gsv-loc-audit                 # write GSV/data/rust_ratio.json
//! cargo run --bin gsv-loc-audit -- --print      # print report, no write
//! cargo run --bin gsv-loc-audit -- --min-ratio 0.95 --advisory
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Formal GSV canon band: Rust 95–100% / wasm 0–5%.
pub const FORMAL_BAND_MIN: f64 = 0.95;
/// Default advisory/regression floor.
pub const DEFAULT_MIN_RATIO: f64 = 0.95;
/// Stretch horizon target (band 120): 96% rust ratio advisory.
pub const STRETCH_96_TARGET: f64 = 0.96;

/// Product categories for the GSV ratio audit (git-tracked files only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductCategory {
    Ignored,
    RustSrc,
    RustTests,
    RustBenches,
    UiHtml,
    UiJs,
    UiCss,
    OpsShell,
}

impl ProductCategory {
    pub fn is_rust(self) -> bool {
        matches!(self, Self::RustSrc | Self::RustTests | Self::RustBenches)
    }

    pub fn is_non_rust_product(self) -> bool {
        matches!(
            self,
            Self::UiHtml | Self::UiJs | Self::UiCss | Self::OpsShell
        )
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Ignored => "ignored",
            Self::RustSrc => "rust_src",
            Self::RustTests => "rust_tests",
            Self::RustBenches => "rust_benches",
            Self::UiHtml => "ui_html",
            Self::UiJs => "ui_js",
            Self::UiCss => "ui_css",
            Self::OpsShell => "ops_shell",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuditConfig {
    pub min_ratio: f64,
    /// Warn and exit 0 when ratio below `min_ratio` (CI advisory).
    pub advisory: bool,
    /// Stretch-96 advisory: report 96% target, warn (exit 0) when not met.
    pub stretch_96: bool,
    pub write_output: bool,
    pub print: bool,
    pub output: Option<PathBuf>,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            min_ratio: DEFAULT_MIN_RATIO,
            advisory: false,
            stretch_96: false,
            write_output: true,
            print: false,
            output: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryLoc {
    pub files: u64,
    pub loc: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustRatioReport {
    pub generated_at: String,
    pub rust_loc: u64,
    pub non_rust_product_loc: u64,
    pub product_loc_total: u64,
    pub rust_ratio: f64,
    pub rust_ratio_pct: f64,
    pub formal_band_min: f64,
    pub min_ratio: f64,
    pub meets_min_ratio: bool,
    #[serde(default)]
    pub stretch_target: f64,
    #[serde(default)]
    pub meets_stretch_96: bool,
    pub by_category: BTreeMap<String, CategoryLoc>,
    pub notes: Vec<String>,
}

/// Classify a product path relative to the GSV workspace (a leading `GSV/` is
/// tolerated for git-top-level-relative inputs).
fn classify_product_path(path: &str) -> ProductCategory {
    let p = path.replace('\\', "/");
    let p = p.strip_prefix("GSV/").unwrap_or(&p);
    if p.starts_with("src/") && p.ends_with(".rs") {
        return ProductCategory::RustSrc;
    }
    if p.starts_with("tests/") && p.ends_with(".rs") {
        return ProductCategory::RustTests;
    }
    if p.starts_with("benches/") && p.ends_with(".rs") {
        return ProductCategory::RustBenches;
    }
    if p.starts_with("ui/") && p.ends_with(".html") {
        return ProductCategory::UiHtml;
    }
    if p.starts_with("ui/") && p.ends_with(".js") {
        return ProductCategory::UiJs;
    }
    if p.starts_with("ui/") && p.ends_with(".css") {
        return ProductCategory::UiCss;
    }
    if (p.starts_with("bin/") || p.starts_with("scripts/")) && p.ends_with(".sh") {
        return ProductCategory::OpsShell;
    }
    ProductCategory::Ignored
}

/// Convert an MSYS git root like `/s/rust/poolAI` to a Windows path `S:/rust/poolAI`.
fn normalize_git_root(root: &str) -> String {
    let bytes = root.as_bytes();
    if root.starts_with('/')
        && bytes.len() >= 3
        && bytes[1].is_ascii_alphabetic()
        && bytes[2] == b'/'
    {
        format!("{}:{}", (bytes[1] as char).to_ascii_uppercase(), &root[2..])
    } else {
        root.to_string()
    }
}

/// Git-tracked product paths in this GSV repo: `(absolute, git-relative)`.
fn git_tracked_gsv_files(root: &Path) -> Result<Vec<(PathBuf, String)>, String> {
    let top = crate::vision::command("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("git rev-parse: {e}"))?;
    if !top.status.success() {
        return Err("git rev-parse --show-toplevel failed".to_string());
    }
    let top = PathBuf::from(normalize_git_root(
        String::from_utf8_lossy(&top.stdout).trim(),
    ));
    let output = crate::vision::command("git")
        .args(["ls-files", "-z", "--full-name"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("git ls-files: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "git ls-files failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(output
        .stdout
        .split(|&b| b == 0)
        .filter(|chunk| !chunk.is_empty())
        .filter_map(|chunk| std::str::from_utf8(chunk).ok().map(str::to_string))
        .map(|p| {
            let rel = p.replace('\\', "/");
            (top.join(&p), rel)
        })
        .collect())
}

/// Non-blank line count (mirror of poolAI loc-audit).
fn count_loc(text: &str) -> u64 {
    text.lines().filter(|line| !line.trim().is_empty()).count() as u64
}

/// Count one tracked file; `Ok(None)` when the category is ignored/missing.
fn count_one(path: &Path, rel: &str) -> Result<Option<(String, u64)>, String> {
    let category = classify_product_path(rel);
    if category == ProductCategory::Ignored || !path.is_file() {
        return Ok(None);
    }
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    Ok(Some((category.label().to_string(), count_loc(&text))))
}

/// Per-file category sums over tracked files (band 232): rayon parallel
/// read/count, ordered collect so the **first read error in input order** is
/// the one returned — byte-identical to the old sequential walk. Reduction
/// walks the ordered results, so the BTreeMap is deterministic.
fn aggregate(files: &[(PathBuf, String)]) -> Result<BTreeMap<String, CategoryLoc>, String> {
    use rayon::prelude::*;
    let counted: Vec<Result<Option<(String, u64)>, String>> =
        files.par_iter().map(|(p, rel)| count_one(p, rel)).collect();
    let mut by_category: BTreeMap<String, CategoryLoc> = BTreeMap::new();
    for item in counted {
        if let Some((label, loc)) = item? {
            let entry = by_category
                .entry(label)
                .or_insert(CategoryLoc { files: 0, loc: 0 });
            entry.files += 1;
            entry.loc += loc;
        }
    }
    Ok(by_category)
}

/// Run the LOC audit over the GSV git workspace.
pub fn audit(root: &Path) -> Result<RustRatioReport, String> {
    let files = git_tracked_gsv_files(root)?;
    let by_category = aggregate(&files)?;

    let rust_loc: u64 = by_category
        .iter()
        .filter(|(label, _)| label.starts_with("rust_"))
        .map(|(_, c)| c.loc)
        .sum();
    let non_rust_product_loc: u64 = by_category
        .iter()
        .filter(|(label, _)| matches!(label.as_str(), "ui_html" | "ui_js" | "ui_css" | "ops_shell"))
        .map(|(_, c)| c.loc)
        .sum();
    let product_loc_total = rust_loc + non_rust_product_loc;
    let rust_ratio = if product_loc_total > 0 {
        rust_loc as f64 / product_loc_total as f64
    } else {
        1.0
    };

    let mut notes = Vec::new();
    if product_loc_total == 0 {
        notes.push("no product files found".to_string());
    }
    if by_category.get("ops_shell").is_some_and(|c| c.loc > 0) {
        notes.push(
            "ops_shell: product tests/benches/scripts belong in tests/, benches/, src/bin (cargo xtask), not .sh"
                .into(),
        );
    }

    Ok(RustRatioReport {
        generated_at: crate::vision::rfc3339_now(),
        rust_loc,
        non_rust_product_loc,
        product_loc_total,
        rust_ratio,
        rust_ratio_pct: rust_ratio * 100.0,
        formal_band_min: FORMAL_BAND_MIN,
        min_ratio: DEFAULT_MIN_RATIO,
        meets_min_ratio: rust_ratio >= DEFAULT_MIN_RATIO,
        stretch_target: STRETCH_96_TARGET,
        meets_stretch_96: rust_ratio >= STRETCH_96_TARGET,
        by_category,
        notes,
    })
}

/// Persist the report to `{data_dir}/rust_ratio.json`.
pub fn save(report: &RustRatioReport, data_dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(data_dir).map_err(|e| format!("create data dir: {e}"))?;
    let raw = serde_json::to_string_pretty(report).map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(data_dir.join("rust_ratio.json"), raw).map_err(|e| format!("write: {e}"))
}

/// Load the persisted report from `{data_dir}/rust_ratio.json`.
pub fn load(data_dir: &Path) -> Result<RustRatioReport, String> {
    let path = data_dir.join("rust_ratio.json");
    let raw =
        std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    serde_json::from_str(&raw).map_err(|e| format!("parse rust_ratio.json: {e}"))
}

/// API wire — report or an `ok:false` payload when the store is missing.
pub fn wire(data_dir: &Path) -> serde_json::Value {
    match load(data_dir) {
        Ok(report) => {
            let mut v = serde_json::to_value(&report).unwrap_or_default();
            if let serde_json::Value::Object(map) = &mut v {
                map.insert("ok".to_string(), serde_json::Value::Bool(true));
            }
            v
        }
        Err(e) => serde_json::json!({ "ok": false, "error": e }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregate_matches_sequential_walk() {
        // Synthetic tree covering every product category + an ignored file.
        let dir = std::env::temp_dir().join(format!("gsv-ratio-aggr-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for sub in ["src", "tests", "benches", "ui", "scripts", "data"] {
            std::fs::create_dir_all(dir.join(sub)).expect("mkdir");
        }
        std::fs::write(dir.join("src/a.rs"), "fn a() {}\n\n// x\n").unwrap();
        std::fs::write(dir.join("src/b.rs"), "let b = 1;\n").unwrap();
        std::fs::write(dir.join("tests/t.rs"), "#[test]\nfn t() {}\n").unwrap();
        std::fs::write(dir.join("benches/g.rs"), "fn g() {}\n").unwrap();
        std::fs::write(dir.join("ui/index.html"), "<html>\n<body>\n</body>\n").unwrap();
        std::fs::write(dir.join("ui/app.js"), "const x = 1;\n").unwrap();
        std::fs::write(dir.join("ui/s.css"), "a{}\n").unwrap();
        std::fs::write(dir.join("scripts/y.sh"), "echo y\n").unwrap();
        std::fs::write(dir.join("data/x.json"), "ignored\n").unwrap();
        let files: Vec<(PathBuf, String)> = [
            ("src/a.rs", 3u64),
            ("src/b.rs", 1),
            ("tests/t.rs", 2),
            ("benches/g.rs", 1),
            ("ui/index.html", 3),
            ("ui/app.js", 1),
            ("ui/s.css", 1),
            ("scripts/y.sh", 1),
            ("data/x.json", 0),
        ]
        .iter()
        .map(|(rel, _)| (dir.join(rel), (*rel).to_string()))
        .collect();
        let got = aggregate(&files).expect("aggregate");
        // Reference sequential walk over the same list.
        let mut want: BTreeMap<String, CategoryLoc> = BTreeMap::new();
        for (p, rel) in &files {
            if let Some((label, loc)) = count_one(p, rel).expect("count_one") {
                let e = want
                    .entry(label)
                    .or_insert(CategoryLoc { files: 0, loc: 0 });
                e.files += 1;
                e.loc += loc;
            }
        }
        assert_eq!(got, want, "rayon aggregate must equal the sequential walk");
        assert_eq!(got["rust_src"], CategoryLoc { files: 2, loc: 3 });
        assert_eq!(got["ui_html"], CategoryLoc { files: 1, loc: 3 });
        assert_eq!(got.get("ops_shell").map(|c| c.loc), Some(1));
        assert!(
            !got.contains_key("ignored"),
            "ignored files never aggregated"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn aggregate_reports_first_read_error_in_input_order() {
        // A non-UTF8 .rs forces a read error; rayon must not scramble which
        // error surfaces first — the EARLIEST input file's error wins.
        let dir = std::env::temp_dir().join(format!("gsv-ratio-err-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).expect("mkdir");
        std::fs::write(dir.join("src/first.rs"), b"\xff\xfe not utf8").unwrap();
        std::fs::write(dir.join("src/second.rs"), b"\xfe\xff also bad").unwrap();
        let files = vec![
            (dir.join("src/first.rs"), "src/first.rs".to_string()),
            (dir.join("src/second.rs"), "src/second.rs".to_string()),
        ];
        let err = aggregate(&files).expect_err("must fail");
        assert!(
            err.contains("first.rs"),
            "earliest failing file wins: {err}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn classify_gsv_paths() {
        assert_eq!(
            classify_product_path("GSV/src/boxes/omni/mod.rs"),
            ProductCategory::RustSrc
        );
        assert_eq!(
            classify_product_path("GSV/src/bin/gsv_server.rs"),
            ProductCategory::RustSrc
        );
        assert_eq!(
            classify_product_path("GSV/tests/gsv_omni_contracts.rs"),
            ProductCategory::RustTests
        );
        assert_eq!(
            classify_product_path("GSV/ui/index.html"),
            ProductCategory::UiHtml
        );
        assert_eq!(
            classify_product_path("GSV/ui/app.js"),
            ProductCategory::UiJs
        );
        assert_eq!(
            classify_product_path("GSV/ui/style.css"),
            ProductCategory::UiCss
        );
        assert_eq!(
            classify_product_path("GSV/scripts/tool.sh"),
            ProductCategory::OpsShell
        );
        assert_eq!(
            classify_product_path("GSV/benches/gsv_dev.rs"),
            ProductCategory::RustBenches
        );
        assert_eq!(
            classify_product_path("GSV/README.md"),
            ProductCategory::Ignored
        );
        assert_eq!(
            classify_product_path("GSV/Cargo.toml"),
            ProductCategory::Ignored
        );
        assert_eq!(
            classify_product_path("GSV/docs/gsv/GSV_BOXES.md"),
            ProductCategory::Ignored
        );
        assert_eq!(
            classify_product_path("GSV/target/debug/foo.rs"),
            ProductCategory::Ignored
        );
    }

    #[test]
    fn rust_vs_non_rust_membership() {
        assert!(ProductCategory::RustSrc.is_rust());
        assert!(ProductCategory::RustTests.is_rust());
        assert!(ProductCategory::UiHtml.is_non_rust_product());
        assert!(ProductCategory::UiJs.is_non_rust_product());
        assert!(!ProductCategory::UiHtml.is_rust());
        assert!(!ProductCategory::Ignored.is_rust());
    }

    #[test]
    fn count_loc_skips_blank_lines() {
        assert_eq!(count_loc("a\n\n  \nb\n\t\nc"), 3);
        assert_eq!(count_loc(""), 0);
    }

    #[test]
    fn ratio_math() {
        let mut by_category = BTreeMap::new();
        by_category.insert("rust_src".to_string(), CategoryLoc { files: 1, loc: 95 });
        by_category.insert("ui_html".to_string(), CategoryLoc { files: 1, loc: 5 });
        let rust_loc = 95;
        let non_rust = 5;
        let total = rust_loc + non_rust;
        let ratio = rust_loc as f64 / total as f64;
        assert!(ratio >= 0.95);
    }

    #[test]
    fn stretch_96_target_above_formal_band() {
        const { assert!(STRETCH_96_TARGET > FORMAL_BAND_MIN) };
        assert_eq!(STRETCH_96_TARGET, 0.96);
        assert!(!AuditConfig::default().stretch_96);
        let c = AuditConfig {
            stretch_96: true,
            ..AuditConfig::default()
        };
        assert!(c.stretch_96);
    }

    #[test]
    fn stretch_96_flag_gates_report_fields() {
        let cfg = AuditConfig {
            stretch_96: true,
            ..AuditConfig::default()
        };
        assert!(cfg.stretch_96);
        let report = RustRatioReport {
            generated_at: "t".to_string(),
            rust_loc: 96,
            non_rust_product_loc: 4,
            product_loc_total: 100,
            rust_ratio: 0.96,
            rust_ratio_pct: 96.0,
            formal_band_min: FORMAL_BAND_MIN,
            min_ratio: DEFAULT_MIN_RATIO,
            meets_min_ratio: true,
            stretch_target: STRETCH_96_TARGET,
            meets_stretch_96: true,
            by_category: BTreeMap::new(),
            notes: Vec::new(),
        };
        assert!(report.meets_min_ratio);
        assert!(report.meets_stretch_96);
        assert_eq!(report.stretch_target, 0.96);
        let below = RustRatioReport {
            rust_ratio: 0.958,
            rust_ratio_pct: 95.8,
            meets_min_ratio: true,
            meets_stretch_96: false,
            ..report
        };
        assert!(below.meets_min_ratio);
        assert!(!below.meets_stretch_96);
    }
}
