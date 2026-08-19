//! GSV server — axum router, static UI, REST API boxes, SSE events.

use std::convert::Infallible;
use std::time::Duration;

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, Query, Request, State};
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::stream::{self, StreamExt};
use serde_json::{json, Value};
use tokio_stream::wrappers::BroadcastStream;

use crate::boxes::ide::IdeSelection;
use crate::boxes::preview::{resolve as preview_resolve, PreviewParams};
use crate::boxes::terminal::{run as terminal_run, TerminalRequest, TerminalResponse};
use crate::boxes::update::UpdateCheckParams;
use crate::boxes::{hooks, sli, toolchain};
use crate::state::AppState;
use crate::tracker::{TrackerRecord, TrackerStore};
use crate::vision;

/// Embedded single-page UI (canon file: `GSV/ui/index.html`).
pub const INDEX_HTML: &str = include_str!("../../ui/index.html");

/// Ported vision diagram (`GSV/ui/vision.svg`), ratio-safe: `.svg` is audit-ignored.
pub const VISION_SVG: &str = include_str!("../../ui/vision.svg");

/// Canonical JSON error response — every error carries `{ok:false, error}`.
fn err_json(status: StatusCode, msg: impl Into<String>) -> Response {
    (status, Json(json!({ "ok": false, "error": msg.into() }))).into_response()
}

/// `/api/health` payload.
fn health(state: &AppState) -> Value {
    let fp = crate::boxes::fingerprint::latest(
        &crate::boxes::fingerprint::jsonl_path(&state.repo_root),
        1,
    );
    let latest = fp.first();
    let selected = product_selected_id(state);
    let selected_version = selected.as_deref().and_then(|id| {
        let rows = crate::boxes::products::discover(&state.repo_root);
        crate::boxes::products::lookup(&rows, id)
            .and_then(|row| crate::boxes::fingerprint::pkg_version(std::path::Path::new(&row.path)))
    });
    json!({
        "name": crate::GSV_SERVER_NAME,
        "version": *state.version,
        "product": "gsv",
        "ok": true,
        "uptime_secs": state.started_at.elapsed().map(|d| d.as_secs()).unwrap_or(0),
        "update_available": crate::boxes::update::effective_available(state),
        "crate_version": crate::boxes::update::crate_version(&state.repo_root),
        "version_lag": crate::boxes::update::version_lag(&state.repo_root, state.version.as_ref()),
        "watchdog_alive": crate::boxes::watchdog::wire(&state.repo_root)
            .get("alive")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "selected_product": selected,
        "selected_version": selected_version,
        "fingerprint_actor": latest.map(|f| f.actor.as_str()),
        "fingerprint_ide": latest.map(|f| f.ide.as_str()),
        "fingerprint_model": latest.map(|f| f.model.as_str()),
        "fingerprint_product": latest.map(|f| f.product.as_str()),
        "fingerprint_version": latest.map(|f| f.version.as_str()),
    })
}

/// Tracker wire.
fn tracker_wire(state: &AppState) -> Value {
    let tracker = state.tracker.clone();
    // Snapshot under read lock (no async needed — RwLock from tokio is Sync).
    json!({
        "sprints": tracker.try_read().map(|t| t.sprints().clone()).ok(),
        "records": tracker.try_read().map(|t| t.records().to_vec()).unwrap_or_default(),
        "generated_at": vision::rfc3339_now(),
    })
}

/// Build the full axum router with `AppState`.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/sw.js", get(api_sw_js))
        .route("/api/sw", get(api_sw))
        .route("/api", get(api_index))
        .route("/api/", get(api_index))
        .route("/api/vision/", get(api_vision_index))
        .route("/api/ui", get(api_ui_index))
        .route("/api/ui/", get(api_ui_index))
        .route("/api/omni/", get(api_omni_index))
        .route("/api/health", get(api_health))
        .route("/api/watchdog", get(api_watchdog))
        .route("/api/usage", get(api_usage))
        .route("/api/settings", get(api_settings).post(api_settings_post))
        .route("/api/telegram", get(api_telegram))
        .route(
            "/api/telegram/bus",
            get(api_telegram_bus).post(api_telegram_bus_post),
        )
        .route("/api/telegram/ticket", post(api_telegram_ticket))
        .route("/api/tickets", get(api_tickets).post(api_tickets_post))
        .route("/api/tickets/claim", post(api_tickets_claim))
        .route("/api/tickets/done", post(api_tickets_done))
        .route("/api/tickets/error", post(api_tickets_error))
        .route("/api/tickets/presence", post(api_tickets_presence))
        .route("/api/tickets/reclaim", post(api_tickets_reclaim))
        .route("/api/tickets/walk", post(api_tickets_walk))
        .route("/api/tickets/hook", post(api_tickets_hook))
        .route(
            "/api/tickets/bench",
            get(api_tickets_bench).post(api_tickets_bench_post),
        )
        .route("/api/mds", get(api_mds))
        .route("/api/xtask", get(api_xtask))
        .route("/api/disk", get(api_disk))
        .route("/api/tracker", get(api_tracker))
        .route("/api/sli", get(api_sli))
        .route("/api/toolchain", get(api_toolchain))
        .route("/api/ide/sessions", get(api_ide_sessions))
        .route("/api/ide/select", post(api_ide_select))
        .route("/api/products", get(api_products))
        .route("/api/products/select", post(api_products_select))
        .route("/api/products/open", post(api_products_open))
        .route("/api/products/scan", get(api_products_scan))
        .route("/api/fingerprints", get(api_fingerprints))
        .route("/api/update", get(api_update))
        .route("/api/update/notify", post(api_update_notify))
        .route("/api/update/apply", post(api_update_apply))
        .route("/api/preview", get(api_preview))
        .route("/api/terminal", post(api_terminal))
        .route("/api/hooks/tests", get(api_hooks_tests))
        .route("/api/hooks/bench", get(api_hooks_bench))
        .route("/api/ratio", get(api_ratio))
        .route("/api/ratio/history", get(api_ratio_history))
        .route("/api/ratio/compare", get(api_ratio_compare))
        .route("/api/ratio/target", get(api_ratio_target))
        .route("/api/ratio/trend", get(api_ratio_trend))
        .route("/api/ui/card/{name}", get(api_ui_card))
        .route("/api/ui/layout", get(api_ui_layout))
        .route("/ui/{*path}", get(api_ui_path))
        .route("/api/ui/load-palette", get(api_ui_load_palette))
        .route("/api/ui/load-theme", get(api_ui_load_theme))
        .route("/api/ui/visual-toggle", get(api_ui_visual_toggle))
        .route("/api/vision", get(api_vision))
        .route("/api/vision/manifest", get(api_vision_manifest))
        .route("/api/vision/map", get(api_vision_map))
        .route("/api/vision/feed", get(api_vision_feed))
        .route("/api/vision/sprint-map", get(api_vision_sprint_map))
        .route("/api/vision/doc-preview", get(api_vision_doc_preview))
        .route("/api/vision/node-search", get(api_vision_node_search))
        .route("/api/vision/sync", get(api_vision_sync))
        .route("/api/vision/extensions", get(api_vision_extensions))
        .route("/api/vision/sprint-queue", get(api_vision_sprint_queue))
        .route("/api/vision/sprint-board", get(api_vision_sprint_board))
        .route("/api/vision/sprint-theme", get(api_vision_sprint_theme))
        .route("/api/vision/palette", get(api_vision_palette))
        .route(
            "/api/vision/sprint-progress",
            get(api_vision_sprint_progress),
        )
        .route("/api/vision/speeds", get(api_vision_speeds))
        .route(
            "/api/vision/rust-diagnostics",
            get(api_vision_rust_diagnostics),
        )
        .route("/api/vision/speeds.svg", get(api_vision_speeds_svg))
        .route(
            "/api/vision/rust-diagnostics.svg",
            get(api_vision_rust_diagnostics_svg),
        )
        .route(
            "/api/vision/sprint-focus.svg",
            get(api_vision_sprint_focus_svg),
        )
        .route("/api/vision/focus-svg", get(api_vision_sprint_focus_svg))
        .route("/api/vision/starfield.svg", get(api_vision_starfield_svg))
        .route("/api/vision/galaxy.svg", get(api_vision_galaxy_svg))
        .route("/api/vision/theme-svg", get(api_vision_theme_svg))
        .route(
            "/api/vision/sprint-priority",
            get(api_vision_sprint_priority),
        )
        .route("/api/vision/tracker", get(api_vision_tracker))
        .route("/api/vision/events", get(api_vision_events))
        .route("/api/vision/ide-session", get(api_vision_ide_session))
        .route("/api/vision/control-status", get(api_vision_control_status))
        .route("/api/omni", get(api_omni))
        .route(
            "/api/omni/config",
            get(api_omni_config).post(api_omni_config_post),
        )
        .route("/api/omni/v1/models", get(api_omni_v1_models))
        .route("/api/omni/v1/chat/completions", post(api_omni_chat))
        .route("/api/omni/test", post(api_omni_test))
        .route("/api/omni/status", get(api_omni_status))
        .route("/api/omni/route", get(api_omni_route))
        .route("/api/toolchain/rustc", get(api_toolchain_rustc))
        .route("/api/toolchain/cargo", get(api_toolchain_cargo))
        .route("/api/toolchain/clippy", get(api_toolchain_clippy))
        .route("/api/toolchain/detailed", get(api_toolchain_detailed))
        .route("/api/toolchain/build", post(api_toolchain_build))
        .route("/api/toolchain/test", post(api_toolchain_test))
        .route("/api/toolchain/clean", post(api_toolchain_clean))
        .route("/api/vision/resync", post(api_vision_resync))
        .route("/api/vision/setOffline", post(api_vision_set_offline))
        .route("/api/vision/reload", post(api_vision_reload))
        .route("/api/vision/snapshot", post(api_vision_snapshot))
        .route("/api/vision/shutdown", post(api_vision_shutdown))
        .route("/api/vision/restart", post(api_vision_restart))
        .route("/api/ide/opencode", get(api_ide_opencode))
        .route("/api/ide/cursor", get(api_ide_cursor))
        .route("/api/ide/pending-rebuild", get(api_ide_pending_rebuild))
        .route("/api/ide/active-session", get(api_ide_active_session))
        .route("/api/ide/session-history", get(api_ide_session_history))
        .route("/data/{file}", get(api_data_file))
        .route("/events", get(events))
        .route("/assets/vision.svg", get(api_vision_svg))
        .route(
            "/mcp",
            get(api_mcp_get).post(api_mcp_post).delete(api_mcp_delete),
        )
        .layer(DefaultBodyLimit::max(crate::security::MAX_BODY_BYTES))
        .layer(middleware::from_fn(security_gate))
        .with_state(state)
}

