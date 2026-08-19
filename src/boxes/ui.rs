//! UI fragments — server-rendered card body HTML (Rust, ratio-safe).
//!
//! The dashboard cards are rendered here instead of in `ui/index.html` so the
//! JS glue stays thin and the Rust ratio holds. Each renderer takes the card's
//! wire `Value` and returns the card-body HTML exactly as the former JS
//! `render*` functions produced it.

use serde_json::Value;

/// HTML-escape a string (`&`, `<`, `>`), matching the JS `esc` helper.
pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Table markup, matching the JS `tab` helper (headers raw, cells pre-built).
pub fn tab(headers: &[&str], rows: Vec<Vec<String>>) -> String {
    let rows = if rows.is_empty() {
        vec![headers
            .iter()
            .map(|_| "<span class='dim'>—</span>".to_string())
            .collect()]
    } else {
        rows
    };
    let mut out = String::from("<table><tr>");
    for h in headers {
        out.push_str(&format!("<th>{h}</th>"));
    }
    out.push_str("</tr>");
    for row in rows {
        out.push_str("<tr>");
        for cell in row {
            out.push_str(&format!("<td>{cell}</td>"));
        }
        out.push_str("</tr>");
    }
    out.push_str("</table>");
    out
}

/// Progress bar markup, matching the JS `bar` helper.
pub fn bar(pct: f64) -> String {
    let w = pct.clamp(0.0, 100.0);
    format!(
        "<div style='background:var(--line);border-radius:6px;height:10px;margin:6px 0;overflow:hidden'><div style='width:{w}%;height:100%;background:var(--accent)'></div></div><div class='dim'>{w}% closed</div>"
    )
}

/// Consistent error-state marker (HTML fragment).
fn err_html(msg: &str) -> String {
    format!("<span class='err'>{}</span>", esc(msg))
}

/// Consistent empty-state marker (HTML fragment).
fn empty_html(label: &str) -> String {
    format!("<div class='dim'>{label} — no data</div>")
}

/// `Some(msg)` when the wire explicitly reports `ok:false` (missing `ok` = ok).
fn not_ok(d: &Value) -> Option<String> {
    if d.get("ok").and_then(Value::as_bool) == Some(false) {
        Some(
            d.get("error")
                .and_then(Value::as_str)
                .unwrap_or("unavailable")
                .to_string(),
        )
    } else {
        None
    }
}

fn s(v: &Value) -> String {
    v.as_str().unwrap_or("").to_string()
}

fn u(v: &Value) -> u64 {
    v.as_u64().unwrap_or(0)
}

fn b(v: &Value) -> bool {
    v.as_bool().unwrap_or(false)
}

fn f(v: &Value) -> f64 {
    v.as_f64().unwrap_or(0.0)
}

fn arr(v: &Value) -> Vec<Value> {
    v.as_array().cloned().unwrap_or_default()
}

/// One dashboard group (sidebar nav).
#[derive(Debug, Clone, Copy)]
pub struct UiGroup {
    pub id: &'static str,
    pub label: &'static str,
    pub cards: &'static [&'static str],
}

/// Default hash/localStorage group.
pub const DEFAULT_GROUP: &str = "sprint";

/// Grouped information architecture (ops / vision / sprint / studio).
pub const UI_GROUPS: [UiGroup; 4] = [
    UiGroup {
        id: "ops",
        label: "Ops",
        cards: &[
            "health",
            "products",
            "fingerprints",
            "sw",
            "watchdog",
            "mcp",
            "settings",
            "telegram",
            "tickets",
            "update",
            "tracker",
            "sli",
            "toolchain",
            "hooks-tests",
            "hooks-bench",
            "preview",
            "terminal",
        ],
    },
    UiGroup {
        id: "vision",
        label: "Vision",
        cards: &["vision", "vision-map", "vision-sync", "doc-preview"],
    },
    UiGroup {
        id: "sprint",
        label: "Sprint",
        cards: &[
            "sprint-queue",
            "sprint-board",
            "sprint-progress",
            "sprint-map",
            "sprint-focus",
        ],
    },
    UiGroup {
        id: "studio",
        label: "Studio",
        cards: &[
            "ide",
            "omni",
            "usage",
            "ratio",
            "speed-index",
            "rust-diagnostics",
        ],
    },
];

/// Chrome-only CARD_NAMES (header/backdrop — not dashboard groups).
pub const CHROME_CARDS: [&str; 8] = [
    "galaxy-backdrop",
    "starfield",
    "rss-ticker",
    "gpu-mode",
    "power-menu",
    "panel-dock",
    "fullscreen",
    "node-search",
];

/// `GET /api/ui/layout` — groups + default group + chrome card ids + nav HTML + header.
pub fn layout_wire() -> Value {
    serde_json::json!({
        "ok": true,
        "default_group": DEFAULT_GROUP,
        "groups": UI_GROUPS.iter().map(|g| serde_json::json!({
            "id": g.id,
            "label": g.label,
            "cards": g.cards,
        })).collect::<Vec<_>>(),
        "chrome": CHROME_CARDS,
        "html": render_nav(DEFAULT_GROUP),
        "header": render_header(),
    })
}

/// Default GPU-mode chrome wire (FX matches StarfieldMode fallback).
pub fn chrome_gpu_wire() -> Value {
    serde_json::json!({
        "ok": true,
        "mode": "fx",
        "active": true,
        "modes": ["eco", "fx", "ms"],
    })
}

/// Power-menu chrome wire — actions the header already exposes.
pub fn chrome_power_wire() -> Value {
    serde_json::json!({
        "ok": true,
        "level": "eco",
        "actions": [
            {"id": "soft", "label": "Soft sync Vision"},
            {"id": "reload", "label": "Reload UI"},
            {"id": "offline", "label": "Force offline"},
        ],
    })
}

/// Panel-dock chrome wire — collapsed set is client-owned, so default empty.
pub fn chrome_dock_wire() -> Value {
    serde_json::json!({ "ok": true, "panels": [] })
}

/// Fullscreen chrome wire — off until a card is expanded.
pub fn chrome_fullscreen_wire() -> Value {
    serde_json::json!({
        "ok": true,
        "active": false,
        "label": "fullscreen",
    })
}

/// Inner sidebar HTML (tabs + chips) for `#shellNav` — no wrapping `<nav>`.
pub fn render_nav(active: &str) -> String {
    let mut out = String::new();
    for g in &UI_GROUPS {
        let cls = if g.id == active {
            "nav-tab active"
        } else {
            "nav-tab"
        };
        out.push_str(&format!(
            "<button type='button' class='{cls}' data-group='{id}' aria-current='{cur}'>{label}</button><div class='nav-chips' data-for='{id}'>",
            id = g.id,
            cur = if g.id == active { "page" } else { "false" },
            label = esc(g.label),
        ));
        for c in g.cards {
            out.push_str(&format!(
                "<a href='#b-{c}' data-group='{id}' data-card-jump='{c}'>{c}</a>",
                id = g.id,
            ));
        }
        out.push_str("</div>");
    }
    out
}

/// Inner header-actions HTML for `#headerActions` — GPU / Auto / Resync / Power.
pub fn render_header() -> String {
    concat!(
        "<button id='btnGpu' class='badge gpu' title='GPU mode — Eco low GPU, FX full glow, Ms medium. Click → cycle' aria-label='GPU mode — cycle Eco, FX, Ms' type='button' data-action='gpu-cycle'>FX</button>",
        "<button id='btnAuto' class='badge' title='Auto-reload when vision files change' aria-label='Toggle auto-reload' type='button' data-action='auto-toggle'>Auto</button>",
        "<button type='button' data-action='resync'>Resync</button>",
        "<button type='button' data-action='notify-update'>notify update</button>",
        "<button id='btnPower' class='badge' title='Vision power — soft sync / reload' aria-haspopup='true' aria-expanded='false' aria-label='Vision power menu' type='button' data-action='power-toggle'>⏻ Power</button>",
        "<div id='powerMenu' class='power-menu' role='menu' aria-label='Vision power'>",
        "<button type='button' role='menuitem' data-action='power-soft'>Soft sync Vision</button>",
        "<button type='button' role='menuitem' data-action='power-reload'>Reload UI</button>",
        "<button type='button' role='menuitem' class='err' data-action='power-offline'>Force offline</button>",
        "</div>",
    )
    .into()
}

/// Every dashboard card id appears in exactly one [`UI_GROUPS`] entry.
pub fn layout_card_ids() -> Vec<&'static str> {
    UI_GROUPS
        .iter()
        .flat_map(|g| g.cards.iter().copied())
        .collect()
}

/// Tracker card (`/api/tracker`).
pub fn render_tracker(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let sp = &d["sprints"];
    let open = arr(&sp["open"]).len();
    let closed = arr(&sp["closed"]).len();
    let total = u(&sp["total"]);
    let next_raw = s(&sp["next"]);
    let next = if next_raw.is_empty() {
        "—".to_string()
    } else {
        esc(&next_raw)
    };
    let mut out = format!(
        "<div class='dim'>open {open} · closed {closed} · total {total} · next <kbd>{next}</kbd></div>"
    );
    let rows: Vec<Vec<String>> = arr(&d["records"])
        .iter()
        .map(|r| {
            vec![
                esc(&s(&r["kind"])),
                esc(&s(&r["label"])),
                esc(&s(&r["status"])),
                format!("<span class='dim'>{}</span>", esc(&s(&r["at"]))),
            ]
        })
        .collect();
    if rows.is_empty() {
        out.push_str(&empty_html("tracker records"));
        return out;
    }
    out.push_str(&tab(&["kind", "label", "status", "at"], rows));
    out
}

/// SLI console card (`/api/sli`).
pub fn render_sli(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let c = &d["catalog"];
    let used = u(&c["used_count"]);
    let unused = u(&c["unused_count"]);
    let mut out =
        format!("<div class='dim'>used {used} · unused (new SLI candidates) {unused}</div>");
    let rows: Vec<Vec<String>> = arr(&c["entries"])
        .iter()
        .map(|e| {
            let mark = if b(&e["used"]) {
                "<span class='ok'>●</span>".to_string()
            } else {
                "<span class='dim'>○</span>".to_string()
            };
            vec![
                mark,
                format!("<kbd>{}</kbd>", esc(&s(&e["name"]))),
                esc(&s(&e["kind"])),
                format!("<span class='dim'>{}</span>", esc(&s(&e["description"]))),
            ]
        })
        .collect();
    if rows.is_empty() {
        out.push_str(&empty_html("sli entries"));
        return out;
    }
    out.push_str(&tab(&["", "cmd", "kind", "desc"], rows));
    out
}

/// Toolchain card (`/api/toolchain`).
pub fn render_toolchain(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let rows: Vec<Vec<String>> = arr(&d["entries"])
        .iter()
        .map(|e| {
            vec![
                format!("<kbd>{}</kbd>", esc(&s(&e["tool"]))),
                esc(&s(&e["version"])),
                format!("<span class='dim'>{}</span>", esc(&s(&e["source"]))),
            ]
        })
        .collect();
    if rows.is_empty() {
        return empty_html("toolchain entries");
    }
    tab(&["tool", "version", "source"], rows)
}

/// Ratio card (`/api/ratio`).
pub fn render_ratio(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let cls = if b(&d["meets_min_ratio"]) {
        "ok"
    } else {
        "err"
    };
    let pct = f(&d["rust_ratio_pct"]);
    let band = f(&d["formal_band_min"]) * 100.0;
    let mut out =
        format!("<div>Rust ratio <span class='{cls}'>{pct:.2}%</span> · band min {band:.0}%</div>");
    out.push_str(&format!(
        "<div class='dim'>rust {} / non-rust {} · product {}</div>",
        u(&d["rust_loc"]),
        u(&d["non_rust_product_loc"]),
        u(&d["product_loc_total"]),
    ));
    let mut rows: Vec<Vec<String>> = Vec::new();
    if let Some(obj) = d["by_category"].as_object() {
        rows = obj
            .iter()
            .map(|(k, v)| {
                vec![
                    k.clone(),
                    u(&v["files"]).to_string(),
                    u(&v["loc"]).to_string(),
                ]
            })
            .collect();
    }
    if rows.is_empty() {
        out.push_str(&empty_html("ratio by-category"));
        return out;
    }
    out.push_str(&tab(&["category", "files", "loc"], rows));
    out
}

