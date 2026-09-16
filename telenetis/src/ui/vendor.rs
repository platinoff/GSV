//! Self-hosted runtime + model bytes for the tensor worker (swarm path b).
//!
//! Phone browsers failed CDN loads (jsdelivr 404 on moved paths, CSP,
//! flaky mobile networks), so the kit serves everything same-origin from
//! `TELENETIS_VENDOR_DIR` (default `{crate}/target/live/vendor`, gitignored
//! scratch seeded once from the fast PC: wllama ESM + wasm) and
//! `TELENETIS_MODEL_DIR` (no default; set to the GGUF library, e.g. the
//! llama-rs `models/` tree).
//!
//! Nothing third-party lands in the repo, so the Rust-first ratio is
//! untouched. Served with single-range (`206`) support because model
//! loaders fetch in ranged chunks; traversal is rejected (`..`, absolute
//! paths, non-canonical escapes).

use std::path::{Path, PathBuf};

/// Vendored runtime root (wllama ESM + wasm). Deterministic without env:
/// compiled manifest dir + `target/live/vendor` (sits next to the live exe
/// the supervisor runs, survives rebuilds, never staged).
pub fn vendor_dir() -> PathBuf {
    std::env::var_os("TELENETIS_VENDOR_DIR")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/live/vendor"))
}

/// GGUF library root: `TELENETIS_MODEL_DIR` first, then the sibling
/// `llama-rs/models` tree next to this kit (`S:/rust/<kit>/telenetis`
/// → `S:/rust/llama-rs/models`). The compiled fallback keeps device
/// catalogs working even when dotenv fails to load the local `.env`.
/// `None` only when neither exists.
pub fn model_dir() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("TELENETIS_MODEL_DIR")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .filter(|p| p.is_dir())
    {
        return Some(p);
    }
    let sibling = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../llama-rs/models");
    if sibling.is_dir() {
        return Some(sibling);
    }
    None
}

/// Clean a URL tail into a relative path, or reject it. Allows
/// `[a-zA-Z0-9._-/]` segments only; rejects empty, absolute, `..` and
/// backslash escapes. Callers must additionally verify the canonicalized
/// path stays under the root (symlink escapes).
pub fn sanitize(rel: &str) -> Option<String> {
    let rel = rel.trim().trim_start_matches('/');
    if rel.is_empty() || rel.contains('\\') || rel.contains('\0') {
        return None;
    }
    let mut out: Vec<&str> = Vec::new();
    for seg in rel.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." {
            return None;
        }
        if !seg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '+'))
        {
            return None;
        }
        out.push(seg);
    }
    if out.is_empty() {
        return None;
    }
    Some(out.join("/"))
}

/// Resolve a sanitized tail under `root`; `None` on escape/missing.
pub fn resolve(root: &Path, rel: &str) -> Option<PathBuf> {
    let clean = sanitize(rel)?;
    let joined = root.join(&clean);
    let canon_root = root.canonicalize().ok()?;
    let canon_file = joined.canonicalize().ok()?;
    if !canon_file.starts_with(&canon_root) {
        return None;
    }
    if !canon_file.is_file() {
        return None;
    }
    Some(canon_file)
}

pub fn content_type(name: &str) -> &'static str {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".js") || lower.ends_with(".mjs") {
        "application/javascript; charset=utf-8"
    } else if lower.ends_with(".wasm") {
        "application/wasm"
    } else if lower.ends_with(".json") {
        "application/json; charset=utf-8"
    } else {
        // `.gguf` model bytes and anything unknown: opaque download.
        "application/octet-stream"
    }
}

/// Parse a single `Range: bytes=start-end | start- | -suffix` header into
/// an inclusive `(start, end)` clamped to `len`. Anything else → `None`
/// (caller answers 416).
pub fn parse_range(header: &str, len: u64) -> Option<(u64, u64)> {
    if len == 0 {
        return None;
    }
    let spec = header.trim().strip_prefix("bytes=")?.trim();
    if spec.is_empty() {
        return None;
    }
    let (start, end) = if let Some(suffix) = spec.strip_prefix('-') {
        let n: u64 = suffix.trim().parse().ok()?;
        if n == 0 {
            return None;
        }
        (len.saturating_sub(n), len - 1)
    } else {
        let (s, e) = spec.split_once('-')?;
        let start: u64 = s.trim().parse().ok()?;
        let end: u64 = if e.trim().is_empty() {
            len - 1
        } else {
            e.trim().parse().ok()?
        };
        if start >= len {
            return None;
        }
        (start, end.min(len - 1))
    };
    if start > end {
        return None;
    }
    Some((start, end))
}

