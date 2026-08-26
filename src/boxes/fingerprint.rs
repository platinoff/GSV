//! Drain fingerprint JSONL — who / which client / which model / when / which product.
//!
//! Path: `{kit}/docs/gsv/fingerprints.jsonl` (git-tracked, never `data/`).
//! Each row is tagged with `product` so GSV crate semver is not mistaken for
//! poolAI / omniroute versions. Legacy rows without `product` deserialize as `gsv`.

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

fn default_product() -> String {
    "gsv".to_string()
}

/// Auto-detect the IDE from explicit env vars (no process env access — testable).
///
/// Priority: `gsv_ide` (explicit) → `opencode_client` (OpenCode) → `cursor_model` / `cursor_session` (Cursor) → `"unknown"`.
pub fn detect_ide_from(
    gsv_ide: Option<&str>,
    opencode_client: bool,
    cursor_model: bool,
    cursor_session: bool,
) -> String {
    if let Some(ide) = gsv_ide {
        let ide = ide.trim().to_string();
        if !ide.is_empty() {
            return ide;
        }
    }
    if opencode_client {
        return "opencode".to_string();
    }
    if cursor_model || cursor_session {
        return "cursor".to_string();
    }
    "unknown".to_string()
}

/// Auto-detect the IDE from process environment variables.
///
/// Priority: `GSV_IDE` (explicit) → `OPENCODE_CLIENT` (OpenCode) → `CURSOR_*` (Cursor) → `"unknown"`.
pub fn detect_ide() -> String {
    detect_ide_from(
        std::env::var("GSV_IDE").ok().as_deref(),
        std::env::var("OPENCODE_CLIENT").is_ok(),
        std::env::var("CURSOR_MODEL").is_ok(),
        std::env::var("CURSOR_SESSION_FILE").is_ok(),
    )
}

/// One drain close record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Fingerprint {
    pub ts: String,
    pub actor: String,
    pub ide: String,
    pub model: String,
    pub agent: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_head: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub band: Option<String>,
    pub summary: String,
    /// VDT product id (`gsv` / `poolai` / `omniroute`). Missing JSONL → `gsv`.
    #[serde(default = "default_product")]
    pub product: String,
}

/// Canonical JSONL path under the **kit** repo (GSV), not the selected product.
pub fn jsonl_path(repo_root: &Path) -> PathBuf {
    repo_root.join("docs/gsv/fingerprints.jsonl")
}

/// Package version from a product tree (`Cargo.toml` [package], else `package.json`).
pub fn pkg_version(root: &Path) -> Option<String> {
    parse_cargo_version(&root.join("Cargo.toml"))
        .or_else(|| parse_npm_version(&root.join("package.json")))
}

fn parse_cargo_version(toml: &Path) -> Option<String> {
    let text = fs::read_to_string(toml).ok()?;
    let mut in_package = false;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_package = t == "[package]";
            continue;
        }
        if in_package {
            if let Some(rest) = t.strip_prefix("version") {
                // Only a plain `version = "…"` key — not `version.workspace`,
                // `version_override`, …
                if !rest.starts_with([' ', '\t', '=']) {
                    continue;
                }
                let rest = rest.trim().trim_start_matches('=').trim();
                let ver = rest.trim_matches('"').trim_matches('\'').trim();
                if !ver.is_empty() && ver.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                    return Some(ver.to_string());
                }
            }
        }
    }
    None
}

fn parse_npm_version(json_path: &Path) -> Option<String> {
    let text = fs::read_to_string(json_path).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    v.get("version")
        .and_then(Value::as_str)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Append one record (creates the file if missing).
pub fn append(path: &Path, fp: &Fingerprint) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    serde_json::to_writer(&mut f, fp)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    f.write_all(b"\n")?;
    Ok(())
}

/// Last `n` records, newest first. Missing file or bad lines → skip.
pub fn latest(path: &Path, n: usize) -> Vec<Fingerprint> {
    let Ok(f) = fs::File::open(path) else {
        return Vec::new();
    };
    let mut all = Vec::new();
    for line in BufReader::new(f).lines().map_while(Result::ok) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(fp) = serde_json::from_str::<Fingerprint>(line) {
            all.push(fp);
        }
    }
    if n == 0 || all.is_empty() {
        return Vec::new();
    }
    let start = all.len().saturating_sub(n);
    let mut out: Vec<Fingerprint> = all.drain(start..).collect();
    out.reverse();
    out
}