/// Sprint Queue card (`/api/vision/sprint-queue`).
pub fn render_sprint_queue(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let active = esc(&s(&d["active_sprint"]));
    let mut out = format!(
        "<div class='dim'>rev <kbd>{}</kbd> · next <span class='sprint-pill'>{}</span> · last closed <kbd>{}</kbd></div>",
        esc(&s(&d["revision"])),
        esc(&s(&d["next_sprint"])),
        esc(&s(&d["last_sprint_closed"])),
    );
    out.push_str(&format!(
        "<div>active <span class='sprint-pill'>{active}</span> · open <span class='ok'>{}</span></div>",
        u(&d["open_count"]),
    ));
    let planned = arr(&d["planned"]);
    out.push_str(&format!(
        "<details style='margin-top:6px'><summary>planned ({})</summary>",
        planned.len()
    ));
    let rows = planned
        .iter()
        .map(|p| {
            let pclass = if s(&p["id"]) == s(&d["active_sprint"]) {
                "open"
            } else if s(&p["id"]) == s(&d["next_sprint"]) {
                "next"
            } else {
                "closed"
            };
            vec![
                format!("<span class='squeue {pclass}'>{}</span>", esc(&s(&p["id"]))),
                format!("<span class='squeue-st'>{}</span>", esc(&s(&p["status"]))),
                esc(&s(&p["category"])),
                esc(&s(&p["title"])),
            ]
        })
        .collect();
    out.push_str(&tab(&["id", "status", "category", "title"], rows));
    out.push_str("</details>");
    out
}

/// Sprint Progress card (`/api/vision/sprint-progress`).
pub fn render_sprint_progress(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let mut out = format!(
        "<div class='dim'>rev <kbd>{}</kbd></div>",
        esc(&s(&d["revision"]))
    );
    out.push_str(&format!(
        "<div>open <span class='warn'>{}</span> · closed <span class='ok'>{}</span> · planned <span class='dim'>{}</span> · total <span class='dim'>{}</span></div>",
        u(&d["open_count"]),
        u(&d["closed_count"]),
        u(&d["planned_count"]),
        u(&d["total"]),
    ));
    out.push_str(&bar(f(&d["progress_pct"])));
    let rows = arr(&d["layers"])
        .iter()
        .map(|l| {
            vec![
                format!("<kbd>{}</kbd>", esc(&s(&l["id"]))),
                s(&l["z"]),
                format!("<span class='dim'>{}</span>", u(&l["node_count"])),
                format!("<span class='ok'>{}</span>", u(&l["linked_count"])),
            ]
        })
        .collect();
    out.push_str(&tab(&["layer", "z", "nodes", "linked"], rows));
    out
}

/// Speed Index card (`/api/vision/speeds`).
pub fn render_speed_index(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    if !b(&d["present"]) {
        return "<div class='dim'>no speed_index.json — run cargo xtask record-speed</div>"
            .to_string();
    }
    let si = &d["speed_index"];
    let l = &si["latest"];
    let wall = f(&l["test_ci_wall_secs"]);
    let bench_ns = l["last_bench_median_ns"].as_f64();
    let bench = match bench_ns {
        Some(ns) => format!("{:.2} ms", ns / 1e6),
        None => "—".to_string(),
    };
    let okmark = if b(&l["test_ci_ok"]) {
        " <span class='ok'>ok</span>".to_string()
    } else {
        " <span class='err'>fail</span>".to_string()
    };
    let mut out = format!(
        "<div class='dim'>{} · {} · git {}</div>",
        esc(&s(&si["host_label"])),
        esc(&s(&si["generated_at"])),
        esc(&s(&si["git_head"])),
    );
    out.push_str(&format!(
        "<div>test-ci <kbd>{wall:.1}s</kbd>{okmark} · bench <kbd>{bench}</kbd></div>"
    ));
    out.push_str(&format!(
        "<div class='dim'>{} test-ci rows · {} bench rows</div>",
        u(&si["test_ci_count"]),
        u(&si["bench_count"]),
    ));
    out
}

/// Rust Diagnostics card (`/api/vision/rust-diagnostics`).
pub fn render_rust_diagnostics(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    if !b(&d["present"]) {
        return "<div class='dim'>no rust_diagnostics.json — run cargo xtask record-rust</div>"
            .to_string();
    }
    let rd = &d["rust_diagnostics"];
    let l = &rd["latest"];
    let warnings = u(&l["warnings"]);
    let errors = u(&l["errors"]);
    let wcls = if warnings > 0 { "warn" } else { "ok" };
    let ecls = if errors > 0 { "err" } else { "ok" };
    let okmark = if b(&l["ok"]) {
        " <span class='ok'>clean</span>".to_string()
    } else {
        " <span class='err'>fail</span>".to_string()
    };
    let mut out = format!(
        "<div class='dim'>{} · {} · git {}</div>",
        esc(&s(&rd["host_label"])),
        esc(&s(&rd["generated_at"])),
        esc(&s(&rd["git_head"])),
    );
    out.push_str(&format!(
        "<div>warnings <span class='{wcls}'>{warnings}</span> · errors <span class='{ecls}'>{errors}</span>{okmark} · <span class='dim'>{} history rows</span></div>",
        u(&rd["history_count"]),
    ));
    let codes = arr(&l["top_codes"]);
    if !codes.is_empty() {
        let top: Vec<String> = codes
            .iter()
            .take(5)
            .map(|c| format!("<kbd>{}</kbd>", esc(&s(c))))
            .collect();
        out.push_str(&format!("<div class='dim'>top: {}</div>", top.join(" ")));
    }
    out
}

/// Tests hooks card (`/api/hooks/tests`).
pub fn render_hooks_tests(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let diag = &d["diagnostics"];
    let bins = arr(&d["test_bins"]);
    let mut out = format!(
        "<div class='dim'>status: <kbd>{}</kbd> · test bins: {}</div>",
        esc(&s(&d["status"])),
        bins.len(),
    );
    if !diag.is_null() {
        let errors = u(&diag["errors"]);
        let cls = if errors > 0 { "err" } else { "ok" };
        out.push_str(&format!(
            "<div>diagnostics: <span class='{cls}'>warnings {} · errors {errors}</span></div>",
            u(&diag["warnings"]),
        ));
    }
    let joined: Vec<String> = bins.iter().take(30).map(|b| esc(&s(b))).collect();
    out.push_str(&format!(
        "<div style='margin-top:6px' class='dim'>{}</div>",
        joined.join(" · ")
    ));
    out
}

/// Bench hooks card (`/api/hooks/bench`).
pub fn render_hooks_bench(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let sp = &d["speed_index"];
    let dirs = arr(&d["criterion_dirs"]);
    let mut out = format!(
        "<div class='dim'>status: <kbd>{}</kbd> · criterion dirs: {}</div>",
        esc(&s(&d["status"])),
        dirs.len(),
    );
    if !sp.is_null() {
        let cls = if b(&sp["test_ci_ok"]) { "ok" } else { "err" };
        out.push_str(&format!(
            "<div>test-ci wall: <span class='{cls}'>{}s</span></div>",
            f(&sp["test_ci_wall_secs"]),
        ));
    }
    let joined: Vec<String> = dirs.iter().map(|dir| esc(&s(dir))).collect();
    out.push_str(&format!(
        "<div style='margin-top:6px' class='dim'>{}</div>",
        joined.join(" · ")
    ));
    out
}

/// Sprint Map card (`/api/vision/sprint-map`).
pub fn render_sprint_map(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let mut out = format!(
        "<div class='dim'>rev <kbd>{}</kbd> · nodes {} · next <kbd>{}</kbd> · last closed <kbd>{}</kbd></div>",
        esc(&s(&d["revision"])),
        u(&d["nodes_count"]),
        esc(&s(&d["next_sprint"])),
        esc(&s(&d["last_sprint_closed"])),
    );
    let rows: Vec<Vec<String>> = arr(&d["modules"])
        .iter()
        .map(|m| {
            vec![
                format!("<kbd>{}</kbd>", esc(&s(&m["id"]))),
                esc(&s(&m["layer"])),
                format!("<span class='ok'>{}</span>", u(&m["targets"])),
            ]
        })
        .collect();
    out.push_str(&tab(&["module", "layer", "targets"], rows));
    let kinds: Vec<String> = arr(&d["kinds"])
        .iter()
        .map(|k| format!("<kbd>{}</kbd>×{}", esc(&s(&k["kind"])), u(&k["count"])))
        .collect();
    out.push_str(&format!(
        "<div class='dim' style='margin-top:6px'>kinds: {}</div>",
        kinds.join(" ")
    ));
    let links = arr(&d["links"]);
    out.push_str(&format!(
        "<details style='margin-top:6px'><summary>links ({})</summary>",
        links.len()
    ));
    let rows: Vec<Vec<String>> = links
        .iter()
        .map(|l| {
            vec![
                format!("<kbd>{}</kbd>", esc(&s(&l["kind"]))),
                esc(&s(&l["from"]["id"])),
                "→".to_string(),
                esc(&s(&l["to"]["id"])),
            ]
        })
        .collect();
    out.push_str(&tab(&["kind", "from", "→", "to"], rows));
    out.push_str("</details>");
    out
}

/// Sprint Board card (`/api/vision/sprint-board`).
pub fn render_sprint_board(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let mut out = format!(
        "<div class='dim'>rev <kbd>{}</kbd> · next <span class='sprint-pill'>{}</span> · active <span class='sprint-pill'>{}</span></div>",
        esc(&s(&d["revision"])),
        esc(&s(&d["next_sprint"])),
        esc(&s(&d["active_sprint"])),
    );
    out.push_str(&format!(
        "<div>open <span class='warn'>{}</span> · closed <span class='ok'>{}</span> · total <span class='dim'>{}</span></div>",
        u(&d["open_count"]),
        u(&d["closed_count"]),
        u(&d["total"]),
    ));
    out.push_str(&bar(f(&d["progress_pct"])));
    for c in arr(&d["columns"]) {
        let entries = arr(&c["entries"]);
        out.push_str(&format!(
            "<details style='margin-top:6px'><summary>{} ({})</summary>",
            esc(&s(&c["name"])),
            u(&c["count"]),
        ));
        let rows: Vec<Vec<String>> = entries
            .iter()
            .map(|e| {
                let pclass = if s(&e["id"]) == s(&d["active_sprint"]) {
                    "open"
                } else if s(&e["status"]) == "closed" {
                    "closed"
                } else {
                    ""
                };
                vec![
                    format!("<span class='squeue {pclass}'>{}</span>", esc(&s(&e["id"]))),
                    format!("<span class='squeue-st'>{}</span>", esc(&s(&e["status"]))),
                    esc(&s(&e["title"])),
                ]
            })
            .collect();
        out.push_str(&tab(&["id", "status", "title"], rows));
        out.push_str("</details>");
    }
    out
}

