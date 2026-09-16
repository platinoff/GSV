use crate::state::AppState;
use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

pub mod miniapp;
pub mod webgpu;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(dashboard))
        .route("/app", get(app_page))
        .route("/board", get(board_page))
        .route("/flows", get(flows_page))
        .route("/roles", get(roles_page))
        .route("/workers", get(workers_page))
        .route("/api/edge/workers", get(api_edge_workers))
        .route("/vm", get(vm_page))
        .route("/api/edge/vm", get(api_edge_vm))
        .route("/api/edge/vm/ensure", post(api_edge_vm_ensure))
        .route("/api/edge/shards", get(api_edge_shards))
        .route("/chat", get(chat_page))
        .route("/probe", get(probe_page))
        .route("/tensor", get(tensor_page))
        .route(
            "/api/edge/chat",
            get(api_edge_chat_result).post(api_edge_chat_send),
        )
        .route("/api/edge/endpoints", get(api_edge_endpoints))
        .route("/api/edge/webgpu", post(api_edge_webgpu_submit))
        .route(
            "/edge/upstream/{service}/{*tail}",
            get(proxy_upstream).post(proxy_upstream),
        )
        .route("/health", get(health))
        .route("/api/status", get(status))
        .route("/api/tickets", get(api_tickets))
        .route("/api/roles", get(api_roles).post(api_roles_assign))
        .route("/api/roles/remove", post(api_roles_remove))
        .route("/api/flows", get(api_flows))
        .route("/static/app.css", get(serve_css))
        .route("/static/app.js", get(serve_js))
        .route("/static/probe.js", get(serve_probe_js))
        .route("/static/tensor.js", get(serve_tensor_js))
        .route("/api/verify", get(api_verify_init_data))
        .route("/api/mini-app/i18n", get(api_mini_app_i18n))
        .route("/api/live/config", get(api_live_config))
        .route("/api/snapshot", get(api_snapshot))
        .route("/api/board/claim", post(api_board_claim))
        .route("/api/board/done", post(api_board_done))
        .route("/api/board/error", post(api_board_error))
        .route("/api/board/reclaim", post(api_board_reclaim))
        .with_state(state)
}

/// Resolve the Mini App UI strings for a requested language. The client asks
/// with its `initDataUnsafe.user.language_code`; unknown codes fall back to
/// English via the table in [`miniapp`].
#[derive(Deserialize)]
struct I18nQuery {
    lang: Option<String>,
}

async fn api_mini_app_i18n(Query(q): Query<I18nQuery>) -> Json<serde_json::Value> {
    let lang = miniapp::Lang::parse(q.lang.as_deref().unwrap_or("en"));
    let mut strings = serde_json::Map::new();
    for key in miniapp::I18N_KEYS {
        strings.insert((*key).to_string(), json!(miniapp::t(key, lang)));
    }
    Json(json!({
        "lang": lang.as_str(),
        "strings": serde_json::Value::Object(strings),
    }))
}

/// Server-authoritative live-stream config for the Mini App JS client
/// (plan P2). Mirrors [`crate::stream::backoff`] so reconnecting clients wait
/// the same exponential-backoff schedule the Rust server defines and tests.
async fn api_live_config(State(state): State<AppState>) -> Json<serde_json::Value> {
    let backoff = state.live_reconnect();
    Json(json!({
        "reconnect": {
            "base_ms": backoff.base_ms,
            "cap_ms": backoff.cap_ms,
            "max_attempts": backoff.max_attempts,
        },
        "keepalive_secs": crate::stream::backoff::WS_KEEPALIVE_SECS,
    }))
}

#[derive(Deserialize)]
struct SnapshotQuery {
    lang: Option<String>,
}

/// Consolidated cold-start bundle (plan P3). The Mini App fetches this once on
/// `/start` and hydrates every region of the app from a single round-trip
/// instead of issuing status + tickets + flows + workers + i18n + live-config
/// requests sequentially — that is what makes the first paint fast in a cold
/// Telegram WebView. The optional `?lang=` selects the i18n table (en default).
async fn api_snapshot(
    State(state): State<AppState>,
    Query(query): Query<SnapshotQuery>,
) -> Json<serde_json::Value> {
    let lang = miniapp::Lang::parse(query.lang.as_deref().unwrap_or("en"));
    let tickets = state.tickets().await;
    let presence = state.presence_map().await;
    let flows = state.recent_flows(50).await;
    let backoff = state.live_reconnect();
    let roles = state.list_roles().await;

    let mut strings = serde_json::Map::new();
    for key in miniapp::I18N_KEYS {
        strings.insert((*key).to_string(), json!(miniapp::t(key, lang)));
    }

    Json(json!({
        "v": 1,
        "ts": chrono::Utc::now().timestamp_millis(),
        "status": {
            "online": state.is_online(),
            "jail_id": state.jail_id(),
            "tickets_count": tickets.len(),
            "workers_online": presence.len(),
            "recent_flows": flows.len(),
        },
        "tickets": wire_tickets(&tickets),
        "workers": wire_workers(&presence),
        "roles": wire_roles(&roles),
        "flows": wire_flows(&flows),
        "i18n": {"lang": lang.as_str(), "strings": serde_json::Value::Object(strings)},
        "live": {
            "reconnect": {
                "base_ms": backoff.base_ms,
                "cap_ms": backoff.cap_ms,
                "max_attempts": backoff.max_attempts,
            },
            "keepalive_secs": crate::stream::backoff::WS_KEEPALIVE_SECS,
        },
    }))
}

/// Wire ticket rows for the JSON surface; the router, board rows and snapshot
/// all share the same shape so the Mini App can render one `<tr>` template.
/// `actions` is the server-authoritative, status-driven set of board actions
/// the row may offer (band 219) — the Mini App renders exactly these buttons,
/// never a fixed claim/done/error triad.
fn wire_tickets(tickets: &[crate::state::TicketRow]) -> Vec<serde_json::Value> {
    tickets
        .iter()
        .map(|t| {
            let actions: Vec<&str> = crate::actions::available_actions(&t.status)
                .iter()
                .map(|a| a.as_str())
                .collect();
            json!({
                "id": t.id,
                "title": t.title,
                "body": t.body,
                "status": t.status,
                "product": t.product,
                "claimed_by": t.claimed_by,
                "scenario": t.scenario,
                "actions": actions,
            })
        })
        .collect()
}

fn wire_workers(
    presence: &std::collections::HashMap<String, crate::state::WorkerPresence>,
) -> Vec<serde_json::Value> {
    let mut rows: Vec<serde_json::Value> = presence
        .values()
        .map(|w| {
            let status_str = match w.status {
                crate::state::WorkerStatus::Ready => "ready",
                crate::state::WorkerStatus::Busy => "busy",
                crate::state::WorkerStatus::Offline => "offline",
            };
            json!({
                "jail_id": w.jail_id,
                "actor": w.actor,
                "ide": w.ide,
                "model": w.model,
                "agent": w.agent,
                "rank": w.rank,
                "status": status_str,
                "timezone": w.timezone,
            })
        })
        .collect();
    rows.sort_by(|a, b| a["jail_id"].as_str().cmp(&b["jail_id"].as_str()));
    rows
}

fn wire_roles(roles: &[crate::roles::store::RoleEntry]) -> Vec<serde_json::Value> {
    let mut rows: Vec<serde_json::Value> = roles
        .iter()
        .map(|r| {
            json!({
                "jail_id": r.jail_id,
                "role": r.role.as_str(),
                "assigned_at": r.assigned_at,
            })
        })
        .collect();
    rows.sort_by(|a, b| a["jail_id"].as_str().cmp(&b["jail_id"].as_str()));
    rows
}

fn wire_flows(flows: &[crate::state::FlowEvent]) -> Vec<serde_json::Value> {
    flows
        .iter()
        .map(|f| {
            json!({
                "ts": f.ts.to_rfc3339(),
                "jail_id": f.jail_id,
                "action": f.action,
                "detail": f.detail,
            })
        })
        .collect()
}

#[derive(Deserialize)]
struct VerifyQuery {
    #[serde(rename = "initData", default)]
    init_data: String,
    #[serde(rename = "authDate", default)]
    auth_date: Option<i64>,
}

/// Resolve the "now" anchor for the initData freshness gate.
///
/// Production MUST anchor on the server clock so a captured `initData` (rides
/// the Mini App URL through browser history / logs) cannot be refreshed by a
/// client-supplied `authDate` that zeroes its age. The `authDate` query param
/// is kept only as a test-only clock override and is ignored outside `cfg(test)`.
fn freshness_now(client_auth_date: Option<i64>) -> i64 {
    #[cfg(test)]
    {
        client_auth_date.unwrap_or_else(|| chrono::Utc::now().timestamp())
    }
    #[cfg(not(test))]
    {
        let _ = client_auth_date;
        chrono::Utc::now().timestamp()
    }
}