fn insert_security_headers(headers: &mut HeaderMap) {
    for (name, value) in crate::security::SECURITY_HEADERS {
        if let (Ok(n), Ok(v)) = (
            HeaderName::from_bytes(name.as_bytes()),
            HeaderValue::from_str(value),
        ) {
            headers.insert(n, v);
        }
    }
}

fn secured(mut res: Response) -> Response {
    insert_security_headers(res.headers_mut());
    res
}

/// CSRF POST gate, POST body cap, then CSP / nosniff / no-store on every reply.
///
/// `/mcp` skips the Origin / Sec-Fetch-Site gate: Cursor / OpenCode / Grok Bot
/// JSON-RPC clients are not the Galaxy UI (they send `vscode-file:` /
/// `cursor:` origins or `cross-site`). Body cap still applies. Bind stays
/// loopback unless `--allow-lan`.
async fn security_gate(req: Request, next: Next) -> Response {
    if req.method() == Method::POST {
        let content_length = req
            .headers()
            .get(header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok());
        if let Err(msg) = crate::security::gate_content_length(content_length) {
            return secured(err_json(StatusCode::PAYLOAD_TOO_LARGE, msg));
        }
        let mcp_rpc = req.uri().path() == "/mcp";
        if !mcp_rpc {
            let site = req
                .headers()
                .get("sec-fetch-site")
                .and_then(|v| v.to_str().ok());
            let origin = req
                .headers()
                .get(header::ORIGIN)
                .and_then(|v| v.to_str().ok());
            if let Err(msg) = crate::security::gate_post(site, origin) {
                return secured(err_json(StatusCode::FORBIDDEN, msg));
            }
        }
    }
    let res = next.run(req).await;
    if res.status() == StatusCode::PAYLOAD_TOO_LARGE {
        return secured(err_json(
            StatusCode::PAYLOAD_TOO_LARGE,
            "request body too large",
        ));
    }
    secured(res)
}

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn api_sw_js() -> Response {
    (
        StatusCode::OK,
        [
            ("Content-Type", "text/javascript; charset=utf-8"),
            ("Service-Worker-Allowed", "/"),
        ],
        crate::boxes::sw::script(),
    )
        .into_response()
}

async fn api_sw() -> Json<Value> {
    Json(crate::boxes::sw::wire())
}

async fn api_vision_svg() -> Response {
    (
        StatusCode::OK,
        [
            ("Content-Type", "image/svg+xml"),
            ("Cache-Control", "no-cache"),
        ],
        VISION_SVG,
    )
        .into_response()
}

async fn api_health(State(state): State<AppState>) -> Json<Value> {
    Json(health(&state))
}

async fn api_watchdog(State(state): State<AppState>) -> Json<Value> {
    Json(crate::boxes::watchdog::wire(&state.repo_root))
}

async fn api_settings(State(state): State<AppState>) -> Json<Value> {
    Json(crate::boxes::settings::wire(&state.data_dir))
}

async fn api_settings_post(State(state): State<AppState>, Json(patch): Json<Value>) -> Json<Value> {
    Json(crate::boxes::settings::wire_post(&state.data_dir, &patch))
}

async fn api_telegram(State(state): State<AppState>, headers: HeaderMap) -> Json<Value> {
    let dry = crate::boxes::telegram::header_dry_run(&headers);
    Json(crate::boxes::telegram::status(&state.data_dir, dry).await)
}

#[derive(serde::Deserialize)]
struct TelegramBusQuery {
    limit: Option<u64>,
}

async fn api_telegram_bus(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<TelegramBusQuery>,
) -> Json<Value> {
    let dry = crate::boxes::telegram::header_dry_run(&headers);
    Json(crate::boxes::telegram::bus_poll(&state.data_dir, dry, q.limit.map(|n| n as usize)).await)
}

async fn api_telegram_bus_post(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    let dry = crate::boxes::telegram::header_dry_run(&headers);
    Json(crate::boxes::telegram::bus_send(&state.data_dir, dry, &body).await)
}

async fn api_telegram_ticket(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    let dry = crate::boxes::telegram::header_dry_run(&headers);
    Json(
        crate::boxes::telegram::ingest_channel_body(
            &state.repo_root,
            &state.data_dir,
            dry,
            &body,
            Some(&state.ticket_presence),
        )
        .await,
    )
}

async fn api_tickets(State(state): State<AppState>) -> Json<Value> {
    Json(crate::boxes::tickets::wire_list(
        &state.repo_root,
        &state.data_dir,
        &state.ticket_presence,
    ))
}