/// `GET /api/fingerprints?limit=` — default 20, min 1, max 100.
pub fn clamp_limit(raw: Option<usize>) -> usize {
    raw.unwrap_or(20).clamp(1, 100)
}

/// Set `[package] version` so semver minor equals the VDT band.
/// Same band already on minor N → patch +1. New band → `MAJOR.N.0`.
pub fn bump_package_version(toml: &Path, band: u32) -> Result<String, String> {
    let text = fs::read_to_string(toml).map_err(|e| format!("read {}: {e}", toml.display()))?;
    let mut in_package = false;
    let mut bumped = false;
    let mut new_ver = String::new();
    let mut out = String::new();
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_package = t == "[package]";
        }
        if in_package && !bumped {
            if let Some(rest) = t.strip_prefix("version") {
                // Only a plain `version = "…"` key — not `version.workspace`, …
                if !rest.starts_with([' ', '\t', '=']) {
                    out.push_str(line);
                    out.push('\n');
                    continue;
                }
                let rest = rest.trim().trim_start_matches('=').trim();
                let ver = rest.trim_matches('"').trim_matches('\'').trim();
                let mut parts = ver.split('.');
                let major = parts.next().and_then(|s| s.parse::<u32>().ok());
                let minor = parts.next().and_then(|s| s.parse::<u32>().ok());
                let patch = parts.next().and_then(|s| s.parse::<u32>().ok());
                if let (Some(major), Some(minor), Some(patch)) = (major, minor, patch) {
                    let (minor, patch) = if minor == band {
                        (minor, patch.saturating_add(1))
                    } else {
                        (band, 0)
                    };
                    new_ver = format!("{major}.{minor}.{patch}");
                    out.push_str(&format!("version = \"{new_ver}\"\n"));
                    bumped = true;
                    in_package = false;
                    continue;
                }
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    if !bumped {
        return Err(format!(
            "no [package] version = \"X.Y.Z\" in {}",
            toml.display()
        ));
    }
    fs::write(toml, out).map_err(|e| format!("write {}: {e}", toml.display()))?;
    Ok(new_ver)
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key)
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn nonempty_opt(s: Option<&str>) -> Option<String> {
    s.map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
}

/// Fingerprint `model`: explicit `GSV_MODEL` env, then Cursor session (`CURSOR_MODEL`
/// or session JSON), then the latest Cursor `renderer.log` `catalogModelId`, else
/// `unknown`. The literal `unknown` is a valid recorded value.
pub fn resolve_model_from(
    gsv_model: Option<&str>,
    cursor_model: Option<&str>,
    session_model: Option<&str>,
) -> String {
    nonempty_opt(gsv_model)
        .or_else(|| nonempty_opt(cursor_model))
        .or_else(|| nonempty_opt(session_model))
        .unwrap_or_else(|| "unknown".to_string())
}

fn json_str_model(v: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(s) = v.get(*key).and_then(Value::as_str).map(str::trim) {
            if !s.is_empty() {
                return Some(s.to_string());
            }
        }
    }
    None
}

/// Read `model` from a Cursor/session JSON blob (`model` / `modelId` / nested `session`).
pub fn session_model_from_json(text: &str) -> Option<String> {
    let v: Value = serde_json::from_str(text).ok()?;
    const KEYS: [&str; 4] = ["model", "modelId", "catalogModelId", "composerModelName"];
    json_str_model(&v, &KEYS).or_else(|| v.get("session").and_then(|s| json_str_model(s, &KEYS)))
}

/// Last `catalogModelId=` (else `composerModelName=`) in a Cursor renderer log.
pub fn cursor_model_from_renderer_log(text: &str) -> Option<String> {
    let mut last = None;
    for line in text.lines() {
        if let Some(id) =
            log_kv(line, "catalogModelId=").or_else(|| log_kv(line, "composerModelName="))
        {
            last = Some(id);
        }
    }
    last
}

fn log_kv(line: &str, key: &str) -> Option<String> {
    let i = line.find(key)?;
    let id = line[i + key.len()..]
        .split_whitespace()
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())?;
    Some(id.to_string())
}

const RENDERER_TAIL_BYTES: usize = 64 * 1024;

fn read_tail_text(path: &Path, max: usize) -> Option<String> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = fs::File::open(path).ok()?;
    let len = f.metadata().ok()?.len();
    let start = len.saturating_sub(max as u64);
    f.seek(SeekFrom::Start(start)).ok()?;
    let mut bytes = Vec::new();
    f.read_to_end(&mut bytes).ok()?;
    // The seek can land mid-UTF-8; drop the partial prefix instead of
    // failing the whole read (model discovery must stay best-effort).
    let mut i = 0usize;
    while i < bytes.len() && i < 4 && (bytes[i] & 0xC0) == 0x80 {
        i += 1;
    }
    Some(String::from_utf8_lossy(&bytes[i..]).into_owned())
}