/// Server-side verification surface for the Telegram Mini App handshake.
/// The server HMAC-SHA256 verifies the signature against the bot token and
/// enforces `auth_date` freshness against the **server** clock, returning
/// `{ok, error?}` so the Mini App can decide whether to trust requests.
async fn api_verify_init_data(
    State(state): State<AppState>,
    Query(query): Query<VerifyQuery>,
) -> Json<serde_json::Value> {
    let token = &state.config().bot_token;
    if token.is_empty() {
        return Json(json!({"ok": false, "error": "bot token not configured"}));
    }
    if query.init_data.is_empty() {
        return Json(json!({"ok": false, "error": "no initData"}));
    }
    let now = freshness_now(query.auth_date);
    match crate::security::verify_init_data(
        &query.init_data,
        token,
        now,
        crate::security::initdata::DEFAULT_MAX_AGE_SECS,
    ) {
        Ok(()) => Json(json!({"ok": true})),
        Err(e) => Json(json!({"ok": false, "error": e.to_string()})),
    }
}

/// Query params for the board-action surface: the Telegram `initData`
/// handshake string (band 214) plus an optional `authDate` clock override that
/// mirrors `/api/verify`, so the freshness window is testable.
#[derive(Deserialize)]
struct ActionQuery {
    #[serde(rename = "initData", default)]
    init_data: String,
    #[serde(rename = "authDate", default)]
    auth_date: Option<i64>,
}

async fn api_board_claim(
    State(state): State<AppState>,
    Query(q): Query<ActionQuery>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    api_board_action(crate::actions::BoardAction::Claim, state, q, body).await
}

async fn api_board_done(
    State(state): State<AppState>,
    Query(q): Query<ActionQuery>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    api_board_action(crate::actions::BoardAction::Done, state, q, body).await
}

async fn api_board_error(
    State(state): State<AppState>,
    Query(q): Query<ActionQuery>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    api_board_action(crate::actions::BoardAction::Error, state, q, body).await
}

async fn api_board_reclaim(
    State(state): State<AppState>,
    Query(q): Query<ActionQuery>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    api_board_action(crate::actions::BoardAction::Reclaim, state, q, body).await
}

/// Shared board-action handler (band 218, plan P4). Verifies the Telegram
/// Mini App `initData` handshake before anything is mutated — a public tunnel
/// must never let an anonymous caller claim/close tickets — then parses
/// `{id, note?}` and forwards server-side to GSV's `/api/tickets/{verb}`.
async fn api_board_action(
    action: crate::actions::BoardAction,
    state: AppState,
    q: ActionQuery,
    body: serde_json::Value,
) -> Response {
    let token = &state.config().bot_token;
    if token.is_empty() {
        return (
            StatusCode::BAD_GATEWAY,
            Json(crate::actions::err_json("bot token not configured")),
        )
            .into_response();
    }
    let now = freshness_now(q.auth_date);
    if let Err(e) = crate::security::verify_init_data(
        &q.init_data,
        token,
        now,
        crate::security::initdata::DEFAULT_MAX_AGE_SECS,
    ) {
        return (
            StatusCode::FORBIDDEN,
            Json(crate::actions::err_json(&format!("initData: {e}"))),
        )
            .into_response();
    }
    let parsed = match crate::actions::parse_body(&body) {
        Ok(p) => p,
        Err(message) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(crate::actions::err_json(&message)),
            )
                .into_response();
        }
    };
    let client = crate::gsv::client::GsvClient::new(state.config());
    match client
        .board_action(action, &parsed.id, parsed.note.as_deref())
        .await
    {
        Ok(v) => Json(v).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(crate::actions::err_json(&format!("GSV: {e}"))),
        )
            .into_response(),
    }
}

/// GET /api/roles — read-only role directory (same shape as the snapshot
/// `roles` region, sorted by jail_id). No auth: reading the directory is not
/// a mutation, mirroring `/api/status`.
async fn api_roles(State(state): State<AppState>) -> Json<serde_json::Value> {
    let roles = state.list_roles().await;
    Json(json!({ "ok": true, "roles": wire_roles(&roles) }))
}

/// POST /api/roles — assign (or overwrite) a role for a jail. Mutating, so it
/// verifies the Telegram `initData` handshake exactly like the board actions.
/// Body: `{"jail_id": "...", "role": "host|mate|guest|observer"}`.
async fn api_roles_assign(
    State(state): State<AppState>,
    Query(q): Query<ActionQuery>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let token = &state.config().bot_token;
    if token.is_empty() {
        return (
            StatusCode::BAD_GATEWAY,
            Json(crate::actions::err_json("bot token not configured")),
        )
            .into_response();
    }
    let now = freshness_now(q.auth_date);
    if let Err(e) = crate::security::verify_init_data(
        &q.init_data,
        token,
        now,
        crate::security::initdata::DEFAULT_MAX_AGE_SECS,
    ) {
        return (
            StatusCode::FORBIDDEN,
            Json(crate::actions::err_json(&format!("initData: {e}"))),
        )
            .into_response();
    }
    let jail_id = match body.get("jail_id").and_then(serde_json::Value::as_str) {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(crate::actions::err_json("missing jail_id")),
            )
                .into_response()
        }
    };
    let role = match body.get("role").and_then(serde_json::Value::as_str) {
        Some(s) => match crate::roles::store::Role::parse(s) {
            Some(r) => r,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(crate::actions::err_json(
                        "invalid role: expected host|mate|guest|observer",
                    )),
                )
                    .into_response();
            }
        },
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(crate::actions::err_json("missing role")),
            )
                .into_response();
        }
    };
    let entry = state.assign_role(&jail_id, role).await;
    Json(json!({
        "ok": true,
        "role": {
            "jail_id": entry.jail_id,
            "role": entry.role.as_str(),
            "assigned_at": entry.assigned_at,
        },
    }))
    .into_response()
}

/// POST /api/roles/remove — revoke a jail's role. Mutating, initData-checked.
/// Body: `{"jail_id": "..."}`.
async fn api_roles_remove(
    State(state): State<AppState>,
    Query(q): Query<ActionQuery>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let token = &state.config().bot_token;
    if token.is_empty() {
        return (
            StatusCode::BAD_GATEWAY,
            Json(crate::actions::err_json("bot token not configured")),
        )
            .into_response();
    }
    let now = freshness_now(q.auth_date);
    if let Err(e) = crate::security::verify_init_data(
        &q.init_data,
        token,
        now,
        crate::security::initdata::DEFAULT_MAX_AGE_SECS,
    ) {
        return (
            StatusCode::FORBIDDEN,
            Json(crate::actions::err_json(&format!("initData: {e}"))),
        )
            .into_response();
    }
    let jail_id = match body.get("jail_id").and_then(serde_json::Value::as_str) {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(crate::actions::err_json("missing jail_id")),
            )
                .into_response();
        }
    };
    let had = state.get_role(&jail_id).await.is_some();
    state.remove_role(&jail_id).await;
    Json(json!({ "ok": true, "removed": had, "jail_id": jail_id })).into_response()
}