async fn api_tickets_post(State(state): State<AppState>, Json(body): Json<Value>) -> Response {
    match crate::boxes::tickets::wire_create(
        &state.repo_root,
        &state.data_dir,
        &body,
        Some(&state.ticket_presence),
    ) {
        Ok(v) => Json(v).into_response(),
        Err(e) => err_json(ticket_http_status(&e), e.to_string()),
    }
}

async fn api_tickets_claim(State(state): State<AppState>, Json(body): Json<Value>) -> Response {
    match crate::boxes::tickets::wire_claim(
        &state.repo_root,
        &state.data_dir,
        &body,
        Some(&state.ticket_presence),
    ) {
        Ok(v) => Json(v).into_response(),
        Err(e) => err_json(ticket_http_status(&e), e.to_string()),
    }
}

async fn api_tickets_done(State(state): State<AppState>, Json(body): Json<Value>) -> Response {
    match crate::boxes::tickets::wire_done(
        &state.repo_root,
        &state.data_dir,
        &body,
        Some(&state.ticket_presence),
    ) {
        Ok(v) => Json(v).into_response(),
        Err(e) => err_json(ticket_http_status(&e), e.to_string()),
    }
}

async fn api_tickets_error(State(state): State<AppState>, Json(body): Json<Value>) -> Response {
    match crate::boxes::tickets::wire_error(
        &state.repo_root,
        &state.data_dir,
        &body,
        Some(&state.ticket_presence),
    ) {
        Ok(v) => Json(v).into_response(),
        Err(e) => err_json(ticket_http_status(&e), e.to_string()),
    }
}

async fn api_tickets_presence(State(state): State<AppState>, Json(body): Json<Value>) -> Response {
    Json(crate::boxes::tickets::wire_presence(
        &state.repo_root,
        &state.data_dir,
        &state.ticket_presence,
        &body,
    ))
    .into_response()
}

async fn api_tickets_reclaim(State(state): State<AppState>, Json(body): Json<Value>) -> Response {
    match crate::boxes::tickets::wire_reclaim(&state.repo_root, &state.data_dir, &body) {
        Ok(v) => Json(v).into_response(),
        Err(e) => err_json(ticket_http_status(&e), e.to_string()),
    }
}

async fn api_tickets_walk(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    match crate::boxes::telegram::sync_walk(
        &state.repo_root,
        &state.data_dir,
        &body,
        Some(&state.ticket_presence),
        crate::boxes::telegram::header_dry_run(&headers),
    )
    .await
    {
        Ok(v) => Json(v).into_response(),
        Err(e) => err_json(ticket_http_status(&e), e.to_string()),
    }
}

async fn api_tickets_hook(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    match crate::boxes::telegram::sync_hook(
        &state.repo_root,
        &state.data_dir,
        &body,
        Some(&state.ticket_presence),
        crate::boxes::telegram::header_dry_run(&headers),
    )
    .await
    {
        Ok(v) => Json(v).into_response(),
        Err(e) => err_json(ticket_http_status(&e), e.to_string()),
    }
}

async fn api_tickets_bench(State(state): State<AppState>) -> Json<Value> {
    Json(crate::boxes::tickets::wire_bench(&state.repo_root))
}

async fn api_tickets_bench_post(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Response {
    match crate::boxes::tickets::wire_bench_post(&state.repo_root, &body) {
        Ok(v) => Json(v).into_response(),
        Err(e) => err_json(ticket_http_status(&e), e.to_string()),
    }
}

async fn api_mds(State(state): State<AppState>) -> Json<Value> {
    Json(crate::boxes::mds::wire(&state.repo_root))
}

fn ticket_http_status(err: &crate::boxes::tickets::TicketError) -> StatusCode {
    use crate::boxes::tickets::TicketError;
    match err {
        TicketError::NotFound => StatusCode::NOT_FOUND,
        TicketError::Forbidden => StatusCode::FORBIDDEN,
        TicketError::BadRequest(_) => StatusCode::BAD_REQUEST,
        TicketError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn api_usage(State(state): State<AppState>) -> Json<Value> {
    crate::boxes::usage::merge_omniroute_pull(&state).await;
    Json(crate::boxes::usage::wire_state(&state).await)
}

#[derive(serde::Deserialize)]
struct XtaskQuery {
    task: Option<String>,
}

async fn api_xtask(
    State(state): State<AppState>,
    Query(q): Query<XtaskQuery>,
) -> impl IntoResponse {
    let task = q.task.as_deref().unwrap_or("catalog");
    match crate::boxes::xtask::mcp_run(&state.repo_root, task) {
        Ok(v) => Json(v).into_response(),
        Err(e) => err_json(StatusCode::BAD_REQUEST, &e),
    }
}

#[derive(serde::Deserialize)]
struct DiskQuery {
    enforce: Option<bool>,
}

async fn api_disk(State(state): State<AppState>, Query(q): Query<DiskQuery>) -> Json<Value> {
    Json(crate::boxes::xtask::disk_wire(
        &state.repo_root,
        q.enforce.unwrap_or(false),
    ))
}

fn accept_sse(headers: &HeaderMap) -> bool {
    crate::mcp::wants_sse(headers.get(header::ACCEPT).and_then(|v| v.to_str().ok()))
}

fn mcp_sse_reply(notes: Vec<Value>, rpc: Option<Value>) -> Response {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/event-stream"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        crate::mcp::sse_body(notes, rpc),
    )
        .into_response()
}

fn attach_mcp_session(res: &mut Response, session: Option<&str>) {
    if let Some(id) = session {
        if let Ok(v) = HeaderValue::from_str(id) {
            res.headers_mut().insert(crate::mcp::MCP_SESSION_HEADER, v);
        }
    }
}

fn mcp_unknown_session() -> Response {
    err_json(StatusCode::NOT_FOUND, "mcp session not found")
}

async fn api_mcp_get(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let session = crate::mcp::mcp_session_id_from_headers(&headers);
    if let Some(id) = session.as_deref() {
        if !state.mcp_session_ok(id) {
            return mcp_unknown_session();
        }
    }
    if accept_sse(&headers) {
        if session.is_some() {
            let mut res = mcp_sse_hold(state).into_response();
            attach_mcp_session(&mut res, session.as_deref());
            return res;
        }
        return mcp_sse_reply(state.drain_mcp_notifications(), None);
    }
    Json(crate::mcp::http_info(&state)).into_response()
}

/// Long-lived Streamable HTTP GET (Cursor). Finite flush stays on sessionless GET.
fn mcp_sse_hold(
    state: AppState,
) -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
    let initial = state.drain_mcp_notifications();
    let first = stream::iter(
        initial
            .into_iter()
            .map(|v| Ok::<_, Infallible>(Event::default().event("message").data(v.to_string()))),
    );
    let later =
        stream::unfold(state, |state| async move {
            loop {
                tokio::time::sleep(Duration::from_millis(400)).await;
                let notes = state.drain_mcp_notifications();
                if !notes.is_empty() {
                    return Some((notes, state));
                }
            }
        })
        .flat_map(|notes| {
            stream::iter(notes.into_iter().map(|v| {
                Ok::<_, Infallible>(Event::default().event("message").data(v.to_string()))
            }))
        });
    Sse::new(first.chain(later)).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keepalive"),
    )
}

