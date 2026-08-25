//! GSV UI fragment contracts — Rust integration tests for `/api/ui/card/:name`.
//!
//! Server-rendered card bodies (band 120) must render HTML markers that match
//! the wire data, and unknown cards must 404. Uses `tower::ServiceExt::oneshot`
//! against the axum router (no port binding).

use std::path::PathBuf;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use gsv::boxes::ui::{bar, esc, render_card, tab, CARD_NAMES};
use gsv::server::router;
use gsv::AppState;
use serde_json::Value;
use tokio::sync::broadcast;
use tower::ServiceExt;

fn app() -> (axum::Router, AppState) {
    let (tx, _rx) = broadcast::channel(64);
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let state = AppState::new(Some(repo_root), None, tx);
    (router(state.clone()), state)
}

async fn get_card(app: &axum::Router, name: &str) -> (StatusCode, Value) {
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/ui/card/{name}"))
                .method(Method::GET)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("body");
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

/// Fetch the served single-page UI HTML (`/`) as text.
async fn get_index_html(app: &axum::Router) -> String {
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/")
                .method(Method::GET)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("body");
    String::from_utf8(bytes.to_vec()).expect("utf8")
}

#[tokio::test]
async fn ui_card_unknown_name_is_404() {
    let (app, _state) = app();
    let (status, json) = get_card(&app, "does-not-exist").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(json["ok"], false);
    assert!(json["error"].is_string());
}

#[tokio::test]
async fn ui_card_tracker_renders_table_markers() {
    let (app, _state) = app();
    let (status, json) = get_card(&app, "tracker").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
    assert_eq!(json["card"], "tracker");
    let html = json["html"].as_str().expect("html");
    assert!(html.contains("<table><tr><th>kind</th><th>label</th><th>status</th><th>at</th></tr>"));
    assert!(html.contains("next <kbd>"));
}

#[tokio::test]
async fn ui_card_ratio_renders_band_or_missing_store() {
    let (app, _state) = app();
    let (status, json) = get_card(&app, "ratio").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
    let html = json["html"].as_str().expect("html");
    // Without a stored rust_ratio.json the server renders ok:false (read error);
    // with one it renders the band summary.
    if html.contains("rust_ratio.json") {
        assert!(html.contains("<span class='err'>"));
    } else {
        assert!(html.contains("Rust ratio"));
        assert!(html.contains("band min"));
    }
}

#[tokio::test]
async fn ui_card_all_registered_names_respond_ok() {
    let (app, _state) = app();
    for name in CARD_NAMES {
        let (status, json) = get_card(&app, name).await;
        assert_eq!(status, StatusCode::OK, "{name} status");
        assert_eq!(json["ok"], true, "{name} ok");
        assert!(json["html"].is_string(), "{name} html");
    }
}

#[tokio::test]
async fn ui_card_html_is_escaped_not_markup_injected() {
    let (app, _state) = app();
    let (status, json) = get_card(&app, "sli").await;
    assert_eq!(status, StatusCode::OK);
    let html = json["html"].as_str().expect("html");
    assert!(!html.contains("<script"));
}

#[test]
fn ui_helpers_match_js_semantics() {
    assert_eq!(esc("<x&y>"), "&lt;x&amp;y&gt;");
    assert!(tab(&["a"], Vec::new()).contains("<span class='dim'>—</span>"));
    assert!(bar(50.0).contains("width:50%"));
    assert!(bar(120.0).contains("width:100%"));
    assert_eq!(CARD_NAMES.len(), 42);
}

#[tokio::test]
async fn ui_card_omni_renders_summary_providers_models() {
    let (app, _state) = app();
    let (status, json) = get_card(&app, "omni").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
    assert_eq!(json["card"], "omni");
    let html = json["html"].as_str().expect("html");
    assert!(html.contains("providers "), "summary: {html}");
    assert!(html.contains("default <kbd>"));
    assert!(html.contains("<summary>Providers ("));
    assert!(html.contains("<summary>Models ("));
    assert!(html.contains("<th>id</th><th>name</th><th>state</th><th>key</th><th>base_url</th>"));
}

#[tokio::test]
async fn ui_card_mcp_renders_openbot_tools() {
    let (app, _state) = app();
    let (status, json) = get_card(&app, "mcp").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
    assert_eq!(json["card"], "mcp");
    let html = json["html"].as_str().expect("html");
    assert!(html.contains("gsv_mcp_openbot"), "name: {html}");
    assert!(html.contains("stdio <kbd>gsv-mcp</kbd>"), "stdio: {html}");
    assert!(
        html.contains("live <kbd>target/live/gsv-mcp"),
        "stdio live: {html}"
    );
    assert!(html.contains("http <kbd>/mcp</kbd>"), "http: {html}");
    assert!(
        html.contains("url <kbd>http://127.0.0.1:9999/mcp</kbd>"),
        "http_url: {html}"
    );
    assert!(html.contains("sandbox <kbd>"), "sandbox: {html}");
    assert!(
        html.contains(&format!("ver <kbd>{}</kbd>", env!("CARGO_PKG_VERSION"))),
        "version: {html}"
    );
    assert!(html.contains("<kbd>gsv_health</kbd>"), "tool: {html}");
    assert!(html.contains("<th>tool</th>"), "table: {html}");
    assert!(html.contains("resources "), "counts: {html}");
    assert!(html.contains("prompts "), "counts: {html}");
    assert!(html.contains("gsv://vision/manifest"), "resource: {html}");
    assert!(html.contains("<kbd>gsv_status</kbd>"), "prompt: {html}");
    assert!(html.contains("logging <kbd>info</kbd>"), "logging: {html}");
    assert!(html.contains("completions"), "completions: {html}");
    assert!(html.contains("subscribe <kbd>0</kbd>"), "subscribe: {html}");
    assert!(html.contains(" · sse"), "sse: {html}");
    assert!(html.contains("sessions <kbd>0</kbd>"), "sessions: {html}");
    assert!(html.contains("catalogNotify"), "catalogNotify: {html}");
}

/// The rustCards the thin JS glue fetches via `getText` (mirror of
/// `rustCards` in `GSV/ui/index.html`).
const RUST_CARDS: [&str; 33] = [
    "about",
    "health",
    "products",
    "fingerprints",
    "sw",
    "watchdog",
    "usage",
    "settings",
    "telegram",
    "tickets",
    "mcp",
    "update",
    "tracker",
    "sli",
    "toolchain",
    "hooks-tests",
    "hooks-bench",
    "ratio",
    "vision",
    "vision-map",
    "sprint-map",
    "sprint-queue",
    "sprint-progress",
    "sprint-board",
    "speed-index",
    "rust-diagnostics",
    "omni",
    "ide",
    "vision-sync",
    "doc-preview",
    "preview",
    "terminal",
    "sprint-focus",
];

#[test]
fn card_renderers_error_state_contract() {
    // Every rustCard renderer must turn `ok:false` into the canonical error
    // marker `<span class='err'>…</span>` — never a raw panic or blank body.
    let err_wire = serde_json::json!({ "ok": false, "error": "stand-error" });
    for name in RUST_CARDS {
        let html = render_card(name, &err_wire).expect(name);
        assert!(
            html.contains("<span class='err'>stand-error</span>"),
            "{name} error marker: {html}"
        );
    }
}

#[test]
fn card_renderers_empty_state_contract() {
    // Empty-table renderers must emit the canonical empty marker
    // `<div class='dim'>… — no data</div>` instead of a blank body.
    let tracker = render_card(
        "tracker",
        &serde_json::json!({
            "sprints": { "open": [], "closed": [], "total": 0, "next": "" },
            "records": []
        }),
    )
    .expect("tracker");
    assert!(tracker.contains("tracker records — no data"), "{tracker}");

    let sli = render_card(
        "sli",
        &serde_json::json!({
            "catalog": { "used_count": 0, "unused_count": 0, "entries": [] }
        }),
    )
    .expect("sli");
    assert!(sli.contains("sli entries — no data"), "{sli}");

    let toolchain =
        render_card("toolchain", &serde_json::json!({ "entries": [] })).expect("toolchain");
    assert!(
        toolchain.contains("toolchain entries — no data"),
        "{toolchain}"
    );

    let ratio = render_card(
        "ratio",
        &serde_json::json!({
            "ok": true, "meets_min_ratio": true, "rust_ratio_pct": 96.0,
            "formal_band_min": 0.95, "rust_loc": 1, "non_rust_product_loc": 0,
            "product_loc_total": 1, "by_category": {}
        }),
    )
    .expect("ratio");
    assert!(ratio.contains("ratio by-category — no data"), "{ratio}");

    let preview =
        render_card("preview", &serde_json::json!({ "ok": true, "path": "" })).expect("preview");
    assert!(preview.contains("preview — no data"), "{preview}");

    let terminal = render_card(
        "terminal",
        &serde_json::json!({ "ok": true, "whitelist": [] }),
    )
    .expect("terminal");
    assert!(terminal.contains("terminal — no data"), "{terminal}");

    let focus = render_card(
        "sprint-focus",
        &serde_json::json!({ "ok": true, "active_sprint": "" }),
    )
    .expect("sprint-focus");
    assert!(focus.contains("sprint focus — no data"), "{focus}");

    let settings = render_card(
        "settings",
        &serde_json::json!({
            "ok": true,
            "token_set": false,
            "source": "none",
            "godfather": { "channel_id": "", "allowed_user_ids": [] },
            "workflows": { "enabled": [] }
        }),
    )
    .expect("settings");
    assert!(settings.contains("settings — no data"), "{settings}");

    let telegram = render_card(
        "telegram",
        &serde_json::json!({
            "ok": true,
            "token_set": false,
            "channel_id": "",
            "polling": false,
            "dry_run": true
        }),
    )
    .expect("telegram");
    assert!(telegram.contains("telegram — no data"), "{telegram}");

    let tickets =
        render_card("tickets", &serde_json::json!({ "ok": true, "tickets": [] })).expect("tickets");
    assert!(tickets.contains("tickets — no data"), "{tickets}");
}

/// A11y contract: the served UI HTML carries axe-friendly markers — lang,
/// `role="status"` live regions, `aria-live`, `aria-label`s, image `alt`.
#[tokio::test]
async fn ui_index_a11y_markers_present() {
    let (app, _state) = app();
    let html = get_index_html(&app).await;
    assert!(html.contains("lang=\"en\""), "html lang");
    assert!(html.contains("role=\"status\""), "live status regions");
    assert!(html.contains("aria-live=\"polite\""), "aria-live polite");
    assert!(html.contains("aria-haspopup=\"true\""), "power menu popup");
    assert!(html.contains("role=\"menu\""), "power menu role");
    assert!(
        html.contains("aria-label=\"Vision controls\""),
        "controls label"
    );
    assert!(html.contains("aria-label=\"GPU mode"), "gpu aria-label");
    // JS-created card action buttons carry aria-labels in the script source.
    assert!(
        html.contains("setAttribute(\"aria-label\", \"Minimize / Restore card\")"),
        "min btn label"
    );
    assert!(
        html.contains("setAttribute(\"aria-label\", \"Fullscreen toggle card\")"),
        "max btn label"
    );
    // Every decorative/visual `<img>` has an `alt` attribute.
    assert!(
        html.contains("alt=\"\"") || html.contains("alt=''"),
        "empty alt for decorative images"
    );
    assert!(
        html.contains("alt=\"cargo test duration line chart"),
        "speed chart alt"
    );
    assert!(html.contains("alt=\"clippy stacked bars"), "diag chart alt");
    assert!(html.contains("alt=\"sprint focus map"), "focus map alt");
    assert!(html.contains("href=\"#grid\""), "skip link");
    assert!(html.contains("data-group=\"sprint\""), "grouped cards");
    assert!(html.contains("id=\"shellNav\""), "sidebar nav");
    assert!(html.contains(":focus-visible"), "focus-visible");
}

/// A11y contract: cards expose `aria-live` region so updates announce.
#[tokio::test]
async fn ui_index_vision_sync_status_is_live() {
    let (app, _state) = app();
    let html = get_index_html(&app).await;
    assert!(html.contains(
        "id=\"b-vision-sync-status\" class=\"dim\" role=\"status\" aria-live=\"polite\""
    ));
}

/// Offline-stability contract: every rustCard has a `data-card` hook and the
/// `getText` glue keeps the last-good render on fetch failure (badge, no wipe).
#[tokio::test]
async fn ui_index_cards_are_offline_stable() {
    let (app, _state) = app();
    let html = get_index_html(&app).await;
    for name in RUST_CARDS {
        assert!(
            html.contains(&format!("data-card=\"{name}\"")),
            "{name} data-card hook"
        );
    }
    // Badge element + keep-last-good path exist in the served script.
    assert!(html.contains(".card-status"), "card-status css");
    assert!(html.contains("markStatus"), "status badge helper");
    assert!(
        html.contains("keep the last-good render"),
        "offline-stable path in getText"
    );
    assert!(html.contains("innerHTML === \"…\""), "no-wipe guard");
    assert!(
        html.contains("\"preview\"")
            && html.contains("\"terminal\"")
            && html.contains("\"sprint-focus\"")
            && html.contains("\"mcp\"")
            && html.contains("\"sw\"")
            && html.contains("\"watchdog\"")
            && html.contains("\"settings\"")
            && html.contains("\"telegram\"")
            && html.contains("\"tickets\"")
            && html.contains("\"about\""),
        "rustCards includes layout ops/sprint cards"
    );
    assert!(
        html.contains("data-card-jump"),
        "sidebar chips jump to cards"
    );
    assert!(
        html.contains("api/ui/card/rss-ticker"),
        "rss ticker uses rust chrome card"
    );
    assert!(
        html.contains("href=\"/api/ui/load-palette\""),
        "live palette stylesheet"
    );
    assert!(
        html.contains("href=\"/api/ui/load-theme\""),
        "live sprint theme stylesheet"
    );
    assert!(
        html.contains("d.html") && html.contains("shellNav"),
        "layout nav HTML from layout_wire"
    );
    assert!(
        html.contains("d.header") && html.contains("headerActions"),
        "layout header HTML from layout_wire"
    );
    assert!(
        html.contains("api/ui/card/node-search"),
        "node search uses rust chrome card"
    );
}

/// Logic-audit X (band 206): the offline badge follows real outcomes —
/// `refreshMeta`/`getText` resolve `true` on success and `false` on failure,
/// and `resync` counts explicit `false` results, not just rejected promises.
/// Before the fix every wrapper swallowed errors, so `failed` was always 0
/// and each silent resync forced the badge back to «online».
#[tokio::test]
async fn ui_index_resync_offline_follows_results() {
    let (app, _state) = app();
    let html = get_index_html(&app).await;
    assert!(
        html.contains("r.status===\"rejected\"||r.value===false"),
        "failed counts false results, not just rejections"
    );
    assert!(
        html.contains("setOffline(true);return false;}}"),
        "health probe reports failure instead of swallowing"
    );
    assert!(
        html.contains("markStatus(e.message||\"offline\");return false;}}"),
        "card load reports failure instead of swallowing"
    );
    assert!(
        !html.contains(".catch(()=>setOffline(true)),30000"),
        "30s health timer has no dead rejection path"
    );
}

/// Logic-audit X (band 206): a hook click with no scenario id, no source and
/// an empty phrase guides the owner instead of silently placing band-177
/// tickets (the old blind `{source:"band",id:"177"}` fallback).
#[tokio::test]
async fn ui_index_hook_requires_input() {
    let (app, _state) = app();
    let html = get_index_html(&app).await;
    assert!(
        !html.contains("{source:\"band\",id:\"177\",from:\"solo\"}"),
        "no blind closed-band fallback in hookTickets"
    );
    assert!(
        html.contains("hook needs a scenario id, band id, plan stem, or phrase"),
        "guidance toast on empty hook click"
    );
}

/// Band 143: power menu must stack above workspace cards (P1).
#[tokio::test]
async fn ui_index_power_menu_stacks_above_workspace() {
    let (app, _state) = app();
    let html = get_index_html(&app).await;
    assert!(
        html.contains("z-index:80") || html.contains("z-index: 80"),
        "power menu must stack above cards: {html}"
    );
    assert!(
        html.contains(".power-menu") && html.contains("z-index"),
        "power-menu rule missing"
    );
    // Regression: later rule must not pin body>header to the same z-index as .workspace.
    assert!(
        !html.contains("body>header,.workspace{position:relative;z-index:2}"),
        "header must not share z-index 2 with workspace"
    );
}

/// Band 143: exclusive fullscreen + named Esc target (P3/P4).
#[tokio::test]
async fn ui_index_card_actions_use_data_action() {
    let (app, _state) = app();
    let html = get_index_html(&app).await;
    assert!(
        html.contains("data-action=\"card-fs\"")
            || html.contains("data-action='card-fs'")
            || html.contains("setAttribute(\"data-action\", \"card-fs\")"),
        "card-fs data-action missing"
    );
    assert!(
        html.contains("data-action=\"card-min\"")
            || html.contains("data-action='card-min'")
            || html.contains("setAttribute(\"data-action\", \"card-min\")"),
        "card-min data-action missing"
    );
    assert!(html.contains("function exitFullscreen"));
    assert!(html.contains("function placeFullscreen"));
    assert!(html.contains("--fs-top"));
    assert!(
        html.contains("panel-fs-active .workspace")
            || html.contains("panel-fs-active .workspace{z-index:60}"),
        "fullscreen stacking lifts workspace above header trap"
    );
}

/// Band 143: collapsed cards leave the grid (P2).
#[tokio::test]
async fn ui_index_collapsed_card_leaves_grid() {
    let (app, _state) = app();
    let html = get_index_html(&app).await;
    assert!(
        html.contains(".card.collapsed{display:none")
            || html.contains(".card.collapsed { display: none"),
        "collapsed cards must leave the grid"
    );
}

/// Band 156: chart imgs (outside .body) have no max-height in fullscreen.
#[tokio::test]
async fn ui_index_fullscreen_chart_img_unclipped() {
    let (app, _state) = app();
    let html = get_index_html(&app).await;
    assert!(
        html.contains(".card.fullscreen img{max-height:none"),
        "fullscreen chart img must drop max-height"
    );
}

/// Band 143: shared type scale CSS variables (P0 typography).
#[tokio::test]
async fn ui_index_defines_type_scale() {
    let (app, _state) = app();
    let html = get_index_html(&app).await;
    assert!(html.contains("--ui:14px"));
    assert!(html.contains("--fs-ui:var(--ui)"));
    assert!(html.contains("--fs-card:calc(var(--ui)"));
    assert!(html.contains("--fs-meta:calc(var(--ui)"));
    assert!(html.contains("--fs-chart:calc(var(--ui)"));
    assert!(
        html.contains("data-action=\"type-up\"") || html.contains("data-action='type-up'"),
        "text size control"
    );
}

/// Band 147: header/card density vs presentation shots (not pixel-perfect).
#[tokio::test]
async fn ui_index_readme_density_tokens() {
    let (app, _state) = app();
    let html = get_index_html(&app).await;
    assert!(
        html.contains("--card-radius:12px"),
        "card radius token vs presentation shots"
    );
    assert!(
        html.contains("--card-gap:16px"),
        "card gap token vs presentation shots"
    );
    assert!(
        html.contains("--header-pad:8px 16px"),
        "denser header padding vs presentation shots"
    );
    assert!(
        html.contains("loadRssTicker")
            && (html.contains(".rss-ticker{display:none")
                || html.contains(".rss-ticker { display: none")),
        "RSS ticker hidden until feed items exist"
    );
}

/// Auto-resync must not toast, not rewrite identical HTML, and not refresh
/// hidden groups (those probes spawn git/rustc and flash consoles on Windows).
#[tokio::test]
async fn ui_auto_resync_is_silent_skips_identical_and_visible_only() {
    let (app, _state) = app();
    let html = get_index_html(&app).await;
    assert!(
        html.contains("resync({silent:true})"),
        "60s Auto must silent-resync: {html}"
    );
    assert!(
        html.contains("el.innerHTML===d.html") || html.contains("el.innerHTML === d.html"),
        "getText must skip identical HTML (box blink): {html}"
    );
    assert!(
        html.contains("function visibleRustCards"),
        "silent auto refreshes the visible group only"
    );
    assert!(
        html.contains("sseOpened"),
        "first SSE onopen must not double-resync all boxes"
    );
}

/// Band 144: Update badge POSTs apply and stays offline until SSE reconnects.
#[tokio::test]
async fn ui_index_do_update_posts_apply() {
    let (app, _state) = app();
    let html = get_index_html(&app).await;
    assert!(
        html.contains("api/update/apply"),
        "doUpdate must POST apply"
    );
    assert!(html.contains("function doUpdate"));
    assert!(html.contains("setOffline(true)"));
}

#[tokio::test]
async fn ui_card_about_is_english_howto() {
    let (app, _state) = app();
    let (status, json) = get_card(&app, "about").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
    let html = json["html"].as_str().expect("html");
    assert!(html.contains("How to move around"), "{html}");
    assert!(html.contains("127.0.0.1:9999"), "{html}");
    assert!(html.contains("data-card-jump='tickets'"), "{html}");
    assert!(html.contains("/api/ui/icons.svg"), "{html}");
    assert!(html.contains("Vision Map"), "{html}");
    assert!(html.contains("static L0"), "{html}");
}

#[tokio::test]
async fn ui_index_about_and_hover_tips() {
    let (app, _state) = app();
    let html = get_index_html(&app).await;
    assert!(html.contains("data-card=\"about\""), "about card");
    assert!(html.contains("id=\"gsvTip\""), "hover tip layer");
    assert!(
        html.contains("function decorateGuide"),
        "layout applies blurbs"
    );
    assert!(html.contains("function bindTips"), "pointer hover tips");
    assert!(html.contains("\"about\""), "rustCards includes about");
    assert!(
        html.contains("data-action=\"about-jump\""),
        "header About before layout"
    );
    assert!(
        html.contains("setAttribute(\"data-title\""),
        "dock uses human titles"
    );
}

#[tokio::test]
async fn ui_icon_svg_is_distinct_and_known() {
    let (app, _state) = app();
    let health = {
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/ui/icon/health.svg")
                    .method(Method::GET)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(res.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .expect("body");
        String::from_utf8(bytes.to_vec()).expect("utf8")
    };
    assert!(health.starts_with("<svg"), "{health}");
    assert!(health.contains("Health"), "{health}");
    let sheet = {
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/ui/icons.svg")
                    .method(Method::GET)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(res.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .expect("body");
        String::from_utf8(bytes.to_vec()).expect("utf8")
    };
    assert!(sheet.contains("Galaxy card icons"), "{sheet}");
    let unknown = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/ui/icon/does-not-exist")
                .method(Method::GET)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(unknown.status(), StatusCode::NOT_FOUND);
}