/// HTML page with `Cache-Control: no-store` — Telegram WebView caches
/// aggressively; without it phones keep showing stale pages (old JS that
/// hangs on skeleton forever) after a redeploy.
fn page(html: String) -> ([(axum::http::HeaderName, &'static str); 1], Html<String>) {
    ([(header::CACHE_CONTROL, "no-store")], Html(html))
}

async fn dashboard() -> impl IntoResponse {
    page(include_str!("templates/dashboard.html").to_string())
}

async fn app_page() -> impl IntoResponse {
    page(include_str!("templates/base.html").to_string())
}

async fn board_page() -> impl IntoResponse {
    page(include_str!("templates/board.html").to_string())
}

async fn flows_page() -> impl IntoResponse {
    page(include_str!("templates/flows.html").to_string())
}

async fn roles_page() -> impl IntoResponse {
    page(include_str!("templates/roles.html").to_string())
}

/// Minimal HTML escaping for server-rendered rows.
fn esc_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Server-rendered edge-workers rows so the table has data even when the
/// phone's JS never runs. Client JS overwrites on successful fetch.
async fn workers_rows(state: &AppState) -> String {
    use crate::edge::{apply_task_status, assemble_views, PoolClient};

    let pool = PoolClient::new(state.config());
    let bindings = match pool.bindings().await {
        Ok(b) => b,
        Err(e) => {
            return format!(
                "<tr><td colspan=\"5\">poolAI unreachable: {}</td></tr>",
                esc_html(&e.to_string())
            )
        }
    };
    let mut views = assemble_views(&bindings);
    for v in &mut views {
        if let Ok(st) = pool.task_status(&v.peer_id).await {
            apply_task_status(v, &st);
        }
    }
    if views.is_empty() {
        return "<tr><td colspan=\"5\">(none bound)</td></tr>".to_string();
    }
    views
        .iter()
        .map(|v| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                esc_html(&v.telegram_user_id),
                esc_html(&v.peer_id),
                esc_html(&v.bound_at),
                v.pending,
                v.completed
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

async fn workers_page(State(state): State<AppState>) -> impl IntoResponse {
    let rows = workers_rows(&state).await;
    page(
        include_str!("templates/workers.html")
            .to_string()
            .replace("<!--WORKERS_ROWS-->", &rows),
    )
}

/// Server-rendered VM + shard rows (same no-JS guarantee as workers).
async fn vm_rows(state: &AppState) -> (String, String) {
    use crate::edge::{assemble_views, find_vm, shard_map, PoolClient};

    let pool = PoolClient::new(state.config());
    let bindings = match pool.bindings().await {
        Ok(b) => b,
        Err(e) => {
            let row = format!(
                "<tr><td colspan=\"4\">poolAI unreachable: {}</td></tr>",
                esc_html(&e.to_string())
            );
            return (row.clone(), row);
        }
    };
    let views = assemble_views(&bindings);
    let peers: Vec<String> = views.iter().map(|v| v.peer_id.clone()).collect();
    let shards = shard_map(&pool, &peers).await;
    let vms = pool.vm_list().await.ok();
    let shard_rows = if shards.is_empty() {
        "<tr><td colspan=\"4\">(no assignments yet)</td></tr>".to_string()
    } else {
        shards
            .iter()
            .map(|s| {
                format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                    esc_html(&s.peer_id),
                    esc_html(&s.model),
                    esc_html(&s.layers),
                    if s.rpc_reachable { "yes" } else { "no" }
                )
            })
            .collect::<Vec<_>>()
            .join("")
    };
    let mut vm_rows = String::new();
    if views.is_empty() {
        vm_rows = "<tr><td colspan=\"4\">(none bound — /start, then bind)</td></tr>".to_string();
    }
    for v in &views {
        let vm = vms
            .as_ref()
            .and_then(|l| find_vm(l, &crate::edge::vm_name_for_peer(&v.peer_id)));
        match vm {
            Some(m) => {
                let id = m
                    .get("id")
                    .and_then(|x| x.as_str())
                    .unwrap_or("?")
                    .to_string();
                let status = m
                    .get("status")
                    .and_then(|x| x.as_str())
                    .unwrap_or("?")
                    .to_string();
                let mut health = "n/a".to_string();
                if let Ok(h) = pool.vm_health(&id).await {
                    if let Some(s) = h.get("status").and_then(|x| x.as_str()) {
                        health = s.to_string();
                    }
                }
                let short: String = id.chars().take(8).collect();
                vm_rows.push_str(&format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                    esc_html(&v.peer_id),
                    esc_html(&short),
                    esc_html(&status),
                    esc_html(&health)
                ));
            }
            None => vm_rows.push_str(&format!(
                "<tr><td>{}</td><td>(no VM)</td><td>—</td><td>—</td></tr>",
                esc_html(&v.peer_id)
            )),
        }
    }
    (shard_rows, vm_rows)
}

async fn vm_page(State(state): State<AppState>) -> impl IntoResponse {
    let (shards, vms) = vm_rows(&state).await;
    page(
        include_str!("templates/vm.html")
            .to_string()
            .replace("<!--SHARDS_ROWS-->", &shards)
            .replace("<!--VMS_ROWS-->", &vms),
    )
}

async fn chat_page() -> impl IntoResponse {
    page(include_str!("templates/chat.html").to_string())
}

/// WebGPU probe page (swarm browser path): one tap runs
/// `navigator.gpu.requestAdapter()` on the phone and registers a
/// `class=webgpu` hub profile under the caller's bound peer.
async fn probe_page() -> impl IntoResponse {
    page(include_str!("templates/probe.html").to_string())
}

/// Browser tensor worker page (swarm path b PoC): loads a GGUF in the
/// phone via wllama and serves `llama_chat` tasks from the bound peer.
async fn tensor_page() -> impl IntoResponse {
    page(include_str!("templates/tensor.html").to_string())
}

/// Layer map: latest `llama_shard` assignment per bound peer.
/// Source of truth for who holds which layers (tensors follow separately).
async fn api_edge_shards(State(state): State<AppState>) -> Json<serde_json::Value> {
    use crate::edge::{assemble_views, shard_map, PoolClient};

    let pool = PoolClient::new(state.config());
    let bindings = match pool.bindings().await {
        Ok(b) => b,
        Err(e) => return Json(json!({"ok": false, "error": e.to_string()})),
    };
    let peers: Vec<String> = assemble_views(&bindings)
        .into_iter()
        .map(|v| v.peer_id)
        .collect();
    let map = shard_map(&pool, &peers).await;
    let shards: Vec<serde_json::Value> = map
        .iter()
        .map(|a| {
            json!({
                "peer_id": a.peer_id,
                "task_id": a.task_id,
                "model": a.model,
                "layers": a.layers,
                "rpc_reachable": a.rpc_reachable,
            })
        })
        .collect();
    Json(json!({"ok": true, "shards": shards}))
}

/// Browser WebGPU probe submit: `POST /api/edge/webgpu?initData=…&authDate=…`
/// body `{user, probe}`. Mutating (writes a durable hub profile), so the
/// Telegram `initData` handshake is required like the board actions. The
/// probe is validated by [`webgpu::parse_probe`], keyed by the caller's
/// bound poolAI peer, and forwarded as a `class=webgpu` patch to GSV
/// `POST /api/grid/profile`. Unsupported adapters short-circuit with
/// `{ok, unsupported}` and no profile write.
async fn api_edge_webgpu_submit(
    State(state): State<AppState>,
    Query(q): Query<ActionQuery>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    use crate::edge::PoolClient;

    let token = &state.config().bot_token;
    if token.is_empty() {
        return (
            StatusCode::BAD_GATEWAY,
            Json(crate::actions::err_json("bot token not configured")),
        )
            .into_response();
    }
    let now = freshness_now(q.auth_date);
    if let Err(e) = crate::security::verify_init_data(
        &q.init_data,
        token,
        now,
        crate::security::initdata::DEFAULT_MAX_AGE_SECS,
    ) {
        return (
            StatusCode::FORBIDDEN,
            Json(crate::actions::err_json(&format!("initData: {e}"))),
        )
            .into_response();
    }
    let user = body.get("user").and_then(Value::as_str).unwrap_or("");
    if user.trim().is_empty() {
        return Json(json!({"ok": false, "error": "user required"})).into_response();
    }
    let probe = match body.get("probe") {
        Some(p) => match crate::ui::webgpu::parse_probe(p) {
            Ok(pr) => pr,
            Err(e) => {
                return (StatusCode::BAD_REQUEST, Json(crate::actions::err_json(&e)))
                    .into_response()
            }
        },
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(crate::actions::err_json("missing probe")),
            )
                .into_response()
        }
    };
    if !probe.supported {
        return Json(json!({"ok": true, "unsupported": true})).into_response();
    }
    let pool = PoolClient::new(state.config());
    let peer = match pool.peer_for_user(user).await {
        Ok(Some(p)) => p,
        Ok(None) => return Json(json!({"ok": false, "error": "no bound peer"})).into_response(),
        Err(e) => return Json(json!({"ok": false, "error": e.to_string()})).into_response(),
    };
    let patch = crate::ui::webgpu::profile_patch(&peer, &probe);
    let gsv = crate::gsv::client::GsvClient::new(state.config());
    match gsv.grid_profile(&patch).await {
        Ok(v) => match webgpu_profile_row(&v) {
            Ok(row) => Json(json!({"ok": true, "peer": peer, "profile": row})).into_response(),
            Err(e) => (
                StatusCode::BAD_GATEWAY,
                Json(crate::actions::err_json(&format!("GSV: {e}"))),
            )
                .into_response(),
        },
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(crate::actions::err_json(&format!("GSV: {e}"))),
        )
            .into_response(),
    }
}

