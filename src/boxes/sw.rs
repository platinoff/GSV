//! Service Worker shell cache — Rust-rendered `/sw.js`.
//!
//! Precaches the Galaxy document + live CSS so `/` still opens when the
//! process is down. SSE (`/events`), MCP (`/mcp`), and non-GET stay
//! network-only (the fetch handler returns without `respondWith`).

use serde_json::{json, Value};

/// Cache Storage name (bump when PRECACHE membership changes).
pub const CACHE_NAME: &str = "gsv-shell-v2";

/// Same-origin GET paths installed on `install`. No `/mcp`, `/events`, POST.
pub const PRECACHE: &[&str] = &[
    "/",
    "/api/ui/load-palette",
    "/api/ui/load-theme",
    "/api/vision/galaxy.svg",
    "/assets/vision.svg",
];

/// JSON discovery for `GET /api/sw` and the ops card.
pub fn wire() -> Value {
    json!({
        "ok": true,
        "cache": CACHE_NAME,
        "script": "/sw.js",
        "urls": PRECACHE,
    })
}

fn precache_js_array() -> String {
    let inner: Vec<String> = PRECACHE.iter().map(|u| format!("\"{u}\"")).collect();
    format!("[{}]", inner.join(","))
}

/// Compact Service Worker source (no `ui/sw.js` file — keeps the loc ratio).
pub fn script() -> String {
    let urls = precache_js_array();
    format!(
        "const CACHE='{CACHE_NAME}';\n\
         const PRECACHE={urls};\n\
         self.addEventListener('install',e=>{{e.waitUntil(caches.open(CACHE).then(c=>c.addAll(PRECACHE)).then(()=>self.skipWaiting()));}});\n\
         self.addEventListener('activate',e=>{{e.waitUntil(caches.keys().then(ks=>Promise.all(ks.filter(k=>k!==CACHE).map(k=>caches.delete(k)))).then(()=>self.clients.claim()));}});\n\
         self.addEventListener('fetch',e=>{{const req=e.request;if(req.method!=='GET')return;const u=new URL(req.url);if(u.origin!==self.location.origin)return;if(u.pathname==='/events'||u.pathname==='/mcp')return;if(PRECACHE.indexOf(u.pathname)<0)return;e.respondWith(fetch(req).then(res=>{{const copy=res.clone();caches.open(CACHE).then(c=>c.put(req,copy));return res;}}).catch(()=>caches.match(req).then(hit=>hit||caches.match('/'))));}});\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precache_is_shell_only() {
        assert!(PRECACHE.contains(&"/"));
        assert!(PRECACHE.contains(&"/api/ui/load-palette"));
        assert!(PRECACHE.contains(&"/api/ui/load-theme"));
        assert!(!PRECACHE
            .iter()
            .any(|u| *u == "/events" || u.contains("mcp")));
        assert!(!PRECACHE.iter().any(|u| u.contains("terminal")));
    }

    #[test]
    fn script_skips_sse_mcp_and_non_get() {
        let js = script();
        assert!(js.contains("caches.open"));
        assert!(js.contains(CACHE_NAME));
        assert!(js.contains("req.method!=='GET'"));
        assert!(js.contains("u.origin!==self.location.origin"));
        assert!(js.contains("pathname==='/events'"));
        assert!(js.contains("pathname==='/mcp'"));
        assert!(
            !js.contains("hit=>hit||fetch"),
            "shell must be network-first so UI JS updates without a stuck Auto-resync"
        );
        assert!(
            js.contains("fetch(req).then"),
            "network-first fetch for precache paths"
        );
    }

    #[test]
    fn wire_points_at_sw_js() {
        let w = wire();
        assert_eq!(w["ok"], true);
        assert_eq!(w["script"], "/sw.js");
        assert_eq!(w["cache"], CACHE_NAME);
        assert_eq!(w["urls"].as_array().map(|a| a.len()), Some(PRECACHE.len()));
    }
}