/// OmniRouter card (`/api/omni`).
pub fn render_omni(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let providers = arr(&d["providers"]);
    let models = arr(&d["models"]);
    let rec = arr(&d["recommended"]);
    let routing = &d["routing"];
    let default_provider = s(&routing["default_provider"]);
    let mut out = format!(
        "<div class='dim'>providers {} · models {} · default <kbd>{}</kbd> · research {}</div>",
        providers.len(),
        models.len(),
        esc(&default_provider),
        esc(&s(&d["researched_at"])),
    );
    let selected = s(&d["selected_product"]);
    let account = s(&d["account_product"]);
    if !selected.is_empty() && selected != account && selected != "gsv" {
        let acc = if account.is_empty() { "gsv" } else { &account };
        out.push_str(&err_html(&format!(
            "OmniRouter keys are {acc} data/omni.toml — not {selected} account"
        )));
    }
    if default_provider.is_empty() {
        out.push_str(&err_html("omni unavailable"));
        return out;
    }
    let rec_ids: Vec<String> = rec
        .iter()
        .map(|m| format!("<kbd>{}</kbd>", esc(&s(&m["id"]))))
        .collect();
    out.push_str(&format!(
        "<div style='margin-top:6px'>recommended: {}</div>",
        if rec_ids.is_empty() {
            "—".to_string()
        } else {
            rec_ids.join(" ")
        }
    ));
    out.push_str(&format!(
        "<details style='margin-top:6px'><summary>Providers ({})</summary>",
        providers.len()
    ));
    let rows: Vec<Vec<String>> = providers
        .iter()
        .map(|p| {
            vec![
                format!("<kbd>{}</kbd>", esc(&s(&p["id"]))),
                esc(&s(&p["name"])),
                if b(&p["enabled"]) {
                    "<span class='ok'>on</span>".to_string()
                } else {
                    "<span class='err'>off</span>".to_string()
                },
                if b(&p["key_set"]) {
                    "<span class='ok'>key</span>".to_string()
                } else {
                    "<span class='dim'>no key</span>".to_string()
                },
                format!("<span class='dim'>{}</span>", esc(&s(&p["base_url"]))),
                {
                    let q = &p["quota"];
                    let rpm = q["rpm"]
                        .as_u64()
                        .map(|n| format!("{n}rpm"))
                        .unwrap_or_default();
                    let cool = q["cooldown_secs"].as_u64().unwrap_or(0);
                    if cool > 0 {
                        format!("<span class='err'>wait {cool}s</span>")
                    } else if b(&p["free"]) {
                        format!("<span class='ok'>free {rpm}</span>")
                    } else {
                        rpm
                    }
                },
            ]
        })
        .collect();
    out.push_str(&tab(
        &["id", "name", "state", "key", "base_url", "quota"],
        rows,
    ));
    out.push_str("</details>");
    out.push_str(&format!(
        "<details style='margin-top:6px'><summary>Models ({})</summary>",
        models.len()
    ));
    let rows: Vec<Vec<String>> = models
        .iter()
        .map(|m| {
            let ctx = m["context_window"]
                .as_u64()
                .map(format_number)
                .unwrap_or_else(|| "varies".to_string());
            let out_max = m["max_output"]
                .as_u64()
                .map(format_number)
                .unwrap_or_else(|| "varies".to_string());
            vec![
                format!("<kbd>{}</kbd>", esc(&s(&m["id"]))),
                esc(&s(&m["provider"])),
                ctx,
                out_max,
                if b(&m["free"]) {
                    "free".to_string()
                } else {
                    String::new()
                },
                if b(&m["recommended"]) {
                    "<span class='ok'>★</span>".to_string()
                } else {
                    String::new()
                },
                {
                    let mut tags = Vec::new();
                    if b(&m["rust"]) {
                        tags.push("rs");
                    }
                    if b(&m["web"]) {
                        tags.push("web");
                    }
                    tags.join(" ")
                },
            ]
        })
        .collect();
    out.push_str(&tab(&["id", "provider", "ctx", "out", "", "", "fit"], rows));
    out.push_str("</details>");
    out
}

fn format_number(n: u64) -> String {
    let mut s = n.to_string();
    let mut i = s.len();
    while i > 3 {
        i -= 3;
        s.insert(i, ',');
    }
    s
}

/// MCP `gsv_mcp_openbot` card (`GET /mcp`).
pub fn render_mcp(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let tools = arr(&d["tools"]);
    if tools.is_empty() && s(&d["name"]).is_empty() {
        return empty_html("mcp");
    }
    let count = u(&d["tool_count"]).max(tools.len() as u64);
    let resources = arr(&d["resources"]);
    let prompts = arr(&d["prompts"]);
    let resource_count = u(&d["resource_count"]).max(resources.len() as u64);
    let prompt_count = u(&d["prompt_count"]).max(prompts.len() as u64);
    let mut out = format!(
        "<div class='dim'>{} · {} · tools {} · resources {} · prompts {}",
        esc(&s(&d["name"])),
        esc(&s(&d["protocol"])),
        count,
        resource_count,
        prompt_count
    );
    let log_level = s(&d["log_level"]);
    if d["logging"].as_bool().unwrap_or(false) || !log_level.is_empty() {
        out.push_str(&format!(
            " · logging <kbd>{}</kbd>",
            esc(if log_level.is_empty() {
                "on"
            } else {
                &log_level
            })
        ));
    }
    if d["completions"].as_bool().unwrap_or(false) {
        out.push_str(" · completions");
    }
    if d["subscribe"].as_bool().unwrap_or(false) {
        out.push_str(&format!(
            " · subscribe <kbd>{}</kbd>",
            u(&d["subscription_count"])
        ));
    }
    if d["sse"].as_bool().unwrap_or(false) || d["streamable"].as_bool().unwrap_or(false) {
        out.push_str(" · sse");
    }
    if d["sessions"].as_bool().unwrap_or(false) {
        out.push_str(&format!(
            " · sessions <kbd>{}</kbd>",
            u(&d["session_count"])
        ));
    }
    out.push_str("</div>");
    let stdio = s(&d["stdio"]);
    let stdio_live = s(&d["stdio_live"]);
    let http = s(&d["http"]);
    let http_url = s(&d["http_url"]);
    let version = s(&d["version"]);
    if !stdio.is_empty() || !http.is_empty() || !stdio_live.is_empty() {
        out.push_str(&format!(
            "<div>stdio <kbd>{}</kbd> · live <kbd>{}</kbd> · http <kbd>{}</kbd></div>",
            esc(if stdio.is_empty() { "—" } else { &stdio }),
            esc(if stdio_live.is_empty() {
                "—"
            } else {
                &stdio_live
            }),
            esc(if http.is_empty() { "—" } else { &http })
        ));
    }
    if !version.is_empty() || !http_url.is_empty() {
        let crate_ver = s(&d["crate_version"]);
        let lag = b(&d["version_lag"]);
        let crate_bit = if crate_ver.is_empty() {
            String::new()
        } else if lag {
            format!(
                " · crate <kbd>{}</kbd> <span class='warn'>lag</span>",
                esc(&crate_ver)
            )
        } else {
            format!(" · crate <kbd>{}</kbd>", esc(&crate_ver))
        };
        out.push_str(&format!(
            "<div>ver <kbd>{}</kbd>{} · url <kbd>{}</kbd></div>",
            esc(if version.is_empty() { "—" } else { &version }),
            crate_bit,
            esc(if http_url.is_empty() {
                "—"
            } else {
                &http_url
            })
        ));
    }
    let sandbox = s(&d["sandbox"]);
    if !sandbox.is_empty() {
        out.push_str(&format!("<div>sandbox <kbd>{}</kbd></div>", esc(&sandbox)));
    }
    if tools.is_empty() {
        out.push_str(&empty_html("mcp tools"));
        return out;
    }
    let rows: Vec<Vec<String>> = tools
        .iter()
        .map(|t| {
            let name = t.as_str().map(str::to_string).unwrap_or_else(|| s(t));
            vec![format!("<kbd>{}</kbd>", esc(&name))]
        })
        .collect();
    out.push_str(&tab(&["tool"], rows));
    if !resources.is_empty() {
        let rrows: Vec<Vec<String>> = resources
            .iter()
            .map(|t| {
                let name = t.as_str().map(str::to_string).unwrap_or_else(|| s(t));
                vec![format!("<kbd>{}</kbd>", esc(&name))]
            })
            .collect();
        out.push_str(&tab(&["resource"], rrows));
    }
    if !prompts.is_empty() {
        let prows: Vec<Vec<String>> = prompts
            .iter()
            .map(|t| {
                let name = t.as_str().map(str::to_string).unwrap_or_else(|| s(t));
                vec![format!("<kbd>{}</kbd>", esc(&name))]
            })
            .collect();
        out.push_str(&tab(&["prompt"], prows));
    }
    out
}