async fn api_mcp_post(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response {
    if let Some(id) = crate::mcp::mcp_session_id_from_headers(&headers) {
        if !state.mcp_session_ok(&id) {
            return mcp_unknown_session();
        }
    }
    let sse = accept_sse(&headers);
    if body.is_empty() {
        return Json(crate::mcp::rpc_error(None, -32700, "empty body")).into_response();
    }
    match serde_json::from_slice::<Value>(&body) {
        Ok(v) => {
            let issued = if crate::mcp::jsonrpc_mentions_initialize(&v) {
                Some(state.mcp_issue_session())
            } else {
                None
            };
            let session = crate::mcp::mcp_session_id_from_headers(&headers);
            match crate::mcp::handle_value_in(&state, v, session.as_deref()).await {
                Some(out) => {
                    let notes = state.drain_mcp_notifications();
                    let mut res = if sse {
                        mcp_sse_reply(notes, Some(out))
                    } else {
                        Json(out).into_response()
                    };
                    attach_mcp_session(&mut res, issued.as_deref());
                    res
                }
                None => {
                    let notes = state.drain_mcp_notifications();
                    let mut res = if sse {
                        if notes.is_empty() {
                            StatusCode::NO_CONTENT.into_response()
                        } else {
                            mcp_sse_reply(notes, None)
                        }
                    } else {
                        StatusCode::NO_CONTENT.into_response()
                    };
                    attach_mcp_session(&mut res, issued.as_deref());
                    res
                }
            }
        }
        Err(e) => Json(crate::mcp::rpc_error(None, -32700, format!("parse: {e}"))).into_response(),
    }
}

async fn api_mcp_delete(State(state): State<AppState>, headers: HeaderMap) -> Response {
    match crate::mcp::mcp_session_id_from_headers(&headers) {
        None => err_json(StatusCode::BAD_REQUEST, "mcp session id required"),
        Some(id) if !state.mcp_session_delete(&id) => mcp_unknown_session(),
        Some(_) => Json(json!({ "ok": true })).into_response(),
    }
}

async fn api_index() -> Json<Value> {
    Json(json!({
        "ok": true,
        "api": "GSV",
        "port": 9999,
        "categories": [
            "/api/vision/", "/api/ui/", "/api/ratio/", "/api/toolchain/",
            "/api/ide/", "/api/omni/", "/api/sli", "/api/tracker", "/api/products",
            "/api/fingerprints", "/api/sw", "/api/watchdog", "/api/usage", "/api/settings", "/api/telegram", "/api/telegram/bus", "/api/telegram/ticket", "/api/tickets", "/api/mds", "/api/xtask", "/api/disk", "/sw.js",
            "/api/hooks/", "/api/preview", "/api/terminal", "/data/", "/mcp"
        ],
        "example": "/api/vision",
        "docs": "/assets/vision.svg"
    }))
}

async fn api_vision_index() -> Json<Value> {
    Json(json!({
        "ok": true,
        "endpoints": [
            "/api/vision", "/api/vision/manifest", "/api/vision/feed",
            "/api/vision/sprint-map", "/api/vision/sprint-board",
            "/api/vision/speeds", "/api/vision/rust-diagnostics",
            "/api/vision/sprint-theme", "/api/vision/sprint-focus.svg",
            "/api/vision/focus-svg", "/api/vision/palette",
            "/api/vision/starfield.svg", "/api/vision/galaxy.svg",
            "/api/vision/sync", "/api/vision/extensions",
            "/api/vision/sprint-queue", "/api/vision/node-search",
            "/api/vision/speeds.svg", "/api/vision/rust-diagnostics.svg",
            "/api/vision/theme-svg", "/api/vision/sprint-priority",
            "/api/vision/tracker", "/api/vision/events",
            "/api/vision/ide-session", "/api/vision/sprint-progress",
            "/api/vision/control-status"
        ],
        "control": [
            "POST /api/vision/resync", "POST /api/vision/setOffline",
            "POST /api/vision/reload", "POST /api/vision/snapshot",
            "POST /api/vision/shutdown", "POST /api/vision/restart"
        ]
    }))
}

async fn api_ui_index() -> Json<Value> {
    Json(json!({
        "ok": true,
        "endpoints": [
            "/api/ui/card/{name}", "/api/ui/layout", "/api/ui/load-palette",
            "/api/ui/load-theme", "/api/ui/visual-toggle"
        ],
        "widgets": "/ui/{path}"
    }))
}

async fn api_omni_index() -> Json<Value> {
    Json(json!({
        "ok": true,
        "endpoints": [
            "/api/omni", "/api/omni/config", "/api/omni/status", "/api/omni/route",
            "/api/omni/v1/models", "POST /api/omni/v1/chat/completions",
            "POST /api/omni/test"
        ]
    }))
}

async fn api_tracker(State(state): State<AppState>) -> Json<Value> {
    Json(tracker_wire(&state))
}

async fn api_sli(State(state): State<AppState>) -> Json<Value> {
    Json(json!(sli::wire(&state.repo_root)))
}

async fn api_toolchain(State(state): State<AppState>) -> Json<Value> {
    Json(json!(toolchain::wire(&state.repo_root)))
}

async fn api_ide_sessions(State(state): State<AppState>) -> Json<Value> {
    let selection = state.ide_selection.try_read().ok().and_then(|s| s.clone());
    Json(json!(crate::boxes::ide::wire(selection.as_ref())))
}

#[derive(serde::Deserialize)]
struct IdeSelectBody {
    tool: String,
    session: String,
}

async fn api_ide_select(
    State(state): State<AppState>,
    Json(body): Json<IdeSelectBody>,
) -> Json<Value> {
    let selection = IdeSelection {
        tool: body.tool,
        session: body.session,
        selected_at: vision::rfc3339_now(),
    };
    if let Ok(mut sel) = state.ide_selection.try_write() {
        *sel = Some(selection.clone());
    }
    state.emit(format!(
        "event: ide_selected\ndata: {}",
        serde_json::to_string(&selection).unwrap_or_default()
    ));
    Json(json!({ "ok": true, "selection": selection }))
}

#[derive(serde::Deserialize)]
struct ProductIdBody {
    id: String,
}

fn product_selected_id(state: &AppState) -> Option<String> {
    state.product_selected.lock().ok().and_then(|g| g.clone())
}

async fn api_products(State(state): State<AppState>) -> Json<Value> {
    let selected = product_selected_id(&state);
    Json(crate::boxes::products::wire(
        &state.repo_root,
        selected.as_deref(),
    ))
}

async fn api_products_select(
    State(state): State<AppState>,
    Json(body): Json<ProductIdBody>,
) -> Response {
    let rows = crate::boxes::products::discover(&state.repo_root);
    if crate::boxes::products::lookup(&rows, &body.id).is_none() {
        return err_json(StatusCode::NOT_FOUND, "unknown product");
    }
    if let Ok(mut g) = state.product_selected.lock() {
        *g = Some(body.id.clone());
    }
    Json(json!({ "ok": true, "selected": body.id })).into_response()
}

async fn api_products_open(
    State(state): State<AppState>,
    Json(body): Json<ProductIdBody>,
) -> Response {
    match crate::boxes::products::open_folder(&state.repo_root, &body.id) {
        Ok(how) => Json(json!({ "ok": true, "opened": true, "how": how })).into_response(),
        Err(_) => err_json(StatusCode::NOT_FOUND, "unknown product"),
    }
}

async fn api_products_scan(State(state): State<AppState>) -> Response {
    let Some(id) = product_selected_id(&state) else {
        return err_json(StatusCode::BAD_REQUEST, "no product selected");
    };
    match crate::boxes::products::scan(&state.repo_root, &id) {
        Ok(s) => Json(json!(s)).into_response(),
        Err(_) => err_json(StatusCode::NOT_FOUND, "unknown product"),
    }
}

#[derive(serde::Deserialize)]
struct FingerprintQuery {
    limit: Option<usize>,
}

async fn api_fingerprints(
    State(state): State<AppState>,
    Query(q): Query<FingerprintQuery>,
) -> Json<Value> {
    Json(crate::boxes::fingerprint::wire(
        &state.repo_root,
        product_selected_id(&state).as_deref(),
        crate::boxes::fingerprint::clamp_limit(q.limit),
    ))
}

async fn api_update(
    State(state): State<AppState>,
    Query(_params): Query<UpdateCheckParams>,
) -> Json<Value> {
    Json(json!(crate::boxes::update::wire(&state)))
}

async fn api_update_notify(State(state): State<AppState>) -> Json<Value> {
    state
        .update_flag
        .store(true, std::sync::atomic::Ordering::SeqCst);
    state.emit("event: update_available\ndata: true".to_string());
    Json(json!({ "ok": true, "update_available": true }))
}

async fn api_update_apply(State(state): State<AppState>) -> Json<Value> {
    let body = crate::boxes::update::apply_update(&state);
    if crate::boxes::update::apply_should_exit() {
        tokio::spawn(async {
            tokio::time::sleep(Duration::from_millis(200)).await;
            std::process::exit(0);
        });
    }
    Json(body)
}

async fn api_preview(
    State(state): State<AppState>,
    Query(params): Query<PreviewParams>,
) -> Response {
    let path = match preview_resolve(&state.repo_root, &params.file) {
        Ok(p) => p,
        Err(e) => return err_json(StatusCode::NOT_FOUND, e),
    };
    match crate::boxes::preview::render(&path, &params.file) {
        Ok(wire) => Json(wire).into_response(),
        Err(e) => err_json(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

async fn api_terminal(
    State(state): State<AppState>,
    Json(body): Json<TerminalRequest>,
) -> Json<TerminalResponse> {
    let resp = terminal_run(&body.command);
    let mut tracker = TrackerStore::load(&state.repo_root, &state.data_dir).unwrap_or_default();
    let status = if resp.allowed && resp.exit_code == Some(0) {
        "closed"
    } else if resp.allowed {
        "error"
    } else {
        "blocked"
    };
    let _ = tracker.push(
        &state.data_dir,
        TrackerRecord::new(
            "command",
            body.command,
            format!("exit={:?} ms={}", resp.exit_code, resp.duration_ms),
            status,
        ),
    );
    state.emit("event: terminal\ndata: done".to_string());
    Json(resp)
}

async fn api_hooks_tests(State(state): State<AppState>) -> Json<Value> {
    Json(json!(hooks::tests_wire(&state.repo_root)))
}

async fn api_hooks_bench(State(state): State<AppState>) -> Json<Value> {
    Json(json!(hooks::bench_wire(&state.repo_root)))
}

async fn api_ratio(State(state): State<AppState>) -> Json<Value> {
    Json(crate::boxes::ratio::wire(&state.data_dir))
}

async fn api_ratio_history(State(state): State<AppState>) -> Json<Value> {
    let w = crate::boxes::ratio::wire(&state.data_dir);
    Json(json!({
        "ok": w.get("ok").and_then(Value::as_bool).unwrap_or(false),
        "generated_at": w.get("generated_at").cloned().unwrap_or_default(),
        "history": w.get("history").cloned().unwrap_or(Value::Array(Vec::new())),
        "current": w.get("rust_ratio_pct").cloned().unwrap_or_default(),
    }))
}

async fn api_ratio_compare(State(state): State<AppState>) -> Json<Value> {
    let w = crate::boxes::ratio::wire(&state.data_dir);
    let current = w
        .get("rust_ratio_pct")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let min = w.get("min_ratio").and_then(Value::as_f64).unwrap_or(0.0);
    let stretch = w
        .get("stretch_target")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let meets = w
        .get("meets_min_ratio")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Json(json!({
        "current": current,
        "min": min,
        "stretch": stretch,
        "gap_to_min": current - min,
        "gap_to_stretch": current - stretch,
        "meets_min": meets,
    }))
}

async fn api_ratio_target(State(state): State<AppState>) -> Json<Value> {
    let w = crate::boxes::ratio::wire(&state.data_dir);
    Json(json!({
        "min_ratio": w.get("min_ratio").cloned().unwrap_or_default(),
        "stretch_target": w.get("stretch_target").cloned().unwrap_or_default(),
        "formal_band_min": w.get("formal_band_min").cloned().unwrap_or_default(),
        "meets_min": w.get("meets_min_ratio").cloned().unwrap_or_default(),
        "meets_stretch_96": w.get("meets_stretch_96").cloned().unwrap_or_default(),
    }))
}

async fn api_ratio_trend(State(state): State<AppState>) -> Json<Value> {
    let w = crate::boxes::ratio::wire(&state.data_dir);
    Json(json!({
        "current": w.get("rust_ratio_pct").cloned().unwrap_or_default(),
        "direction": if w.get("meets_min_ratio").and_then(Value::as_bool).unwrap_or(false) {
            "stable"
        } else {
            "below-min"
        },
        "generated_at": w.get("generated_at").cloned().unwrap_or_default(),
    }))
}

fn css_response(body: String) -> Response {
    (
        StatusCode::OK,
        [
            ("Content-Type", "text/css; charset=utf-8"),
            ("Cache-Control", "no-cache"),
        ],
        body,
    )
        .into_response()
}

async fn api_ui_load_palette() -> Response {
    css_response(crate::boxes::vision::palette_stylesheet())
}

async fn api_ui_load_theme(State(state): State<AppState>) -> Response {
    css_response(crate::boxes::vision::sprint_theme_stylesheet(
        &state.repo_root,
        &state.data_dir,
    ))
}

async fn api_ui_visual_toggle() -> Json<Value> {
    Json(json!({
        "ok": true,
        "galaxy": true,
        "starfield": true,
        "generated_at": vision::rfc3339_now(),
    }))
}

async fn api_ui_layout() -> Json<Value> {
    Json(crate::boxes::ui::layout_wire())
}

#[derive(serde::Deserialize, Default)]
struct CardQuery {
    id: Option<String>,
    q: Option<String>,
    layer: Option<String>,
}

/// `GET /api/ui/card/:name` — server-rendered card body HTML fragment.
///
/// Fetches the card's wire payload, renders it with the Rust UI fragment box,
/// and returns `{ok, card, html}`. Unknown cards return `ok:false` (404).
async fn card_wire(state: &AppState, name: &str, q: &CardQuery) -> Result<Value, ()> {
    let wire = match name {
        "tracker" => tracker_wire(state),
        "sli" => json!(sli::wire(&state.repo_root)),
        "toolchain" => json!(toolchain::wire(&state.repo_root)),
        "ratio" => crate::boxes::ratio::wire(&state.data_dir),
        "hooks-tests" => json!(hooks::tests_wire(&state.repo_root)),
        "hooks-bench" => json!(hooks::bench_wire(&state.repo_root)),
        "sprint-map" => crate::boxes::vision::wire_sprint_map(&state.repo_root, &state.data_dir),
        "sprint-queue" => {
            crate::boxes::vision::wire_sprint_queue(&state.repo_root, &state.data_dir)
        }
        "sprint-progress" => {
            crate::boxes::vision::wire_sprint_progress(&state.repo_root, &state.data_dir)
        }
        "sprint-board" => {
            crate::boxes::vision::wire_sprint_board(&state.repo_root, &state.data_dir)
        }
        "speed-index" => crate::boxes::vision::wire_speed_index(&state.repo_root, &state.data_dir),
        "rust-diagnostics" => {
            crate::boxes::vision::wire_rust_diagnostics(&state.repo_root, &state.data_dir)
        }
        "sprint-focus" => {
            let theme = crate::boxes::vision::wire_sprint_theme(&state.repo_root, &state.data_dir);
            json!({
                "ok": theme.get("ok").cloned().unwrap_or(json!(true)),
                "active_sprint": theme.get("active_sprint").cloned().unwrap_or(json!("")),
                "error": theme.get("error").cloned().unwrap_or(Value::Null),
            })
        }
        "preview" => json!({
            "ok": true,
            "path": "src/bin/gsv_server.rs",
            "extension": "rs",
        }),
        "terminal" => json!({
            "ok": true,
            "whitelist": crate::boxes::terminal::WHITELIST,
        }),
        "health" => health(state),
        "products" => {
            let selected = product_selected_id(state);
            crate::boxes::products::card_wire(&state.repo_root, selected.as_deref())
        }
        "fingerprints" => {
            let selected = product_selected_id(state);
            crate::boxes::fingerprint::wire(&state.repo_root, selected.as_deref(), 20)
        }
        "sw" => crate::boxes::sw::wire(),
        "watchdog" => crate::boxes::watchdog::wire(&state.repo_root),
        "usage" => {
            crate::boxes::usage::merge_omniroute_pull(state).await;
            crate::boxes::usage::wire_state(state).await
        }
        "mcp" => crate::mcp::http_info(state),
        "settings" => crate::boxes::settings::wire(&state.data_dir),
        "telegram" => {
            crate::boxes::telegram::status(&state.data_dir, crate::boxes::telegram::env_dry_run())
                .await
        }
        "tickets" => crate::boxes::tickets::wire_list(
            &state.repo_root,
            &state.data_dir,
            &state.ticket_presence,
        ),
        "update" => json!(crate::boxes::update::wire(state)),
        "ide" => {
            let selection = state.ide_selection.try_read().ok().and_then(|s| s.clone());
            json!(crate::boxes::ide::wire(selection.as_ref()))
        }
        "vision" => crate::boxes::vision::wire_summary(&state.repo_root, &state.data_dir),
        "vision-map" => crate::boxes::vision::wire_map(&state.repo_root, &state.data_dir),
        "vision-sync" => crate::boxes::vision::wire_sync_status(&state.repo_root, &state.data_dir),
        "doc-preview" => crate::boxes::vision::wire_doc_preview(
            &state.repo_root,
            &state.data_dir,
            q.id.as_deref().unwrap_or("galaxy_grid"),
        ),
        "ratio-box" => crate::boxes::ratio::wire(&state.data_dir),
        "omni" => {
            let selected = product_selected_id(state);
            serde_json::to_value(crate::boxes::omni::wire(&state.omni, selected.as_deref()).await)
                .unwrap_or(serde_json::Value::Null)
        }
        "galaxy-backdrop" => crate::boxes::vision::chrome_galaxy_wire(),
        "starfield" => crate::boxes::vision::chrome_starfield_wire(),
        "rss-ticker" => crate::boxes::vision::chrome_rss_wire(&state.repo_root, &state.data_dir),
        "gpu-mode" => crate::boxes::ui::chrome_gpu_wire(),
        "power-menu" => crate::boxes::ui::chrome_power_wire(),
        "panel-dock" => crate::boxes::ui::chrome_dock_wire(),
        "fullscreen" => crate::boxes::ui::chrome_fullscreen_wire(),
        "node-search" => crate::boxes::vision::wire_node_search(
            &state.repo_root,
            &state.data_dir,
            q.q.as_deref().unwrap_or(""),
            q.layer.as_deref().filter(|s| !s.is_empty()),
        ),
        _ => return Err(()),
    };
    Ok(wire)
}

async fn api_ui_card(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(q): Query<CardQuery>,
) -> Response {
    let wire = match card_wire(&state, &name, &q).await {
        Ok(w) => w,
        Err(()) => return err_json(StatusCode::NOT_FOUND, format!("unknown card: {name}")),
    };
    let html = crate::boxes::ui::render_card(&name, &wire).unwrap_or_default();
    Json(json!({ "ok": true, "card": name, "html": html })).into_response()
}

fn sprint_counts(state: &AppState) -> Value {
    let p = crate::boxes::vision::wire_sprint_progress(&state.repo_root, &state.data_dir);
    json!({
        "total": p.get("total").and_then(|v| v.as_u64()).unwrap_or(0),
        "open": p.get("open_count").and_then(|v| v.as_u64()).unwrap_or(0),
        "closed": p.get("closed_count").and_then(|v| v.as_u64()).unwrap_or(0),
        "planned": p.get("planned_count").and_then(|v| v.as_u64()).unwrap_or(0),
        "progress_pct": p.get("progress_pct").and_then(|v| v.as_u64()).unwrap_or(0),
        "remaining": p.get("open_count").and_then(|v| v.as_u64()).unwrap_or(0),
        "elapsed": p.get("closed_count").and_then(|v| v.as_u64()).unwrap_or(0),
    })
}

async fn api_ui_path(State(state): State<AppState>, Path(segments): Path<Vec<String>>) -> Response {
    let path = segments.join("/");
    if let Ok(wire) = card_wire(&state, &path, &CardQuery::default()).await {
        let html = crate::boxes::ui::render_card(&path, &wire).unwrap_or_default();
        return Json(json!({ "ok": true, "card": path, "html": html })).into_response();
    }
    let data = match path.as_str() {
        "ratio" => crate::boxes::ratio::wire(&state.data_dir),
        "ratio/current" | "ratio/percent" => json!({
            "value": crate::boxes::ratio::wire(&state.data_dir)
                .get("rust_ratio").cloned().unwrap_or_default()
        }),
        "ratio/advisory" => json!({ "advisory": "maintain >=95%" }),
        "ratio/goal" => json!({ "goal": 0.95 }),
        "sprint-columns"
        | "progress-layers"
        | "sprint-open-count"
        | "sprint-closed-count"
        | "sprint-planned-count"
        | "sprint-progress-pct"
        | "sprint-remaining"
        | "sprint-elapsed" => sprint_counts(&state),
        _ => {
            return err_json(StatusCode::NOT_FOUND, format!("unknown ui path: {path}"));
        }
    };
    Json(json!({ "ok": true, "path": path, "data": data })).into_response()
}

async fn api_vision(State(state): State<AppState>) -> Json<Value> {
    Json(crate::boxes::vision::wire_summary(
        &state.repo_root,
        &state.data_dir,
    ))
}

async fn api_vision_manifest(State(state): State<AppState>) -> Json<Value> {
    Json(crate::boxes::vision::wire_manifest(
        &state.repo_root,
        &state.data_dir,
    ))
}

async fn api_vision_map(State(state): State<AppState>) -> Json<Value> {
    Json(crate::boxes::vision::wire_map(
        &state.repo_root,
        &state.data_dir,
    ))
}

#[derive(serde::Deserialize)]
struct VisionFeedParams {
    status: Option<String>,
}

async fn api_vision_feed(
    State(state): State<AppState>,
    Query(params): Query<VisionFeedParams>,
) -> Json<Value> {
    Json(crate::boxes::vision::wire_feed_filter(
        &state.repo_root,
        &state.data_dir,
        params.status.as_deref(),
    ))
}

async fn api_vision_sprint_map(State(state): State<AppState>) -> Json<Value> {
    Json(crate::boxes::vision::wire_sprint_map(
        &state.repo_root,
        &state.data_dir,
    ))
}

#[derive(serde::Deserialize)]
struct VisionDocPreviewParams {
    id: Option<String>,
}

async fn api_vision_doc_preview(
    State(state): State<AppState>,
    Query(params): Query<VisionDocPreviewParams>,
) -> Json<Value> {
    let id = params.id.unwrap_or_default();
    Json(crate::boxes::vision::wire_doc_preview(
        &state.repo_root,
        &state.data_dir,
        &id,
    ))
}

async fn api_vision_sync(State(state): State<AppState>) -> Json<Value> {
    crate::boxes::usage::merge_omniroute_pull(&state).await;
    Json(crate::boxes::vision::wire_sync(
        &state.repo_root,
        &state.data_dir,
    ))
}

#[derive(serde::Deserialize)]
struct VisionNodeSearchParams {
    q: Option<String>,
    layer: Option<String>,
}

async fn api_vision_node_search(
    State(state): State<AppState>,
    Query(params): Query<VisionNodeSearchParams>,
) -> Json<Value> {
    Json(crate::boxes::vision::wire_node_search(
        &state.repo_root,
        &state.data_dir,
        params.q.as_deref().unwrap_or_default(),
        params.layer.as_deref(),
    ))
}

async fn api_vision_extensions(State(state): State<AppState>) -> Json<Value> {
    Json(crate::boxes::vision::wire_extensions(
        &state.repo_root,
        &state.data_dir,
    ))
}

async fn api_vision_sprint_queue(State(state): State<AppState>) -> Json<Value> {
    Json(crate::boxes::vision::wire_sprint_queue(
        &state.repo_root,
        &state.data_dir,
    ))
}

async fn api_vision_sprint_theme(State(state): State<AppState>) -> Json<Value> {
    Json(crate::boxes::vision::wire_sprint_theme(
        &state.repo_root,
        &state.data_dir,
    ))
}

async fn api_vision_palette(State(state): State<AppState>) -> Json<Value> {
    Json(crate::boxes::vision::wire_palette(
        &state.repo_root,
        &state.data_dir,
    ))
}

async fn api_vision_sprint_board(State(state): State<AppState>) -> Json<Value> {
    Json(crate::boxes::vision::wire_sprint_board(
        &state.repo_root,
        &state.data_dir,
    ))
}

async fn api_vision_sprint_progress(State(state): State<AppState>) -> Json<Value> {
    Json(crate::boxes::vision::wire_sprint_progress(
        &state.repo_root,
        &state.data_dir,
    ))
}

async fn api_vision_speeds(State(state): State<AppState>) -> Json<Value> {
    Json(crate::boxes::vision::wire_speed_index(
        &state.repo_root,
        &state.data_dir,
    ))
}

async fn api_vision_rust_diagnostics(State(state): State<AppState>) -> Json<Value> {
    Json(crate::boxes::vision::wire_rust_diagnostics(
        &state.repo_root,
        &state.data_dir,
    ))
}

async fn api_vision_speeds_svg(State(state): State<AppState>) -> Response {
    (
        StatusCode::OK,
        [
            ("Content-Type", "image/svg+xml"),
            ("Cache-Control", "no-cache"),
        ],
        crate::boxes::vision::speed_index_chart_svg(&state.repo_root, &state.data_dir),
    )
        .into_response()
}

async fn api_vision_rust_diagnostics_svg(State(state): State<AppState>) -> Response {
    (
        StatusCode::OK,
        [
            ("Content-Type", "image/svg+xml"),
            ("Cache-Control", "no-cache"),
        ],
        crate::boxes::vision::rust_diagnostics_chart_svg(&state.repo_root, &state.data_dir),
    )
        .into_response()
}

#[derive(serde::Deserialize)]
struct VisionSprintFocusParams {
    sprint: Option<String>,
}

async fn api_vision_sprint_focus_svg(
    State(state): State<AppState>,
    Query(params): Query<VisionSprintFocusParams>,
) -> Response {
    let sprint = params.sprint.as_deref().unwrap_or_default();
    (
        StatusCode::OK,
        [
            ("Content-Type", "image/svg+xml"),
            ("Cache-Control", "no-cache"),
        ],
        crate::boxes::vision::sprint_focus_svg(&state.repo_root, &state.data_dir, sprint),
    )
        .into_response()
}

#[derive(serde::Deserialize)]
struct VisionStarfieldParams {
    mode: Option<String>,
}

async fn api_vision_starfield_svg(
    State(state): State<AppState>,
    Query(params): Query<VisionStarfieldParams>,
) -> Response {
    (
        StatusCode::OK,
        [
            ("Content-Type", "image/svg+xml"),
            ("Cache-Control", "no-cache"),
        ],
        crate::boxes::vision::starfield_svg_wire(
            &state.repo_root,
            &state.data_dir,
            params.mode.as_deref(),
        ),
    )
        .into_response()
}

async fn api_vision_galaxy_svg() -> Response {
    (
        StatusCode::OK,
        [
            ("Content-Type", "image/svg+xml"),
            ("Cache-Control", "no-cache"),
        ],
        crate::boxes::vision::galaxy_svg(),
    )
        .into_response()
}

async fn api_vision_theme_svg() -> Response {
    (
        StatusCode::OK,
        [
            ("Content-Type", "image/svg+xml"),
            ("Cache-Control", "no-cache"),
        ],
        crate::boxes::vision::galaxy_svg(),
    )
        .into_response()
}

async fn api_vision_sprint_priority(State(state): State<AppState>) -> Json<Value> {
    let queue = crate::boxes::vision::wire_sprint_queue(&state.repo_root, &state.data_dir);
    let next = queue.get("next_sprint").cloned().unwrap_or_default();
    let active = queue.get("active_sprint").cloned().unwrap_or_default();
    let open = queue.get("open_count").cloned().unwrap_or_default();
    Json(json!({
        "priority": ["critical", "high", "medium", "low"],
        "next_sprint": next,
        "active_sprint": active,
        "open_count": open,
        "generated_at": vision::rfc3339_now(),
    }))
}

async fn api_vision_tracker(State(state): State<AppState>) -> Json<Value> {
    Json(tracker_wire(&state))
}

async fn api_vision_events(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "ok": true,
        "stream": "/events",
        "subscribers": state.events.receiver_count(),
        "keepalive_secs": 15,
        "generated_at": vision::rfc3339_now(),
    }))
}

async fn api_vision_ide_session(State(state): State<AppState>) -> Json<Value> {
    let selection = state.ide_selection.try_read().ok().and_then(|s| s.clone());
    Json(json!({
        "ok": true,
        "selection": selection,
        "generated_at": vision::rfc3339_now(),
    }))
}

async fn api_vision_control_status(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "ok": true,
        "server": crate::GSV_SERVER_NAME,
        "version": *state.version,
        "uptime_secs": state.started_at.elapsed().map(|d| d.as_secs()).unwrap_or(0),
        "generated_at": vision::rfc3339_now(),
    }))
}

