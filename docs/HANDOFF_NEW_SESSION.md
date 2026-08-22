# Передача контексту новій сесії (GSV)

**Оновлено:** 2026-08-22 (band **200** · next = **owner pick**)

**Наступна сесія:** відкрити Cursor на **`S:\rust\GSV`** (або `gsv.code-workspace`) →
**`абракадабра` / `abrakadabra`** → `cargo xtask products` → **AskQuestion на проєкти з environment**
(не `gsv | poolai` з голови) → S0 диск/git → project scan (warnings first) →
якщо **gsv:** settings/Telegram/tickets spec bands **166–200 ✅**
+ **band 200** logic-audit fixes IV (ranks TG override thread-scope · tickets board write-lock races)
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
**band 175 ✅** MDS scenario band + solo walk + Telegram sync. **band 176 ✅** visible MCP session walk (solo / squad / bench on Godfather). **band 177 ✅** roadmap/plan hook-up. **band 178 ✅** scenario benchmark. **band 179 ✅** Godfather inbound poller. **band 180 ✅** watchdog process lockstep. **band 181 ✅** Galaxy glue + S0 disk on health. **band 182 ✅** MCP-readable Godfather envelopes + Galaxy MCP signal. **band 183 ✅** squad next-action + MCP catalog lockstep. **band 184 ✅** MCP session catalog lockstep. **band 185 ✅** Cursor catalog restart lockstep. **band 186 ✅** solo/squad/jail. **band 187** live Godfather member_count. **band 188** README SMIL + docs tidy. **band 189** Settings Galaxy polish. **band 190** Galaxy About + type/fullscreen chrome. **band 191** channel roles + GitHub origin. **band 192** ranks + no CMD flash. **band 193** federated `kind:presence`. **band 194** federated `kind:claim`. **band 195** federated `kind:done`. **band 196** federated `kind:reclaim`. **band 197** logic-audit fixes. **band 198** logic-audit fixes II. **band 199** logic-audit fixes III. **band 200** logic-audit fixes IV. **Next = owner pick**. MCP canon: [`gsv/GSV_MCP_OPENBOT.md`](gsv/GSV_MCP_OPENBOT.md).
Jail/squad join: [`gsv/GSV_SOLO_SQUAD_JAIL.md`](gsv/GSV_SOLO_SQUAD_JAIL.md).
Omni catalog: [`gsv/GSV_OMNI_CATALOG.md`](gsv/GSV_OMNI_CATALOG.md).
Rust-dev canon: [`gsv/GSV_RUST_DEV.md`](gsv/GSV_RUST_DEV.md).
Канон ролей: [`GSV_ROLES.md`](GSV_ROLES.md). Реєстр: [`gsv/PRODUCTS.md`](gsv/PRODUCTS.md).

## Стан зараз

- **GSV** — окремий Rust-first проєкт (`S:\rust\GSV`), bands 102 · 108–121 · 125–200.
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
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **99.41%** (rust 39931 / product 40168) · **668** tests · clippy 0 · fmt clean.
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