/// Settings / Godfather card (`/api/settings`) — never echoes the bot token.
pub fn render_settings(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let token_set = b(&d["token_set"]);
    let source = s(&d["source"]);
    let channel = s(&d["godfather"]["channel_id"]);
    let users = arr(&d["godfather"]["allowed_user_ids"]);
    let enabled = arr(&d["workflows"]["enabled"]);
    let empty = !token_set && channel.is_empty();
    let mut out = String::new();
    if empty {
        out.push_str(&empty_html("settings"));
    }
    let token_bit = if token_set { "set" } else { "unset" };
    let src_bit = if source.is_empty() { "none" } else { &source };
    out.push_str(&format!(
        "<div class='dim'>Godfather · token <kbd>{}</kbd> · source <kbd>{}</kbd> · file <kbd>data/gsv_settings.json</kbd></div>",
        token_bit,
        esc(src_bit)
    ));
    let user_join = users
        .iter()
        .filter_map(|v| v.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let wf_join = enabled
        .iter()
        .filter_map(|v| v.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    out.push_str(&tab(
        &["field", "value"],
        vec![
            vec![
                "channel".into(),
                if channel.is_empty() {
                    "<span class='dim'>—</span>".into()
                } else {
                    format!("<kbd>{}</kbd>", esc(&channel))
                },
            ],
            vec![
                "allowed users".into(),
                if user_join.is_empty() {
                    "<span class='dim'>—</span>".into()
                } else {
                    esc(&user_join)
                },
            ],
            vec![
                "workflows".into(),
                if wf_join.is_empty() {
                    "<span class='dim'>—</span>".into()
                } else {
                    esc(&wf_join)
                },
            ],
        ],
    ));
    let tok_ph = if token_set {
        "token set — paste to replace"
    } else {
        "bot token"
    };
    out.push_str(&format!(
        "<div class='dim'>owner POST · never shown again</div>\
<input id='setChannel' type='text' value='{}' placeholder='Godfather channel id' aria-label='Godfather channel id'>\
<input id='setUsers' type='text' value='{}' placeholder='allowed user ids' aria-label='allowed Telegram user ids'>\
<input id='setToken' type='password' value='' placeholder='{}' aria-label='Godfather bot token' autocomplete='off'>\
<input id='setWorkflows' type='text' value='{}' placeholder='drain, ticket-claim' aria-label='co-workflow ids'>\
<button type='button' data-action='settings-save'>Save</button>",
        esc(&channel),
        esc(&user_join),
        esc(tok_ph),
        esc(&wf_join)
    ));
    out
}

/// Telegram Godfather bind card (`/api/telegram`) — never echoes the bot token.
pub fn render_telegram(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let channel = s(&d["channel_id"]);
    let token_set = b(&d["token_set"]);
    let empty = !token_set && channel.is_empty();
    let mut out = String::new();
    if empty {
        out.push_str(&empty_html("telegram"));
    }
    let dry = b(&d["dry_run"]);
    let polling = b(&d["polling"]);
    let bot = s(&d["bot_username"]);
    let title = s(&d["chat_title"]);
    let probe = s(&d["last_probe"]);
    let bus_ts = s(&d["last_bus_ts"]);
    let bus_err = s(&d["last_bus_error"]);
    out.push_str(&format!(
        "<div class='dim'>Godfather bind · token <kbd>{}</kbd> · dry-run <kbd>{}</kbd></div>",
        if token_set { "set" } else { "unset" },
        if dry { "yes" } else { "no" }
    ));
    out.push_str(&tab(
        &["field", "value"],
        vec![
            vec![
                "channel".into(),
                if channel.is_empty() {
                    "<span class='dim'>—</span>".into()
                } else {
                    format!("<kbd>{}</kbd>", esc(&channel))
                },
            ],
            vec![
                "bot".into(),
                if bot.is_empty() {
                    "<span class='dim'>—</span>".into()
                } else {
                    format!("<kbd>{}</kbd>", esc(&bot))
                },
            ],
            vec![
                "chat".into(),
                if title.is_empty() {
                    "<span class='dim'>—</span>".into()
                } else {
                    esc(&title)
                },
            ],
            vec![
                "last probe".into(),
                if probe.is_empty() {
                    "<span class='dim'>—</span>".into()
                } else {
                    esc(&probe)
                },
            ],
            vec![
                "polling".into(),
                format!(
                    "<span class='{}'>{}</span>",
                    if polling { "ok" } else { "dim" },
                    if polling { "on" } else { "off" }
                ),
            ],
            vec![
                "last bus".into(),
                if bus_ts.is_empty() {
                    "<span class='dim'>—</span>".into()
                } else {
                    esc(&bus_ts)
                },
            ],
            vec![
                "bus error".into(),
                if bus_err.is_empty() {
                    "<span class='dim'>—</span>".into()
                } else {
                    format!("<span class='err'>{}</span>", esc(&bus_err))
                },
            ],
            vec!["last ticket".into(), {
                let id = s(&d["last_ticket_id"]);
                if id.is_empty() {
                    "<span class='dim'>—</span>".into()
                } else {
                    format!("<kbd>{}</kbd>", esc(&id))
                }
            }],
        ],
    ));
    out.push_str("<div class='dim'>solo bot · MCP <kbd>gsv_telegram_ticket</kbd> · <kbd>/ticket</kbd> title</div>");
    out
}

/// Ticket board card (`/api/tickets`) — open tickets are the board.
pub fn render_tickets(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let tickets = arr(&d["tickets"]);
    let mode = s(&d["mode"]);
    let mode_bit = if mode.is_empty() { "solo" } else { &mode };
    let online = arr(&d["online"]);
    let scenarios = arr(&d["scenarios"]);
    let mut out = String::from("<div class='dim'>open tickets are the board</div>");
    out.push_str("<div class='dim'>walk posts session lines · solo / squad / bench</div>");
    out.push_str(
        "<div class='dim'>hook · <kbd>run mcp bot hook up scenario</kbd> · band / plan</div>",
    );
    let bench_line = s(&d["bench"]["line"]);
    if bench_line.is_empty() {
        out.push_str("<div class='dim'>scenario bench · <kbd>gsv_tickets_bench</kbd></div>");
    } else {
        out.push_str(&format!(
            "<div class='dim'>last bench <kbd>{}</kbd></div>",
            esc(&bench_line)
        ));
    }
    out.push_str(&format!(
        "<div class='dim'>mode <kbd>{}</kbd> · online <kbd>{}</kbd></div>",
        esc(mode_bit),
        online.len()
    ));
    if tickets.is_empty() {
        out.push_str(&empty_html("tickets"));
    }
    for col in ["open", "in_progress", "done", "blocked"] {
        let rows: Vec<Vec<String>> = tickets
            .iter()
            .filter(|t| s(&t["status"]) == col)
            .map(|t| {
                let id = s(&t["id"]);
                let title = s(&t["title"]);
                let action = if col == "open" && !id.is_empty() {
                    format!(
                        "<button type='button' data-action='tickets-claim' data-ticket-id='{}'>claim</button>",
                        esc(&id)
                    )
                } else if col == "in_progress" && !id.is_empty() {
                    let until = u(&t["lease_until"]);
                    let lease_bit = if until == 0 {
                        "lease —".to_string()
                    } else {
                        format!("lease <kbd>{until}</kbd>")
                    };
                    format!(
                        "<button type='button' data-action='tickets-done' data-ticket-id='{}'>done</button> \
<button type='button' data-action='tickets-error' data-ticket-id='{}'>error</button> \
<button type='button' data-action='tickets-reclaim' data-ticket-id='{}'>reclaim</button> \
<span class='dim'>{}</span>",
                        esc(&id),
                        esc(&id),
                        esc(&id),
                        lease_bit
                    )
                } else {
                    format!("<kbd>{}</kbd>", esc(&id))
                };
                vec![esc(&title), action]
            })
            .collect();
        out.push_str(&format!("<div class='dim'>{col}</div>"));
        out.push_str(&tab(&["title", "id"], rows));
    }
    if !scenarios.is_empty() {
        out.push_str("<div class='dim'>scenarios</div>");
        let rows: Vec<Vec<String>> = scenarios
            .iter()
            .map(|sc| {
                let sid = s(&sc["id"]);
                let title = s(&sc["title"]);
                let wf = s(&sc["workflow"]);
                let btn = if sid.is_empty() {
                    "—".into()
                } else {
                    format!(
                        "<button type='button' data-action='tickets-from-scenario' data-scenario-id='{}'>add</button> \
<button type='button' data-action='tickets-walk' data-scenario-id='{}'>walk</button> \
<button type='button' data-action='tickets-hook' data-scenario-id='{}'>hook</button>",
                        esc(&sid),
                        esc(&sid),
                        esc(&sid)
                    )
                };
                vec![esc(&sid), esc(&title), esc(&wf), btn]
            })
            .collect();
        out.push_str(&tab(&["id", "title", "workflow", "add"], rows));
    }
    out.push_str(
        "<div class='dim'>create</div>\
<input id='tixTitle' type='text' value='' placeholder='ticket title' aria-label='ticket title'>\
<input id='tixBody' type='text' value='' placeholder='body' aria-label='ticket body'>\
<input id='tixProduct' type='text' value='gsv' placeholder='product' aria-label='product'>\
<button type='button' data-action='tickets-create'>Create</button>\
<button type='button' data-action='tickets-presence'>I'm online</button>\
<button type='button' data-action='tickets-walk'>solo walk</button>\
<button type='button' data-action='tickets-bench'>record scenario bench</button>\
<input id='tixHook' type='text' value='run mcp bot hook up scenario band 177' placeholder='hook phrase' aria-label='hook phrase'>\
<button type='button' data-action='tickets-hook'>hook phrase</button>",
    );
    out
}

/// Health card (`/api/health`).
pub fn render_health(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let avail = b(&d["update_available"]);
    tab(
        &["field", "value"],
        vec![
            vec!["name".into(), esc(&s(&d["name"]))],
            vec!["product".into(), esc(&s(&d["product"]))],
            vec!["version".into(), esc(&s(&d["version"]))],
            vec!["crate_version".into(), {
                let v = s(&d["crate_version"]);
                if v.is_empty() {
                    "—".into()
                } else {
                    esc(&v)
                }
            }],
            vec![
                "version_lag".into(),
                format!(
                    "<span class='{}'>{}</span>",
                    if b(&d["version_lag"]) { "warn" } else { "ok" },
                    b(&d["version_lag"])
                ),
            ],
            vec!["selected".into(), {
                let sel = s(&d["selected_product"]);
                if sel.is_empty() {
                    "—".into()
                } else {
                    format!(
                        "<kbd>{}</kbd> v{}",
                        esc(&sel),
                        esc(&s(&d["selected_version"]))
                    )
                }
            }],
            vec!["uptime_secs".into(), u(&d["uptime_secs"]).to_string()],
            vec![
                "ok".into(),
                format!("<span class='ok'>{}</span>", b(&d["ok"])),
            ],
            vec![
                "update_available".into(),
                format!(
                    "<span class='{}'>{}</span>",
                    if avail { "warn" } else { "ok" },
                    avail
                ),
            ],
            vec![
                "watchdog".into(),
                format!(
                    "<span class='{}'>{}</span>",
                    if b(&d["watchdog_alive"]) { "ok" } else { "dim" },
                    b(&d["watchdog_alive"])
                ),
            ],
        ],
    )
}

/// VDT products card (`/api/products`) — list / select / open.
pub fn render_products(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let products = arr(&d["products"]);
    if products.is_empty() {
        return empty_html("products");
    }
    let selected = s(&d["selected"]);
    let mut out = format!(
        "<div class='dim'>selected <kbd>{}</kbd> · {} products</div>",
        esc(if selected.is_empty() {
            "—"
        } else {
            &selected
        }),
        products.len()
    );
    let rows: Vec<Vec<String>> = products
        .iter()
        .map(|p| {
            let id = s(&p["id"]);
            vec![
                format!("<kbd>{}</kbd>", esc(&id)),
                esc(&s(&p["name"])),
                esc(&s(&p["kind"])),
                if b(&p["registered"]) {
                    "<span class='ok'>yes</span>".into()
                } else {
                    "<span class='dim'>no</span>".into()
                },
                format!(
                    "<button type='button' data-action='product-select' data-product-id='{}'>select</button>",
                    esc(&id)
                ),
                format!(
                    "<button type='button' data-action='product-open' data-product-id='{}'>open</button>",
                    esc(&id)
                ),
            ]
        })
        .collect();
    out.push_str(&tab(&["id", "name", "kind", "reg", "", ""], rows));
    let scan = &d["scan"];
    if scan.is_object() {
        out.push_str(&format!(
            "<div class='dim'>scan git <kbd>{}</kbd> · cargo <kbd>{}</kbd> · handoff {}</div>",
            esc(&s(&scan["git_head"])),
            esc(&s(&scan["cargo_name"])),
            if b(&scan["handoff_exists"]) {
                "yes"
            } else {
                "no"
            }
        ));
    }
    out
}

/// Drain fingerprints card (`/api/fingerprints`).
pub fn render_fingerprints(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let fps = arr(&d["fingerprints"]);
    if fps.is_empty() {
        return empty_html("fingerprints");
    }
    let mut out = format!(
        "<div class='dim'>{} fingerprints · <kbd>docs/gsv/fingerprints.jsonl</kbd></div>",
        fps.len()
    );
    let server = s(&d["server_product"]);
    let server_ver = s(&d["server_version"]);
    if !server.is_empty() {
        out.push_str(&format!(
            "<div class='dim'>server <kbd>{}</kbd> v{}</div>",
            esc(&server),
            esc(&server_ver)
        ));
    }
    let selected = s(&d["selected"]);
    if !selected.is_empty() {
        let sel_ver = s(&d["selected_version"]);
        let warn = b(&d["cross_product"]);
        out.push_str(&format!(
            "<div class='{}'>selected <kbd>{}</kbd> v{}{}</div>",
            if warn { "err" } else { "dim" },
            esc(&selected),
            esc(if sel_ver.is_empty() { "—" } else { &sel_ver }),
            if warn {
                " · not GSV crate version"
            } else {
                ""
            }
        ));
    }
    let rows: Vec<Vec<String>> = fps
        .iter()
        .map(|f| {
            let product = s(&f["product"]);
            vec![
                esc(if product.is_empty() { "gsv" } else { &product }),
                esc(&s(&f["ts"])),
                esc(&s(&f["ide"])),
                esc(&s(&f["model"])),
                esc(&s(&f["agent"])),
                esc(&s(&f["version"])),
                esc(&s(&f["summary"])),
            ]
        })
        .collect();
    out.push_str(&tab(
        &["product", "ts", "ide", "model", "agent", "ver", "summary"],
        rows,
    ));
    out
}

/// Session token usage (`/api/usage`) — OmniRouter + MCP + OmniRoute.
pub fn render_usage(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let sessions = arr(&d["sessions"]);
    let prompt = u(&d["process"]["prompt_tokens"]);
    let completion = u(&d["process"]["completion_tokens"]);
    let total = u(&d["process"]["total_tokens"]);
    let requests = u(&d["process"]["requests"]);
    let or_ok = b(&d["omniroute"]["ok"]);
    if sessions.is_empty() && total == 0 && !or_ok {
        return empty_html("usage");
    }
    let mut out = format!(
        "<div class='dim'>session tokens · requests <kbd>{}</kbd> · prompt <kbd>{}</kbd> · completion <kbd>{}</kbd> · total <kbd>{}</kbd></div>",
        format_number(requests),
        format_number(prompt),
        format_number(completion),
        format_number(total)
    );
    if !sessions.is_empty() {
        let rows: Vec<Vec<String>> = sessions
            .iter()
            .map(|row| {
                vec![
                    esc(&s(&row["session"])),
                    esc(&s(&row["source"])),
                    format_number(u(&row["requests"])),
                    format_number(u(&row["prompt_tokens"])),
                    format_number(u(&row["completion_tokens"])),
                    format_number(u(&row["total_tokens"])),
                ]
            })
            .collect();
        out.push_str(&tab(
            &["session", "source", "req", "prompt", "completion", "total"],
            rows,
        ));
    }
    let or = &d["omniroute"];
    if or_ok || !s(&or["error"]).is_empty() || u(&or["total_tokens"]) > 0 {
        out.push_str(&format!(
            "<div class='dim'>omniroute <kbd>{}</kbd> · req <kbd>{}</kbd> · tokens <kbd>{}</kbd></div>",
            esc(&s(&or["base_url"])),
            format_number(u(&or["requests"])),
            format_number(u(&or["total_tokens"]))
        ));
    }
    out
}

/// Live watchdog ops card (`/api/watchdog`).
pub fn render_watchdog(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let path = s(&d["path"]);
    if path.is_empty() && d.get("pid").is_none() && d.get("last_action").is_none() {
        return empty_html("watchdog");
    }
    let alive = b(&d["alive"]);
    let mut out = format!(
        "<div class='dim'>heartbeat <kbd>{}</kbd></div>",
        esc(if path.is_empty() { "—" } else { &path })
    );
    let age = if d["age_secs"].is_null() {
        "—".into()
    } else {
        u(&d["age_secs"]).to_string()
    };
    let pid = if d.get("pid").and_then(Value::as_u64).is_some() {
        u(&d["pid"]).to_string()
    } else {
        "—".into()
    };
    let action = s(&d["last_action"]);
    let action_cls =
        if action == "lockstep-fail" || action == "lockstep-err" || action == "lockstep-wait" {
            "warn"
        } else if action == "lockstep-apply" {
            "ok"
        } else {
            ""
        };
    let action_html = if action.is_empty() {
        "—".into()
    } else if action_cls.is_empty() {
        esc(&action)
    } else {
        format!("<span class='{action_cls}'>{}</span>", esc(&action))
    };
    out.push_str(&tab(
        &["field", "value"],
        vec![
            vec![
                "alive".into(),
                format!(
                    "<span class='{}'>{}</span>",
                    if alive { "ok" } else { "dim" },
                    alive
                ),
            ],
            vec!["pid".into(), pid],
            vec!["bin_version".into(), {
                let v = s(&d["bin_version"]);
                if v.is_empty() {
                    "—".into()
                } else {
                    esc(&v)
                }
            }],
            vec!["crate_version".into(), {
                let v = s(&d["crate_version"]);
                if v.is_empty() {
                    "—".into()
                } else {
                    esc(&v)
                }
            }],
            vec![
                "version_lag".into(),
                format!(
                    "<span class='{}'>{}</span>",
                    if b(&d["version_lag"]) { "warn" } else { "ok" },
                    b(&d["version_lag"])
                ),
            ],
            vec!["age_secs".into(), age],
            vec!["last_action".into(), action_html],
            vec![
                "consecutive_failures".into(),
                u(&d["consecutive_failures"]).to_string(),
            ],
            vec![
                "debug_newer".into(),
                format!(
                    "<span class='{}'>{}</span>",
                    if b(&d["debug_newer"]) { "warn" } else { "ok" },
                    b(&d["debug_newer"])
                ),
            ],
            vec!["last_apply_status".into(), {
                let st = u(&d["last_apply_status"]);
                if st == 0 {
                    "—".into()
                } else {
                    st.to_string()
                }
            }],
            vec!["lockstep_note".into(), {
                let note = s(&d["lockstep_note"]);
                esc(if note.is_empty() { "—" } else { &note })
            }],
        ],
    ));
    out
}

/// Service Worker shell-cache card (`/api/sw`).
pub fn render_sw(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let urls = arr(&d["urls"]);
    if urls.is_empty() {
        return empty_html("sw precache");
    }
    let mut out = format!(
        "<div class='dim'>cache <kbd>{}</kbd> · script <kbd>{}</kbd> · {} urls</div>",
        esc(&s(&d["cache"])),
        esc(&s(&d["script"])),
        urls.len()
    );
    let rows: Vec<Vec<String>> = urls.iter().map(|u| vec![esc(&s(u))]).collect();
    out.push_str(&tab(&["url"], rows));
    out
}

/// Update card (`/api/update`).
pub fn render_update(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let avail = b(&d["update_available"]);
    let head = s(&d["git_head"]);
    tab(
        &["field", "value"],
        vec![
            vec!["version".into(), esc(&s(&d["version"]))],
            vec!["crate_version".into(), {
                let v = s(&d["crate_version"]);
                if v.is_empty() {
                    "—".into()
                } else {
                    esc(&v)
                }
            }],
            vec![
                "version_lag".into(),
                format!(
                    "<span class='{}'>{}</span>",
                    if b(&d["version_lag"]) { "warn" } else { "ok" },
                    b(&d["version_lag"])
                ),
            ],
            vec![
                "git_head".into(),
                esc(if head.is_empty() { "—" } else { &head }),
            ],
            vec![
                "update_available".into(),
                format!(
                    "<span class='{}'>{}</span>",
                    if avail { "warn" } else { "ok" },
                    avail
                ),
            ],
            vec!["binary_mtime".into(), u(&d["binary_mtime"]).to_string()],
            vec![
                "newest_src_mtime".into(),
                u(&d["newest_src_mtime"]).to_string(),
            ],
            vec![
                "live_copy".into(),
                format!(
                    "<span class='{}'>{}</span>",
                    if b(&d["live_copy"]) { "ok" } else { "dim" },
                    b(&d["live_copy"])
                ),
            ],
        ],
    )
}

/// IDE card (`/api/ide/sessions`) — sessions + last messages of the selection.
pub fn render_ide(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let sel = &d["selection"];
    let mut out = String::new();
    if sel.is_object() {
        out.push_str(&format!(
            "<div class='dim'>selected: <kbd>{}</kbd> / <kbd>{}</kbd></div>",
            esc(&s(&sel["tool"])),
            esc(&s(&sel["session"]))
        ));
    }
    let preview = arr(&d["preview"]);
    if preview.is_empty() {
        out.push_str(&empty_html("ide preview"));
    } else {
        out.push_str("<div class='dim' style='margin-top:6px'>last messages</div>");
        let rows: Vec<Vec<String>> = preview
            .iter()
            .map(|m| {
                vec![
                    format!("<kbd>{}</kbd>", esc(&s(&m["role"]))),
                    esc(&s(&m["text"])),
                ]
            })
            .collect();
        out.push_str(&tab(&["role", "text"], rows));
    }
    let sessions = arr(&d["sessions"]);
    if sessions.is_empty() {
        out.push_str(&empty_html("ide sessions"));
        return out;
    }
    let rows: Vec<Vec<String>> = sessions
        .iter()
        .take(40)
        .map(|srow| {
            let tool = s(&srow["tool"]);
            let id = s(&srow["id"]);
            vec![
                esc(&tool),
                esc(&s(&srow["label"])),
                format!("<span class='dim'>{}</span>", esc(&s(&srow["modified"]))),
                format!(
                    "<button type='button' data-ide-tool='{}' data-ide-session='{}'>select</button>",
                    esc(&tool),
                    esc(&id)
                ),
            ]
        })
        .collect();
    out.push_str(&tab(&["tool", "session", "modified", ""], rows));
    out
}

/// Vision summary card (`/api/vision`).
pub fn render_vision(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let degraded = if b(&d["degraded"]) {
        "<span class='err'>degraded — snapshot fallback</span> "
    } else {
        ""
    };
    let feed = arr(&d["feed_items"]);
    let mut out = format!(
        "{degraded}<div>Vision rev <kbd>{}</kbd> · git <span class='dim'>{}</span> · updated <span class='dim'>{}</span></div><div class='dim'>nodes {} · edges {} · next <kbd>{}</kbd> · last closed <kbd>{}</kbd></div>",
        u(&d["revision"]),
        esc(&s(&d["git_head"])),
        esc(&s(&d["updated_at"])),
        u(&d["nodes_count"]),
        u(&d["edges_count"]),
        esc(&s(&d["next_sprint"])),
        esc(&s(&d["last_sprint_closed"])),
    );
    if feed.is_empty() {
        out.push_str(&empty_html("vision feed"));
        return out;
    }
    let rows: Vec<Vec<String>> = feed
        .iter()
        .map(|i| {
            let st = s(&i["status"]);
            vec![
                format!("<kbd>{}</kbd>", esc(&s(&i["id"]))),
                format!(
                    "<span class='{}'>{}</span>",
                    if st == "closed" { "ok" } else { "warn" },
                    esc(&st)
                ),
                esc(&s(&i["title"])),
            ]
        })
        .collect();
    out.push_str(&tab(&["id", "status", "title"], rows));
    out
}

/// Vision map card (`/api/vision/map`).
pub fn render_vision_map(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let layers = arr(&d["layers"]);
    let kinds = arr(&d["edge_kinds"]);
    let mut cols = String::new();
    for l in &layers {
        let id = s(&l["id"]);
        cols.push_str(&format!(
            "<div class='vmap-col' data-map-layer='{id}' style='cursor:pointer' title='filter map by {id}'><div class='vmap-l'>{} · {}</div><div class='dim' style='font-size:11px'>nodes {} · edges {}</div></div>",
            esc(&id),
            esc(&s(&l["name"])),
            u(&l["node_count"]),
            u(&l["edges_from"]),
        ));
    }
    let kinds_html = if kinds.is_empty() {
        "<span class='dim'>—</span>".into()
    } else {
        kinds
            .iter()
            .map(|e| format!("<kbd>{}</kbd>×{}", esc(&s(&e["kind"])), u(&e["count"])))
            .collect::<Vec<_>>()
            .join(" ")
    };
    format!(
        "<div class='dim'>rev <kbd>{}</kbd> · nodes {} · edges {} · <a href='assets/vision.svg' target='_blank' style='font-size:12px'>open vision.svg ↗</a></div><img src='assets/vision.svg' alt='galaxy vision map' style='width:100%;margin:8px 0;border:1px solid var(--line);border-radius:8px;background:var(--panel2)'><div class='vmap'>{cols}</div><div class='dim' style='margin-top:6px'>edge kinds: {kinds_html}</div><div style='margin-top:8px'><input id='nodeSearchQ' type='text' placeholder='search nodes (id / label / path)'><button type='button' data-action='node-search' style='margin-top:6px'>Search</button></div><div id='b-node-search' style='margin-top:8px'></div>",
        u(&d["revision"]),
        u(&d["nodes_count"]),
        u(&d["edges_count"]),
    )
}

/// Doc preview card (`/api/vision/doc-preview`).
pub fn render_doc_preview(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let n = &d["node"];
    if n.get("id").and_then(Value::as_str).unwrap_or("").is_empty() {
        return empty_html("doc preview");
    }
    let sections = arr(&n["sections"]);
    let sec = if sections.is_empty() {
        String::new()
    } else {
        format!(
            "<div style='margin-top:4px'>sections: {}</div>",
            sections
                .iter()
                .map(|s| format!("<kbd>{}</kbd>", esc(s.as_str().unwrap_or(""))))
                .collect::<Vec<_>>()
                .join(" ")
        )
    };
    let links_out = arr(&d["links_out"]);
    let links_in = arr(&d["links_in"]);
    let row = |links: &[Value]| -> String {
        tab(
            &["kind", "target"],
            links
                .iter()
                .map(|l| {
                    vec![
                        format!("<kbd>{}</kbd>", esc(&s(&l["kind"]))),
                        format!(
                            "<kbd>{}</kbd> <span class='dim'>{}</span>",
                            esc(&s(&l["node"]["id"])),
                            esc(&s(&l["node"]["label"]))
                        ),
                    ]
                })
                .collect(),
        )
    };
    format!(
        "<div><kbd>{}</kbd> · <span class='dim'>{}</span></div><div class='dim'>{}</div><div class='dim'>{}</div>{sec}<div style='margin-top:6px' class='dim'>links out {} · links in {}</div><details style='margin-top:6px'><summary>out</summary>{}</details><details style='margin-top:6px'><summary>in</summary>{}</details>",
        esc(&s(&n["id"])),
        esc(&s(&n["layer"])),
        esc(&s(&n["label"])),
        esc(&s(&n["path"])),
        links_out.len(),
        links_in.len(),
        row(&links_out),
        row(&links_in),
    )
}

/// Vision sync status card (read-only; Resync button stays in the shell).
pub fn render_vision_sync(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let drift = arr(&d["drift"]);
    let drift_html = if drift.is_empty() {
        "<span class='ok'>drift ok</span>".to_string()
    } else {
        format!("<span class='err'>drift {}</span>", drift.len())
    };
    format!(
        "<div>rev <kbd>{}</kbd> · git <span class='dim'>{}</span> · {drift_html}</div><div class='dim'>nodes {} · edges {} · feed {}</div><div class='dim' style='font-size:11px'>{}</div>",
        u(&d["revision"]),
        esc(&s(&d["git_head"])),
        u(&d["nodes_count"]),
        u(&d["edges_count"]),
        u(&d["feed_items"]),
        esc(&s(&d["synced_at"])),
    )
}

/// Box preview status (file render stays on `GET /api/preview` via the shell).
pub fn render_preview(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let path = s(&d["path"]);
    if path.is_empty() {
        return empty_html("preview");
    }
    let ext = s(&d["extension"]);
    let ext_bit = if ext.is_empty() {
        String::new()
    } else {
        format!(" · {}", esc(&ext))
    };
    format!(
        "<div class='dim'>preview <kbd>{}</kbd>{ext_bit}</div>",
        esc(&path)
    )
}

/// SLI terminal status (command run stays on `POST /api/terminal` via the shell).
pub fn render_terminal(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let n = arr(&d["whitelist"]).len();
    if n == 0 {
        return empty_html("terminal");
    }
    format!("<div class='dim'>whitelist {n} commands · POST /api/terminal</div>")
}

/// Sprint focus status (SVG stays on `GET /api/vision/sprint-focus.svg`).
pub fn render_sprint_focus(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let active = s(&d["active_sprint"]);
    if active.is_empty() {
        return empty_html("sprint focus");
    }
    format!(
        "<div>focus <kbd>{}</kbd> · <span class='dim'>GET /api/vision/sprint-focus.svg</span></div>",
        esc(&active)
    )
}

/// Node-search results table (`/api/vision/node-search` → `/api/ui/card/node-search`).
pub fn render_node_search(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let results = arr(&d["results"]);
    let total = u(&d["total_matches"]);
    let layer = s(&d["layer"]);
    let layer_bit = if layer.is_empty() {
        String::new()
    } else {
        format!(" · layer <kbd>{}</kbd>", esc(&layer))
    };
    let rows: Vec<Vec<String>> = results
        .iter()
        .map(|r| {
            let id = s(&r["id"]);
            vec![
                format!("<kbd>{}</kbd>", esc(&id)),
                esc(&s(&r["layer"])),
                u(&r["links_out"]).to_string(),
                u(&r["links_in"]).to_string(),
                format!("<span class='dim'>{}</span>", esc(&s(&r["label"]))),
                format!(
                    "<button type='button' data-open-node='{}'>open</button>",
                    esc(&id)
                ),
            ]
        })
        .collect();
    format!(
        "<div class='dim'>matches <kbd>{total}</kbd> · shown {}{layer_bit}</div>{}",
        results.len(),
        tab(&["id", "layer", "out", "in", "label", ""], rows)
    )
}

/// Render a named card's body HTML, or `None` for an unknown card name.
pub fn render_card(name: &str, d: &Value) -> Option<String> {
    match name {
        "tracker" => Some(render_tracker(d)),
        "sli" => Some(render_sli(d)),
        "toolchain" => Some(render_toolchain(d)),
        "ratio" => Some(render_ratio(d)),
        "hooks-tests" => Some(render_hooks_tests(d)),
        "hooks-bench" => Some(render_hooks_bench(d)),
        "sprint-map" => Some(render_sprint_map(d)),
        "sprint-queue" => Some(render_sprint_queue(d)),
        "sprint-progress" => Some(render_sprint_progress(d)),
        "sprint-board" => Some(render_sprint_board(d)),
        "speed-index" => Some(render_speed_index(d)),
        "rust-diagnostics" => Some(render_rust_diagnostics(d)),
        "omni" => Some(render_omni(d)),
        "health" => Some(render_health(d)),
        "products" => Some(render_products(d)),
        "fingerprints" => Some(render_fingerprints(d)),
        "sw" => Some(render_sw(d)),
        "watchdog" => Some(render_watchdog(d)),
        "usage" => Some(render_usage(d)),
        "mcp" => Some(render_mcp(d)),
        "settings" => Some(render_settings(d)),
        "telegram" => Some(render_telegram(d)),
        "tickets" => Some(render_tickets(d)),
        "update" => Some(render_update(d)),
        "ide" => Some(render_ide(d)),
        "vision" => Some(render_vision(d)),
        "vision-map" => Some(render_vision_map(d)),
        "vision-sync" => Some(render_vision_sync(d)),
        "doc-preview" => Some(render_doc_preview(d)),
        "preview" => Some(render_preview(d)),
        "terminal" => Some(render_terminal(d)),
        "sprint-focus" => Some(render_sprint_focus(d)),
        "galaxy-backdrop" => Some(render_galaxy_backdrop(d)),
        "starfield" => Some(render_starfield(d)),
        "rss-ticker" => Some(render_rss_ticker(d)),
        "gpu-mode" => Some(render_gpu_mode(d)),
        "power-menu" => Some(render_power_menu(d)),
        "panel-dock" => Some(render_panel_dock(d)),
        "fullscreen" => Some(render_fullscreen(d)),
        "node-search" => Some(render_node_search(d)),
        _ => None,
    }
}

/// Server-rendered card names (stable contract for `/api/ui/card/:name`).
pub const CARD_NAMES: [&str; 40] = [
    "tracker",
    "sli",
    "toolchain",
    "ratio",
    "hooks-tests",
    "hooks-bench",
    "sprint-map",
    "sprint-queue",
    "sprint-progress",
    "sprint-board",
    "speed-index",
    "rust-diagnostics",
    "omni",
    "health",
    "products",
    "fingerprints",
    "sw",
    "watchdog",
    "usage",
    "mcp",
    "settings",
    "telegram",
    "tickets",
    "update",
    "ide",
    "vision",
    "vision-map",
    "vision-sync",
    "doc-preview",
    "preview",
    "terminal",
    "sprint-focus",
    "galaxy-backdrop",
    "starfield",
    "rss-ticker",
    "gpu-mode",
    "power-menu",
    "panel-dock",
    "fullscreen",
    "node-search",
];

/// Galaxy backdrop card body (SVG-backed visual).
fn render_galaxy_backdrop(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let src = s(&d["src"]);
    if src.is_empty() {
        return empty_html("galaxy backdrop");
    }
    let mode = s(&d["mode"]);
    let mode = if mode.is_empty() { "dark".into() } else { mode };
    let opacity = s(&d["opacity"]);
    format!(
        "<div class='dim'>galaxy backdrop · {} · opacity {}</div><div>src <kbd>{}</kbd></div>",
        esc(&mode),
        esc(&opacity),
        esc(&src)
    )
}

/// Starfield card body (eco/fx/ms star counts).
fn render_starfield(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let eco = u(&d["eco"]);
    let fx = u(&d["fx"]);
    let ms = u(&d["ms"]);
    if eco == 0 && fx == 0 && ms == 0 {
        return empty_html("starfield");
    }
    let default = s(&d["default"]);
    let default_bit = if default.is_empty() {
        String::new()
    } else {
        format!(" · default <kbd>{}</kbd>", esc(&default))
    };
    format!(
        "<div>starfield · Eco <span class='ok'>{eco}</span> · FX <span class='ok'>{fx}</span> · Ms <span class='ok'>{ms}</span> stars{default_bit}</div>"
    )
}

/// RSS ticker body — `<li class='rss-ticker-item'>` rows, duplicated for marquee scroll.
fn render_rss_ticker(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let items = arr(&d["items"]);
    if items.is_empty() {
        return empty_html("rss ticker");
    }
    let mut lis = String::new();
    for it in &items {
        let id = {
            let raw = s(&it["id"]);
            if raw.is_empty() {
                s(&it["label"])
            } else {
                raw
            }
        };
        let title = s(&it["title"]);
        let status = s(&it["status"]);
        let cls = if status == "closed" { "closed" } else { "open" };
        let title_attr = esc(&format!("{id}: {title}"));
        lis.push_str(&format!(
            "<li class='rss-ticker-item {cls}' title='{title_attr}'><strong>{}</strong><span>{}</span></li>",
            esc(&id),
            esc(&title)
        ));
    }
    format!("{lis}{lis}")
}

/// GPU mode card body (current accelerator mode).
fn render_gpu_mode(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let mode = s(&d["mode"]);
    if mode.is_empty() {
        return empty_html("gpu mode");
    }
    let active = b(&d["active"]);
    let modes = arr(&d["modes"]);
    let modes_s = if modes.is_empty() {
        String::new()
    } else {
        let joined = modes
            .iter()
            .filter_map(Value::as_str)
            .map(esc)
            .collect::<Vec<_>>()
            .join("/");
        format!(" · {joined}")
    };
    format!(
        "<div>gpu mode <span class='ok'>{}</span> {}{modes_s}</div>",
        esc(&mode),
        if active { "· active" } else { "· idle" },
    )
}

/// Power menu card body (header actions).
fn render_power_menu(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let actions = arr(&d["actions"]);
    if actions.is_empty() {
        return empty_html("power menu");
    }
    let chips: Vec<String> = actions
        .iter()
        .map(|a| format!("<kbd>{}</kbd>", esc(&s(&a["label"]))))
        .collect();
    let level = s(&d["level"]);
    format!(
        "<div>power <span class='ok'>{}</span> · {}</div>",
        esc(&level),
        chips.join(" ")
    )
}

/// Panel dock card body (docked panel list).
fn render_panel_dock(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let panels = arr(&d["panels"]);
    if panels.is_empty() {
        return empty_html("panel dock");
    }
    let names: Vec<String> = panels
        .iter()
        .filter_map(Value::as_str)
        .map(|p| format!("<kbd>{}</kbd>", esc(p)))
        .collect();
    format!("<div>panel dock · {}</div>", names.join(" "))
}

/// Fullscreen card body (fullscreen toggle state).
fn render_fullscreen(d: &Value) -> String {
    if let Some(msg) = not_ok(d) {
        return err_html(&msg);
    }
    let active = b(&d["active"]);
    let label = s(&d["label"]);
    let label = if label.is_empty() {
        "fullscreen".into()
    } else {
        label
    };
    format!(
        "<div>{} <span class='{}'>{}</span></div>",
        esc(&label),
        if active { "ok" } else { "dim" },
        if active { "on" } else { "off" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn esc_matches_js_helper() {
        assert_eq!(esc("<a & b>"), "&lt;a &amp; b&gt;");
        assert_eq!(esc("plain"), "plain");
        assert_eq!(esc(""), "");
        assert_eq!(esc("\"q\""), "\"q\"");
    }

    #[test]
    fn tab_renders_empty_fallback_row() {
        let html = tab(&["a", "b"], Vec::new());
        assert!(html.contains("<table><tr><th>a</th><th>b</th></tr>"));
        assert!(html.contains("<td><span class='dim'>—</span></td>"));
    }

    #[test]
    fn tab_renders_rows() {
        let html = tab(
            &["k", "v"],
            vec![vec!["<kbd>x</kbd>".to_string(), "y".to_string()]],
        );
        assert_eq!(
            html,
            "<table><tr><th>k</th><th>v</th></tr><tr><td><kbd>x</kbd></td><td>y</td></tr></table>"
        );
    }

    #[test]
    fn bar_clamps_and_formats() {
        let html = bar(120.0);
        assert!(html.contains("width:100%"));
        assert!(html.contains("100% closed"));
        let low = bar(-5.0);
        assert!(low.contains("width:0%"));
    }

    #[test]
    fn render_tracker_uses_escaping_and_counts() {
        let d = serde_json::json!({
            "sprints": { "open": ["A"], "closed": [], "total": 1, "next": "<next>" },
            "records": [{ "kind": "k", "label": "<l>", "status": "open", "at": "t" }]
        });
        let html = render_tracker(&d);
        assert!(html.contains("open 1 · closed 0 · total 1"));
        assert!(html.contains("&lt;next&gt;"));
        assert!(html.contains("&lt;l&gt;"));
    }

    #[test]
    fn render_ratio_error_when_not_ok() {
        let d = serde_json::json!({ "ok": false, "error": "missing rust_ratio.json" });
        let html = render_ratio(&d);
        assert!(html.contains("missing rust_ratio.json"));
        assert!(html.contains("err"));
    }

    #[test]
    fn render_ratio_empty_by_category_markers() {
        let d = serde_json::json!({
            "ok": true, "meets_min_ratio": true, "rust_ratio_pct": 96.7,
            "formal_band_min": 0.95, "rust_loc": 1000, "non_rust_product_loc": 40,
            "product_loc_total": 1040, "by_category": {}
        });
        let html = render_ratio(&d);
        assert!(html.contains("Rust ratio"));
        assert!(html.contains("by-category — no data"));
    }

    #[test]
    fn render_tracker_empty_records_markers() {
        let d = serde_json::json!({
            "sprints": { "open": [], "closed": [], "total": 0, "next": "" },
            "records": []
        });
        let html = render_tracker(&d);
        assert!(html.contains("open 0 · closed 0 · total 0"));
        assert!(html.contains("tracker records — no data"));
        let bad = serde_json::json!({ "ok": false, "error": "tracker broken" });
        assert!(render_tracker(&bad).contains("tracker broken"));
    }

    #[test]
    fn render_sli_empty_and_error_markers() {
        let d = serde_json::json!({
            "catalog": { "used_count": 0, "unused_count": 0, "entries": [] }
        });
        let html = render_sli(&d);
        assert!(html.contains("sli entries — no data"));
        let bad = serde_json::json!({ "ok": false, "error": "sli broken" });
        assert!(render_sli(&bad).contains("sli broken"));
    }

    #[test]
    fn render_toolchain_empty_markers() {
        let d = serde_json::json!({ "entries": [] });
        let html = render_toolchain(&d);
        assert!(html.contains("toolchain entries — no data"));
        let bad = serde_json::json!({ "ok": false, "error": "toolchain broken" });
        assert!(render_toolchain(&bad).contains("toolchain broken"));
    }

    #[test]
    fn render_hooks_error_guards() {
        let bad = serde_json::json!({ "ok": false, "error": "hooks broken" });
        assert!(render_hooks_tests(&bad).contains("hooks broken"));
        assert!(render_hooks_bench(&bad).contains("hooks broken"));
    }

    #[test]
    fn render_sprint_queue_marks_active_next_closed() {
        let d = serde_json::json!({
            "ok": true, "revision": "472", "next_sprint": "PH-S1839",
            "last_sprint_closed": "PH-S1838", "active_sprint": "PH-S1839", "open_count": 1,
            "planned": [
                { "id": "PH-S1839", "status": "open", "category": "GSV canon", "title": "a" },
                { "id": "PH-S1848", "status": "planned", "category": "GSV_ROLES", "title": "b" }
            ]
        });
        let html = render_sprint_queue(&d);
        assert!(html.contains("class='squeue open'>PH-S1839"));
        assert!(html.contains("class='squeue closed'>PH-S1848"));
        assert!(html.contains("planned (2)"));
    }

    #[test]
    fn render_hooks_tests_marks_diagnostics_and_bins() {
        let d = serde_json::json!({
            "status": "ready", "test_bins": ["poolai-abc", "test_xyz"],
            "diagnostics": { "warnings": 3, "errors": 1, "ok": false, "recorded_at": "t" }
        });
        let html = render_hooks_tests(&d);
        assert!(html.contains("test bins: 2"));
        assert!(html.contains("class='err'>warnings 3 · errors 1"));
        assert!(html.contains("poolai-abc"));
        let nodiag =
            serde_json::json!({ "status": "no-artifacts", "test_bins": [], "diagnostics": null });
        let html2 = render_hooks_tests(&nodiag);
        assert!(!html2.contains("diagnostics"));
        assert!(html2.contains("no-artifacts"));
    }

    #[test]
    fn render_hooks_bench_marks_speed_index() {
        let d = serde_json::json!({
            "status": "ready", "criterion_dirs": ["hash1", "hash2"],
            "speed_index": { "test_ci_wall_secs": 12.5, "test_ci_ok": true, "recorded_at": "t" }
        });
        let html = render_hooks_bench(&d);
        assert!(html.contains("criterion dirs: 2"));
        assert!(html.contains("class='ok'>12.5s</span>"));
        assert!(html.contains("hash1"));
    }

    #[test]
    fn render_sprint_map_renders_modules_kinds_links() {
        let d = serde_json::json!({
            "ok": true, "revision": "472", "nodes_count": 3, "next_sprint": "PH-S1839",
            "last_sprint_closed": "PH-S1838",
            "modules": [{ "id": "GSV", "layer": "core", "targets": 12 }],
            "kinds": [{ "kind": "depends_on", "count": 2 }],
            "links": [{ "kind": "depends_on", "from": { "id": "a" }, "to": { "id": "<b>" } }]
        });
        let html = render_sprint_map(&d);
        assert!(html.contains("nodes 3"));
        assert!(html.contains("&lt;b&gt;"));
        assert!(html.contains("<span class='ok'>12</span>"));
        assert!(html.contains("links (1)"));
        let bad = serde_json::json!({ "ok": false, "error": "boom" });
        assert!(render_sprint_map(&bad).contains("boom"));
    }

    #[test]
    fn render_sprint_board_renders_columns_and_bar() {
        let d = serde_json::json!({
            "ok": true, "revision": "472", "next_sprint": "PH-S1839", "active_sprint": "PH-S1839",
            "open_count": 1, "closed_count": 2, "total": 3, "progress_pct": 66.0,
            "columns": [
                { "name": "done", "count": 2, "entries": [
                    { "id": "PH-S1838", "status": "closed", "title": "x" },
                    { "id": "PH-S1839", "status": "closed", "title": "y" }
                ] }
            ]
        });
        let html = render_sprint_board(&d);
        assert!(html.contains("class='squeue open'>PH-S1839"));
        assert!(html.contains("class='squeue closed'>PH-S1838"));
        assert!(html.contains("66% closed"));
        assert!(html.contains("done (2)"));
        let bad = serde_json::json!({ "ok": false, "error": "nope" });
        assert!(render_sprint_board(&bad).contains("nope"));
    }

    #[test]
    fn render_card_dispatch_known_and_unknown() {
        let d = serde_json::json!({ "ok": true, "revision": "472", "open_count": 0, "closed_count": 0, "planned_count": 0, "total": 0, "progress_pct": 0.0, "layers": [] });
        assert!(render_card("sprint-progress", &d).is_some());
        assert!(render_card("hooks-tests", &d).is_some());
        assert!(render_card("omni", &d).is_some());
        assert!(render_card("galaxy-backdrop", &d).is_some());
        assert!(render_card("starfield", &d).is_some());
        assert!(render_card("rss-ticker", &d).is_some());
        assert!(render_card("gpu-mode", &d).is_some());
        assert!(render_card("power-menu", &d).is_some());
        assert!(render_card("panel-dock", &d).is_some());
        assert!(render_card("fullscreen", &d).is_some());
        assert!(render_card("node-search", &d).is_some());
        assert!(render_card("mcp", &d).is_some());
        assert!(render_card("settings", &d).is_some());
        assert!(render_card("telegram", &d).is_some());
        assert!(render_card("tickets", &d).is_some());
        assert!(render_card("products", &d).is_some());
        assert!(render_card("fingerprints", &d).is_some());
        assert!(render_card("nope", &d).is_none());
        assert_eq!(CARD_NAMES.len(), 40);
        assert!(render_card("usage", &d).is_some());
        assert!(render_card("sw", &d).is_some());
        assert!(render_card("watchdog", &d).is_some());
        assert!(render_card("health", &d).is_some());
        assert!(render_card("ide", &d).is_some());
        assert!(render_card("vision", &d).is_some());
        assert!(render_card("preview", &d).is_some());
        assert!(render_card("terminal", &d).is_some());
        assert!(render_card("sprint-focus", &d).is_some());
        let mcp = render_mcp(&serde_json::json!({
            "ok": true,
            "name": "gsv_mcp_openbot",
            "protocol": "2025-03-26",
            "stdio": "gsv-mcp",
            "stdio_live": "target/live/gsv-mcp.exe",
            "http": "/mcp",
            "http_url": "http://127.0.0.1:9999/mcp",
            "version": "0.159.0",
            "crate_version": "0.161.0",
            "version_lag": true,
            "sandbox": "S:/rust/GSV",
            "tool_count": 2,
            "tools": ["gsv_health", "gsv_update"],
            "resource_count": 1,
            "resources": ["gsv://vision/manifest"],
            "prompt_count": 1,
            "prompts": ["gsv_status"],
            "logging": true,
            "completions": true,
            "subscribe": true,
            "subscription_count": 2,
            "sse": true,
            "streamable": true,
            "sessions": true,
            "session_count": 3,
            "log_level": "info"
        }));
        assert!(mcp.contains("gsv_mcp_openbot"), "{mcp}");
        assert!(mcp.contains("tools 2"), "{mcp}");
        assert!(mcp.contains("resources 1"), "{mcp}");
        assert!(mcp.contains("prompts 1"), "{mcp}");
        assert!(mcp.contains("logging <kbd>info</kbd>"), "{mcp}");
        assert!(mcp.contains("completions"), "{mcp}");
        assert!(mcp.contains("subscribe <kbd>2</kbd>"), "{mcp}");
        assert!(mcp.contains(" · sse"), "{mcp}");
        assert!(mcp.contains("sessions <kbd>3</kbd>"), "{mcp}");
        assert!(mcp.contains("<kbd>gsv_health</kbd>"), "{mcp}");
        assert!(mcp.contains("<kbd>gsv://vision/manifest</kbd>"), "{mcp}");
        assert!(mcp.contains("<kbd>gsv_status</kbd>"), "{mcp}");
        assert!(mcp.contains("stdio <kbd>gsv-mcp</kbd>"), "{mcp}");
        assert!(
            mcp.contains("live <kbd>target/live/gsv-mcp.exe</kbd>"),
            "{mcp}"
        );
        assert!(mcp.contains("ver <kbd>0.159.0</kbd>"), "{mcp}");
        assert!(mcp.contains("crate <kbd>0.161.0</kbd>"), "{mcp}");
        assert!(mcp.contains("lag"), "{mcp}");
        assert!(
            mcp.contains("url <kbd>http://127.0.0.1:9999/mcp</kbd>"),
            "{mcp}"
        );
        assert!(mcp.contains("sandbox <kbd>S:/rust/GSV</kbd>"), "{mcp}");
        assert!(render_mcp(&serde_json::json!({ "ok": false, "error": "down" })).contains("down"));
        assert!(render_mcp(&serde_json::json!({})).contains("mcp — no data"));
        let settings = render_settings(&serde_json::json!({
            "ok": true,
            "token_set": false,
            "source": "none",
            "godfather": { "channel_id": "", "allowed_user_ids": [] },
            "workflows": { "enabled": [] }
        }));
        assert!(settings.contains("settings — no data"), "{settings}");
        assert!(
            settings.contains("data-action='settings-save'"),
            "{settings}"
        );
        assert!(!settings.contains("bot_token"), "{settings}");
        assert!(render_settings(&serde_json::json!({ "ok": false, "error": "io" })).contains("io"));
        let telegram = render_telegram(&serde_json::json!({
            "ok": true,
            "token_set": false,
            "channel_id": "",
            "polling": false,
            "dry_run": true
        }));
        assert!(telegram.contains("telegram — no data"), "{telegram}");
        assert!(!telegram.contains("bot_token"), "{telegram}");
        let tg_ok = render_telegram(&serde_json::json!({
            "ok": true,
            "token_set": true,
            "channel_id": "-100",
            "polling": false,
            "dry_run": true,
            "last_ticket_id": "t-174"
        }));
        assert!(tg_ok.contains("t-174"), "{tg_ok}");
        assert!(tg_ok.contains("gsv_telegram_ticket"), "{tg_ok}");
        assert!(render_telegram(&serde_json::json!({ "ok": false, "error": "io" })).contains("io"));
        let tickets = render_tickets(&serde_json::json!({ "ok": true, "tickets": [] }));
        assert!(tickets.contains("tickets — no data"), "{tickets}");
        assert!(tickets.contains("open tickets are the board"), "{tickets}");
        assert!(tickets.contains("session lines"), "{tickets}");
        assert!(tickets.contains("data-action='tickets-hook'"), "{tickets}");
        assert!(tickets.contains("data-action='tickets-bench'"), "{tickets}");
        assert!(
            tickets.contains("data-action='tickets-create'"),
            "{tickets}"
        );
        assert!(render_tickets(&serde_json::json!({ "ok": false, "error": "io" })).contains("io"));
    }

    #[test]
    fn render_omni_renders_summary_recommended_providers_models() {
        let d = serde_json::json!({
            "providers": [
                { "id": "openai", "name": "OpenAI", "enabled": true, "key_set": true, "base_url": "https://api.openai.com/v1" },
                { "id": "free-h", "name": "Free Host", "enabled": false, "key_set": false, "base_url": "" }
            ],
            "models": [
                { "id": "gpt-5.2", "provider": "openai", "context_window": 400000, "max_output": 128000, "free": false, "recommended": true }
            ],
            "recommended": [
                { "id": "gpt-5.2", "provider": "openai" }
            ],
            "routing": { "default_provider": "openai", "auto": true }
        });
        let html = render_omni(&d);
        assert!(html.contains("providers 2 · models 1"));
        assert!(html.contains("default <kbd>openai</kbd>"));
        assert!(html.contains("recommended: <kbd>gpt-5.2</kbd>"));
        assert!(html.contains("Providers (2)"));
        assert!(html.contains("Models (1)"));
        assert!(html.contains("400,000"));
        assert!(html.contains("128,000"));
        assert!(html.contains("<span class='ok'>★</span>"));
        assert!(html.contains("<span class='err'>off</span>"));
        assert!(html.contains("no key"));
    }

    #[test]
    fn render_omni_warns_when_selected_is_not_gsv() {
        let d = serde_json::json!({
            "providers": [
                { "id": "openai", "name": "OpenAI", "enabled": true, "key_set": true, "base_url": "https://api.openai.com/v1" }
            ],
            "models": [],
            "recommended": [],
            "routing": { "default_provider": "openai", "auto": true },
            "account_product": "gsv",
            "selected_product": "omniroute"
        });
        let html = render_omni(&d);
        assert!(html.contains("omniroute"), "{html}");
        assert!(html.contains("gsv"), "{html}");
        assert!(
            html.contains("class='err'") || html.contains("class='warn'"),
            "{html}"
        );
    }

    #[test]
    fn render_omni_empty_state_marks_unavailable() {
        let d = serde_json::json!({
            "providers": [], "models": [], "recommended": [],
            "routing": { "default_provider": "", "auto": false }
        });
        let html = render_omni(&d);
        assert!(html.contains("providers 0 · models 0"));
        assert!(html.contains("omni unavailable"));
    }

    #[test]
    fn layout_assigns_each_dashboard_card_once() {
        let ids = layout_card_ids();
        let mut seen = std::collections::BTreeSet::new();
        for id in &ids {
            assert!(seen.insert(*id), "duplicate layout card {id}");
        }
        for chrome in CHROME_CARDS {
            assert!(
                !seen.contains(chrome),
                "chrome card {chrome} must not be in UI_GROUPS"
            );
        }
        for name in CARD_NAMES {
            if CHROME_CARDS.contains(&name) {
                continue;
            }
            assert!(
                seen.contains(name),
                "functional card {name} missing from UI_GROUPS"
            );
        }
        for id in &ids {
            assert!(
                CARD_NAMES.contains(id),
                "layout card {id} missing from CARD_NAMES"
            );
        }
        let html = render_nav(DEFAULT_GROUP);
        assert!(html.contains("data-group='sprint'"));
        assert!(html.contains("data-card-jump='sprint-queue'"));
        assert!(html.contains("class='nav-tab active'"));
        assert!(
            !html.contains("<nav"),
            "inner HTML only — #shellNav already wraps"
        );
        let wire = layout_wire();
        assert_eq!(wire["ok"], true);
        assert_eq!(wire["default_group"], DEFAULT_GROUP);
        assert_eq!(wire["groups"].as_array().map(|a| a.len()), Some(4));
        let chrome = wire["chrome"].as_array().expect("chrome");
        assert_eq!(chrome.len(), CHROME_CARDS.len());
        assert_eq!(chrome[0], "galaxy-backdrop");
        assert_eq!(chrome[chrome.len() - 1], "node-search");
        let nav_html = wire["html"].as_str().expect("html");
        assert!(nav_html.contains("data-card-jump='health'"), "{nav_html}");
        assert!(nav_html.contains("data-group='ops'"), "{nav_html}");
        let header = wire["header"].as_str().expect("header");
        assert!(header.contains("data-action='gpu-cycle'"), "{header}");
        assert!(header.contains("data-action='power-toggle'"), "{header}");
        assert!(header.contains("id='powerMenu'"), "{header}");
        assert!(
            !header.contains("<header"),
            "inner HTML only — #headerActions already wraps"
        );
    }

    #[test]
    fn render_header_and_node_search_fragments() {
        let header = render_header();
        assert!(header.contains("data-action='resync'"), "{header}");
        assert!(header.contains("data-action='power-soft'"), "{header}");
        assert!(header.contains("aria-haspopup='true'"), "{header}");
        let empty = render_node_search(&serde_json::json!({
            "ok": true, "total_matches": 0, "results": []
        }));
        assert!(empty.contains("matches <kbd>0</kbd>"), "{empty}");
        assert!(empty.contains("<span class='dim'>—</span>"), "{empty}");
        let html = render_node_search(&serde_json::json!({
            "ok": true,
            "total_matches": 2,
            "layer": "L0",
            "results": [{
                "id": "galaxy_grid",
                "label": "Galaxy",
                "layer": "L0",
                "links_out": 3,
                "links_in": 1
            }]
        }));
        assert!(
            html.contains("matches <kbd>2</kbd> · shown 1 · layer <kbd>L0</kbd>"),
            "{html}"
        );
        assert!(html.contains("<kbd>galaxy_grid</kbd>"), "{html}");
        assert!(html.contains("data-open-node='galaxy_grid'"), "{html}");
        assert!(html.contains("<th>id</th>"), "{html}");
    }

    #[test]
    fn chrome_renderers_error_empty_and_rss_items() {
        let err = serde_json::json!({ "ok": false, "error": "stand-error" });
        for name in CHROME_CARDS {
            let html = render_card(name, &err).expect(name);
            assert!(
                html.contains("<span class='err'>stand-error</span>"),
                "{name} error: {html}"
            );
        }
        assert!(
            render_rss_ticker(&serde_json::json!({ "ok": true, "items": [] }))
                .contains("rss ticker — no data")
        );
        let rss = render_rss_ticker(&serde_json::json!({
            "ok": true,
            "items": [{ "id": "PH-S1939", "title": "chrome", "status": "open" }]
        }));
        assert!(rss.contains("rss-ticker-item open"), "{rss}");
        assert!(rss.contains("<strong>PH-S1939</strong>"), "{rss}");
        assert_eq!(
            rss.matches("rss-ticker-item").count(),
            2,
            "marquee duplicates items: {rss}"
        );
        let stars = render_starfield(&serde_json::json!({
            "ok": true, "eco": 48, "fx": 160, "ms": 96, "default": "FX"
        }));
        assert!(stars.contains("Eco <span class='ok'>48</span>"), "{stars}");
        assert!(stars.contains("default <kbd>FX</kbd>"), "{stars}");
        assert!(
            render_starfield(&serde_json::json!({ "ok": true })).contains("starfield — no data")
        );
        let gpu = render_gpu_mode(&chrome_gpu_wire());
        assert!(gpu.contains("gpu mode <span class='ok'>fx</span>"), "{gpu}");
        assert!(gpu.contains("eco/fx/ms"), "{gpu}");
        let power = render_power_menu(&chrome_power_wire());
        assert!(power.contains("Soft sync Vision"), "{power}");
        assert!(render_panel_dock(&chrome_dock_wire()).contains("panel dock — no data"));
        assert!(render_fullscreen(&chrome_fullscreen_wire()).contains("off"));
        let galaxy = render_galaxy_backdrop(&serde_json::json!({
            "ok": true, "mode": "dark", "src": "/api/vision/galaxy.svg", "opacity": "0.15"
        }));
        assert!(galaxy.contains("galaxy.svg"), "{galaxy}");
        assert!(
            render_galaxy_backdrop(&serde_json::json!({ "ok": true, "src": "" }))
                .contains("galaxy backdrop — no data")
        );
    }

    #[test]
    fn render_health_and_ide_error_and_empty() {
        let err = serde_json::json!({ "ok": false, "error": "stand-error" });
        assert!(render_health(&err).contains("<span class='err'>stand-error</span>"));
        assert!(render_ide(&err).contains("<span class='err'>stand-error</span>"));
        assert!(render_vision(&err).contains("<span class='err'>stand-error</span>"));
        let empty_ide = render_ide(&serde_json::json!({ "sessions": [], "preview": [] }));
        assert!(empty_ide.contains("ide sessions — no data"), "{empty_ide}");
        let empty_doc = render_doc_preview(&serde_json::json!({ "ok": true, "node": {} }));
        assert!(empty_doc.contains("doc preview — no data"), "{empty_doc}");
    }
}