async fn api_omni_status(State(state): State<AppState>) -> Json<Value> {
    let selected = product_selected_id(&state);
    Json(json!(
        crate::boxes::omni::wire(&state.omni, selected.as_deref()).await
    ))
}

#[derive(serde::Deserialize)]
struct OmniRouteQuery {
    task: Option<String>,
    prefer_free: Option<bool>,
}

async fn api_omni_route(
    State(state): State<AppState>,
    Query(q): Query<OmniRouteQuery>,
) -> Json<Value> {
    let task = q.task.as_deref().unwrap_or("rust");
    let prefer_free = q.prefer_free.unwrap_or(true);
    Json(crate::boxes::omni::route_wire(&state.omni, task, prefer_free).await)
}

fn toolchain_entry(wire: &Value, tool: &str) -> Value {
    wire.get("entries")
        .and_then(Value::as_array)
        .and_then(|es| {
            es.iter()
                .find(|e| e.get("tool").and_then(Value::as_str).unwrap_or("") == tool)
                .cloned()
        })
        .unwrap_or(Value::Null)
}

async fn api_toolchain_rustc(State(state): State<AppState>) -> Json<Value> {
    let wire = json!(toolchain::wire(&state.repo_root));
    Json(json!({ "ok": true, "tool": "rustc", "entry": toolchain_entry(&wire, "rustc") }))
}

