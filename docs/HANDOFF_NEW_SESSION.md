# Передача контексту новій сесії (GSV)

**Оновлено:** 2026-09-01 (band **223** · next = **owner pick**)

**Наступна сесія:** відкрити Cursor на **`S:\rust\GSV`** (або `gsv.code-workspace`) →
**`абракадабра` / `abrakadabra`** → `cargo xtask products` → **AskQuestion на проєкти з environment**
(не `gsv | poolai` з голови) → S0 диск/git → project scan (warnings first) →
якщо **gsv:** settings/Telegram/tickets spec bands **166–212 ✅**
+ **band 212** telenetis to 100% — security wiring + HTTP checks + graceful shutdown + 13 new tests (55 total)
+ **band 214** telenetis Mini App initData HMAC-SHA256 verify (P0 from band-213 plan) — real handshake
  verification replaces the placeholder `csrf_check`; `/api/verify` HTTP surface; ref-pinned vs OpenSSL;
  14 new tests → telenetis **105**
+ **band 215** telenetis Mini App Telegram-native layer (plan P1) — `src/ui/miniapp.rs`
  (Platform classify/body-class, injection-safe `ThemeVar` for `--tg-theme-*`, en/uk/ru i18n table),
  `GET /api/mini-app/i18n`, JS `whereAmI()`/`applyTheme()`/BackButton/haptics/safe-area
  `--tg-viewport-stable-height` (never `100vh`); 12 new tests → telenetis **117**;
  version **0.215.0** + `cargo xtask bump --band 215` re-syncs vision lockstep (was drifted
  from band-214 close). Next telenetis code band lands plan **P2** (live stream backoff).
+ **band 216** telenetis live stream primacy (plan P2) — `stream/backoff.rs`
  (`ReconnectPolicy` base 1s/cap 30s/max 6 exponential backoff + deterministic jitter),
  `stream/ws.rs` server keep-alive heartbeat (`{"type":"ping"}` every 25s) + broadcast `Lagged`
  drop-tolerance, `GET /api/live/config` server-authoritative schedule, `app.js` exponential-backoff
  WS reconnect + SSE `/events` fallback after `max_attempts` (unified `renderFlow`, `data-feed`
  WS/SSE/offline badge); 9 new tests → telenetis **126**; version **0.216.0**. Next telenetis code band lands plan **P3** (cold start).
+ **band 217** telenetis cold start (plan P3) — skeleton screens in all 5 templates (`.skeleton`
  shimmer rows per `data-area`, `aria-busy`, `.skeleton::after` animation) so a cold Telegram
  WebView is never blank; consolidated **`GET /api/snapshot?lang=`** bundle (status + tickets +
  flows + workers + i18n + live config in ONE round-trip, shared `wire_tickets`/`wire_workers`/
  `wire_flows`); `app.js` `bootstrap()` opens the **WS immediately** (early upgrade, in parallel
  with the snapshot prefetch) then `hydrateFromSnapshot` clears skeletons and renders
  status/tickets/board/workers/roles/flows; `<link rel="preload" href="/api/snapshot?lang=en">`
  hint in every template; **`/start` server-side prefetch** (`bot/webhook.rs` `warm_start()` —
  best-effort GSV sync + `cold_start` flow event, unreachable GSV tolerated, offline fallbacks
  `fetchI18n()` + `loadLiveConfig()`); hydration lists CSS (.ticket-list/.worker-list/.status-badge);
  11 new tests → telenetis **137**; version **0.217.0**. Next telenetis code band lands plan **P4**
  (ranks + messaging polish is GSV-hosted, not telenetis) — candidate: remaining P-scope i18n
   board buttons / claim action in the Mini App UI.
+ **band 218** telenetis Mini App board actions (plan P4) — `src/actions.rs` `BoardAction` enum
   (Claim/Done/Error) + payload parsing (`BoardActionBody`/`parse_body`) + i18n keys (`label_key`/
   `busy_key`/`ok_key`) + GSV forward (`forward_body`); GSV `client.rs` `post_json` + `board_action`;
   `/api/board/claim|done|error` POST routes (initData HMAC verify → parse body → forward to GSV
   `/api/tickets/claim|done|error`); Mini App board **Actions** column (`board.actions` i18n key)
   with claim/done/error buttons (`makeActionButton` + `postBoardAction` — haptics, busy/ok states)
    wired in `app.js` `renderTicketRows` + `actionButtonsCell`; template `board.html` i18n header +
    6 columns; 18 new tests → telenetis **155** (137 → 155); version **0.218.0**. Next telenetis code
    band lands remaining P-scope (ranks + messaging polish is GSV-hosted).
+ **band 219** telenetis Mini App ticket lifecycle UX (plan P4 remaining P-scope) —
   `src/actions.rs` adds `BoardAction::Reclaim` + `available_actions(&str)` (`open` → `[Claim]`,
   `in_progress` → `[Done, Error, Reclaim]`, terminal/unknown → `[]`); `POST /api/board/reclaim`
   (initData HMAC verify → parse → forward to GSV `/api/tickets/reclaim`); snapshot now wires
   server-authoritative `actions` + ticket `body`; board renders a detail row
   (`.ticket-detail` expand/collapse), offline/error empty states (`board.offline`/`board.empty`),
   and i18n coverage of remaining visible strings (`action.reclaim*`, `status.*`, `board.detail`/
   `no_description`/`no_actions`/`offline`); `app.js` renders action buttons from `tk.actions`
   (fixt `renderTicketRows` 7-column vs 6-column header bug); 12 new tests → telenetis **167**
    (163 lib + 4 integration; 155 → 167); version **0.219.0**. Next telenetis code band lands
    remaining P-scope (ranks + messaging polish is GSV-hosted).
+ **band 220** GSV ranks + messaging polish (owner pick, GSV-hosted — the deferred telenetis plan
+   "ranks + messaging polish") — reason-bearing Godfather lines, demotion grace, top-tier decay,
+   peak snapshot. `src/boxes/ranks.rs`: `RosterRow` gains `peak: u8` + `last_move_ts: String`;
+   new `RankMove { row, delta, held }` returned by `apply`/`award`/`demote`/`on_ticket_done`/
+   `on_ticket_error`; `apply` tracks the peak high-water mark, demotion grace (`GRACE_SECS` 1 h →
+   held, event kind `grace-held`), and updates `last_move_ts`; `grace_blocked(...)` (chrono RFC3339);
+   `maybe_decay_host(...)` decays a host-only row at level ≥ `TOP_DECAY_LEVEL` (14) with no move
+   for `HOST_DECAY_SECS` (14 days) — called from `wire()`; `redact_row` emits `peak`.
+   `src/boxes/tickets.rs`: `done`/`error_ticket` return `(Ticket, RankMove)`; `WalkStep` gains
+   `rank_delta`/`rank_held`; `solo_walk` threads them. `src/boxes/telegram.rs`: `polished_session_line`
+   appends ` +1` / ` −1` (or ` (grace-held)`) for done/error phases — never a bare `+1`, always the
+   ticket reason. Federation (`done_remote`/`reclaim_remote`) stays rank-free. GSV tests: 296 lib +
+   contracts pass (2 new `peak_snapshot_tracks_high_water_mark`, `demotion_grace_holds_fresh_award`);
+   clippy 0; version **0.220.0**. (`ui_card_tracker_renders_table_markers` pre-existing failure —
+   missing tracker records in the test data store — fails identically on clean 551f61e, unrelated.)
**Next = owner pick** after a warnings-first scan. Speeds + Rust panel →
vision-sync → **один commit** → **`git push` + самарі**.