/// Unwrap the hub-profile row from a GSV `POST /api/grid/profile` answer
/// (`{ok, profile: row}`). GSV errors arrive as HTTP 200 with `ok: false`,
/// so the flag must be checked — otherwise the phone renders an empty
/// profile (`?` id, zero ram) for a write that never happened.
fn webgpu_profile_row(v: &serde_json::Value) -> Result<serde_json::Value, String> {
    if !v.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return Err(v
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("grid profile rejected")
            .to_string());
    }
    Ok(v.get("profile").cloned().unwrap_or_else(|| v.clone()))
}

/// Phone-reachable service map (loopback / LAN / via-Telenetis / public).
async fn api_edge_endpoints(State(state): State<AppState>) -> Json<serde_json::Value> {
    let public = state.tunnel_url().await;
    Json(crate::edge::service_endpoints(
        state.config(),
        public.as_deref(),
    ))
}

/// Reverse proxy so phones use ONE origin (works out-of-NAT through the
/// tunnel): `/edge/upstream/llama/<path>` → llama_serve,
/// `/edge/upstream/poolai/<path>` → poolAI. Anything else → 404.
/// poolAI calls carry the service token; llama needs none.
async fn proxy_upstream(
    State(state): State<AppState>,
    axum::extract::Path((service, tail)): axum::extract::Path<(String, String)>,
    axum::extract::RawQuery(query): axum::extract::RawQuery,
    method: axum::http::Method,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    use crate::edge::{upstream_base, PoolClient};

    let Some(base) = upstream_base(state.config(), &service) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"ok": false, "error": "unknown service"})),
        )
            .into_response();
    };
    if tail.split('/').any(|s| s == "..") {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": "bad path"})),
        )
            .into_response();
    }
    let mut url = format!("{}/{tail}", base.trim_end_matches('/'));
    if let Some(q) = query {
        url.push('?');
        url.push_str(&q);
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let mut req = match method.as_str() {
        "POST" => client.post(&url),
        "PUT" => client.put(&url),
        "DELETE" => client.delete(&url),
        _ => client.get(&url),
    };
    if service == "poolai" {
        let pool = PoolClient::new(state.config());
        match pool.bearer_token().await {
            Ok(t) => req = req.bearer_auth(t),
            Err(e) => {
                return (
                    StatusCode::BAD_GATEWAY,
                    Json(json!({"ok": false, "error": e.to_string()})),
                )
                    .into_response()
            }
        }
    }
    if !body.is_empty() {
        req = req
            .header("content-type", "application/json")
            .body(body.to_vec());
    }
    let upstream = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({"ok": false, "error": format!("upstream: {e}")})),
            )
                .into_response()
        }
    };
    let status = upstream.status();
    let ctype = upstream
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();
    // Buffered (not chunk-streamed): SSE still arrives as a valid
    // event-stream, just batched. Keeps reqwest without the stream feature.
    let bytes = match upstream.bytes().await {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({"ok": false, "error": format!("upstream body: {e}")})),
            )
                .into_response()
        }
    };
    (status, [(axum::http::header::CONTENT_TYPE, ctype)], bytes).into_response()
}

/// Ask llama through poolAI services: enqueue `llama_chat` for the caller's
/// bound peer. Body: `{user, prompt, max_tokens?}`.
async fn api_edge_chat_send(
    State(state): State<AppState>,
    Query(q): Query<ActionQuery>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    use crate::edge::PoolClient;

    // Mutating (enqueues minutes of 27B inference): same initData handshake
    // as board actions, or anyone on the LAN/tunnel could burn GPU time.
    let token = &state.config().bot_token;
    if token.is_empty() {
        return (
            StatusCode::BAD_GATEWAY,
            Json(crate::actions::err_json("bot token not configured")),
        )
            .into_response();
    }
    let now = freshness_now(q.auth_date);
    if let Err(e) = crate::security::verify_init_data(
        &q.init_data,
        token,
        now,
        crate::security::initdata::DEFAULT_MAX_AGE_SECS,
    ) {
        return (
            StatusCode::FORBIDDEN,
            Json(crate::actions::err_json(&format!("initData: {e}"))),
        )
            .into_response();
    }
    let user = body.get("user").and_then(Value::as_str).unwrap_or("");
    let prompt = body.get("prompt").and_then(Value::as_str).unwrap_or("");
    if user.trim().is_empty() || prompt.trim().is_empty() {
        return Json(json!({"ok": false, "error": "user + prompt required"})).into_response();
    }
    let pool = PoolClient::new(state.config());
    let peer = match pool.peer_for_user(user).await {
        Ok(Some(p)) => p,
        Ok(None) => return Json(json!({"ok": false, "error": "no bound peer"})).into_response(),
        Err(e) => return Json(json!({"ok": false, "error": e.to_string()})).into_response(),
    };
    let max_tokens = body.get("max_tokens").and_then(Value::as_u64).unwrap_or(64);
    // Tier routing: "fast" (default, interactive) or "deep" (27B).
    let model = body
        .get("model")
        .and_then(Value::as_str)
        .map(|m| {
            if m.trim().eq_ignore_ascii_case("deep") {
                "deep"
            } else {
                "fast"
            }
        })
        .unwrap_or("fast");
    match pool.enqueue_chat(&peer, prompt, max_tokens, model).await {
        Ok(id) => {
            Json(json!({"ok": true, "peer": peer, "task_id": id, "model": model})).into_response()
        }
        Err(e) => Json(json!({"ok": false, "error": e.to_string()})).into_response(),
    }
}

/// Ensure the caller's VM exists and runs: find `{peer}-vm`, create it
/// (2 cpu / 1024 MB) when missing, start it when not Running. Mutating,
/// so the initData handshake is required like the other POSTs.
/// Body: `{user}` (Telegram id, resolved to the bound peer).
async fn api_edge_vm_ensure(
    State(state): State<AppState>,
    Query(q): Query<ActionQuery>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    use crate::edge::PoolClient;

    let token = &state.config().bot_token;
    if token.is_empty() {
        return (
            StatusCode::BAD_GATEWAY,
            Json(crate::actions::err_json("bot token not configured")),
        )
            .into_response();
    }
    let now = freshness_now(q.auth_date);
    if let Err(e) = crate::security::verify_init_data(
        &q.init_data,
        token,
        now,
        crate::security::initdata::DEFAULT_MAX_AGE_SECS,
    ) {
        return (
            StatusCode::FORBIDDEN,
            Json(crate::actions::err_json(&format!("initData: {e}"))),
        )
            .into_response();
    }
    let user = body.get("user").and_then(Value::as_str).unwrap_or("");
    if user.trim().is_empty() {
        return Json(json!({"ok": false, "error": "user required"})).into_response();
    }
    let pool = PoolClient::new(state.config());
    let peer = match pool.peer_for_user(user).await {
        Ok(Some(p)) => p,
        Ok(None) => return Json(json!({"ok": false, "error": "no bound peer"})).into_response(),
        Err(e) => return Json(json!({"ok": false, "error": e.to_string()})).into_response(),
    };
    let name = crate::edge::vm_name_for_peer(&peer);
    let list = match pool.vm_list().await {
        Ok(v) => v,
        Err(e) => return Json(json!({"ok": false, "error": e.to_string()})).into_response(),
    };
    let id = match crate::edge::find_vm(&list, &name)
        .and_then(|v| v.get("id"))
        .and_then(Value::as_str)
        .map(str::to_string)
    {
        Some(id) => id,
        None => match pool.vm_create(&name, 2, 1024).await {
            Ok(v) => match v.get("id").and_then(Value::as_str) {
                Some(id) => id.to_string(),
                None => {
                    return Json(json!({"ok": false, "error": "create: no id"})).into_response()
                }
            },
            Err(e) => return Json(json!({"ok": false, "error": e.to_string()})).into_response(),
        },
    };
    // Start when not Running (start is idempotent on this build).
    let status = pool
        .vm_list()
        .await
        .ok()
        .and_then(|l| {
            crate::edge::find_vm(&l, &name)?
                .get("status")?
                .as_str()
                .map(str::to_string)
        })
        .unwrap_or_default();
    if status != "Running" {
        let _ = pool.vm_start(&id).await;
    }
    Json(json!({"ok": true, "peer": peer, "vm": name, "id": id})).into_response()
}