async fn api_toolchain_cargo(State(state): State<AppState>) -> Json<Value> {
    let wire = json!(toolchain::wire(&state.repo_root));
    Json(json!({ "ok": true, "tool": "cargo", "entry": toolchain_entry(&wire, "cargo") }))
}

async fn api_toolchain_clippy(State(state): State<AppState>) -> Json<Value> {
    let wire = json!(toolchain::wire(&state.repo_root));
    Json(
        json!({ "ok": true, "tool": "clippy-driver", "entry": toolchain_entry(&wire, "clippy-driver") }),
    )
}

async fn api_toolchain_detailed(State(state): State<AppState>) -> Json<Value> {
    Json(json!({ "ok": true, "toolchain": toolchain::wire(&state.repo_root) }))
}

fn spawn_cargo(args: &[&str], repo_root: &std::path::Path) -> Response {
    match crate::vision::command("cargo")
        .args(args)
        .current_dir(repo_root)
        .spawn()
    {
        Ok(child) => {
            Json(json!({ "ok": true, "started": true, "pid": child.id() })).into_response()
        }
        Err(e) => err_json(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

async fn api_toolchain_build(State(state): State<AppState>) -> Response {
    spawn_cargo(&["build"], &state.repo_root)
}

async fn api_toolchain_test(State(state): State<AppState>) -> Response {
    spawn_cargo(&["test"], &state.repo_root)
}

async fn api_toolchain_clean(State(state): State<AppState>) -> Response {
    spawn_cargo(&["clean"], &state.repo_root)
}

async fn api_vision_resync(State(state): State<AppState>) -> Json<Value> {
    state.emit("event: resync\ndata: requested".to_string());
    Json(json!({ "ok": true, "action": "resync", "generated_at": vision::rfc3339_now() }))
}

async fn api_vision_set_offline(State(state): State<AppState>) -> Json<Value> {
    let offline = true;
    state.emit(format!("event: offline\ndata: {offline}"));
    Json(json!({ "ok": true, "offline": offline, "generated_at": vision::rfc3339_now() }))
}

async fn api_vision_reload(State(state): State<AppState>) -> Json<Value> {
    state.emit("event: reload\ndata: requested".to_string());
    Json(json!({ "ok": true, "action": "reload", "generated_at": vision::rfc3339_now() }))
}

async fn api_vision_snapshot(State(state): State<AppState>) -> Json<Value> {
    let mut tracker = TrackerStore::load(&state.repo_root, &state.data_dir).unwrap_or_default();
    let _ = tracker.push(
        &state.data_dir,
        TrackerRecord::new(
            "snapshot",
            "manual snapshot",
            format!("at={}", vision::rfc3339_now()),
            "closed",
        ),
    );
    state.emit("event: snapshot\ndata: taken".to_string());
    Json(json!({ "ok": true, "action": "snapshot", "generated_at": vision::rfc3339_now() }))
}

async fn api_vision_shutdown(State(state): State<AppState>) -> Json<Value> {
    state.emit("event: shutdown\ndata: requested".to_string());
    tokio::spawn(async {
        tokio::time::sleep(Duration::from_millis(200)).await;
        std::process::exit(0);
    });
    Json(json!({ "ok": true, "action": "shutdown", "generated_at": vision::rfc3339_now() }))
}

async fn api_vision_restart(State(state): State<AppState>) -> Json<Value> {
    state.emit("event: restart\ndata: requested".to_string());
    let exe = std::env::current_exe().unwrap_or_default();
    let dir = std::env::current_dir().unwrap_or_default();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        let _ = crate::vision::command(&exe).current_dir(&dir).spawn();
        std::process::exit(0);
    });
    Json(json!({ "ok": true, "action": "restart", "generated_at": vision::rfc3339_now() }))
}

async fn api_ide_opencode() -> Json<Value> {
    let sessions = crate::boxes::ide::discover();
    let opencode: Vec<_> = sessions
        .iter()
        .filter(|s| s.tool == "opencode")
        .cloned()
        .collect();
    Json(json!({ "ok": true, "tool": "opencode", "sessions": opencode }))
}

async fn api_ide_cursor() -> Json<Value> {
    let sessions = crate::boxes::ide::discover();
    let cursor: Vec<_> = sessions
        .iter()
        .filter(|s| s.tool == "cursor")
        .cloned()
        .collect();
    Json(json!({ "ok": true, "tool": "cursor", "sessions": cursor }))
}

async fn api_ide_pending_rebuild() -> Json<Value> {
    Json(json!({ "ok": true, "pending_rebuild": false, "reason": null }))
}

async fn api_ide_active_session(State(state): State<AppState>) -> Json<Value> {
    let selection = state.ide_selection.try_read().ok().and_then(|s| s.clone());
    Json(json!({ "ok": true, "selection": selection }))
}

async fn api_ide_session_history() -> Json<Value> {
    Json(json!({ "ok": true, "sessions": Vec::<Value>::new() }))
}

async fn api_data_file(State(state): State<AppState>, Path(file): Path<String>) -> Response {
    let safe = match crate::security::data_file_name(&file) {
        Ok(name) => name,
        Err(e) => return err_json(StatusCode::BAD_REQUEST, e),
    };
    let path = state.data_dir.join(&safe);
    match tokio::fs::read_to_string(&path).await {
        Ok(content) => (
            StatusCode::OK,
            [
                ("Content-Type", "application/json"),
                ("Cache-Control", "no-cache"),
            ],
            content,
        )
            .into_response(),
        Err(_) => err_json(StatusCode::NOT_FOUND, format!("missing data file: {safe}")),
    }
}

// ── OmniRouter box ─────────────────────────────────────────────────────────────

async fn api_omni(State(state): State<AppState>) -> Json<Value> {
    let selected = product_selected_id(&state);
    Json(json!(
        crate::boxes::omni::wire(&state.omni, selected.as_deref()).await
    ))
}

async fn api_omni_config(State(state): State<AppState>) -> Json<Value> {
    Json(state.omni.config.read().await.redacted())
}

async fn api_omni_config_post(
    State(state): State<AppState>,
    Json(patch): Json<Value>,
) -> Json<Value> {
    let applied = {
        let mut cfg = state.omni.config.write().await;
        cfg.apply(&patch)
    };
    match applied {
        Ok(()) => {
            state.omni.persist();
            state.emit("event: omni_config\ndata: changed".to_string());
            Json(json!({
                "ok": true,
                "config": state.omni.config.read().await.redacted(),
            }))
        }
        Err(msg) => Json(json!({
            "ok": false,
            "error": msg,
        })),
    }
}

async fn api_omni_v1_models(State(state): State<AppState>) -> Response {
    crate::boxes::omni::proxy::v1_models(&state.omni)
        .await
        .unwrap_or_else(api_error_response)
}

async fn api_omni_chat(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response {
    crate::boxes::omni::proxy::chat_completions(&state, &headers, &body)
        .await
        .unwrap_or_else(api_error_response)
}

async fn api_omni_test(State(state): State<AppState>, Json(body): Json<Value>) -> Response {
    let provider = body
        .get("provider")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if provider.is_empty() {
        return err_json(StatusCode::BAD_REQUEST, "provider required");
    }
    match crate::boxes::omni::proxy::test_provider(&state.omni, provider).await {
        Ok(res) => Json(json!({ "ok": true, "result": res })).into_response(),
        Err(e) => err_json(StatusCode::BAD_REQUEST, e.message()),
    }
}

/// Convert an `AppError` into a 400 JSON response (route/config errors).
fn api_error_response(err: crate::AppError) -> Response {
    err_json(StatusCode::BAD_REQUEST, err.message())
}

/// Server-Sent Events stream: broadcasts state events + periodic keepalive.
async fn events(
    State(state): State<AppState>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
    let rx = state.events.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|item| async move {
        match item {
            Ok(payload) => Some(Ok(Event::default().data(payload))),
            Err(_) => None, // lagged behind — skip, client reconnects
        }
    });
    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keepalive"),
    )
}