Якщо вибір **gsv:** scan [`GSV_TECH_ROADMAP.md`](gsv/GSV_TECH_ROADMAP.md) first
(always-on [`gsv/GSV_ALWAYS_ON_UI.md`](gsv/GSV_ALWAYS_ON_UI.md) **143–150 ✅**;
MCP catch-up **151 ✅**; MCP products select **152 ✅**; rust-first xtask **153 ✅**;
watchdog card **154 ✅**; session token usage **155 ✅**; streaming + VDT git + tunnel **156 ✅**;
OmniRouter catalog + quota timers **157 ✅**; live MCP stdio + sync check **158 ✅**;
Cursor HTTP MCP + session SSE hold **159 ✅**;
GSV sandbox MCP / no User leak **160 ✅**;
vision lockstep + disk MiB **161 ✅**;
live crate/version lockstep **162 ✅**;
vision queue lockstep + bump auto-advance **163 ✅**;
Cursor 3.16.29 kit lockstep **164 ✅**;
watchdog live copy + lockstep observability **165 ✅**;
**band 166 ✅** settings / Godfather — [`gsv/GSV_SETTINGS_TELEGRAM.md`](gsv/GSV_SETTINGS_TELEGRAM.md);
**band 175 ✅** MDS scenario band + solo walk + Telegram sync. **band 176 ✅** visible MCP session walk (solo / squad / bench on Godfather). **band 177 ✅** roadmap/plan hook-up. **band 178 ✅** scenario benchmark. **band 179 ✅** Godfather inbound poller. **band 180 ✅** watchdog process lockstep. **band 181 ✅** Galaxy glue + S0 disk on health. **band 182 ✅** MCP-readable Godfather envelopes + Galaxy MCP signal. **band 183 ✅** squad next-action + MCP catalog lockstep. **band 184 ✅** MCP session catalog lockstep. **band 185 ✅** Cursor catalog restart lockstep. **band 186 ✅** solo/squad/jail. **band 187** live Godfather member_count. **band 188** README SMIL + docs tidy. **band 189** Settings Galaxy polish. **band 190** Galaxy About + type/fullscreen chrome. **band 191** channel roles + GitHub origin. **band 192** ranks + no CMD flash. **band 193** federated `kind:presence`. **band 194** federated `kind:claim`. **band 195** federated `kind:done`. **band 196** federated `kind:reclaim`. **band 197** logic-audit fixes. **band 198** logic-audit fixes II. **band 199** logic-audit fixes III. **band 200** logic-audit fixes IV. **band 201** logic-audit fixes V. **band 202** logic-audit fixes VI. **band 203** logic-audit VII tests sweep (clean). **band 204** logic-audit VIII src sweep complete (SseUsageTap fix). **band 205** logic-audit IX cosmetic-notes close (5 fixes). **band 206** logic-audit X non-Rust sweep (ui glue + bench). **band 208** logic-audit XII server/watchdog/vision (byte-slice panics + theme-svg alias contract). **Next = owner pick**. MCP canon: [`gsv/GSV_MCP_OPENBOT.md`](gsv/GSV_MCP_OPENBOT.md).
Jail/squad join: [`gsv/GSV_SOLO_SQUAD_JAIL.md`](gsv/GSV_SOLO_SQUAD_JAIL.md).
Omni catalog: [`gsv/GSV_OMNI_CATALOG.md`](gsv/GSV_OMNI_CATALOG.md).
Rust-dev canon: [`gsv/GSV_RUST_DEV.md`](gsv/GSV_RUST_DEV.md).
Канон ролей: [`GSV_ROLES.md`](GSV_ROLES.md). Реєстр: [`gsv/PRODUCTS.md`](gsv/PRODUCTS.md).

## Стан зараз

- **Band 223:** keep-live health aggregation (owner pick) — GSV supervises the always-on kit (GSV :9999 + Telenetis :9800 + llama-rs + OmniRoute), **aggregation only, no respawn**. `src/boxes/keep_live.rs`: `KeepLiveReport { gsv, telenetis, llama_rs, omniroute }` each `{ alive, url, version?, lag? }`, probes 1s timeout, `ok` stays true when a peer is down (like `disk_ok` band 181); env overrides `GSV_KEEP_LIVE_GSV_URL` / `GSV_KEEP_LIVE_TELENETIS_URL` / `OMNIROUTE_URL` / `LLAMA_HEARTBEAT_PATH`; llama_rs liveness = fresh `llama_heartbeat.json` (age ≤ 60s, file probe, no HTTP; llama-rs does **not** write it yet — band 225). Wire: `GET /api/keep-live` + `GET /api/health { keep_live }` (async `wire_async`, no blocking); MCP `gsv_keep_live` read-only → **57** tools; Galaxy studio card `keep-live` (`CARD_NAMES` **43**, 4-peer table peer/alive/url/version); `src/bin/gsv_http_stand_smoke.rs` CARDS **43**; contracts `tests/gsv_keep_live_contracts.rs` (dead ports → `ok:true` + `alive:false`, render rows/empty/error, card list). **Fixed pre-existing recursive `cargo test` hang**: `terminal_cargo_test_allowed` (committed `b1edb98`) ran a nested `cargo test` → build-lock deadlock + infinite recursion → renamed `terminal_cargo_version_allowed` (`cargo --version`) + whitelist unit assert `validate("cargo test").is_ok()`. Global rules: `AGENTS.md` **`## Formats + local rules`** section (allowed `.rs/.md/.mdc/.json/.js/.wasm`, Rust 95–100%/wasm 0–5%, product local rules stay in product trees). Gate: fmt clean · clippy 0 · **782** tests (300 lib + bins + 475 contract; was 712 at band 207) · `--stretch-96` **99.46%** · vision rev **516** · version **0.223.0** (`cargo xtask bump --band 223`, queue last `PH-S2878` next `PH-S2879`). Next = owner pick.
- **Band 222:** telenetis↔GSV bus wire-contract fix + hardening (owner pick) — fixes a **confirmed dead** production bridge: GSV `/api/telegram/bus` answers `{"ok":true,"messages":[…]}` (`src/boxes/telegram.rs:2540-2569`, `:2705-2751`), but telenetis `src/gsv/poll.rs` `spawn_poll_loop` read `envelopes` / a bare array → presence/flow/forwarding never fired. Now `extract_messages` reads the GSV `messages` key (tolerates legacy bare array), rejects at `warn!`, and `post_bus_envelope` gets a 5s timeout — 5 contract tests. **Webhook secret-token auth**: `TELENETIS_WEBHOOK_SECRET` is sent to Telegram with `setWebhook` and every inbound `POST /webhook` must echo it in `X-Telegram-Bot-Api-Secret-Token` (constant-time `ct_eq`) or is rejected **403** — hermetic forged-update test. **initData freshness** on the server clock (`ui/mod.rs` `freshness_now`: `Utc::now()`, `authDate` ignored in prod). **Outbound timeouts**: `GsvClient` 5s/3s, `TelegramBot` 65s/10s. Ops: `.env.example` + README/ops + `boot-verify` secret support. Gate: fmt clean · clippy 0 · telenetis **177** (173 lib + 4 integration; was 167) · version **0.222.0** (`cargo xtask bump --band 222`). Next = owner pick.

- **Band 221:** Telenetis prod ops / deploy (owner pick) — ships telenetis for real use, closing the deferred prod-deploy intent (`PH-S2745/2746`). No Rust change (clippy 0, 167 tests green); adds prod **packaging + verifiable boot flow**: `telenetis/Dockerfile` (multi-stage rust→debian-slim, non-root, binds 9800, `HEALTHCHECK /health`, runs plain `--bin telenetis` with graceful SIGTERM), `docker-compose.yml` (`restart: unless-stopped`, `9800:9800`, `env_file .env`, `telenetis_data` volume, GSV `host.docker.internal:9999`), `.env.example` (env matrix, `PUBLIC_URL`/`WEBHOOK_URL`/`TUNNEL_ENABLED=0` in prod; secrets stay in gitignored `.env`), `deploy/systemd/telenetis.service` (bare-metal Linux: `Restart=always`, `EnvironmentFile`, strict hardening), `scripts/telenetis-boot-verify.sh` (probes `/health`, snapshot/status/live-config, `POST /webhook`, `GET /ws` 101, `GET /events` SSE — **7/7 pass** against the live release binary), `docs/telenetis/ops.md` runbook + README deploy link. Version **0.221.0** (`cargo xtask bump --band 221`). Next = owner pick.

- **GSV** — окремий Rust-first проєкт (`S:\rust\GSV`), bands 102 · 108–121 · 125–208.
- **Band 220:** GSV ranks + messaging polish (owner pick, GSV-hosted) — the deferred telenetis plan "ranks + messaging polish" lands in the GSV box. `src/boxes/ranks.rs` (metrics): `RosterRow` gains `peak: u8` (high-water mark) + `last_move_ts: String` (RFC3339); a new `RankMove { row, delta: i8, held: bool }` is returned from `apply`/`award`/`demote`/`on_ticket_done`/`on_ticket_error`; `apply` implements peak tracking, **demotion grace** (`GRACE_SECS` 1 h — a `−1` on a fresh award is held, event kind `grace-held`, level kept), and `last_move_ts` update; new `grace_blocked(...)` (chrono RFC3339 parse) + `maybe_decay_host(...)` — a host-only row at level ≥ `TOP_DECAY_LEVEL` (14) with no move for `HOST_DECAY_SECS` (14 days) decays −1 on `wire()` (earned marshal does not decay); `redact_row` emits `peak`. `src/boxes/tickets.rs`: `done`/`error_ticket` now return `(Ticket, ranks::RankMove)`; `WalkStep` gains `rank_delta: i8` + `rank_held: bool`; `solo_walk` threads `mv.delta`/`mv.held`. `src/boxes/telegram.rs`: `PolishedLineParams` gains `rank_delta`/`rank_held`; `polished_session_line` appends ` +1` / ` −1` (or ` (grace-held)`) for done/error phases — the Godfather line is **never a bare `+1`**, it always carries the ticket reason (Δ + held). Federation (`done_remote`/`reclaim_remote`) stays process-local and rank-free — no invented reason. GSV tests: 296 lib + all contracts green, incl. 2 new `peak_snapshot_tracks_high_water_mark` + `demotion_grace_holds_fresh_award`; clippy 0; **GSV_RANKS.md** updated; version **0.220.0**. Note: `ui_card_tracker_renders_table_markers` is a **pre-existing** failure (tracker records absent from the test data store → card renders empty-state not a table) — fails identically on clean `551f61e` (v0.219.0), unrelated to this band.
- **Band 218:** Telenetis Mini App board actions (owner pick, plan P4) — the Mini App ticket board now has an **Actions** column with claim/done/error buttons so a worker can drive the ticket lifecycle from the Telegram Mini App, not just from the GSV Galaxy. Rust core `telenetis/src/actions.rs`: `BoardAction` enum (Claim/Done/Error) with `parse`/`as_str`/`gsv_path`/`label_key`/`busy_key`/`ok_key`/`ALL`; `BoardActionBody` + `parse_body` (JSON `{action, ticket_id, note}`) with `err_json` for canonical `{ok:false,error}` responses and `forward_body` building the GSV forward JSON. GSV client (`telenetis/src/gsv/client.rs`) grew `post_json` + `board_action(action, id, note)` posting to the GSV `/api/tickets/claim|done|error` endpoints (server-to-server — the GSV CSRF gate allows it). HTTP surface in `telenetis/src/ui/mod.rs`: `POST /api/board/claim|done|error` with `ActionQuery {initData, authDate}`, a shared `api_board_action` handler that verifies the initData HMAC (band 214 verifier), parses the body, and forwards to GSV; plus HTTP + contract tests. Mini App UI: board Actions header (`board.actions` i18n key) + 6 columns in `templates/board.html`; `app.js` `actionLabel`/`makeActionButton`/`actionButtonsCell` (per-ticket row, disabled for non-claimable actions) and `postBoardAction` (POST initData+authDate, haptic, busy/ok button states); `app.css` `.board-actions`/`.board-btn` variants. i18n en/uk/ru keys added (board.actions + action.claiming/doing/erroring/claimed/done_ok/error_ok). 18 new tests (13 actions unit + 5 endpoint/contract) → telenetis **155** (137 → 155). Gate: fmt clean · clippy 0 · version **0.218.0**. Next telenetis code band lands remaining P-scope (ranks + messaging polish is GSV-hosted).
- **Band 219:** Telenetis Mini App ticket lifecycle UX (owner pick, plan P4 remaining P-scope) — the board now drives the full ticket lifecycle with correct per-status affordances. `src/actions.rs` adds `BoardAction::Reclaim` (`as_str="reclaim"`, `gsv_path="/api/tickets/reclaim"`, label/busy/ok keys) and `available_actions(&str)`: `open` → `[Claim]`, `in_progress` → `[Done, Error, Reclaim]`, terminal (`done`/`error`/…) or unknown → `[]`. HTTP: `POST /api/board/reclaim` joins the existing claim/done/error routes — initData HMAC verify (band 214) → parse body → forward to GSV `/api/tickets/reclaim`. The consolidated `GET /api/snapshot` now wires server-authoritative `actions` (per status, from `available_actions`) + ticket `body` (notes). Mini App UI: the board renders an expandable `.ticket-detail` row per ticket (toggled by a `board.detail` button) showing the body, `board.no_description` when empty, and `board.no_actions` when no action applies; `setBoardOffline()` displays the `board.offline` empty-state on snapshot failure; i18n coverage of the remaining visible strings — `action.reclaim*`, `status.*`, `board.detail`/`no_description`/`no_actions`/`offline` — in en/uk/ru. `app.js` action buttons now render from `tk.actions` (this also fixed a pre-existing `renderTicketRows` bug that appended 7 cells against a 6-column header; new `renderBoardRowData` emits 5 data cells + actions for 6 total). 12 new tests (Reclaim verbs/availability + snapshot wires actions/body + reclaim endpoints + status i18n keys) → telenetis **167** (163 lib + 4 integration; 155 → 167). Gate: fmt clean · clippy 0 · version **0.219.0**. Next telenetis code band lands remaining P-scope (ranks + messaging polish is GSV-hosted).
- **Band 216:** Telenetis live stream primacy (owner pick, plan P2) — WS `/ws` is now the *primary* live channel with a server keep-alive heartbeat (`{"type":"ping"}` every 25 s) so a silent-but-open socket is detectable and intermediate proxies stay alive, and it is drop-tolerant (a `Lagged` slow broadcast receiver keeps streaming instead of tearing down). The reconnect schedule is server-authoritative and Rust-defined in the new `telenetis/src/stream/backoff.rs`: `ReconnectPolicy` (base 1 s / cap 30 s / max 6 attempts) exponential backoff with a deterministic (`splitmix`-based, seed+attempt) jitter so a reconnecting fleet does not thundering-herd, served to the browser at `GET /api/live/config` (`{reconnect:{base_ms,cap_ms,max_attempts}, keepalive_secs}`). The Mini App client (`src/ui/static/app.js`) now reconnects the WS with that exact schedule and falls back to SSE `/events` only after `max_attempts` consecutive WS failures; both feeds render through one `renderFlow` into `#flow-log`, and a `data-feed` badge (`ws` / `sse` / `offline`) plus `sse-fallback` styling make the active channel legible. `stream/sse.rs` keeps its 30 s keep-alive as the fallback path — unchanged. 9 new tests (7 backoff unit + 1 `state::live_reconnect` + 1 HTTP live-config route) → telenetis **126** (117 → 126). Gate: fmt clean · clippy 0 · version **0.216.0**. Next telenetis code band lands plan **P3** (cold start — skeleton screens, prefetch on `/start`, WS upgrade as early as the dashboard loads).
- **Band 215:** Telenetis Mini App Telegram-native layer (owner pick, plan P1) — the browser Telegram SDK wiring lives in `static/app.js` (`whereAmI()` platform class, `applyTheme()` `--tg-theme-*` + `themeChanged`, native BackButton on non-root, haptics on claim/done/error, safe-area via `--tg-viewport-stable-height` — never `100vh`, `data-i18n` fill); all *server-testable* logic is new Rust `telenetis/src/ui/miniapp.rs`: `Platform::classify`/`body_class`/`needs_safe_area`, injection-safe `ThemeVar::tg`/`as_css` (feeds `--tg-theme-*` without breaking out of the declaration), `Lang::parse`/`t` en–uk–ru i18n table; `GET /api/mini-app/i18n?lang=` returns `{lang, strings}` (unknown → en). Templates gained Telegram SDK, `viewport-fit=cover`, `data-i18n` and the platform body-class default; CSS resolves surface colors from Telegram theme at runtime with mobile-only safe-area insets. 12 new tests: 8 miniapp unit (platform/theme-var/i18n) + 4 HTTP i18n routes + template-attr contract → telenetis **117** (105 → 117). Gate: fmt clean · clippy 0 · version **0.215.0** · `cargo xtask bump --band 215` re-synced the vision lockstep (manifest was drifted from the band-214 close: `next_sprint` PH-S2779 instead of close-of-215) so `vision_queue_lockstep_matches_crate_band` is green. Next telenetis code band lands plan **P2** (live stream primacy — WS-primary + exponential-backoff reconnect, SSE fallback).
- **Band 214:** Telenetis Mini App initData HMAC verify (owner pick) — code-landing of band-213 plan P0. Replaced the placeholder `csrf_check` (length-only) with real Telegram Mini App handshake verification in `telenetis/src/security/initdata.rs`: parse field pairs → re-sort signable form → secret key = HMAC(key `"WebAppData"`, msg bot_token) → expected hash via HMAC-SHA256 → constant-time hex compare; enforces `auth_date` freshness (default 86400s window) + required `user` field. Wired a `GET /api/verify?initData=&authDate=` HTTP surface that returns `{ok, error?}` bound to the configured bot token; `csrf_check` now delegates to the real verifier. `InitDataError` enum (missing hash/auth_date/user, malformed, stale, signature mismatch). Deps added: `hmac` 0.12 + `sha2` 0.10. Tests pinned against **independently computed OpenSSL reference vectors** (avoids circular self-consistency); 14 new tests — `secret_key_matches_independent_reference`, `hash_matches_independent_reference`, accept / tampered-user / wrong-token / missing hash|user / stale / future auth_date, constant-time compare, percent-decode, 3 `/api/verify` HTTP routes. Gate: telenetis **105** tests (91 → 105) · clippy 0 · fmt clean · GSV version bumped 0.214.0. Next telenetis code bands land plan P1–P4 (whereAmI/theme/BackButton/haptics branch, live stream backoff, cold-start, i18n).
- **Band 208:** Logic-audit XII (owner pick) — `server/mod.rs` + `boxes/watchdog.rs` + `boxes/vision.rs` read end-to-end. Confirmed bugs, all the byte-slice panic class: (1) `vision::rust_diagnostics_chart_svg` truncated `latest.command` via `&s[..48]` — panics when byte 48 lands inside a multi-byte UTF-8 char (500-class on `/api/vision/rust-diagnostics.svg`); now char-count + `chars().take(48)`; (2) `svg_day_label` sliced `[5..10]` under a `len >= 10` guard only — same panic on non-boundary cut; now char-indexed (ASCII ISO timestamps unchanged). Honesty/perf: `wire_speed_index`/`wire_rust_diagnostics` re-read the source artifact a second time just to compute `"present"` — refactored to a single read with identical present/source semantics (read → snapshot → default). Vacuous test rewritten for real: `update::pending_rebuild_logic` asserted literal `2 > 1`; now exercises `newest_src_mtime` on an empty temp `src/` (== 0) and after writing a file (> 0). Contract locked: `/api/vision/theme-svg` was silently byte-identical to `galaxy.svg` — documented as intentional legacy alias (`vision.theme_svg` SLI surface kept) and pinned by `theme_svg_is_byte_alias_of_galaxy`. Regressions: `diag_chart_truncates_multibyte_command_without_panic` · `svg_day_label_multibyte_safe`. Audited clean: watchdog lockstep/parse_apply_ok, security body-length gate, sprint_counts legacy aliases. Known-intentional (no change): sessionless `GET /mcp` finite flush drains the shared notification queue by design (bands 141/142/185, `catalog_stale` mitigation); `health()` runs `products::discover()` per probe — noted as future cache candidate. Gate: **715** tests · clippy 0 · fmt clean · vision sync rev 516.
- **Band 206:** Logic-audit X (owner pick) — first sweep of the two surfaces no prior audit had read end-to-end: Galaxy JS glue (`ui/index.html`) and the bench harness (`benches/gsv_dev.rs`). Fixes: (1) `resync` counted only rejected promises while every awaited wrapper swallows internally, so `failed` was always 0 and each silent resync flipped the badge back «online» even with the server down — `refreshMeta`/`getText` now resolve `true`/`false`, `failed` counts both shapes, and the 30s health timer dropped its dead rejection path; (2) a bare hook click silently POSTed `{source:"band",id:"177"}`, spawning duplicate tickets of a closed band — now guides without posting; (3) bench `"disk_report"` arm was unreachable (absent from the run list) and `_ => {}` benchmarked an empty loop on typos — arm wired in, unknown name is `unreachable!`, `median-ish` label renamed to `mean`. Polish: GPU mode swaps via exact classList, Auto toggle's dead `.off` class styled `.badge.off{opacity:.55}`. Contracts: `ui_index_resync_offline_follows_results`, `ui_index_hook_requires_input`. Gate: **708** tests · clippy 0 · fmt clean · ratio stretch-96 **99.43%**.
- **Band 205:** Logic-audit IX (owner pick) — closed all five cosmetic notes left by bands 197–204. Fixes: `omni/quota.rs` unknown task now routes the union like "any" (`task_matches`; old code collapsed to rust∩web because both `want_*` flags were false); `omni/config.rs apply` stages the patch on a clone and swaps only on success (a late error used to leave the live config half-applied while persist was skipped); `github.rs parse_triple` strips `-pre/+build` before numeric parsing and `version_gt` tie-breaks release > prerelease on equal triples; `preview::highlight_rs` counts the backslash run before a quote (even closes the literal — the old `ends_with("\\\"")` check swallowed the rest of the line after an even run); `xtask install_watchdog` persists a quoted TR (`watchdog_task_tr`) so repo paths with spaces survive `schtasks /TR` / HKCU Run. Regressions: task_matches_unknown_task_is_union_not_intersection · pick_route_unknown_task_equals_any · apply_patch_error_leaves_config_untouched · version_gt_prerelease_ranks_below_release · highlight_even_backslash_run_closes_string · highlight_odd_backslash_run_keeps_quote_escaped · watchdog_task_tr_quotes_paths_with_spaces. Gate: **706** tests · clippy 0 · fmt clean.
- **Band 203:** Logic-audit VII — tests sweep (owner pick: vacuous/tautological assertion hunt over all 21 `tests/*.rs`, 10 796 LOC). Verdict: **clean** — zero fixes required. Mechanical greps (self-comparison asserts · assertion-less fns · always-true shapes · `.contains("")`) all zero; twelve smaller files read end-to-end clean; nine large files audited via assert-line inventories (1 695 lines) — all meaningful; seven `\|\| is_null/is_none` OR-shapes verified as legitimate optional-field wire contracts (none tautological). Known-intentional: `smoke_report_shape_is_stable` asserts a JSON literal it builds itself (bins not importable from integration tests — wire documentation). Gate unchanged: **698** tests · clippy 0 · fmt clean. Docs-only close, no src diff.
- **Band 202:** Logic-audit fixes VI — last unaudited surfaces (`mcp.rs` full dispatch, `ui.rs` renderers, `app_error.rs`, `lib.rs`, `vision.rs`, all eleven `src/bin/*`). Two confirmed bugs: (1) `ui::esc` never escaped quotes while every renderer interpolates it into single-quoted HTML attributes → any wire string with `'` (ticket/feed titles) broke out of the attribute; `&#39;` / `&quot;` folded into `esc()` — guide's `esc_attr` output unchanged; (2) `vision.rs` test asserted `is_none() || is_some()` (vacuous); rewritten as a short-hash contract. Cosmetic notes only elsewhere: `gsv_server --port abc` falls back silently to DEFAULT_PORT; `cargo xtask vault-note --title help` prints help; `gsv_update` reads a `check` arg absent from its declared schema. Regressions: `esc_neutralizes_single_quote_attribute_breakout`, `git_head_short_hash_in_repo_and_none_or_short_outside`.
- **Band 201:** Logic-audit fixes V — `products::parse_cargo_name` matched prefixed keys (`name.workspace = true`, `namespace`) as crate names → garbage scan rows; now only plain `name = "…"` keys count (same guard class as fingerprint PH-S2633); `usage::SseUsageTap` never parsed the final usage line when the upstream stream ends without a trailing newline → silent token undercount; new pub fn `flush()` parses the buffered tail and is called at stream end in `omni/proxy.rs` `UsageTapStream`. Regressions: `parse_cargo_name_ignores_prefixed_and_workspace_keys`, `sse_tap_flush_parses_final_line_without_newline`. Twelve other ops-box files audited clean.
- **Band 200:** Logic-audit fixes IV — `ranks::TG_OVERRIDE` process-global → thread-local request scope (concurrent rank calls could attribute each other's Telegram ids); `tickets.rs` board mutators (create/set_status/stamp_claimed_jail/reclaim_stale/renew_leases/reclaim_remote) now serialize on a reentrancy-aware `BOARD_LOCK` — read-modify-write races could previously lose a whole update; reclaim federation posts moved outside the lock. Regressions: cross-thread TG leak, concurrent creates, nested-call no-deadlock. `update.rs` / `settings.rs` / `telegram.rs` audited clean.
- **Band 199:** Logic-audit fixes III — `omni::OmniRouter::persist` / `persist_quota` skip on lock contention (used to write a `Default` over tuned `omni.toml` / cooldown store); `fingerprint::read_tail_text` drops a partial UTF-8 prefix at the 64 KiB seek boundary (model discovery silently went `unknown`); `fingerprint::pkg_version` only matches plain `version = "…"` keys (`version.workspace` garbage), bump tightened to match. Regressions in omni unit + fingerprint contracts.
- **Band 198:** Logic-audit fixes II — `preview::highlight_rs` escapes comment text + bare `< > &` (raw HTML in comments could execute under the inline-script CSP); `ide::preview_messages` walks the 64 KiB tail back to a UTF-8 char boundary (byte-slice could panic); `hooks::test_bins` lists GSV `gsv_*-<hash>.exe` harnesses by excluding declared lib/bin/bench stems (legacy PoolAI prefixes matched nothing here); `tracker::parse_sprint_snapshot` counts plain `[ ]` rows as open (duplicate condition removed). Regressions in preview / ide / hooks units + tracker fixture.
- **Band 197:** Logic-audit fixes — `ranks::fingerprint_for_head` requires a non-empty `git_head` (legacy fingerprint rows without one no longer vacuously match and demote the wrong worker); `telegram::ticket_from_message` squad seed uses `tickets::assign_seed()` instead of hardcoded `1`; dry-run `poll_once` still reports `update_offset` but never persists `data/telegram_offset.json`. Regressions in ranks unit + telegram contracts.
- **Band 196:** Federated reclaim — lease expiry (`reclaim_stale`) or an explicit release posts Godfather `kind:reclaim` (`from=jail.id`, `ticket_id` required); peer boards transition their copy `in_progress` → `open` via `tickets::reclaim_remote` (+ `kind:reclaimed`). No rank change. Guest mute. Echo skip. Scenario `federated-reclaim`. Federation lifecycle (presence → claim → done → reclaim) complete. Spec [`gsv/GSV_SOLO_SQUAD_JAIL.md`](gsv/GSV_SOLO_SQUAD_JAIL.md).
- **Band 195:** Federated done — the jail that finished a claimed row posts Godfather `kind:done` (`from=jail.id`, `ticket_id` required); host board transitions it `in_progress` → `done` (`tickets::done_remote`). Ranks stay process-local. Guest mute. Echo skip. Scenario `federated-done`. Spec [`gsv/GSV_SOLO_SQUAD_JAIL.md`](gsv/GSV_SOLO_SQUAD_JAIL.md).
- **Band 194:** Federated claim — Godfather `kind:claim` (`from=jail.id`, `ticket_id` required). Host applies to this jail’s `tickets.jsonl`. Remote keeps its own JSONL. Guest mute. Echo skip. Local `try_dispatch` unchanged. Scenario `federated-claim`. Spec [`gsv/GSV_SOLO_SQUAD_JAIL.md`](gsv/GSV_SOLO_SQUAD_JAIL.md).
- **Band 193:** Federated presence — Godfather `kind:presence` (`from=jail.id`); host Galaxy `federation` rows; remote does not fill `squad_cap`; guest mute; echo skip. Scenario `federated-presence`. Spec [`gsv/GSV_SOLO_SQUAD_JAIL.md`](gsv/GSV_SOLO_SQUAD_JAIL.md).
- **Band 192:** Ranks + no console flash — `vision::command(git)` on origin probe; GET `/api/update` caches unless `?check=true`. Merit ladder L0 jun-nub … L15 marshal-orchestrator (IT+army mix). Host *displays* marshal. Ticket done +1 / error or failed tests after commit −1 (fingerprint + Telegram tail). Spec [`gsv/GSV_RANKS.md`](gsv/GSV_RANKS.md). `CARD_NAMES` **42**. MCP **56** tools.
- **Band 191:** Channel roles + GitHub origin lockstep — `chat_role` host/mate/guest/local (guest stays solo; live bus send refused); `gsv_update` `github_ahead` when origin is newer even if local `src/` is not; ticket pick board first then `hook github` (`GH#N`); scenarios `github-issues` / `channel-host` / `channel-mate` / `channel-guest`. Spec [`gsv/GSV_SOLO_SQUAD_JAIL.md`](gsv/GSV_SOLO_SQUAD_JAIL.md) · [`gsv/GSV_SETTINGS_TELEGRAM.md`](gsv/GSV_SETTINGS_TELEGRAM.md).
- **Band 190:** Galaxy About + type/fullscreen chrome — English About card (`CARD_NAMES` **41**); hover tips; distinct glyphs; GSV L0–L5 vision legend; fullscreen below header (`--fs-top`); `--ui:14px` + A−/A+ (12–18); 2-column card grid; nebula/glass. Spec [`gsv/GSV_ALWAYS_ON_UI.md`](gsv/GSV_ALWAYS_ON_UI.md).
- **Band 189:** Settings Galaxy polish + MCP Open Bot debug — labeled Godfather `.set-form` (workflow chips, mode/kind, poll/lease); `squad_cap_override` so Save does not freeze derived cap; dark `color-scheme` + Galaxy scrollbars; Telegram `.tg-head`; MCP `catalog_stale` banner. Spec [`gsv/GSV_SETTINGS_TELEGRAM.md`](gsv/GSV_SETTINGS_TELEGRAM.md).
- **Band 188:** README SMIL presentations + docs tidy — `docs/assets/presentations/{gsv-hero,gsv-install,gsv-flow}.svg` (GitHub-safe SMIL; no missing PNG); root README install (`cargo xtask live` + watchdog) + what-to-do; `docs/` no longer frames GSV as a PoolAI subfolder. `tests/gsv_readme_contracts.rs`.
- **Band 187:** live Godfather member_count — `getChatMemberCount` fills `tickets.member_count` / derived `squad_cap` (dry-run stub n=3 does not persist; poller ≥60s). Telegram card **members** row. `ticket_claims.jsonl` gitignored. Spec [`gsv/GSV_SETTINGS_TELEGRAM.md`](gsv/GSV_SETTINGS_TELEGRAM.md). `CARD_NAMES` **40**.
- **Band 186:** solo/squad/jail — `jail.id` · `tickets.squad_cap` = Godfather `member_count` · `bot_slot_cap` 50 channel / 20 group · join `env` on `GET /api/tickets` / `gsv_tickets` · presence refuses extra workers when full · resource `gsv://docs/solo-squad-jail`. Spec [`gsv/GSV_SOLO_SQUAD_JAIL.md`](gsv/GSV_SOLO_SQUAD_JAIL.md). `CARD_NAMES` **40**.
- **Band 185:** Cursor catalog restart lockstep — GET `/mcp` `catalog_stale` / `catalog_hint` when a session exists but `tools/list` never ran (or listed ≠ `tool_count`) · Galaxy MCP card **restart Cursor** (agent refresh only resubscribes resources) · `gsv_health` same fields. `CARD_NAMES` **40**.
- **Band 184:** MCP session catalog lockstep — JSON `POST /mcp` keeps `notifications/tools/list_changed` for the Streamable HTTP GET hold · `initialize` + SSE hold queue the notify · GET `/mcp` `catalog_notify` / `listed_tool_count` (0 = client never listed) · Galaxy `catalogNotify`. `CARD_NAMES` **40**.
- **Band 183:** squad next-action + MCP catalog lockstep — `next_action` inbox (`hint` → tool) · `POST /api/tickets/next` · MCP `gsv_tickets_next` (**55** tools) · `initialize` `tools.listChanged` · `notifications/tools/list_changed` · Galaxy next row. `CARD_NAMES` **40**.
- **Next drain (gsv):** **owner pick** after a warnings-first scan. Spec [`gsv/GSV_SETTINGS_TELEGRAM.md`](gsv/GSV_SETTINGS_TELEGRAM.md) · [`gsv/GSV_SOLO_SQUAD_JAIL.md`](gsv/GSV_SOLO_SQUAD_JAIL.md). Root landing: [`README.md`](../README.md). `cargo xtask bump --band N` locksteps last/next/active to the **close** of N.
- **Band 182:** MCP-readable Godfather envelopes — dual human line + JSON `data` (`hint` / `next` / disk / crate) · `POST /api/telegram/decode` · MCP `gsv_telegram_decode` (**54** tools) · Galaxy MCP signal row (tickets do not repeat envelope) · walk/hook/bench refresh Telegram · `syncVision` glue. `CARD_NAMES` **40**.
- **Band 181:** Galaxy glue + S0 disk on health — `selectProduct` / `reclaimTicket` in `ui/index.html`; `/api/health` `disk_ok` / `disk_violation` (process `ok` stays true). `CARD_NAMES` **40**.
- **Band 180:** Watchdog process lockstep — `debug_newer_server` (POST apply only when **gsv-server** debug is newer) · `hop_successor` each tick · stop stale peer on `bin_version` lag · wire `server_debug_newer` / `watchdog_debug_newer`. `CARD_NAMES` **40**.
- **Band 179:** Godfather inbound poller — `classify_inbound` · `poll_once` / `spawn_poll_loop` (`gsv-server` only) · `data/telegram_offset.json` · `POST /api/telegram/poll` · MCP `gsv_telegram_poll` (**53** tools) · Galaxy poll now. `CARD_NAMES` **40**.
- **Band 178:** Scenario benchmark — Instant `abrakadabra-session` create+walk → `docs/gsv/scenario_bench.json` · `GET`/`POST /api/tickets/bench` · MCP `gsv_tickets_bench` (**52** tools) · Godfather `session=` ns · Galaxy record button · `cargo xtask record-scenario-bench` · `gsv_dev` `session_walk_abrakadabra`. `CARD_NAMES` **40**.
- **Band 177:** Roadmap/plan hook-up — phrase `run mcp bot hook up scenario <id|band N|plan stem> [walk]` · `POST /api/tickets/hook` · MCP `gsv_tickets_hook` · parse `GSV_TECH_ROADMAP.md` + superpowers `- [ ]` · idempotent · Godfather `hook … n=` · Galaxy hook button · `CARD_NAMES` **40**.
- **Band 176:** Visible MCP session walk — scenario `abrakadabra-session` (6) · session lines (`solo claimed` / `squad assigned … to {worker}` / `bench gsv_dev … ns`) · live `sendMessage` 1/s · dry-run queue · `CARD_NAMES` **40**.
- **Band 175:** MDS scenario band + solo walk + Telegram sync — `tickets[]` on scenarios · `memory-disk-speed` (6) · `gsv-mds` memory/disk/speed · `GET /api/mds` · `POST /api/tickets/walk` · MCP `gsv_tickets_walk` + `gsv_mds` (**50** tools) · `kind:sync` envelopes · `CARD_NAMES` **40**.
- **Band 173:** Vision queue close-lockstep — `queue_ids_for_band(N)` is last sprint of N / first of N+1 so Galaxy does not reopen N's first PH-S* after bump.
- **Band 172:** Live crate lockstep — heartbeat `bin_version`; `GET /api/watchdog` crate/`version_lag`; oneshot apply on debug-newer **or** health lag; yield only if peer pid is alive; `lockstep-wait` during cooldown; stale watchdog exe hops debug → live. Recopy live after bump so MCP catalog matches the crate.
- **Band 171:** Ticket lease + stale reclaim — `lease_until` on `in_progress`; `tickets.lease_secs` default 300s; presence renews holder leases; GET list / claim auto-reclaim expired → `open` + `kind:reclaimed`; HTTP `POST /api/tickets/reclaim`; MCP `gsv_tickets_reclaim` (**47** tools). `CARD_NAMES` **40**.
- **Owner live Godfather (2026-08-19):** `@GSV_OFFICIAL` + `@GsvOfficialBot`; token in `data/gsv_settings.json` (gitignored); workflows `drain, ticket-claim, telegram-relay`. Live `getMe`/`getChat`/`sendMessage` OK. Poll of the bot’s own posts is empty (Telegram); other members need BotFather `/setprivacy` → Disable.
- **Band 169:** Telegram bus — `boxes/telegram.rs` envelope `{v:1,kind:bus,from,to?,ticket_id?,body}`; dry-run VecDeque; `GET`/`POST /api/telegram/bus` (CSRF); MCP `gsv_telegram_bus_send` / `gsv_telegram_bus_poll` (**42** tools); `telegram-relay` gate; allowlist; 2 KiB cap; 1/s rate-limit. No webhook. No Cloudflare. No create-ticket. Poll matches `@username` or numeric chat id.
- **Band 168:** Ticket board + MCP claim — `boxes/tickets.rs`; `docs/gsv/tickets.jsonl` + `ticket_claims.jsonl`; `GET`/`POST /api/tickets` + `POST /api/tickets/claim` (CSRF; unknown 404; `ticket-claim` off 403); Galaxy ops card `tickets` (`CARD_NAMES` **40**); MCP `gsv_tickets` + `gsv_tickets_claim`.
- **Band 166:** Settings box + Godfather secret store — `data/gsv_settings.json` (gitignored); `GET`/`POST /api/settings` redacts `bot_token`; env `GSV_TELEGRAM_BOT_TOKEN` wins; Galaxy ops card `settings`; MCP `gsv_settings` read-only + `gsv://docs/settings-telegram`.
- **Band 165:** watchdog live copy (`target/live/gsv-watchdog.exe`) + lockstep observability (`lockstep-fail`, `last_apply_status`, oneshot apply, health `version_lag`).
- **Band 164:** Cursor desktop **3.16.29** kit lockstep — rules pin, toolchain `cursor` probe, folder MCP `type:http` stays, never User MCP, no Origin-host.
- **Band 163:** vision queue lockstep after 162 close; `lockstep_queue_for_band` + bump auto-advance.
- **Band 160:** `GET /mcp` `sandbox`; no `gsv_products_open` / tunnel / apply on MCP; preview cannot `../poolAI`.
- **Band 159:** `.cursor/mcp.json` talks to live `gsv-server` (not a second stdio `gsv-mcp`). Stdio stays `.mcp.json` / OpenCode / Grok. Recopy `target/live/gsv-server.exe` after drains.
- **Band 158:** `copy_debug_to_live` also copies `gsv-mcp`; `.mcp.json` / OpenCode / Grok spawn the live exe; `gsv_xtask` MCP tasks **catalog|products|disk|sync**.
- **Band 157:** `catalog.rs` + `quota.rs`; `GET /api/omni/route`; MCP `gsv_omni_route` (**36** tools) + `gsv://docs/omni-catalog` (**10** resources); recommended Grok 4.6 / GPT-5.2 Codex / Claude Sonnet 4.6 / Gemini 3 Pro / Kimi K2.7 Code / GPT-5.3 Codex.
- **Band 156:** `stream:true` SSE tap + `stream_options.include_usage`; `.card.fullscreen img{max-height:none`; VDT git kit replaces `comitmsg/*.sh`; Grok Bot tunnel is CLI-only (not MCP).
- **Band 155:** automatic per-session token spend — OmniRouter completions + MCP bot (`Mcp-Session-Id` / stdio) + fail-open OmniRoute `/api/usage/history`; persist `data/gsv_usage.json`; Galaxy studio card `usage`.
- **Band 154:** Galaxy ops card `watchdog` (`render_watchdog` + `/api/ui/card/watchdog`); fingerprint `resolve_model` (`GSV_MODEL` > `CURSOR_MODEL` / `GSV_SESSION_FILE` > Cursor `renderer.log` `catalogModelId` > `unknown`). Health row `watchdog_alive` kept.
- **Band 152:** MCP `gsv_products_select` `{id}` (same allowlist as `POST /api/products/select`; unknown → tool error); `gsv_products_scan` may omit `id` when selected; `gsv_drain` names select then scan.
- **Band 151:** MCP catch-up — `gsv_products` / `gsv_products_scan` / `gsv_watchdog` / `gsv_sw` / `gsv_fingerprints`; resources `gsv://docs/fingerprints` + `gsv://docs/post-always-on`; `gsv_drain` prompt names the new tools.
- **Band 150:** live watchdog — `boxes/watchdog.rs` + bin `gsv-watchdog` probe `GET /api/health` every 3s; after 2 misses copy debug→live and spawn detached; `GET /api/watchdog`; health `watchdog_alive`; `scripts/gsv-watchdog.sh` + install (schtasks ONLOGON, else HKCU Run).
- **Band 149:** owner-picked remaining P2 — `PRODUCTS.md` registers **omniroute** (node; `npm test`; ratio n/a); `gsv-bump-version.sh --band N` sets semver minor = band (`0.149.0`); scan HANDOFF fallback `AGENTS.md` / `docs/ROADMAP.md`; abracadabra node flow (no PH-S* invent).
- **Band 148:** Service Worker shell cache — `GET /sw.js` Rust-rendered; precache document + live CSS + galaxy/vision svg; skip SSE `/events` and `/mcp`; `GET /api/sw`; ops card `sw`; CSP `worker-src 'self'`.
- **Horizon 143–147 (closed):** always-on Galaxy UI (chrome, live copy, products, version/fingerprints, README polish).
- **Band 147:** README-level polish — `--card-radius:12px` / `--card-gap:16px` / `--header-pad:8px 16px`; README Quick start = `bash scripts/gsv-live.sh`; live-copy note in `GSV_ARCHITECTURE.md`; ALWAYS_ON_UI in docs index; stand-smoke leftover contracts for `products` + `fingerprints`.
- **Band 146:** version bump + fingerprints — tests use `env!("CARGO_PKG_VERSION")` (no hardcoded `0.1.0`); `scripts/gsv-bump-version.sh` patch +1; `boxes/fingerprint.rs` append/latest JSONL at `docs/gsv/fingerprints.jsonl`; `GET /api/fingerprints?limit=`; Galaxy ops card `fingerprints`; `scripts/gsv-fingerprint.sh` + commit trailers `Gsv-Actor` / `Gsv-Ide` / `Gsv-Model`. Header meta shows latest ide/model/actor. Drain close = bump + fingerprint **in the same commit**.
- **Band 145:** VDT products picker — `boxes/products.rs` `discover` mirrors `list-vdt-products.sh` (workspace ∪ sibling git ∪ kit). `GET /api/products`, `POST /api/products/select`, `POST /api/products/open` (cursor/explorer, id allowlist), `GET /api/products/scan`. Galaxy ops card `products`. Unknown id → 404 `{ok:false}`.
- **Band 144:** always-on live copy — `scripts/gsv-live.sh` copies `target/debug/gsv-server.exe` → `target/live/` and loops on `:9999`. `POST /api/update/apply` emits SSE `offline` and exits (`GSV_UPDATE_APPLY_EXIT`; cargo-test `deps/` harness skips exit). `doUpdate()` stays offline until SSE `onopen`. Do **not** kill the live copy before `cargo test`.
- **Band 143:** Galaxy chrome + type/chart — power menu `z-index:80` above workspace (header ≥ 40; no shared `z-index:2`); exclusive fullscreen (`data-action='card-fs'`, `exitFullscreen()`, Esc); collapsed cards leave the grid (`display:none` + dock restore); `--fs-ui/card/meta/chart` scale; speed/rust SVG height 168, font-size 11, ui-monospace.
- **Band 142:** MCP HTTP sessions — `POST /mcp` `initialize` issues process-local `Mcp-Session-Id` (cap 32); unknown id → 404 `{ok:false}`; `DELETE /mcp` ends it; JSON discovery stays sessionless (`sessions`/`session_count`); Galaxy card lists sessions. Same `gsv://` confine; no LAN widen; stdio does not issue HTTP sessions.
- **Band 141:** MCP HTTP SSE — `GET`/`POST /mcp` with `Accept: text/event-stream` flush `notifications/message` and `notifications/resources/updated` as finite SSE (`event: message`); JSON discovery stays default (`sse`/`streamable`); Galaxy card lists sse. Same `gsv://` confine; no LAN widen.
- **Band 140:** MCP resource subscribe + logging notifications — `resources/subscribe`+`unsubscribe` (allowlisted `gsv://`; `..` / `file:` → `-32602`); stdio flushes `notifications/message` (filtered by `logging/setLevel`) and `notifications/resources/updated` after `gsv_vision_sync` for subscribed vision URIs. `GET /mcp` `subscribe`/`subscription_count`; Galaxy card lists count.
- **Band 139:** MCP logging + completions — `logging/setLevel` (RFC 5424, process-local) + `completion/complete` (`ref/resource` allowlisted `gsv://` + `ref/prompt` names; `..` / `file:` → `-32602`). `GET /mcp` `logging`/`completions`/`log_level`; Galaxy card lists both.
- **Band 138:** MCP resources + prompts — `resources/list`+`read` (6 `gsv://` URIs, same confine as preview) + `prompts/list`+`get` (`gsv_status` / `gsv_vision_brief` / `gsv_drain`); `GET /mcp` `resource_count`/`prompt_count`; kit trigger alias `abrakadabra`.
- **Band 137:** MCP vision completeness — `gsv_vision` / `gsv_vision_{sprint_map,doc_preview,node_search,sync,extensions}` / `gsv_preview` → **26** tools; preview uses the same path confine as `GET /api/preview`.
- **Band 136:** MCP Galaxy UI — `GET /api/ui/card/mcp` (`render_mcp`, `CARD_NAMES` 32, `rustCards` 24);
  8 extra read tools (vision map/board/progress/speeds/rust + hooks + update) → **19** tools;
  `.grok/config.toml` project overlay. Discovery `GET /mcp` now includes `stdio` / `http` / `tool_count`.
- **Band 135:** `gsv_mcp_openbot` — `gsv-mcp` stdio JSON-RPC + `GET`/`POST /mcp`; tools wrap boxes;
  auto-register `.mcp.json` / `.cursor/mcp.json` / `opencode.json`; Omni dry-run default;
  terminal = HTTP allowlist; Grok Bot = client (tunnel = owner opt-in).
- **Band 134:** HTTP response hardening — CSP / nosniff / DENY / no-store / COOP+CORP; POST 256 KiB cap → 413 `{ok:false}`.
- **VDT kit (band 127):** shared `.agents/skills/` + generic `.cursor/rules/` + `gsv.code-workspace` + `PRODUCTS.md`.
  Discover: `cargo xtask products` (не hardcoded `gsv | poolai`).
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **99.43%** (rust 41625 / product 41862) · **706** tests · clippy 0 · fmt clean.
- **Сервер:** canon порт **9999** (`DEFAULT_PORT`; 8765 — Hyper-V reserved range).
- **Vision rev:** **516** (band 187 `cargo xtask sync`; last `PH-S2518` · next `PH-S2519`).
- **Live UI** — `gsv-server` → `http://127.0.0.1:9999/`. MCP stdio — `target/live/gsv-mcp.exe` (`cargo xtask live`).
- **Band 133:** localhost security — `--allow-lan`; CSRF POST gate; terminal cargo/git allowlists; `/data/{file}` allowlist; preview canonicalize.
- **FM:** band 127 = PoolAI FM §5.108 (PH-S1909…S1918 ✅). Master horizon poolAI: band 128.
- **Vision rev:** **501** (band 134 `gsv-vision-sync`).
- **poolAI ratio:** **95.04%** (advisory hold, `--ratio96-docs-canon --advisory --min-ratio 0.95`).

### Історія боксів (bands 109–126)

- **GSV** bands 102 · 108 · 109 · 110 · 111 · 112 · 113 · 114 · 115 · 116 · 117 · 118 · 119 · 120 · 121 · 125 · 126 **✅**.
- **Тести (band 126):** **230** green (102 lib + 3 stand-smoke bin + 8 omni + 7 ratio + 32 server + 6 stand-smoke contracts + 12 ui + 8 update + 52 vision).
- **Сервер:** canon порт **9999**. Стара нотатка 8870/8891 — історична; див. `src/lib.rs`.
- **FM (до 127):** band 126 = §5.107 (PH-S1899…S1908 ✅).
- **Vision (band 126):** rev 493. Vision box: `boxes/vision.rs` + `gsv-vision-sync` bin +
  `GET /api/vision*`; snapshot `GSV/data/gsv_manifest.json` + `gsv_feed.json` + `gsv_extensions.json` (rev 492).
  Band 110: `GET /api/vision/map`, `GET /assets/vision.svg`, `GET /api/vision/feed?status=`, Vision Map card.
  Band 111: `GET /api/vision/sprint-map` (sprint-scope/queue/session-tracks links + modules + kinds) та
  `GET /api/vision/doc-preview?id=` (node + 1-hop neighbors) — Sprint Map + Doc Preview UI cards.
  Band 112: `GET /api/vision/sync` (auto-sync + drift), `GET /api/vision/extensions` (extension mirror:
  active_sprint + scopes), `GET /api/vision/sprint-queue` (entries ∪ active plan) — Vision Sync + Sprint Queue UI cards.
  Band 113: `GET /api/vision/node-search?q=&layer=` (node search, top-N 25, layer-z-sorted) —
  Vision Map card inline SVG + layer filter + search → doc-preview deep-link.
  Band 114: `GET /api/vision/sprint-board` (open/closed/planned columns + progress pct) та
  `GET /api/vision/sprint-progress` (status counts + per-layer nodes/linked distribution) —
  Sprint Board + Sprint Progress UI cards.
  Band 115: `GET /api/vision/speeds` (SpeedIndexReport: latest test-CI + bench + history counts,
  mirror `gsv_speed_index.json`, empty-tolerant) та `GET /api/vision/rust-diagnostics`
  (RustDiagnosticsReport: latest warnings/errors/top_codes + history count, mirror
  `gsv_rust_diagnostics.json`, empty-tolerant) — Speed Index + Rust Diagnostics UI cards.
  Band 116: `GET /api/vision/speeds.svg` (Speed history chart — Rust-rendered SVG: test-CI
  wall bars green ok / red fail, ≤24 runs, footer latest bench) та
  `GET /api/vision/rust-diagnostics.svg` (Rust history chart — warnings orange + errors red
  grouped bars, command footer); `<img>` charts у Speed Index + Rust Diagnostics cards.
  Band 118: `GET /api/vision/sprint-theme` (sprint UI theme wire: `#a78bfa`/`#c4b5fd`,
  pill/chip/queue colors, layer L0–L5 + edge-kind palettes) та
  `GET /api/vision/sprint-focus.svg?sprint=` (Rust-rendered sprint focus map: in-scope accent,
  out-of-scope dim 0.22/0.28, default active sprint) — Sprint Focus card + sprint-pill/queue
  chips у Sprint Queue/Board cards.
  Band 119: `GET /api/vision/palette` (повний legacy `:root` palette wire: bg-deep/bg/panel/
  panel-solid/border/border-bright/text/muted/accent/accent-2/glow/sidebar-w, layers+layers_dim
  L0–L5, edge-docs/code/toml, ext-md/rs/json/toml, sprint, bg-tone, galaxy-bg-opacity) +
  `GET /api/vision/starfield.svg?mode=eco|fx|ms` (Rust-rendered starfield: deterministic LCG,
  eco sparse/fx glow/ms medium) + `GET /api/vision/galaxy.svg` (Rust-rendered nebula backdrop) —
  Galaxy UI full parity: `loadGalaxyPalette` CSS-змінні, RSS ticker, GPU mode button
  (Eco/FX/Ms cycle), power menu (soft sync / reload / force offline), panel dock +
  Esc-fullscreen.
  Legacy parity: [`LEGACY_PARITY.md`](LEGACY_PARITY.md) — всі legacy-панелі закриті
  (bands 115–119); `vision.js`/`vision.css` superseded (band 115); **band 117: legacy
  deactivated** — `GSV/docs/vision/index.html` = GSV pointer page, `vision.js`/`vision.css` =
  DEACTIVATED banner (архів, не завантажуються); живий UI — `gsv-server` →
  `http://127.0.0.1:9999/`.
  **band 118: sprint UI (theme + focus) migrated** — legacy sprint colors/`sprint-dim`
  recreated в Rust (`vision.rs`), не legacy JS. **band 119: Galaxy UI full parity
  (colors + box behaviors) migrated** — legacy `:root` palette = Rust wire, starfield/galaxy
  backdrop = Rust SVG, header chrome/dock/fullscreen = compact UI glue (не legacy JS/CSS).
   **band 120: Ratio 96% stretch** — `GET /api/ui/card/{name}` (Rust-rendered card body HTML:
   `boxes/ui.rs` `esc`/`tab`/`bar` + 12 renderers + `CARD_NAMES`); `ui/index.html` thin glue
   (`getText` → `rustCards`); `gsv-loc-audit --stretch-96` advisory (**96.51%** ≥96% ✅).
   **band 121: OmniRouter box parity** — `boxes/ui.rs` `render_omni` (summary/routing +
   recommended + providers + models tables) + `format_number`; `CARD_NAMES` 13;
   `server/mod.rs` `api_ui_card` `"omni"` → `boxes::omni::wire`; `renderOmni` JS видалено,
   `rustCards` 13; `gsv-loc-audit --stretch-96` → **96.73%** (rust 10191 / product 10536) ≥96% ✅.
   **band 125: Vision/UI polish** — `boxes/ui.rs`: 13 renderers error/empty-state HTML
   маркери (`err_html`/`empty_html`/`not_ok`, `<span class='err'>` + «— no data», no panic);
   `gsv_ui_contracts`: stand contracts for all 13 (`RUST_CARDS`) + a11y markers +
   offline-stable cards; `server/mod.rs`: canonical JSON error shape `{ok:false,error}`
   (err_json — preview/ui-card/ui-path/data-file/error-response/omni-test/spawn_cargo);
   `gsv_server_contracts`: `error_responses_share_canonical_json_shape` +
   `post_errors_share_canonical_json_shape`; `ui/index.html`: a11y (role=status,
   aria-live/aria-label/alt/aria-haspopup), `data-card` hooks, `getText` keep-last-good +
   `.card-status` badge на fetch fail; `boxes/vision.rs`: `wire_summary` empty-tolerant
   (`degraded` flag, error тільки при fallback), consistent `ok`/`error` across
   `/api/vision*` (`gsv_vision_contracts` wire-shape contracts);
   `gsv-loc-audit --stretch-96` → **96.87%** (rust 11176 / product 11537) ≥96% ✅.
   **band 126: GSV stand smoke + ops canon** — `gsv-http-stand-smoke` bin (мірор poolAI
   `poolai-http-stand-smoke`): `--base-url`/`--json`, `check_ok`/`check_json`/`check_status`/
   `check_card`, 48 live checks (core boxes + vision* ok-gate + SVG status + 20 ui cards),
   exit 1 при FAIL; `tests/gsv_stand_smoke_contracts.rs` (6); docs canon
   (GSV_SERVER.md stand-smoke section, GSV_BOXES.md row, README 230 tests);
   `gsv-loc-audit --stretch-96` → **96.87%** (rust 11176 / product 11537) ≥96% ✅.
- **poolAI ratio:** **95.04%** (advisory hold, `--ratio96-docs-canon --advisory --min-ratio 0.95`).

## S0 (кожна сесія, disk/git first)

1. `df -h /s | tail -1` → `cargo xtask disk` → `cargo xtask disk --clean` (keeps `target/live`) якщо <5G (12G дешево). Do **not** `cargo clean` the live copy.
2. `git fetch` → `git status -sb` → `git log -1 --oneline`.
3. Прочитати цей HANDOFF + `NEXT_SESSION_PROMPT.md` + FM §5.12 §5.100.

## Project scan (якщо §5.12 < 10 відкритих)

- Warnings/diagnostics першими: `cargo run --bin poolai-rust-diagnostics -- --print` (poolAI),
  clippy warnings GSV (`cargo clippy --all-targets`).
- Роадмапи/архітектор-ряди: `GSV/docs/gsv/GSV_TECH_ROADMAP.md`, `GSV/docs/`, FM §5.1.
- Fallback-смуга: ratio contracts, UI compact, docs canon, vision sync, stand smoke.

## Build/test (MSYS2 bash)

```bash
export PATH="/c/Users/${USER}/.cargo/bin:$HOME/.cargo/bin:/ucrt64/bin:/usr/bin:$PATH"
export RUSTUP_TOOLCHAIN="stable-x86_64-pc-windows-gnu"
cd GSV
cargo fmt -- --check && cargo clippy --all-targets && cargo test && cargo run --bin gsv-loc-audit
```

**⚠️ Canon listener is `cargo xtask live` (`target/live/`).** Do not kill that process before `cargo test`/`build`. Only stop `target/debug/gsv-server.exe` if *that* file is still bound to :9999.

## Git (кінець сесії)

- `cargo xtask bump --band N` (semver minor = band **and** vision queue **close of N**: last of N / first of N+1) then `cargo xtask fingerprint` (JSONL + trailers) **before** the commit.
- Один commit (код + docs + FM/HANDOFF/NEXT). Не `git add -A` — тільки файли спринту.
- Trailers: `Gsv-Actor` / `Gsv-Ide` / `Gsv-Model` (no secrets).
- **`git push` + самарі** — обов'язково останній крок.