/// Poll a chat answer: `?peer=&task_id=` → `{ok, done, answer?}`.
async fn api_edge_chat_result(
    State(state): State<AppState>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    use crate::edge::PoolClient;

    let (Some(peer), Some(task)) = (q.get("peer"), q.get("task_id")) else {
        return Json(json!({"ok": false, "error": "peer + task_id required"}));
    };
    let pool = PoolClient::new(state.config());
    match pool.chat_answer(peer, task).await {
        Ok(Some(answer)) => Json(json!({"ok": true, "done": true, "answer": answer})),
        Ok(None) => Json(json!({"ok": true, "done": false})),
        Err(e) => Json(json!({"ok": false, "error": e.to_string()})),
    }
}

/// Virtual workers JSON for the Mini App VM screen. Fail-open like
/// [`api_edge_workers`]; optional `?user=<telegram_user_id|peer_id>` filter.
async fn api_edge_vm(
    State(state): State<AppState>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    use crate::edge::{vm_views, PoolClient};

    let pool = PoolClient::new(state.config());
    let filter = q.get("user").map(String::as_str);
    match vm_views(&pool, filter).await {
        Ok(rows) => {
            let vms: Vec<serde_json::Value> = rows
                .iter()
                .map(|v| {
                    json!({
                        "telegram_user_id": v.telegram_user_id,
                        "peer_id": v.peer_id,
                        "vm_id": v.vm_id,
                        "vm_status": v.vm_status,
                        "vm_health": v.vm_health,
                    })
                })
                .collect();
            Json(json!({"ok": true, "vms": vms}))
        }
        Err(e) => Json(json!({"ok": false, "error": e.to_string()})),
    }
}

/// Edge workers JSON for the Mini App workers screen: poolAI telegram
/// bindings enriched with per-peer task counters, plus seat state.
/// Fail-open: poolAI down returns the error shape, never a 500 page.
async fn api_edge_workers(State(state): State<AppState>) -> Json<serde_json::Value> {
    use crate::edge::{apply_task_status, assemble_views, PoolClient};

    let pool = PoolClient::new(state.config());
    let bindings = match pool.bindings().await {
        Ok(b) => b,
        Err(e) => return Json(json!({"ok": false, "error": e.to_string()})),
    };
    let mut views = assemble_views(&bindings);
    for v in &mut views {
        if let Ok(st) = pool.task_status(&v.peer_id).await {
            apply_task_status(v, &st);
        }
    }
    let seats = pool.seats().await.ok();
    let workers: Vec<serde_json::Value> = views
        .iter()
        .map(|v| {
            json!({
                "telegram_user_id": v.telegram_user_id,
                "peer_id": v.peer_id,
                "bound_at": v.bound_at,
                "pending": v.pending,
                "completed": v.completed,
                "tasks_ok": v.tasks_ok,
            })
        })
        .collect();
    Json(json!({"ok": true, "workers": workers, "seats": seats}))
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "telenetis",
        "version": "0.1.0"
    }))
}

async fn status(State(state): State<AppState>) -> Json<serde_json::Value> {
    let tickets = state.tickets().await;
    let presence = state.presence_map().await;
    let flows = state.recent_flows(10).await;

    Json(json!({
        "online": state.is_online(),
        "jail_id": state.jail_id(),
        "tickets_count": tickets.len(),
        "workers_online": presence.len(),
        "recent_flows": flows.len(),
    }))
}

async fn api_tickets(State(state): State<AppState>) -> Json<serde_json::Value> {
    let tickets = state.tickets().await;
    Json(json!({"tickets": wire_tickets(&tickets)}))
}

async fn api_flows(State(state): State<AppState>) -> Json<serde_json::Value> {
    let flows = state.recent_flows(50).await;
    Json(json!({"flows": flows}))
}

async fn serve_css() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("static/app.css"),
    )
}

async fn serve_js() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )],
        include_str!("static/app.js"),
    )
}

async fn serve_probe_js() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )],
        include_str!("static/probe.js"),
    )
}