/// One GGUF entry for the tensor page config (largest first).
pub fn list_models(dir: &Path) -> Vec<serde_json::Value> {
    let entries = std::fs::read_dir(dir).map(|rd| {
        rd.filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                if !name.to_ascii_lowercase().ends_with(".gguf") {
                    return None;
                }
                let size = e.metadata().ok()?.len();
                if !e.path().is_file() {
                    return None;
                }
                Some((name, size))
            })
            .collect::<Vec<_>>()
    });
    let mut entries = entries.unwrap_or_default();
    entries.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    entries
        .into_iter()
        .map(|(name, size)| {
            let stem = name
                .trim_end_matches(".gguf")
                .trim_end_matches(".GGUF")
                .to_string();
            serde_json::json!({
                "key": stem,
                "label": format!("{stem} ({}MB)", size / (1024 * 1024)),
                "url": format!("/models/{name}"),
                "torrent": format!("/torrents/{stem}.torrent"),
                "size_mb": size / (1024 * 1024),
            })
        })
        .collect()
}

/// Tensor page bootstrap: runtime URLs (always local) + model catalog
/// (empty when `TELENETIS_MODEL_DIR` is unset — the page falls back to
/// its pinned HuggingFace pair).
pub fn tensor_config() -> serde_json::Value {
    let models = model_dir().map(|d| list_models(&d)).unwrap_or_default();
    serde_json::json!({
        "runtime": {
            "esm": "/vendor/wllama/index.js",
            "wasm": "/vendor/wllama/wllama.wasm",
        },
        "models": models,
    })
}

/// Serializes tests that mutate process env (`TELENETIS_MODEL_DIR`):
/// the lib test harness runs threads in parallel. Async-aware so guards
/// may be held across awaits.
#[cfg(test)]
pub(crate) static ENV_GUARD: std::sync::LazyLock<tokio::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_rejects_escapes() {
        assert!(sanitize("").is_none());
        assert!(sanitize("../x").is_none());
        assert!(sanitize("a/../../b").is_none());
        assert!(sanitize("/abs").is_some());
        assert!(sanitize("a\\b").is_none());
        assert!(sanitize("a;b").is_none());
        assert_eq!(
            sanitize("wllama/index.js").as_deref(),
            Some("wllama/index.js")
        );
        assert_eq!(sanitize("a//b/./c").as_deref(), Some("a/b/c"));
    }

    #[test]
    fn resolve_stays_under_root() {
        let dir = std::env::temp_dir().join(format!("tns-vendor-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("sub/f.js"), b"x").unwrap();
        assert!(resolve(&dir, "sub/f.js").is_some());
        assert!(resolve(&dir, "../outside").is_none());
        assert!(resolve(&dir, "missing.js").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn content_types_cover_runtime_and_models() {
        assert!(content_type("index.js").contains("javascript"));
        assert_eq!(content_type("wllama.wasm"), "application/wasm");
        assert_eq!(content_type("m.gguf"), "application/octet-stream");
    }

    #[test]
    fn range_prefix_suffix_and_open_shapes() {
        assert_eq!(parse_range("bytes=0-99", 1000), Some((0, 99)));
        assert_eq!(parse_range("bytes=900-", 1000), Some((900, 999)));
        assert_eq!(parse_range("bytes=-100", 1000), Some((900, 999)));
        assert_eq!(parse_range("bytes=0-99999", 1000), Some((0, 999)));
        assert_eq!(parse_range("bytes=1000-", 1000), None);
        assert_eq!(parse_range("bytes=-0", 1000), None);
        assert_eq!(parse_range("items=0-1", 1000), None);
        assert_eq!(parse_range("bytes=5-3", 1000), None);
        assert_eq!(parse_range("bytes=0-1", 0), None);
    }

    #[test]
    fn list_models_orders_largest_first() {
        let dir = std::env::temp_dir().join(format!("tns-models-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("small.gguf"), vec![0u8; 100]).unwrap();
        std::fs::write(dir.join("big.gguf"), vec![0u8; 300]).unwrap();
        std::fs::write(dir.join("note.txt"), b"no").unwrap();
        let rows = list_models(&dir);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["key"], "big");
        assert!(rows[0]["url"]
            .as_str()
            .unwrap()
            .ends_with("/models/big.gguf"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tensor_config_shape_without_model_dir() {
        // Deterministic everywhere: point the var at an empty temp dir
        // (never rely on machine-global state like the sibling tree).
        let _guard = ENV_GUARD.blocking_lock();
        let prev = std::env::var_os("TELENETIS_MODEL_DIR");
        let dir = std::env::temp_dir().join(format!("tns-nomodels-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("TELENETIS_MODEL_DIR", &dir);
        let cfg = tensor_config();
        assert_eq!(cfg["runtime"]["esm"], "/vendor/wllama/index.js");
        assert_eq!(cfg["runtime"]["wasm"], "/vendor/wllama/wllama.wasm");
        assert_eq!(cfg["models"].as_array().unwrap().len(), 0);
        match prev {
            Some(v) => std::env::set_var("TELENETIS_MODEL_DIR", v),
            None => std::env::remove_var("TELENETIS_MODEL_DIR"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