/// Newest `window*/renderer.log` under a Cursor `logs/` tree (testable).
pub fn discover_cursor_model_from_logs(logs_root: &Path) -> Option<String> {
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    let Ok(sessions) = fs::read_dir(logs_root) else {
        return None;
    };
    let mut guard = 0usize;
    for session in sessions.flatten() {
        let session_path = session.path();
        if !session_path.is_dir() {
            continue;
        }
        let Ok(windows) = fs::read_dir(&session_path) else {
            continue;
        };
        for window in windows.flatten() {
            guard += 1;
            if guard > 200 {
                break;
            }
            let log = window.path().join("renderer.log");
            let Ok(meta) = fs::metadata(&log) else {
                continue;
            };
            if !meta.is_file() {
                continue;
            }
            let mtime = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
            if best.as_ref().map(|(t, _)| mtime >= *t).unwrap_or(true) {
                best = Some((mtime, log));
            }
        }
    }
    let (_, path) = best?;
    let text = read_tail_text(&path, RENDERER_TAIL_BYTES)?;
    cursor_model_from_renderer_log(&text)
}

/// Cursor log root: `%APPDATA%/Cursor/logs` or macOS/Linux config dirs.
pub fn cursor_logs_root() -> Option<PathBuf> {
    if let Some(appdata) = std::env::var_os("APPDATA").map(PathBuf::from) {
        let p = appdata.join("Cursor").join("logs");
        if p.is_dir() {
            return Some(p);
        }
    }
    let home = crate::boxes::ide::home_dir()?;
    for rel in [
        "Library/Application Support/Cursor/logs",
        ".config/Cursor/logs",
        "AppData/Roaming/Cursor/logs",
    ] {
        let p = home.join(rel);
        if p.is_dir() {
            return Some(p);
        }
    }
    None
}

fn read_session_model() -> Option<String> {
    let path = std::env::var_os("GSV_SESSION_FILE")
        .or_else(|| std::env::var_os("CURSOR_SESSION_FILE"))
        .map(PathBuf::from)?;
    let text = fs::read_to_string(path).ok()?;
    session_model_from_json(&text)
}

fn discovered_cursor_model() -> Option<String> {
    discover_cursor_model_from_logs(&cursor_logs_root()?)
}

/// Process env + optional session JSON + Cursor renderer log (no env needed).
pub fn resolve_model() -> String {
    let gsv = std::env::var("GSV_MODEL").ok();
    let cursor = std::env::var("CURSOR_MODEL").ok();
    let session = read_session_model();
    if nonempty_opt(gsv.as_deref()).is_some()
        || nonempty_opt(cursor.as_deref()).is_some()
        || nonempty_opt(session.as_deref()).is_some()
    {
        return resolve_model_from(gsv.as_deref(), cursor.as_deref(), session.as_deref());
    }
    nonempty_opt(discovered_cursor_model().as_deref()).unwrap_or_else(|| "unknown".to_string())
}

fn git_head_short(root: &Path) -> Option<String> {
    crate::vision::command("git")
        .current_dir(root)
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Commit trailers for the drain-close commit.
pub fn trailers(fp: &Fingerprint) -> String {
    format!(
        "Gsv-Actor: {}\nGsv-Ide: {}\nGsv-Model: {}\nGsv-Product: {}\n",
        fp.actor, fp.ide, fp.model, fp.product
    )
}

/// Fields for [`record`].
pub struct RecordOpts<'a> {
    pub kit_root: &'a Path,
    pub jsonl: Option<&'a Path>,
    pub product_root: &'a Path,
    pub actor: &'a str,
    pub ide: &'a str,
    pub model: &'a str,
    pub agent: &'a str,
    pub band: Option<&'a str>,
    pub summary: &'a str,
    pub product: &'a str,
}