async fn serve_tensor_js() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )],
        include_str!("static/tensor.js"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::state::AppState;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn test_state() -> AppState {
        let cfg = Config {
            bot_token: "test".to_string(),
            gsv_url: "http://127.0.0.1:9999".to_string(),
            poolai_url: "http://127.0.0.1:8091".to_string(),
            port: 9800,
            jail_id: "test-jail".to_string(),
            godfather_channel_id: 0,
            webhook_url: None,
            webhook_secret: None,
            public_url: None,
            tunnel_enabled: false,
            ngrok_bin: None,
        };
        AppState::new(cfg)
    }

    // Isolated AppState whose roles persistence goes to a per-test temp file,
    // so role-store tests never read/write the shared default data file.
    fn isolated_state(tag: &str) -> AppState {
        let cfg = Config {
            bot_token: "test".to_string(),
            gsv_url: "http://127.0.0.1:9999".to_string(),
            poolai_url: "http://127.0.0.1:8091".to_string(),
            port: 9800,
            jail_id: "test-jail".to_string(),
            godfather_channel_id: 0,
            webhook_url: None,
            webhook_secret: None,
            public_url: None,
            tunnel_enabled: false,
            ngrok_bin: None,
        };
        let file = std::env::temp_dir().join(format!(
            "telenetis_roles_ui_{tag}_{}.jsonl",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&file);
        AppState::new_with_roles_file(cfg, file)
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn static_css_served() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/static/app.css")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp
            .headers()
            .get(header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .contains("text/css"));
    }

    #[tokio::test]
    async fn board_page_ok() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/board")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn app_page_keeps_html_content_type_through_headers() {
        use crate::security::auth::security_headers;
        use axum::middleware;

        async fn headers(
            req: axum::http::Request<axum::body::Body>,
            next: middleware::Next,
        ) -> axum::response::Response {
            let mut resp = next.run(req).await;
            security_headers(&mut resp);
            resp
        }

        let app = router(test_state()).layer(middleware::from_fn(headers));
        let resp = app
            .oneshot(Request::builder().uri("/app").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp
            .headers()
            .get(header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .contains("text/html"));
    }

    // Reference initData for bot_token == "test" (matches test_state()).
    // Hash computed independently with OpenSSL (band 214). RAW form is
    // percent-encoded at request time via percent_encode_query().
    const TEST_USER_RAW: &str =
        "{\"id\":279058397,\"first_name\":\"Vlad\",\"language_code\":\"en\"}";
    const TEST_HASH: &str = "5cd657d1cc938ded22c052bb9450bb8f9d9842f450195b53703303cf555f410a";
    const TEST_TAMPERED_USER_RAW: &str =
        "{\"id\":999999,\"first_name\":\"Eve\",\"language_code\":\"en\"}";

    fn test_init_data(user: &str) -> String {
        format!(
            "auth_date=1750000000&query_id=AAHdF6IQAAAAAN0XohDhrOrc&user={}&hash={}",
            percent_encode_query(user),
            TEST_HASH
        )
    }

    fn percent_encode_query(input: &str) -> String {
        input
            .bytes()
            .map(|b| match b {
                b'!'
                | b'\''
                | b'('
                | b')'
                | b'*'
                | b'-'
                | b'.'
                | b'_'
                | b'~'
                | b'a'..=b'z'
                | b'A'..=b'Z'
                | b'0'..=b'9' => (b as char).to_string(),
                _ => format!("%{:02X}", b),
            })
            .collect()
    }

    #[tokio::test]
    async fn verify_endpoint_accepts_valid_init_data() {
        let app = router(test_state());
        let init = percent_encode_query(&test_init_data(TEST_USER_RAW));
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/verify?initData={}&authDate=1750000010", init))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], true);
    }

    #[tokio::test]
    async fn verify_endpoint_rejects_tampered_init_data() {
        let app = router(test_state());
        let init = percent_encode_query(&test_init_data(TEST_TAMPERED_USER_RAW));
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/verify?initData={}", init))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], false);
    }

    #[tokio::test]
    async fn verify_endpoint_missing_init_data_fails() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/verify")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], false);
    }

    #[test]
    fn freshness_now_pins_to_client_value_in_tests() {
        // Under cfg(test) the clock override is honored: a pinned anchor is
        // returned verbatim, and an empty one falls back to ~now.
        assert_eq!(freshness_now(Some(1_750_000_010)), 1_750_000_010);
        let fallback = freshness_now(None);
        assert!((chrono::Utc::now().timestamp() - fallback).abs() < 5);
    }

    // ---- band 218 (plan P4) board action endpoints ----

    async fn post_action(
        verb: &str,
        init: &str,
        auth: i64,
        body: serde_json::Value,
    ) -> axum::response::Response {
        let app = router(test_state());
        let init_q = percent_encode_query(&test_init_data(init));
        app.oneshot(
            Request::builder()
                .uri(format!(
                    "/api/board/{verb}?initData={}&authDate={}",
                    init_q, auth
                ))
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn board_action_rejects_missing_init_data() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/board/claim")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"id":"T-1"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn board_action_rejects_tampered_init_data() {
        let resp = post_action(
            "claim",
            TEST_TAMPERED_USER_RAW,
            1750000010,
            serde_json::json!({"id": "T-1"}),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], false);
    }

    #[tokio::test]
    async fn board_action_rejects_missing_ticket_id() {
        let resp = post_action("claim", TEST_USER_RAW, 1750000010, serde_json::json!({})).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], false);
        assert!(json["error"].as_str().unwrap().contains("ticket id"));
    }

    #[tokio::test]
    async fn board_action_valid_claim_forwards_to_gsv() {
        // A verified handshake with a well-formed body passes validation and
        // reaches the GSV forward. Point the client at an unreachable GSV so
        // the test never touches a live board; the forward must surface as a
        // gateway failure rather than a 4xx.
        let mut cfg = Config {
            bot_token: "test".to_string(),
            gsv_url: "http://127.0.0.1:1".to_string(),
            poolai_url: "http://127.0.0.1:9".to_string(),
            port: 9800,
            jail_id: "test-jail".to_string(),
            godfather_channel_id: 0,
            webhook_url: None,
            webhook_secret: None,
            public_url: None,
            tunnel_enabled: false,
            ngrok_bin: None,
        };
        cfg.bot_token = "test".to_string();
        let app = router(AppState::new(cfg));
        let init_q = percent_encode_query(&test_init_data(TEST_USER_RAW));
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/board/claim?initData={}&authDate=1750000010",
                        init_q
                    ))
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"id":"T-1"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], false);
    }

    #[tokio::test]
    async fn board_action_done_endpoint_registered() {
        // Same trust path for the done verb: tampered handshake is refused.
        let resp = post_action(
            "done",
            TEST_TAMPERED_USER_RAW,
            1750000010,
            serde_json::json!({"id": "T-1"}),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn board_action_error_endpoint_registered() {
        // Same trust path for the error verb: tampered handshake is refused.
        let resp = post_action(
            "error",
            TEST_TAMPERED_USER_RAW,
            1750000010,
            serde_json::json!({"id": "T-1"}),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    // ---- role-store HTTP surface (band 230) ----

    #[tokio::test]
    async fn roles_get_lists_empty_directory() {
        let app = router(isolated_state("list_empty"));
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/roles")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 8192).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], true);
        assert_eq!(json["roles"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn roles_assign_requires_valid_init_data() {
        let mut cfg = Config {
            bot_token: "test".to_string(),
            gsv_url: "http://127.0.0.1:9999".to_string(),
            poolai_url: "http://127.0.0.1:8091".to_string(),
            port: 9800,
            jail_id: "test-jail".to_string(),
            godfather_channel_id: 0,
            webhook_url: None,
            webhook_secret: None,
            public_url: None,
            tunnel_enabled: false,
            ngrok_bin: None,
        };
        cfg.bot_token = "test".to_string();
        let file = std::env::temp_dir().join(format!(
            "telenetis_roles_ui_assign_{}.jsonl",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&file);
        let app = router(AppState::new_with_roles_file(cfg, file));
        let init_q = percent_encode_query(&test_init_data(TEST_USER_RAW));

        // Tampered handshake -> 403, nothing stored.
        let tampered = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/roles?initData={}&authDate=1750000010",
                        percent_encode_query(&test_init_data(TEST_TAMPERED_USER_RAW))
                    ))
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"jail_id":"j1","role":"host"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(tampered.status(), StatusCode::FORBIDDEN);

        // Missing initData -> 403 (query param absent entirely).
        let missing = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/roles")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"jail_id":"j1","role":"host"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(missing.status(), StatusCode::FORBIDDEN);

        // Valid handshake with a readable role -> 200 ok, role listed.
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/roles?initData={}&authDate=1750000010",
                        init_q
                    ))
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"jail_id":"j1","role":"host"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 8192).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], true);
        assert_eq!(json["role"]["jail_id"], "j1");
        assert_eq!(json["role"]["role"], "host");
    }

    #[tokio::test]
    async fn roles_assign_rejects_invalid_role() {
        let app = router(isolated_state("invalid_role"));
        let init_q = percent_encode_query(&test_init_data(TEST_USER_RAW));
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/roles?initData={}&authDate=1750000010",
                        init_q
                    ))
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"jail_id":"j1","role":"admin"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], false);
    }

    #[tokio::test]
    async fn roles_remove_revokes_role() {
        let state = isolated_state("remove_j9");
        state
            .assign_role("j9", crate::roles::store::Role::Guest)
            .await;
        let app = router(state.clone());
        let init_q = percent_encode_query(&test_init_data(TEST_USER_RAW));
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/roles/remove?initData={}&authDate=1750000010",
                        init_q
                    ))
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"jail_id":"j9"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 8192).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], true);
        assert_eq!(json["removed"], true);
        assert!(state.get_role("j9").await.is_none());
    }

    #[tokio::test]
    async fn roles_persist_jsonl_and_reload() {
        let file = std::env::temp_dir().join(format!(
            "telenetis_roles_ui_persist_{}.jsonl",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&file);
        let cfg = Config {
            bot_token: "test".to_string(),
            gsv_url: "http://127.0.0.1:9999".to_string(),
            poolai_url: "http://127.0.0.1:8091".to_string(),
            port: 9800,
            jail_id: "test-jail".to_string(),
            godfather_channel_id: 0,
            webhook_url: None,
            webhook_secret: None,
            public_url: None,
            tunnel_enabled: false,
            ngrok_bin: None,
        };
        let state = AppState::new_with_roles_file(cfg.clone(), file.clone());
        state
            .assign_role("j-persist", crate::roles::store::Role::Host)
            .await;
        // A fresh AppState wiring reads the same roles JSONL file.
        let reloaded = AppState::new_with_roles_file(cfg, file);
        assert_eq!(
            reloaded.get_role("j-persist").await,
            Some(crate::roles::store::Role::Host)
        );
    }

    #[tokio::test]
    async fn mini_app_i18n_returns_requested_language() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/mini-app/i18n?lang=uk")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 8192).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["lang"], "uk");
        assert_eq!(json["strings"]["status.online"], "Онлайн");
        assert_eq!(json["strings"]["action.claim"], "Взяти");
    }

    #[tokio::test]
    async fn mini_app_i18n_defaults_to_english() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/mini-app/i18n")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 8192).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["lang"], "en");
        assert_eq!(json["strings"]["status.online"], "Online");
        assert_eq!(json["strings"]["app.title"], "Telenetis");
    }

    #[tokio::test]
    async fn mini_app_i18n_falls_back_from_unknown_lang() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/mini-app/i18n?lang=zz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 8192).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["lang"], "en");
        assert_eq!(json["strings"]["nav.roles"], "Roles");
    }

    #[tokio::test]
    async fn live_config_reports_server_authoritative_backoff() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/live/config")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let p = crate::stream::backoff::ReconnectPolicy::default();
        assert_eq!(json["reconnect"]["base_ms"], p.base_ms);
        assert_eq!(json["reconnect"]["cap_ms"], p.cap_ms);
        assert_eq!(json["reconnect"]["max_attempts"], p.max_attempts);
        assert_eq!(
            json["keepalive_secs"],
            crate::stream::backoff::WS_KEEPALIVE_SECS
        );
    }

    // ---- cold start (band 217, plan P3) snapshot + skeleton contracts ----

    #[tokio::test]
    async fn snapshot_returns_consolidated_bundle() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/snapshot")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 64 * 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        for key in [
            "v", "ts", "status", "tickets", "workers", "flows", "i18n", "live",
        ] {
            assert!(json.get(key).is_some(), "snapshot missing {}", key);
        }
        assert_eq!(json["v"], 1);
        assert_eq!(json["status"]["jail_id"], "test-jail");
        assert_eq!(json["i18n"]["lang"], "en");
    }

    #[tokio::test]
    async fn snapshot_reflects_seeded_state() {
        let state = test_state();
        state
            .set_tickets(vec![
                crate::state::TicketRow {
                    id: "T-1".to_string(),
                    title: "Fix bug".to_string(),
                    body: String::new(),
                    status: "open".to_string(),
                    product: "gsv".to_string(),
                    claimed_by: None,
                    scenario: Some("setup".to_string()),
                },
                crate::state::TicketRow {
                    id: "T-2".to_string(),
                    title: "Ship".to_string(),
                    body: String::new(),
                    status: "in_progress".to_string(),
                    product: "poolai".to_string(),
                    claimed_by: Some("test-jail".to_string()),
                    scenario: None,
                },
            ])
            .await;
        state
            .push_flow(crate::state::FlowEvent {
                ts: chrono::Utc::now(),
                jail_id: "jail-02".to_string(),
                action: "presence".to_string(),
                detail: "heartbeat".to_string(),
            })
            .await;
        state
            .update_presence(crate::state::WorkerPresence {
                jail_id: "jail-02".to_string(),
                actor: "alice".to_string(),
                ide: "cursor".to_string(),
                model: "m".to_string(),
                agent: "orchestrator".to_string(),
                rank: 7,
                status: crate::state::WorkerStatus::Ready,
                last_heartbeat: chrono::Utc::now(),
                timezone: "UTC".to_string(),
            })
            .await;

        let app = router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/snapshot")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 64 * 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"]["tickets_count"], 2);
        assert_eq!(json["status"]["workers_online"], 1);
        assert_eq!(json["tickets"].as_array().unwrap().len(), 2);
        assert_eq!(json["workers"][0]["jail_id"], "jail-02");
        assert_eq!(json["flows"].as_array().unwrap().len(), 1);
        assert_eq!(json["tickets"][0]["scenario"], "setup");
        assert_eq!(json["tickets"][1]["claimed_by"], "test-jail");
    }

    #[tokio::test]
    async fn snapshot_i18n_respects_lang_query() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/snapshot?lang=uk")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 64 * 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["i18n"]["lang"], "uk");
        assert_eq!(json["i18n"]["strings"]["action.claim"], "Взяти");
    }

    #[tokio::test]
    async fn snapshot_live_config_matches_backoff() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/snapshot")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 64 * 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let p = crate::stream::backoff::ReconnectPolicy::default();
        assert_eq!(json["live"]["reconnect"]["base_ms"], p.base_ms);
        assert_eq!(json["live"]["reconnect"]["cap_ms"], p.cap_ms);
        assert_eq!(json["live"]["reconnect"]["max_attempts"], p.max_attempts);
        assert_eq!(
            json["live"]["keepalive_secs"],
            crate::stream::backoff::WS_KEEPALIVE_SECS
        );
    }

    #[test]
    fn all_templates_carry_skeleton_markup() {
        let templates: Vec<(&str, &str)> = vec![
            ("dashboard.html", include_str!("templates/dashboard.html")),
            ("base.html", include_str!("templates/base.html")),
            ("board.html", include_str!("templates/board.html")),
            ("flows.html", include_str!("templates/flows.html")),
            ("roles.html", include_str!("templates/roles.html")),
        ];
        for (name, html) in &templates {
            assert!(
                html.contains("class=\"skeleton"),
                "{} has no skeleton markup",
                name
            );
            assert!(
                html.contains("data-skeleton="),
                "{} has no data-skeleton clears",
                name
            );
            assert!(
                html.contains("aria-busy=\"true\""),
                "{} region is not marked busy",
                name
            );
            assert!(
                html.contains("data-area="),
                "{} regions have no hydration areas",
                name
            );
            assert!(
                html.contains("<link rel=\"preload\" href=\"/api/snapshot?lang=en\" as=\"fetch\""),
                "{} does not preload the snapshot",
                name
            );
        }
    }

    #[test]
    fn flow_log_page_starts_offline_with_skeleton() {
        let html = include_str!("templates/flows.html");
        assert!(html.contains("id=\"flow-log\""));
        assert!(html.contains("data-area=\"flow-log\""));
        assert!(html.contains("data-feed=\"offline\""));
        assert!(html.contains("data-skeleton=\"flow-log\""));
    }

    #[test]
    fn app_js_hydrates_from_snapshot() {
        let js = include_str!("static/app.js");
        // bootstrap() must open the WS immediately and fetch the one snapshot.
        assert!(js.contains("async function bootstrap()"));
        assert!(js.contains("connectWS();"));
        assert!(js.contains("fetchSnapshot"));
        assert!(js.contains("/api/snapshot?lang="));
        assert!(js.contains("hydrateFromSnapshot"));
        // Skeleton clearing + aria contract.
        assert!(js.contains("function clearArea"));
        assert!(js.contains("querySelectorAll('[data-skeleton]')"));
        assert!(js.contains("removeAttribute('aria-busy')"));
    }

    // ---- band 218 (plan P4) board action wiring contracts ----

    #[test]
    fn board_template_has_actions_column() {
        let html = include_str!("templates/board.html");
        assert!(
            html.contains("<th data-i18n=\"board.actions\">Actions</th>"),
            "board.html lacks the localized Actions column header"
        );
        // The actions column makes the board 6 wide; skeleton rows must match.
        assert!(html.contains("colspan=\"6\""));
        assert!(!html.contains("colspan=\"5\""));
    }

    #[test]
    fn board_actions_i18n_key_resolves_in_all_languages() {
        for lang in [
            crate::ui::miniapp::Lang::En,
            crate::ui::miniapp::Lang::Uk,
            crate::ui::miniapp::Lang::Ru,
        ] {
            assert!(
                !crate::ui::miniapp::t("board.actions", lang).is_empty(),
                "board.actions missing for {lang:?}"
            );
        }
    }

    #[test]
    fn action_verb_i18n_keys_resolve_in_all_languages() {
        let keys = [
            "action.claim",
            "action.done",
            "action.error",
            "action.reclaim",
            "action.claiming",
            "action.doing",
            "action.erroring",
            "action.reclaiming",
            "action.claimed",
            "action.done_ok",
            "action.error_ok",
            "action.reclaim_ok",
        ];
        for lang in [
            crate::ui::miniapp::Lang::En,
            crate::ui::miniapp::Lang::Uk,
            crate::ui::miniapp::Lang::Ru,
        ] {
            for key in &keys {
                assert!(
                    !crate::ui::miniapp::t(key, lang).is_empty(),
                    "{} missing for {lang:?}",
                    key
                );
            }
        }
    }

    #[test]
    fn app_js_wires_board_action_buttons() {
        let js = include_str!("static/app.js");
        assert!(js.contains("function makeActionButton"));
        assert!(js.contains("function actionButtonsCell"));
        assert!(js.contains("async function postBoardAction"));
        assert!(js.contains("function actionLabel"));
        // The POST must carry the initData handshake + authDate, then forward.
        assert!(js.contains("/api/board/${action}"));
        assert!(js.contains("initData"));
        assert!(js.contains("authDate"));
        // Buttons render per ticket row.
        assert!(js.contains("tr.appendChild(actionButtonsCell(tk))"));
    }

    // ---- band 219 (ticket lifecycle UX) contract tests ----

    #[test]
    fn app_js_renders_server_authoritative_actions() {
        let js = include_str!("static/app.js");
        // The row renders the actions the server wired onto the ticket, not a
        // fixed claim/done/error triad.
        assert!(js.contains("tk.actions"));
        assert!(js.contains("Array.isArray(tk.actions)"));
        assert!(js.contains("board.no_actions"));
        assert!(js.contains("function statusLabel"));
        // Reclaim rides the same makeActionButton path.
        assert!(js.contains("reclaim"));
    }

    #[test]
    fn app_js_wires_ticket_detail_and_offline_states() {
        let js = include_str!("static/app.js");
        assert!(js.contains("function renderBoardRowData"));
        assert!(js.contains("ticket-detail"));
        assert!(js.contains("board.detail"));
        assert!(js.contains("board.offline"));
        assert!(js.contains("function setBoardOffline"));
        assert!(js.contains("detail.hidden"));
    }

    #[tokio::test]
    async fn snapshot_wires_ticket_body_and_actions() {
        let state = test_state();
        state
            .set_tickets(vec![
                crate::state::TicketRow {
                    id: "T-1".to_string(),
                    title: "Open task".to_string(),
                    body: "   a description   ".to_string(),
                    status: "open".to_string(),
                    product: "gsv".to_string(),
                    claimed_by: None,
                    scenario: None,
                },
                crate::state::TicketRow {
                    id: "T-2".to_string(),
                    title: "Claimed task".to_string(),
                    body: String::new(),
                    status: "in_progress".to_string(),
                    product: "poolai".to_string(),
                    claimed_by: Some("jail-01".to_string()),
                    scenario: None,
                },
                crate::state::TicketRow {
                    id: "T-3".to_string(),
                    title: "Finished".to_string(),
                    body: String::new(),
                    status: "done".to_string(),
                    product: "gsv".to_string(),
                    claimed_by: None,
                    scenario: None,
                },
            ])
            .await;
        let app = router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/snapshot")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 64 * 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let tickets = json["tickets"].as_array().unwrap();
        // Body now rides the wire for the ticket-detail view.
        assert_eq!(tickets[0]["body"], "   a description   ");
        // Context-sensitive actions: open → claim only; in_progress → the
        // three working verbs; terminal → none.
        assert_eq!(tickets[0]["actions"], serde_json::json!(["claim"]));
        assert_eq!(
            tickets[1]["actions"],
            serde_json::json!(["done", "error", "reclaim"])
        );
        assert_eq!(tickets[2]["actions"], serde_json::json!([]));
    }

    #[tokio::test]
    async fn board_reclaim_endpoint_registered() {
        // Reclaim uses the same trust path as the other verbs: a tampered
        // handshake is refused before anything reaches GSV.
        let resp = post_action(
            "reclaim",
            TEST_TAMPERED_USER_RAW,
            1750000010,
            serde_json::json!({"id": "T-1"}),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn board_reclaim_rejects_missing_init_data() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/board/reclaim")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"id":"T-1"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn board_reclaim_valid_forwards_to_gsv() {
        // A verified handshake with a well-formed body passes validation and
        // reaches the GSV reclaim forward; an unreachable GSV must surface as
        // a gateway failure rather than a 4xx.
        let cfg = Config {
            bot_token: "test".to_string(),
            gsv_url: "http://127.0.0.1:1".to_string(),
            poolai_url: "http://127.0.0.1:9".to_string(),
            port: 9800,
            jail_id: "test-jail".to_string(),
            godfather_channel_id: 0,
            webhook_url: None,
            webhook_secret: None,
            public_url: None,
            tunnel_enabled: false,
            ngrok_bin: None,
        };
        let app = router(AppState::new(cfg));
        let init_q = percent_encode_query(&test_init_data(TEST_USER_RAW));
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/board/reclaim?initData={}&authDate=1750000010",
                        init_q
                    ))
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"id":"T-1"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], false);
    }

    #[tokio::test]
    async fn status_display_i18n_keys_resolve() {
        for lang in [
            crate::ui::miniapp::Lang::En,
            crate::ui::miniapp::Lang::Uk,
            crate::ui::miniapp::Lang::Ru,
        ] {
            for key in [
                "status.open",
                "status.in_progress",
                "status.done",
                "status.blocked",
                "status.closed",
                "board.detail",
                "board.no_description",
                "board.no_actions",
                "board.offline",
            ] {
                assert!(
                    !crate::ui::miniapp::t(key, lang).is_empty(),
                    "{} missing for {lang:?}",
                    key
                );
            }
        }
    }

    #[tokio::test]
    async fn i18n_table_matches_html_i18n_attributes() {
        let app = router(test_state());
        let resp = app
            .oneshot(Request::builder().uri("/app").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 64 * 1024)
            .await
            .unwrap();
        let html = String::from_utf8_lossy(&body);

        // /app (dashboard) shell only carries app.* and status.* keys — every
        // data-i18n attribute the template uses must resolve to a non-empty
        // string for at least the default English UI.
        for key in ["app.title", "app.subtitle", "status.loading"] {
            assert!(
                html.contains(&format!("data-i18n=\"{}\"", key)),
                "template missing attribute for {}",
                key
            );
            assert!(!crate::ui::miniapp::t(key, crate::ui::miniapp::Lang::En).is_empty());
        }
    }

    // ---- WebGPU probe surface (swarm browser path) ----

    async fn post_webgpu(
        init: &str,
        auth: i64,
        body: serde_json::Value,
    ) -> axum::response::Response {
        let app = router(test_state());
        let init_q = percent_encode_query(&test_init_data(init));
        app.oneshot(
            Request::builder()
                .uri(format!(
                    "/api/edge/webgpu?initData={}&authDate={}",
                    init_q, auth
                ))
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn app_shell_redirects_startapp_probe() {
        // The t.me startapp link always opens /app; a `probe` start_param
        // must hop to the probe page so group users land on Run directly.
        let app = router(test_state());
        let resp = app
            .oneshot(Request::builder().uri("/app").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 64 * 1024)
            .await
            .unwrap();
        let html = String::from_utf8_lossy(&body);
        assert!(html.contains("start_param"));
        assert!(html.contains("/probe"));
    }

    #[tokio::test]
    async fn tensor_page_ok() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/tensor")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 64 * 1024)
            .await
            .unwrap();
        let html = String::from_utf8_lossy(&body);
        assert!(html.contains("/static/tensor.js"));
        assert!(html.contains("tensor-start"));
    }

    #[tokio::test]
    async fn tensor_js_served_with_worker_markers() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/static/tensor.js")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 128 * 1024)
            .await
            .unwrap();
        let js = String::from_utf8_lossy(&body);
        // Pinned runtime + contract surfaces the worker speaks.
        assert!(js.contains("@wllama/wllama@3.5.1"));
        assert!(js.contains("tasks/poll"));
        assert!(js.contains("llama_chat"));
        assert!(js.contains("loadModelFromHF"));
    }

    #[tokio::test]
    async fn probe_page_ok() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/probe")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 64 * 1024)
            .await
            .unwrap();
        let html = String::from_utf8_lossy(&body);
        assert!(html.contains("/static/probe.js"));
        assert!(html.contains("probe-run"));
    }

    #[tokio::test]
    async fn probe_js_served_as_javascript() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/static/probe.js")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp
            .headers()
            .get(header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .contains("javascript"));
        let body = axum::body::to_bytes(resp.into_body(), 64 * 1024)
            .await
            .unwrap();
        assert!(String::from_utf8_lossy(&body).contains("requestAdapter"));
    }

    #[tokio::test]
    async fn webgpu_rejects_missing_init_data() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/edge/webgpu")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"user":"1","probe":{"supported":false}}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn webgpu_rejects_tampered_init_data() {
        let resp = post_webgpu(
            TEST_TAMPERED_USER_RAW,
            1750000010,
            serde_json::json!({"user": "1", "probe": {"supported": false}}),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn webgpu_rejects_missing_probe() {
        let resp = post_webgpu(
            TEST_USER_RAW,
            1750000010,
            serde_json::json!({"user": "279058397"}),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], false);
    }

    #[tokio::test]
    async fn webgpu_rejects_invalid_probe() {
        let resp = post_webgpu(
            TEST_USER_RAW,
            1750000010,
            serde_json::json!({"user": "279058397", "probe": {"supported": true}}),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn webgpu_unsupported_short_circuits_without_profile() {
        let resp = post_webgpu(
            TEST_USER_RAW,
            1750000010,
            serde_json::json!({"user": "279058397", "probe": {"supported": false}}),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], true);
        assert_eq!(json["unsupported"], true);
    }

    #[test]
    fn webgpu_profile_row_unwraps_nested_gsv_answer() {
        // Live GSV shape: {ok: true, profile: {id, ...}} — the phone must
        // render the inner row, not the envelope (the `?` id bug).
        let v =
            serde_json::json!({"ok": true, "profile": {"id": "a54-01-valhall", "ram_mb": 4096}});
        let row = webgpu_profile_row(&v).expect("row");
        assert_eq!(row["id"], "a54-01-valhall");
        assert_eq!(row["ram_mb"], 4096);
        let err = webgpu_profile_row(&serde_json::json!({"ok": false, "error": "id required"}));
        assert!(err.unwrap_err().contains("id required"));
    }

    #[tokio::test]
    async fn webgpu_unbound_user_fails_before_gsv_forward() {
        // poolAI + GSV both unreachable: user resolution fails first, so no
        // profile write is ever attempted against a dead upstream.
        let mut cfg = Config {
            bot_token: "test".to_string(),
            gsv_url: "http://127.0.0.1:1".to_string(),
            poolai_url: "http://127.0.0.1:9".to_string(),
            port: 9800,
            jail_id: "test-jail".to_string(),
            godfather_channel_id: 0,
            webhook_url: None,
            webhook_secret: None,
            public_url: None,
            tunnel_enabled: false,
            ngrok_bin: None,
        };
        cfg.bot_token = "test".to_string();
        let app = router(AppState::new(cfg));
        let init_q = percent_encode_query(&test_init_data(TEST_USER_RAW));
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/edge/webgpu?initData={}&authDate=1750000010",
                        init_q
                    ))
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "user": "279058397",
                            "probe": {"supported": true, "adapter": {}, "limits": {}},
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], false);
    }
}