/// Append one fingerprint (explicit fields; used by `cargo xtask fingerprint` and tests).
pub fn record(opts: RecordOpts<'_>) -> Result<(Fingerprint, String), String> {
    let version = pkg_version(opts.product_root).ok_or_else(|| {
        format!(
            "gsv-fingerprint: no version in {} (Cargo.toml / package.json)",
            crate::boxes::products::display_path(opts.product_root)
        )
    })?;
    let fp = Fingerprint {
        ts: crate::vision::rfc3339_now(),
        actor: opts.actor.into(),
        ide: opts.ide.into(),
        model: opts.model.into(),
        agent: opts.agent.into(),
        version,
        git_head: git_head_short(opts.product_root),
        band: opts.band.filter(|s| !s.is_empty()).map(str::to_string),
        summary: opts.summary.into(),
        product: opts.product.into(),
    };
    let path = opts
        .jsonl
        .map(Path::to_path_buf)
        .unwrap_or_else(|| jsonl_path(opts.kit_root));
    append(&path, &fp).map_err(|e| format!("append {}: {e}", path.display()))?;
    let msg = format!(
        "{}gsv-fingerprint: appended {} (product={} v{})\n",
        trailers(&fp),
        crate::boxes::products::display_path(&path),
        fp.product,
        fp.version
    );
    Ok((fp, msg))
}

/// Append one fingerprint from env (`GSV_ACTOR` / `GSV_IDE` / …).
/// `model_override` wins over env / Cursor log discovery.
pub fn record_from_env(
    kit_root: &Path,
    jsonl: Option<&Path>,
    product_root: Option<&Path>,
    model_override: Option<&str>,
) -> Result<(Fingerprint, String), String> {
    let product = env_or("GSV_PRODUCT", "gsv");
    let root = product_root
        .map(Path::to_path_buf)
        .or_else(|| std::env::var("GSV_PRODUCT_ROOT").ok().map(PathBuf::from))
        .unwrap_or_else(|| kit_root.to_path_buf());
    let path = jsonl.map(Path::to_path_buf).or_else(|| {
        std::env::var("GSV_FINGERPRINT_FILE")
            .ok()
            .map(PathBuf::from)
    });
    let band = std::env::var("GSV_BAND").ok().filter(|s| !s.is_empty());
    let actor = env_or("GSV_ACTOR", "agent");
    let ide = detect_ide();
    let model = nonempty_opt(model_override).unwrap_or_else(resolve_model);
    let agent = env_or("GSV_AGENT", "orchestrator");
    let summary = env_or("GSV_SUMMARY", "drain close");
    record(RecordOpts {
        kit_root,
        jsonl: path.as_deref(),
        product_root: &root,
        actor: &actor,
        ide: &ide,
        model: &model,
        agent: &agent,
        band: band.as_deref(),
        summary: &summary,
        product: &product,
    })
}

/// HTTP / card wire. `selected` is the VDT product id (may differ from GSV crate).
pub fn wire(repo_root: &Path, selected: Option<&str>, limit: usize) -> Value {
    let fingerprints = latest(&jsonl_path(repo_root), limit);
    let server_version = crate::gsv_version();
    let selected_version = selected.and_then(|id| {
        let rows = crate::boxes::products::discover(repo_root);
        crate::boxes::products::lookup(&rows, id).and_then(|row| pkg_version(Path::new(&row.path)))
    });
    let cross_product = selected.map(|id| id != "gsv").unwrap_or(false);
    json!({
        "ok": true,
        "path": "docs/gsv/fingerprints.jsonl",
        "server_product": "gsv",
        "server_version": server_version,
        "selected": selected,
        "selected_version": selected_version,
        "cross_product": cross_product,
        "count": fingerprints.len(),
        "fingerprints": fingerprints,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_ide_explicit_gsv_ide_wins() {
        assert_eq!(
            detect_ide_from(Some("opencode"), true, false, false),
            "opencode"
        );
    }

    #[test]
    fn detect_ide_opencode_client_env() {
        assert_eq!(detect_ide_from(None, true, false, false), "opencode");
    }

    #[test]
    fn detect_ide_cursor_model_env() {
        assert_eq!(detect_ide_from(None, false, true, false), "cursor");
    }

    #[test]
    fn detect_ide_cursor_session_env() {
        assert_eq!(detect_ide_from(None, false, false, true), "cursor");
    }

    #[test]
    fn detect_ide_fallback_unknown() {
        assert_eq!(detect_ide_from(None, false, false, false), "unknown");
    }

    #[test]
    fn detect_ide_explicit_overrides_opencode() {
        assert_eq!(
            detect_ide_from(Some("cursor"), true, false, false),
            "cursor"
        );
    }

    #[test]
    fn detect_ide_explicit_empty_falls_through() {
        assert_eq!(detect_ide_from(Some("  "), true, false, false), "opencode");
    }
}
