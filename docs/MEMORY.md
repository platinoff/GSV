# GSV — Memory mark (what/why)

Стан проєкту **Galaxy StarWalker Vision** — окремого Rust-first проєкту в `GSV/` репо PoolAI.
Оновлюється в кінці кожного band. Лічильники — вимірювані (`wc -l`, `cargo test`,
`cargo run --bin gsv-loc-audit`), не з пам'яті.

## Стан (2026-08-19 · band 176 ✅)

- **Band 176:** Visible MCP session walk. Scenario `abrakadabra-session` (6) · session lines (`solo claimed` / `squad assigned … to {worker}` / `bench gsv_dev … ns`) · live `sendMessage` 1/s · dry-run queue · Galaxy walk · `CARD_NAMES` **40**.
- **Band 175:** MDS scenario band + solo walk + Telegram sync. Scenario `tickets[]` · catalog `memory-disk-speed` (6) · `gsv-mds` (1 MiB alloc + OS phys / `disk_report` / xor-fold) · `GET /api/mds` · `POST /api/tickets/walk` · MCP `gsv_tickets_walk` + `gsv_mds` (**50** tools) · `kind:sync` on claim/done · Galaxy add+walk · `gsv_dev` band/walk/mds benches. `CARD_NAMES` **40**.
- **Canon:** [`gsv/GSV_SETTINGS_TELEGRAM.md`](gsv/GSV_SETTINGS_TELEGRAM.md).
- **Next drain:** **owner pick** (do not invent band 177).
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 is `cargo xtask products`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **99.38%** (rust 32185 / product 32386) · **596** tests · clippy 0

## Стан (2026-08-19 · band 174 ✅)

- **Band 174:** Solo bot tickets from Telegram. `/ticket` or `{kind:ticket}` → board row · solo MCP auto-claims when one worker is online · `POST /api/telegram/ticket` · MCP `gsv_telegram_ticket` (**48** tools) · events `telegram` then `claimed` · scenario `telegram-solo` · clippy unused `_c` in `watchdog_version_lag`. `CARD_NAMES` **40**.
- **Canon:** [`gsv/GSV_SETTINGS_TELEGRAM.md`](gsv/GSV_SETTINGS_TELEGRAM.md).
- **Next drain:** owner pick after warnings-first scan.
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 is `cargo xtask products`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **99.36%** (rust 31107 / product 31307) · **584** tests · clippy 0

## Стан (2026-08-19 · band 173 ✅)

- **Band 173:** Vision queue close-lockstep. After 172 close, Galaxy still showed `PH-S2359` / last `PH-S2358`. `queue_ids_for_band(N)` is now last sprint of N / first of N+1 so `cargo xtask bump --band N` does not reopen N. Close of 173: last `PH-S2378` · next/active `PH-S2379`.
- **Canon:** [`gsv/GSV_RUST_DEV.md`](gsv/GSV_RUST_DEV.md) · [`gsv/GSV_MCP_OPENBOT.md`](gsv/GSV_MCP_OPENBOT.md).
- **Next drain:** owner pick after warnings-first scan.
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 is `cargo xtask products`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **99.35%** (rust 30559 / product 30759) · **577** tests · clippy 0

## Стан (2026-08-19 · band 172 ✅)

- **Band 172:** Live crate lockstep. Heartbeat `bin_version` · `GET /api/watchdog` crate/`version_lag` · oneshot apply on debug-newer **or** health lag · yield only if peer pid is alive · `lockstep-wait` during cooldown · stale watchdog exe hops debug → live. Recopy live after bump so MCP catalog matches the crate.
- **Canon:** [`gsv/GSV_RUST_DEV.md`](gsv/GSV_RUST_DEV.md) · [`gsv/GSV_MCP_OPENBOT.md`](gsv/GSV_MCP_OPENBOT.md).
- **Next drain:** owner pick after warnings-first scan.
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 is `cargo xtask products`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **99.35%** (rust 30540 / product 30740) · **575** tests · clippy 0

## Стан (2026-08-19 · band 171 ✅)

- **Band 171:** Ticket lease + stale reclaim. `lease_until` on `in_progress` · settings `tickets.lease_secs` default 300s (clamp 60–3600) · presence renews holder leases · GET list / claim auto-reclaim expired → `open` + `kind:reclaimed` · HTTP `POST /api/tickets/reclaim` · MCP `gsv_tickets_reclaim` (**47** tools). `CARD_NAMES` **40**.
- **Canon:** [`gsv/GSV_SETTINGS_TELEGRAM.md`](gsv/GSV_SETTINGS_TELEGRAM.md).
- **Next drain:** owner pick after warnings-first scan.
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 is `cargo xtask products`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **99.34%** (rust 30249 / product 30449) · **568** tests · clippy 0

## Стан (2026-08-19 · band 170 ✅)

- **Band 170:** Ticket scenarios + solo/squad MCP. Catalog `docs/gsv/ticket_scenarios.json` · registered `PRODUCTS.md` product on create · `tickets.mode` solo (one MCP) / squad (random online, workflow `ticket-squad`) · presence TTL 120s · events `claimed`/`assigned`/`done`/`error` · HTTP `/api/tickets/{done,error,presence}` · MCP **46** tools · bench `gsv_dev`. `CARD_NAMES` **40**.
- **Canon:** [`gsv/GSV_SETTINGS_TELEGRAM.md`](gsv/GSV_SETTINGS_TELEGRAM.md).
- **Next drain:** owner pick after warnings-first scan.
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 is `cargo xtask products`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **99.34%** (rust 29843 / product 30042) · clippy 0

## Стан (2026-08-19 · band 169 ✅)

- **Band 169:** Telegram bus between MCP bots. Envelope `{v:1,kind:bus,from,to?,ticket_id?,body}` · dry-run VecDeque · `GET`/`POST /api/telegram/bus` (CSRF) · MCP `gsv_telegram_bus_send` / `gsv_telegram_bus_poll` (**42** tools · **11** resources) · `telegram-relay` gate · allowlist · 2 KiB cap · 1/s rate-limit. Poll matches `@username` or numeric chat id. Owner live bind `@GSV_OFFICIAL` / `@GsvOfficialBot` (token gitignored). No webhook. No Cloudflare. No create-ticket. `CARD_NAMES` **40** (telegram card shows last bus).
- **Canon:** [`gsv/GSV_SETTINGS_TELEGRAM.md`](gsv/GSV_SETTINGS_TELEGRAM.md) · plan [`superpowers/plans/2026-08-19-gsv-settings-telegram-tickets.md`](superpowers/plans/2026-08-19-gsv-settings-telegram-tickets.md) **complete**.
- **Next drain:** owner pick after warnings-first scan. Do not invent 170.
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 is `cargo xtask products`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **99.33%** (rust 28750 / product 28945) · **551** tests · clippy 0

## Стан (2026-08-19 · band 168 ✅)

- **Band 168:** Ticket board + MCP claim. `boxes/tickets.rs` · `docs/gsv/tickets.jsonl` + sibling `ticket_claims.jsonl` · `GET`/`POST /api/tickets` · `POST /api/tickets/claim` (CSRF; unknown 404; `ticket-claim` off 403) · Galaxy ops card `tickets` (`CARD_NAMES` **40**) · MCP `gsv_tickets` + `gsv_tickets_claim` (**40** tools · **11** resources). No Telegram bus.
- **Canon:** [`gsv/GSV_SETTINGS_TELEGRAM.md`](gsv/GSV_SETTINGS_TELEGRAM.md) · plan [`superpowers/plans/2026-08-19-gsv-settings-telegram-tickets.md`](superpowers/plans/2026-08-19-gsv-settings-telegram-tickets.md).
- **Next drain:** band **169** (`PH-S2329…S2338`) Telegram bus. Do not invent 170.
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 is `cargo xtask products`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **99.29%** (rust 27234 / product 27429) · **542** tests · clippy 0

## Стан (2026-08-19 · band 167 ✅)

- **Band 167:** Godfather Telegram channel bind. `boxes/telegram.rs` · `GET /api/telegram` (getMe+getChat; never `bot_token`) · dry-run stub under cargo test / `X-Telegram-Dry-Run: 1` · poller default off (`godfather.poll` or `telegram-relay`) · Galaxy ops card `telegram` (`CARD_NAMES` **39**) · MCP `gsv_telegram` read-only (**38** tools · **11** resources). No bus, no `tickets.jsonl`.
- **Canon:** [`gsv/GSV_SETTINGS_TELEGRAM.md`](gsv/GSV_SETTINGS_TELEGRAM.md) · plan [`superpowers/plans/2026-08-19-gsv-settings-telegram-tickets.md`](superpowers/plans/2026-08-19-gsv-settings-telegram-tickets.md).
- **Next drain:** band **168** (`PH-S2319…S2328`) ticket board + MCP claim. Then **169** bus. Do not invent 170.
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 is `cargo xtask products`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **99.28%** (rust 26604 / product 26796) · **528** tests · clippy 0

## Стан (2026-08-19 · band 166 ✅)

- **Band 166:** Settings + Godfather secret store. `boxes/settings.rs` · `data/gsv_settings.json` (gitignored) · env `GSV_TELEGRAM_BOT_TOKEN` wins without being written back · `GET`/`POST /api/settings` never contains `bot_token` · Galaxy ops card `settings` (`CARD_NAMES` **38**) · MCP `gsv_settings` read-only + resource `gsv://docs/settings-telegram` (**37** tools · **11** resources). No live Telegram.
- **Canon:** [`gsv/GSV_SETTINGS_TELEGRAM.md`](gsv/GSV_SETTINGS_TELEGRAM.md) · plan [`superpowers/plans/2026-08-19-gsv-settings-telegram-tickets.md`](superpowers/plans/2026-08-19-gsv-settings-telegram-tickets.md).
- **Next drain:** band **167** (`PH-S2309…S2318`) Godfather channel bind. Owner 2026-08-19: **167–169 fully specified** in the plan (one band per drain; then 168 tickets, then 169 bus). Do not invent 170.
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 is `cargo xtask products`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **99.27%** (rust 25931 / product 26122) · **517** tests · clippy 0

## Стан (2026-08-19 · band 166 queued → landed)

- **Owner pick:** GSV settings (Godfather channel + token store + co-workflows); Telegram channels; MCP bots talk via Telegram bus; ticket board for joiners; MCP claim → `in_progress` like fingerprint sync.
- **Canon:** [`gsv/GSV_SETTINGS_TELEGRAM.md`](gsv/GSV_SETTINGS_TELEGRAM.md) · plan [`superpowers/plans/2026-08-19-gsv-settings-telegram-tickets.md`](superpowers/plans/2026-08-19-gsv-settings-telegram-tickets.md).
- **Next drain:** band **166** (`PH-S2299…S2308`) — settings + redacted store only. Not 167 live Telegram.
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 is `cargo xtask products`.
- **Secrets:** `data/gsv_settings.json` (gitignored) + env `GSV_TELEGRAM_BOT_TOKEN`; never in MCP/HTTP JSON.

## Стан (2026-08-18 · band 165 ✅)

- **Band 165:** watchdog live copy + lockstep observability. Scan found live **0.163.0** vs crate **0.164.0** with `debug_newer` while heartbeat stayed `probe-ok`. `cargo xtask watchdog` now copies `gsv-watchdog` to `target/live/`. Apply failures are `lockstep-fail` + `last_apply_status` / `lockstep_note`. `--once` locksteps. A second process oneshot-applies when debug is newer. Health `version_lag` also locksteps. Vision queue `PH-S2289` / last closed `PH-S2288`.
- **Canon:** [`gsv/GSV_RUST_DEV.md`](gsv/GSV_RUST_DEV.md) · [`gsv/GSV_MCP_OPENBOT.md`](gsv/GSV_MCP_OPENBOT.md).
- **Next drain:** scan / owner pick.
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 is `cargo xtask products`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **99.27%** (rust 25757 / product 25947) · **503** tests · clippy 0

## Стан (2026-08-18 · band 164 ✅)

- **Band 164:** Cursor desktop **3.16.29** kit lockstep after the 3.13.21 pin. Folder MCP stays Streamable HTTP `type:http` → live `:9999/mcp` (36 tools). Never User MCP. Toolchain inventories `cursor` from `package.json`. Do not Origin-host this kit. Vision queue `PH-S2279` / last closed `PH-S2278`.
- **Canon:** [`gsv/GSV_MCP_OPENBOT.md`](gsv/GSV_MCP_OPENBOT.md) · [`.cursor/rules/cursor-environment-baseline.mdc`](../.cursor/rules/cursor-environment-baseline.mdc).
- **Next drain:** scan / owner pick.
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 is `cargo xtask products`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **99.26%** (rust 25526 / product 25716) · **496** tests · clippy 0.

## Стан (2026-08-18 · band 163 ✅)

- **Band 163:** vision queue lockstep — Galaxy `next_sprint` / `active_sprint` = `PH-S2269`, `last_sprint_closed` = `PH-S2268` (band 162 last). `cargo xtask bump --band N` patches those source JSON fields (no pretty rewrite) using band-102 origin math (`PH-S1659` + 10 per band).
- **Canon:** [`gsv/GSV_RUST_DEV.md`](gsv/GSV_RUST_DEV.md) · [`gsv/GSV_MCP_OPENBOT.md`](gsv/GSV_MCP_OPENBOT.md).
- **Next drain:** scan / owner pick.
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 is `cargo xtask products`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **99.26%** (rust 25387 / product 25577) · **489** tests · clippy 0.

## Стан (2026-08-18 · band 162 ✅)

- **Band 162:** live crate/version lockstep — `crate_version` / `version_lag` on health, update, and `GET /mcp`; health `update_available` matches the Update box (mtime + version lag + notify); watchdog `debug_newer` and POST `/api/update/apply` when debug is newer than a healthy live copy (Windows cannot overwrite a locked exe). Vision queue `PH-S2259` / last closed `PH-S2258`.
- **Canon:** [`gsv/GSV_RUST_DEV.md`](gsv/GSV_RUST_DEV.md) · [`gsv/GSV_MCP_OPENBOT.md`](gsv/GSV_MCP_OPENBOT.md).
- **Next drain:** scan / owner pick.
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 is `cargo xtask products`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **99.25%** (rust 25225 / product 25415) · **485** tests · clippy 0.

## Стан (2026-08-18 · band 161 ✅)

- **Band 161:** vision lockstep (`next_sprint` / `active_sprint` = `PH-S2249`, `last_sprint_closed` = `PH-S2248`) + S0 disk `free_mb` (sub-GiB is not `0 GiB`) + `cargo xtask disk --clean` keeps `target/live` (not on MCP).
- **Canon:** [`gsv/GSV_RUST_DEV.md`](gsv/GSV_RUST_DEV.md) · [`gsv/GSV_MCP_OPENBOT.md`](gsv/GSV_MCP_OPENBOT.md).
- **Next drain:** scan / owner pick.
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 is `cargo xtask products`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **99.25%** (rust 24989 / product 25179) · **481** tests · clippy 0.

## Стан (2026-08-18 · band 160 ✅)

- **Band 160:** Cursor User MCP overlay removed (it showed `gsv_mcp_openbot` in PoolAI). Folder scope is **GSV** (`.cursor/mcp.json`). `GET /mcp` `sandbox` is `S:/rust/GSV`. Preview/terminal stay in that crate; VDT products only via `gsv_products_*` (no open/apply/tunnel tools).
- **Canon:** [`gsv/GSV_MCP_OPENBOT.md`](gsv/GSV_MCP_OPENBOT.md).
- **Next drain:** scan / owner pick.
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 is `cargo xtask products`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **99.23%** (rust 24637 / product 24827) · **475** tests · clippy 0.

## Стан (2026-08-18 · band 159 ✅)

- **Band 159:** Cursor attaches over HTTP `url: http://127.0.0.1:9999/mcp` (live `gsv-server`). `GET /mcp` reports `version` + `http_url`. GET SSE **with** `Mcp-Session-Id` holds the Streamable HTTP stream; sessionless GET stays a finite flush. Stdio remains `.mcp.json` / OpenCode / Grok. Recopy live `gsv-server` after drains or HTTP tools lag the crate. **36** tools · **10** resources.
- **Canon:** [`gsv/GSV_MCP_OPENBOT.md`](gsv/GSV_MCP_OPENBOT.md) · [`gsv/GSV_RUST_DEV.md`](gsv/GSV_RUST_DEV.md).
- **Next drain:** scan / owner pick.
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 is `cargo xtask products`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **99.23%** (rust 24571 / product 24761) · **473** tests · clippy 0.

## Стан (2026-08-18 · band 158 ✅)

- **Band 158:** working MCP bot — `cargo xtask live` copies `gsv-mcp` next to `gsv-server`; client JSON (`.mcp.json` / `.cursor/mcp.json` / `opencode.json` / `.grok/config.toml`) spawn `target/live/gsv-mcp.exe` (no `cargo run`); POST `/mcp` skips browser CSRF; `gsv_xtask` `sync` is `--check` drift; `gsv_vision_sync` notifies every subscribed `gsv://` URI. **36** tools · **10** resources.
- **Canon:** [`gsv/GSV_MCP_OPENBOT.md`](gsv/GSV_MCP_OPENBOT.md) · [`gsv/GSV_RUST_DEV.md`](gsv/GSV_RUST_DEV.md).
- **Next drain:** scan / owner pick.
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 is `cargo xtask products`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **99.23%** (rust 24425 / product 24615) · **471** tests · clippy 0.

## Стан (2026-08-18 · band 157 ✅)

- **Band 157:** OmniRouter shared catalog for Cursor / OpenCode / Grok / Omni (Rust+web). `quota.rs` timers; MCP `gsv_omni_route` auto-skips cooling free hosts; `GET /api/omni/route`; resource `gsv://docs/omni-catalog`. **36** tools · **10** resources.
- **Canon:** [`gsv/GSV_OMNI_CATALOG.md`](gsv/GSV_OMNI_CATALOG.md) · [`gsv/GSV_MCP_OPENBOT.md`](gsv/GSV_MCP_OPENBOT.md).
- **Next drain:** scan / owner pick.
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 is `cargo xtask products`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **99.21%** (rust 23960 / product 24150) · **466** tests · clippy 0.

## Стан (2026-08-18 · band 156 ✅)

- **Band 156:** streaming OmniRouter token usage (`SseUsageTap` + `stream_options.include_usage`); fullscreen chart imgs unclipped; `cargo xtask git` (status/log/fetch/commit `--file comitmsg/*.md`/push) replaces `comitmsg/*.sh`; `cargo xtask tunnel` (cloudflared, owner opt-in, not MCP).
- **Canon:** [`gsv/GSV_RUST_DEV.md`](gsv/GSV_RUST_DEV.md) · [`gsv/GSV_MCP_OPENBOT.md`](gsv/GSV_MCP_OPENBOT.md).
- **Next drain:** scan / owner pick.
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 is `cargo xtask products`. Commit messages: `comitmsg/*.md` (never staged except README).
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **99.17%** (rust 22798 / product 22988) · **457** tests · clippy 0.

## Стан (2026-08-18 · band 155 ✅)

- **Band 155:** session token usage — `boxes/usage.rs` aggregates OmniRouter completions + MCP bot (`gsv_omni_chat` / `Mcp-Session-Id`) + fail-open OmniRoute `/api/usage/history`; persist `data/gsv_usage.json`; `GET /api/usage` + Galaxy studio card `usage` (`CARD_NAMES` **37**); MCP tool `gsv_usage` (**35** tools); vision-sync snapshot. Dry-run / zero usage not counted. Streaming not recorded (P1).
- **Canon:** [`gsv/GSV_POST_ALWAYS_ON.md`](gsv/GSV_POST_ALWAYS_ON.md) · [`gsv/GSV_ALWAYS_ON_UI.md`](gsv/GSV_ALWAYS_ON_UI.md) · [`gsv/GSV_MCP_OPENBOT.md`](gsv/GSV_MCP_OPENBOT.md).
- **Next drain:** scan / owner pick (Grok Bot tunnel stays opt-in).
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 is `cargo xtask products`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **99.14%** (rust 21800 / product 21989) · **445** tests · clippy 0. Vision rev **516**.

## Стан (2026-08-18 · band 154 ✅)

- **Band 154:** watchdog ops card (`render_watchdog`, `CARD_NAMES` **36**, Galaxy ops) + fingerprint `model` from `GSV_MODEL` else Cursor session (`CURSOR_MODEL` / `GSV_SESSION_FILE`); default `unknown` stays valid. Health row `watchdog_alive` kept.
- **Canon:** [`gsv/GSV_POST_ALWAYS_ON.md`](gsv/GSV_POST_ALWAYS_ON.md) · [`gsv/GSV_ALWAYS_ON_UI.md`](gsv/GSV_ALWAYS_ON_UI.md).
- **Next drain:** scan / owner pick (Grok Bot tunnel stays opt-in).
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 is `cargo xtask products`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **99.14%** (rust 21660 / product 21848) · **428** tests · clippy 0. Vision rev **516**.

## Стан (2026-08-18 · band 153 ✅)

- **Band 153:** rust-first tests/benches/scripts — `cargo xtask` (`src/boxes/xtask.rs` + `gsv-xtask` / `gsv-live` bins). Deleted product `scripts/*.sh` / `bin/*.sh`. MCP `gsv_xtask` + `gsv_disk` + resource `gsv://docs/rust-dev`. **34** tools · **9** resources.
- **Canon:** [`gsv/GSV_RUST_DEV.md`](gsv/GSV_RUST_DEV.md) · [`gsv/GSV_MCP_OPENBOT.md`](gsv/GSV_MCP_OPENBOT.md).
- **Next drain:** band **154** (`PH-S2179…S2188`) — watchdog ops card + fingerprint model (owner pick).
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 is `cargo xtask products`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **99.10%** (rust 20561 / product 20748) · **420** tests · clippy 0. Vision rev **515**.

## Стан (2026-08-18 · band 152 ✅)

- **Band 152:** MCP `gsv_products_select` `{id}` (same allowlist as HTTP select; unknown → tool error); `gsv_products_scan` may omit `id` when `AppState` has a selection; `gsv_drain` names select then scan. **32** tools · **8** resources.
- **Canon:** [`gsv/GSV_POST_ALWAYS_ON.md`](gsv/GSV_POST_ALWAYS_ON.md) · [`gsv/GSV_MCP_OPENBOT.md`](gsv/GSV_MCP_OPENBOT.md).
- **Next drain:** band **153** (`PH-S2169…S2178`) — watchdog ops card + fingerprint model (owner pick).
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 still `scripts/list-vdt-products.sh`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **96.61%** (rust 19921 / product 20619) · **399** tests · clippy 0. Vision rev **514**.

## Стан (2026-08-18 · band 151 ✅)

- **Band 151:** MCP catch-up — `gsv_products` / `gsv_products_scan` (`id` required; unknown → tool error) / `gsv_watchdog` / `gsv_sw` / `gsv_fingerprints`; resources `gsv://docs/fingerprints` + `gsv://docs/post-always-on`; `gsv_drain` prompt names the new tools. **31** tools · **8** resources.
- **Canon:** [`gsv/GSV_POST_ALWAYS_ON.md`](gsv/GSV_POST_ALWAYS_ON.md) · [`gsv/GSV_MCP_OPENBOT.md`](gsv/GSV_MCP_OPENBOT.md).
- **Next drain:** band **152** (`PH-S2159…S2168`) — MCP `products_select` + scan-without-id. Then 153 watchdog card (owner pick).
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 still `scripts/list-vdt-products.sh`.
- **Ratio / тести:** measured at band close — `gsv-loc-audit --stretch-96` → **96.59%** (rust 19767 / product 20465) · **393** tests · clippy 0. Vision rev **513**.

## Стан (2026-08-18 · post-always-on spec queued · band 150 ✅)

- **Owner ask → spec/plan, no band-151 code this session:** MCP catch-up. Always-on HTTP boxes (products / fingerprints / sw / watchdog) are not on `gsv_mcp_openbot` (still 26 tools / 6 resources). Scan 2026-08-18: clippy 0, roadmap 102–150 ✅.
- **Canon:** [`gsv/GSV_POST_ALWAYS_ON.md`](gsv/GSV_POST_ALWAYS_ON.md) · plan [`superpowers/plans/2026-08-18-mcp-always-on-catchup.md`](superpowers/plans/2026-08-18-mcp-always-on-catchup.md).
- **Next drain:** band **151** (`PH-S2149…S2158`) — five MCP tools + two `gsv://` resources + `gsv_drain` prompt. Then 152 select, 153 watchdog card (owner pick).
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 still `scripts/list-vdt-products.sh`.
- **Ratio / тести:** last measured band 150 — `gsv-loc-audit --stretch-96` → **96.75%** (rust 19027 / product 19667) · **386** tests · clippy 0. Vision rev **513**.

## Стан (2026-08-18 · band 150 ✅)

- **Band 150:** live watchdog (owner ask) — `gsv-watchdog` probes `/api/health` every 3s and respawns `target/live/gsv-server.exe` after 2 misses (grace for update-apply). Heartbeat `target/live/watchdog.json`; `GET /api/watchdog`; health card `watchdog`. Scripts: `gsv-watchdog.sh` (detach) + `gsv-watchdog-install.sh` (ONLOGON / HKCU Run).
- **Canon:** [`gsv/GSV_ALWAYS_ON_UI.md`](gsv/GSV_ALWAYS_ON_UI.md) · `gsv-live.sh` is the inner supervisor; watchdog is the outer loop when that shell dies.
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 still `scripts/list-vdt-products.sh`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **96.75%** (rust 19027 / product 19667) · **386** tests · clippy 0. Vision rev **513**.
- **Канон продукту:** Rust **95–100%** / wasm **0–5%** (завжди), без Python/Java; bins — лише `src/bin/`.

## Стан (2026-08-18 · band 149 ✅)

- **Band 149:** owner-picked remaining P2 — `docs/gsv/PRODUCTS.md` registers **omniroute** (node, `npm test`, ratio n/a); `scripts/gsv-bump-version.sh --band N` sets crate semver minor = band (`0.{band}.0`, same-band patch +1); `products::scan` treats `AGENTS.md` / `docs/ROADMAP.md` as handoff/next; abracadabra registered-node flow (no PH-S* invent). Do **not** invent band 150.
- **Canon:** [`gsv/GSV_ALWAYS_ON_UI.md`](gsv/GSV_ALWAYS_ON_UI.md) · always-on P2 leftovers closed.
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 still `scripts/list-vdt-products.sh`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **96.74%** (rust 19003 / product 19643) · **375** tests · clippy 0. Vision rev **513**.
- **Канон продукту:** Rust **95–100%** / wasm **0–5%** (завжди), без Python/Java; bins — лише `src/bin/`.

## Стан (2026-08-18 · band 148 ✅)

- **Band 148:** Service Worker shell cache (owner-picked P2 leftover) — `boxes/sw.rs` renders `GET /sw.js` (Cache Storage `gsv-shell-v1`); precache `/` + `/api/ui/load-palette` + `/api/ui/load-theme` + galaxy/vision svg; fetch skips `/events`, `/mcp`, non-GET, cross-origin. `GET /api/sw` discovery; ops card `sw`; CSP `worker-src 'self'`; thin `serviceWorker.register("/sw.js")`. Do **not** invent band 149.
- **Canon:** [`gsv/GSV_ALWAYS_ON_UI.md`](gsv/GSV_ALWAYS_ON_UI.md) · remaining P2: omniroute PRODUCTS.md (owner-opt-in), semver minor = band.
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 still `scripts/list-vdt-products.sh`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **96.87%** (rust 18711 / product 19316) · **371** tests · clippy 0.
- **Канон продукту:** Rust **95–100%** / wasm **0–5%** (завжди), без Python/Java; bins — лише `src/bin/`.

## Стан (2026-08-18 · band 147 ✅)

- **Band 147:** README-level Galaxy polish leftovers — CSS `--card-radius:12px` / `--card-gap:16px` / `--header-pad:8px 16px` vs presentation shots; README Quick start canon-runs `bash scripts/gsv-live.sh`; `GSV_ARCHITECTURE.md` live-copy note; Always-on spec row in `docs/gsv/README.md`; stand-smoke leftover contracts for `products` + `fingerprints`. Always-on Galaxy (bands 143–147) **closed**. Do **not** invent band 148.
- **Canon:** [`gsv/GSV_ALWAYS_ON_UI.md`](gsv/GSV_ALWAYS_ON_UI.md) · plan [`superpowers/plans/2026-08-17-always-on-galaxy.md`](superpowers/plans/2026-08-17-always-on-galaxy.md) · next drain = scan / owner pick.
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 still `scripts/list-vdt-products.sh`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **96.87%** (rust 18666 / product 19269) · **363** tests · clippy 0. Vision rev **513**.
- **Канон продукту:** Rust **95–100%** / wasm **0–5%** (завжди), без Python/Java; bins — лише `src/bin/`.

## Стан (2026-08-18 · band 146 ✅)

- **Band 146:** version bump + fingerprints — tests compare `env!("CARGO_PKG_VERSION")` (no hardcoded `0.1.0`); `scripts/gsv-bump-version.sh` increments `[package]` patch; `boxes/fingerprint.rs` `append`/`latest` on `docs/gsv/fingerprints.jsonl`; `GET /api/fingerprints?limit=`; Galaxy ops card `fingerprints`; `scripts/gsv-fingerprint.sh` prints `Gsv-Actor` / `Gsv-Ide` / `Gsv-Model` trailers. Header meta shows latest ide/model/actor. Drain close runs bump + fingerprint in the same commit.
- **Canon:** [`gsv/GSV_ALWAYS_ON_UI.md`](gsv/GSV_ALWAYS_ON_UI.md) · plan [`superpowers/plans/2026-08-17-always-on-galaxy.md`](superpowers/plans/2026-08-17-always-on-galaxy.md) · next drain **band 147** README polish (`PH-S2109…S2118`).
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 still `scripts/list-vdt-products.sh`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **96.10%** (rust 18303 / product 19045) · **358** tests · clippy 0. Vision rev **512**.
- **Канон продукту:** Rust **95–100%** / wasm **0–5%** (завжди), без Python/Java; bins — лише `src/bin/`.

## Стан (2026-08-17 · band 145 ✅)

- **Band 145:** VDT products picker — `boxes/products.rs` `discover` mirrors `scripts/list-vdt-products.sh` (workspace ∪ sibling git ∪ kit, no shell-out). `GET /api/products`, `POST /api/products/select`, `POST /api/products/open` (cursor if on PATH else explorer; cargo-test harness skips spawn), `GET /api/products/scan` (git HEAD/status, HANDOFF/NEXT, `cargo_name`). Galaxy ops card `products`. Unknown id → 404 `{ok:false}`.
- **Canon:** [`gsv/GSV_ALWAYS_ON_UI.md`](gsv/GSV_ALWAYS_ON_UI.md) · plan [`superpowers/plans/2026-08-17-always-on-galaxy.md`](superpowers/plans/2026-08-17-always-on-galaxy.md) · next drain **band 146** version/fingerprints (`PH-S2099…S2108`).
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 still `scripts/list-vdt-products.sh`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **96.01%** (rust 17737 / product 18474) · **346** tests · clippy 0. Vision rev **511**.
- **Канон продукту:** Rust **95–100%** / wasm **0–5%** (завжди), без Python/Java; bins — лише `src/bin/`.

## Стан (2026-08-17 · band 144 ✅)

- **Band 144:** always-on live copy — `scripts/gsv-live.sh` copies debug → `target/live/gsv-server.exe` and loops on `:9999`. `POST /api/update/apply` emits SSE `offline` + `{ok,applying}`; process exit gated (`GSV_UPDATE_APPLY_EXIT`; cargo-test `deps/` skips exit). `doUpdate()` stays offline until SSE `onopen`. Drain docs: do **not** kill the live copy before `cargo test`.
- **Canon:** [`gsv/GSV_ALWAYS_ON_UI.md`](gsv/GSV_ALWAYS_ON_UI.md) · plan [`superpowers/plans/2026-08-17-always-on-galaxy.md`](superpowers/plans/2026-08-17-always-on-galaxy.md) · next drain **band 145** products (`PH-S2089…S2098`).
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 still `scripts/list-vdt-products.sh`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **96.24%** (rust 17613 / product 18301) · **333** tests · clippy 0. Vision rev **511**.
- **Канон продукту:** Rust **95–100%** / wasm **0–5%** (завжди), без Python/Java; bins — лише `src/bin/`.

## Стан (2026-08-17 · band 143 ✅)

- **Band 143:** Galaxy chrome + type/chart — power menu stacks above cards (`z-index:80`, header ≥ 40); exclusive fullscreen + named Esc (`data-action='card-fs'`, `exitFullscreen()`); collapsed cards leave the grid (dock restore); `--fs-ui:13px` / `--fs-card:12px` / `--fs-meta:11px` / `--fs-chart:11px`; speed/rust SVG height 168, font-size 11, ui-monospace.
- **Canon:** [`gsv/GSV_ALWAYS_ON_UI.md`](gsv/GSV_ALWAYS_ON_UI.md) · plan [`superpowers/plans/2026-08-17-always-on-galaxy.md`](superpowers/plans/2026-08-17-always-on-galaxy.md) · next drain **band 144** live copy (`PH-S2079…S2088`).
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 still `scripts/list-vdt-products.sh`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **96.22%** (rust 17444 / product 18129) · **324** tests · clippy 0. Vision rev **510**.
- **Канон продукту:** Rust **95–100%** / wasm **0–5%** (завжди), без Python/Java; bins — лише `src/bin/`.

## Стан (2026-08-17 · always-on Galaxy spec queued · band 142 ✅)

- **Owner ask → spec/plan, no code this session:** always-on `:9999`; page **offline** only during binary swap; debug collapse/fullscreen/power-menu z-index; type+chart balance; VDT product picker + open folder + auto-parse; patch version per commit; fingerprint (IDE / bot / model / agent / time); UI toward README presentations.
- **Canon:** [`gsv/GSV_ALWAYS_ON_UI.md`](gsv/GSV_ALWAYS_ON_UI.md) · plan [`superpowers/plans/2026-08-17-always-on-galaxy.md`](superpowers/plans/2026-08-17-always-on-galaxy.md) · roadmap bands **143–147**.
- **Next drain:** band **143** chrome (`PH-S2069…S2078`). Live copy = band 144 (until then, debug exe still locks `cargo test`).
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 still `scripts/list-vdt-products.sh`.
- **Ratio / тести:** last measured band 142 — `gsv-loc-audit --stretch-96` → **96.43%** (rust 17360 / product 18002) · **320** tests · clippy 0. Vision rev **509**.

## Стан (2026-08-17 · band 142 ✅)

- **Band 142:** MCP HTTP sessions — `POST /mcp` `initialize` issues process-local `Mcp-Session-Id` (cap 32); unknown id → 404 `{ok:false}`; `DELETE /mcp` ends it; JSON discovery adds `sessions` / `session_count`; Galaxy card lists sessions. Stdio does not issue HTTP sessions.
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 still `scripts/list-vdt-products.sh` (environment, not hardcoded pair).
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **96.43%** (rust 17360 / product 18002) · **320** tests · clippy 0. Vision rev **509**.
- **Канон продукту:** Rust **95–100%** / wasm **0–5%** (завжди), без Python/Java; bins — лише `src/bin/`.

## Стан (2026-08-17 · band 141 ✅)

- **Band 141:** MCP HTTP SSE — `GET`/`POST /mcp` with `Accept: text/event-stream` flush notifications as finite SSE (`event: message`); JSON discovery adds `sse` / `streamable`; Galaxy card lists sse.
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 still `scripts/list-vdt-products.sh` (environment, not hardcoded pair).
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **96.36%** (rust 16990 / product 17632) · **314** tests · clippy 0. Vision rev **508**.
- **Канон продукту:** Rust **95–100%** / wasm **0–5%** (завжди), без Python/Java; bins — лише `src/bin/`.

## Стан (2026-08-17 · band 140 ✅)

- **Band 140:** MCP resource subscribe + logging notifications — `resources/subscribe`+`unsubscribe` (allowlisted `gsv://`; `..` / `file:` → `-32602`); stdio flushes `notifications/message` (filtered by log level) and `notifications/resources/updated` after `gsv_vision_sync` for subscribed vision URIs. `GET /mcp` adds `subscribe` / `subscription_count`.
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 still `scripts/list-vdt-products.sh` (environment, not hardcoded pair).
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **96.32%** (rust 16807 / product 17449) · **310** tests · clippy 0. Vision rev **507**.
- **Канон продукту:** Rust **95–100%** / wasm **0–5%** (завжди), без Python/Java; bins — лише `src/bin/`.

## Стан (2026-08-17 · band 139 ✅)

- **Band 139:** MCP logging + completions — `logging/setLevel` (RFC 5424, process-local on `AppState`) + `completion/complete` (`ref/resource` allowlisted `gsv://` + `ref/prompt` names; `..` / `file:` → `-32602`). `GET /mcp` adds `logging` / `completions` / `log_level`.
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 still `scripts/list-vdt-products.sh` (environment, not hardcoded pair).
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **96.25%** (rust 16471 / product 17113) · **305** tests · clippy 0. Vision rev **506**.
- **Канон продукту:** Rust **95–100%** / wasm **0–5%** (завжди), без Python/Java; bins — лише `src/bin/`.

## Стан (2026-08-17 · band 138 ✅)

- **Band 138:** MCP resources + prompts — `resources/list`+`read` (6 `gsv://` URIs, `preview::resolve` confine) + `prompts/list`+`get` (`gsv_status` / `gsv_vision_brief` / `gsv_drain`). `GET /mcp` adds `resource_count` / `prompt_count`. Kit trigger alias `abrakadabra`.
- **VDT kit:** `абракадабра` / `abrakadabra` Step 0 still `scripts/list-vdt-products.sh` (environment, not hardcoded pair).
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **96.19%** (rust 16202 / product 16844) · **302** tests · clippy 0. Vision rev **505**.
- **Канон продукту:** Rust **95–100%** / wasm **0–5%** (завжди), без Python/Java; bins — лише `src/bin/`.

## Стан (2026-08-17 · band 137 ✅)

- **Band 137:** MCP vision completeness — `gsv_vision` summary + `gsv_vision_{sprint_map,doc_preview,node_search,sync,extensions}` + `gsv_preview` (same confine as HTTP) → **26** tools.
- **VDT kit:** `абракадабра` Step 0 still `scripts/list-vdt-products.sh` (environment, not hardcoded pair).
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **96.10%** (rust 15820 / product 16462) · **296** tests · clippy 0. Vision rev **504**.
- **Канон продукту:** Rust **95–100%** / wasm **0–5%** (завжди), без Python/Java; bins — лише `src/bin/`.

## Стан (2026-08-17 · band 136 ✅)

- **Band 136:** MCP Galaxy UI — `render_mcp` / `GET /api/ui/card/mcp` (`CARD_NAMES` **32**,
  `rustCards` **24**, ops group). Extra read tools: vision map/board/progress/speeds/rust,
  hooks tests/bench, update → **19** tools. `GET /mcp` adds `stdio` / `http` / `tool_count`.
  Grok CLI project overlay: `.grok/config.toml` `[mcp_servers.gsv_mcp_openbot]`.
- **VDT kit:** `абракадабра` Step 0 still `scripts/list-vdt-products.sh` (environment, not hardcoded pair).
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **96.04%** (rust 15590 / product 16232) · **291** tests · clippy 0. Vision rev **503**.
- **Канон продукту:** Rust **95–100%** / wasm **0–5%** (завжди), без Python/Java; bins — лише `src/bin/`.

## Стан (2026-08-17 · band 135 ✅)

- **Band 135:** `gsv_mcp_openbot` — `src/mcp.rs` + `gsv-mcp` stdio JSON-RPC (NDJSON) +
  `GET`/`POST /mcp` on `gsv-server`. Tools wrap health/tracker/ratio/sli/toolchain/vision/omni/ide/terminal.
  Omni chat defaults to dry-run. Terminal = HTTP SLI allowlist (no extra shell). Secrets redacted.
  Auto-register: `.mcp.json`, `.cursor/mcp.json`, `opencode.json`. Grok Bot stays a client;
  public `/mcp` tunnel is owner opt-in.
- **VDT kit:** `абракадабра` Step 0 still `scripts/list-vdt-products.sh` (environment, not hardcoded pair).
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **96.01%** (rust 15414 / product 16055) · **289** tests · clippy 0. Vision rev **502**.
- **Канон продукту:** Rust **95–100%** / wasm **0–5%** (завжди), без Python/Java; bins — лише `src/bin/`.

## Стан (2026-08-17 · kit env-discover + MCP plan · band 134 ✅)

- **`абракадабра` Step 0:** `scripts/list-vdt-products.sh` → AskQuestion на проєкти з environment (workspace + git-сусіди), не hardcoded `gsv | poolai`. `PRODUCTS.md` = enrichment.
- **README:** OmniRouter-style presentations (`docs/assets/presentations/`) + GitHub Sponsors (`platinoff`) + MIT LICENSE + `.github/FUNDING.yml`.
- **Horizon band 135:** [`gsv/GSV_MCP_OPENBOT.md`](gsv/GSV_MCP_OPENBOT.md) — GSV owns MCP `gsv_mcp_openbot`; OpenCode / Cursor / Grok CLI / Grok Bot = clients. **Не реалізовано.**
- **Ratio / тести:** last measured band 134 — `gsv-loc-audit --stretch-96` → **96.34%** · **261** tests · clippy 0. Vision rev **501**.
- **Канон продукту:** Rust **95–100%** / wasm **0–5%** (завжди), без Python/Java; bins — лише `src/bin/`.
- **VDT kit:** GSV = точка входу (`.agents/skills/`, generic `.cursor/rules/`, `gsv.code-workspace`, [`PRODUCTS.md`](gsv/PRODUCTS.md)). `GSV_VDT_KIT.md` Status=Accepted.

## Стан (2026-08-17 · band 134 ✅)

- **Band 134:** HTTP response hardening — CSP / nosniff / `X-Frame-Options: DENY` / `Cache-Control: no-store` / COOP+CORP on every reply; POST body cap 256 KiB → 413 `{ok:false}`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **96.34%** (rust 14498 / product 15049) · **261** tests · clippy 0. Vision rev **501**.
- **Канон продукту:** Rust **95–100%** / wasm **0–5%** (завжди), без Python/Java; bins — лише `src/bin/`.
- **VDT kit:** GSV = точка входу (`.agents/skills/`, generic `.cursor/rules/`, `gsv.code-workspace`, [`PRODUCTS.md`](gsv/PRODUCTS.md)). `GSV_VDT_KIT.md` Status=Accepted.

## Стан (2026-08-17 · band 133 ✅)

- **Band 133:** localhost security — `--allow-lan` for off-loopback bind; CSRF POST gate (`Sec-Fetch-Site` / Origin); SLI terminal cargo/git allowlists (no `bash`/`node`/`npm`/`cat`); `/data/{file}` basename allowlist; preview canonicalize.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **96.22%** (rust 14044 / product 14595) · **256** tests · clippy 0. Vision rev **500**.
- **Канон продукту:** Rust **95–100%** / wasm **0–5%** (завжди), без Python/Java; bins — лише `src/bin/`.
- **VDT kit:** GSV = точка входу (`.agents/skills/`, generic `.cursor/rules/`, `gsv.code-workspace`, [`PRODUCTS.md`](gsv/PRODUCTS.md)). `GSV_VDT_KIT.md` Status=Accepted.

## Стан (2026-08-17 · band 132 ✅)

- **Band 132:** Rust header chrome HTML — `GET /api/ui/layout` `header` (`render_header` + `data-action`); node-search table via `/api/ui/card/node-search?q=&layer=`; `CARD_NAMES` **31** / chrome **8**; JS `tab` helper removed.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **96.20%** (rust 13959 / product 14510) · **246** tests · clippy 0. Vision rev **499**.
- **Канон продукту:** Rust **95–100%** / wasm **0–5%** (завжди), без Python/Java; bins — лише `src/bin/`.
- **VDT kit:** GSV = точка входу (`.agents/skills/`, generic `.cursor/rules/`, `gsv.code-workspace`, [`PRODUCTS.md`](gsv/PRODUCTS.md)). `GSV_VDT_KIT.md` Status=Accepted.

## Стан (2026-08-17 · band 131 ✅)

- **Band 131:** Rust shell — `GET /api/ui/layout` `html` (sidebar nav + `data-card-jump`); live `:root` CSS via `/api/ui/load-palette` + `/api/ui/load-theme` (stubs replaced); thin glue `<link>` + `loadLayout` uses `html`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **96.19%** (rust 13825 / product 14373) · **245** tests · clippy 0. Vision rev **498**.
- **Канон продукту:** Rust **95–100%** / wasm **0–5%** (завжди), без Python/Java; bins — лише `src/bin/`.
- **VDT kit:** GSV = точка входу (`.agents/skills/`, generic `.cursor/rules/`, `gsv.code-workspace`, [`PRODUCTS.md`](gsv/PRODUCTS.md)). `GSV_VDT_KIT.md` Status=Accepted.

## Стан (2026-08-17 · band 130 ✅)

- **Band 130:** chrome shell — rss-ticker from `feed.feed.items`; starfield Eco/FX/Ms counts from `StarfieldMode`; galaxy src+opacity; header ticker via `/api/ui/card/rss-ticker`; layout `chrome` (7).
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **96.06%** (rust 13707 / product 14269) · **244** tests · clippy 0. Vision rev **497**.
- **Канон продукту:** Rust **95–100%** / wasm **0–5%** (завжди), без Python/Java; bins — лише `src/bin/`.
- **VDT kit:** GSV = точка входу (`.agents/skills/`, generic `.cursor/rules/`, `gsv.code-workspace`, [`PRODUCTS.md`](gsv/PRODUCTS.md)). `GSV_VDT_KIT.md` Status=Accepted.

## Стан (2026-08-17 · band 129 ✅)

- **Band 129:** canon live UI port **9999** (`live_ui_url`, feed/pointer retarget) · `CARD_NAMES` **30** (`preview`/`terminal`/`sprint-focus`) · `rustCards` 23 · sidebar chip jump.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **96.00%** (rust 13502 / product 14064) · **242** tests · clippy 0. Vision rev **496**.
- **Канон продукту:** Rust **95–100%** / wasm **0–5%** (завжди), без Python/Java; bins — лише `src/bin/`.
- **VDT kit:** GSV = точка входу (`.agents/skills/`, generic `.cursor/rules/`, `gsv.code-workspace`, [`PRODUCTS.md`](gsv/PRODUCTS.md)). `GSV_VDT_KIT.md` Status=Accepted.

## Стан (2026-08-17 · band 128 ✅)

- **Band 128:** kit scripts (`check_target_disk`, `git-push-only`) + `gsv-speed-index` / `gsv-rust-diagnostics` + record wrappers · OpenCode AGENTS + `.opencode/package.json` · grouped Galaxy UI (`GET /api/ui/layout`, sidebar, 27 Rust cards, IDE preview) · GitHub `platinoff/GSV`.
- **Канон продукту:** Rust **95–100%** / wasm **0–5%** (завжди), без Python/Java; bins — лише `src/bin/`.
- **VDT kit:** GSV = точка входу (`.agents/skills/`, generic `.cursor/rules/`, `gsv.code-workspace`, [`PRODUCTS.md`](gsv/PRODUCTS.md)). `GSV_VDT_KIT.md` Status=Accepted.

## Стан (2026-08-17 · band 127 ✅)

- **VDT kit:** GSV = точка входу (`.agents/skills/`, generic `.cursor/rules/`, `gsv.code-workspace`, [`PRODUCTS.md`](gsv/PRODUCTS.md)). `GSV_VDT_KIT.md` Status=Accepted.
- **Канон продукту:** Rust **95–100%** / wasm **0–5%** (завжди), без Python/Java; bins — лише `src/bin/`.
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **97.04%** (rust 11820 / product 12181) · **230** tests · clippy 0. Vision rev **494**.

## Стан (2026-08-16 · band 126 ✅)

- **Канон:** Rust **95–100%** / wasm **0–5%** (завжди), без Python/Java; bins — лише `src/bin/`.
- **Ratio (виміряно):** `cargo run --bin gsv-loc-audit -- --stretch-96` → **96.87%** (rust 11176 / product 11537) —
  gate ≥95% ✅, stretch-96 ≥96% ✅.
  Звіт: `GSV/data/rust_ratio.json` (gitignored).
- **Тести (виміряно):** `cargo test` → **230** (102 unit + 3 stand-smoke bin unit + 8 `gsv_omni_contracts`
  + 7 `gsv_ratio_contracts` + 32 `gsv_server_contracts` + 6 `gsv_stand_smoke_contracts`
  + 12 `gsv_ui_contracts` + 8 `gsv_update_flow` + 52 `gsv_vision_contracts`).
  `cargo clippy --all-targets` → **0** warnings. `cargo fmt` clean.
- **Бокси:** Tracker · SLI console · Toolchain · IDE · Update/offline · Box preview · SLI terminal ·
  Tests/bench hooks · **Ratio** · **Vision** · **Vision Map** · **Sprint Map** · **Doc Preview** ·
  **Vision Sync** · **Sprint Queue** · **Sprint Board** · **Sprint Progress** · **Sprint Focus** ·
  **Galaxy UI parity** (band 119) · **UI fragments** (band 121: `GET /api/ui/card/:name`, 20 Rust renderers) ·
  **OmniRouter** (Rust AI-проксі/роутер; card `omni` band 121).
- **Band 126 (GSV stand smoke + ops canon):** `gsv-http-stand-smoke` bin (48 live checks: core boxes +
  vision* ok-gate + SVG status + 20 ui cards) · `gsv_stand_smoke_contracts` (6) · docs canon
  (GSV_SERVER/GSV_BOXES/README/roadmap band 126) · ratio hold **96.87%** · **Vision rev 493**.

## Що зроблено

### Band 134 (PH-S1979…S1988, ✅ 2026-08-17) — HTTP response hardening
- `security::SECURITY_HEADERS` + `CSP` + `MAX_BODY_BYTES` (256 KiB) + `gate_content_length`.
- `server::security_gate`: CSRF POST gate + body cap + insert headers on every reply (including 403/413).
- Axum `DefaultBodyLimit::max(MAX_BODY_BYTES)` for chunked bodies.
- `tests/gsv_security_contracts.rs`: health headers, 403 still nosniff/no-store, oversized POST 413 JSON.

### Band 133 (PH-S1969…S1978, ✅ 2026-08-17) — localhost security hardening
- `src/security.rs`: loopback bind (`ensure_bind_host` + `--allow-lan`), POST gate (`gate_post`), `/data/{file}` allowlist (`DATA_FILES`; `omni.toml` not served).
- Terminal: drop `bash`/`node`/`npm`/`cat`; cargo/git subcommand allowlists; `..` `\\` `~` forbidden.
- Preview `resolve`: reject absolute/`ParentDir`; canonicalize under repo root.
- `tests/gsv_security_contracts.rs`: loopback Origin allowed, non-local Origin + cross-site POST 403, data `..` / unknown / `omni.toml` 400, Omni GET has no `api_key` field.

### Band 132 (PH-S1959…S1968, ✅ 2026-08-17) — Rust header chrome HTML + node-search fragment
- `render_header` emits `#headerActions` inner HTML with `data-action` (gpu-cycle / auto-toggle / resync / notify-update / power-*).
- `layout_wire` includes `header`; thin glue `loadLayout` injects it (static markup remains offline fallback).
- `render_node_search` table from node-search wire; `GET /api/ui/card/node-search?q=&layer=`; `CARD_NAMES` 31 / `CHROME_CARDS` 8.
- JS `tab` helper and search-result HTML builder removed; header onclick → event delegation.

### Band 131 (PH-S1949…S1958, ✅ 2026-08-17) — Rust shell: live palette/theme CSS + layout nav HTML
- `render_nav` emits inner sidebar HTML with `data-card-jump` / `data-group`; `layout_wire` includes `html`.
- `GET /api/ui/load-palette` → `GalaxyPalette::as_css_root` `:root` CSS (was hardcoded stub).
- `GET /api/ui/load-theme` → `SprintThemeReport::as_css_root` `text/css` (was unused JS stub).
- Thin glue: `<link rel="stylesheet" href="/api/ui/load-palette|load-theme">`; `loadLayout` uses `d.html`; JS CSS-var mappers removed.
- Offline fallback: inline `:root` in `ui/index.html` remains.

### Band 130 (PH-S1939…S1948, ✅ 2026-08-17) — chrome shell: real wires + Rust RSS ticker
- Chrome wires: rss-ticker reads `feed.feed.items` (was empty `feed.items`); starfield 48/160/96 from `StarfieldMode::star_count`; galaxy `/api/vision/galaxy.svg` + palette opacity; gpu default `fx`; power actions; empty client-owned dock.
- Renderers: `err_html`/`empty_html`; rss ticker `<li class='rss-ticker-item'>` duplicated for marquee.
- `GET /api/ui/layout` `chrome` array (7). Thin glue: `loadRssTicker` + `resync` use `/api/ui/card/rss-ticker`.
- Docs: ARCHITECTURE (`ui/` not `src/ui/`, axum, metrics in `docs/vision/`); GSV_ROLES GitHub `platinoff/GSV`.

### Band 129 (PH-S1929…S1938, ✅ 2026-08-17) — canon port 9999 + dashboard card registry
- `live_ui_url` (`DEFAULT_HOST`:`DEFAULT_PORT`) for feed samples; `docs/vision/feed.json` + pointer page retarget 8891 → 9999.
- `CARD_NAMES` 30: `preview` / `terminal` / `sprint-focus` renderers + `card_wire`; layout cards ⊆ registry.
- Thin glue: `data-card` + `rustCards` 23; sidebar chips `data-card-jump` set group + scroll.
- `extensions.json` `active_sprint` catch-up (was stale PH-S1899 vs `manifest.next_sprint`).

### Band 128 (PH-S1919…S1928, ✅ 2026-08-17) — kit ops + grouped Galaxy UI
- Kit scripts: `scripts/check_target_disk.sh`, `scripts/git-push-only.sh`, `bin/record-test-speed.sh`, `bin/record-rust-diagnostics.sh`, `bin/gsv-vision-sync.sh`.
- Bins: `gsv-speed-index`, `gsv-rust-diagnostics` → `docs/vision/{speed_index,rust_diagnostics}.json`.
- OpenCode: `AGENTS.md` MSYS2/`question` canon; `.opencode/package.json`; `.cursor/commands/git-push.md`.
- UI: `GET /api/ui/layout` + sidebar groups (ops/vision/sprint/studio); Rust cards for health/update/ide/vision*; IDE last-8 jsonl preview; skip link + `:focus-visible`.
- Git: `Cargo.toml` repository `https://github.com/platinoff/GSV`.

### Band 127 (PH-S1909…S1918, ✅ 2026-08-17) — GSV VDT kit
- Shared kit in this repo: `.agents/skills/` (abracadabra host + marketplace, no `poolai-documentation`).
- Generic `.cursor/rules/` (session, roles, MSYS2, git, rust, cursor baseline) + `gsv-vdt-entry.mdc`.
- Client mirrors `.cursor/skills/` + `.opencode/skills/` (`scripts/sync-vdt-skill-mirrors.sh`).
- `docs/gsv/GSV_VDT_KIT.md` Accepted · `docs/gsv/PRODUCTS.md` · `gsv.code-workspace`.
- PoolAI thin pointer (`gsv-kit-pointer.mdc`, fallback abracadabra). `AGENTS.md` + `GSV_ROLES.md` entry-point.

### Band 102 (PH-S1659…S1668, ✅ 2026-08-01) — GSV migration
- `GSV/docs/gsv/` канон + `GSV/Cargo.toml` (окремий workspace, `.cargo/config.toml` → `target-dir`).
- `gsv-server` bin (axum + tokio, SSE `/events`, single-page UI `ui/index.html` embedded).
- Бокси: Tracker, SLI console, Toolchain, IDE, Update/offline, Box preview, SLI terminal, Tests/bench hooks.
- 52 tests green (на той момент), clippy 0. FM §5.12 §5.83 ✅.

### Band 108 (PH-S1719…S1728, ✅ 2026-08-05) — roles/ratio/roles canon (poolAI дисципліна)
- **PH-S1719** `GSV/docs/GSV_ROLES.md` — ролі VDT (Власник/Оркестратор/Субагенти) + канон сесії
  (S0 disk-first → project scan warnings-first → drain ≤10 PH-S* → Speeds + Rust panel → vision-sync
  → один commit → `git push` + самарі).
- **PH-S1720** `GSV/src/bin/gsv_loc_audit.rs` + `GSV/src/boxes/ratio.rs` — LOC ratio audit
  (дзеркало poolAI `poolai_loc_audit.rs`): `git ls-files --full-name`, `classify_product_path`,
  `--print/--no-write/--advisory/--min-ratio/--output/--data-dir`, gate ≥95%.
- **PH-S1721** `tests/gsv_ratio_contracts.rs` — 7 integration contracts (audit/save/load/wire/API).
- **PH-S1722** Ratio box + `GET /api/ratio` + UI Ratio card.
- **PH-S1723** `GSV/docs/MEMORY.md` (цей файл) + `GSV/docs/README.md` індекс.
- **PH-S1724** `GSV/docs/HANDOFF_NEW_SESSION.md` + `NEXT_SESSION_PROMPT.md`.
- **PH-S1725** FM §5.12 §5.89 band 108 + `GSV/docs/gsv/GSV_TECH_ROADMAP.md` band 108.
- **PH-S1726** poolAI docs parity (FUNCTIONALITY_DIGEST / vision README / GSV rows).
- **PH-S1727** poolAI `docs/development/HANDOFF_NEW_SESSION.md` + `NEXT_SESSION_PROMPT.md`.
- **PH-S1728** Band close: ratio hold, fmt, clippy, cargo test, docs canon, vision-sync, push.

### Band 109 (PH-S1729…S1738, ✅ 2026-08-05) — Vision box (poolAI vision canon mirror)
- **PH-S1729** `GSV/src/boxes/vision.rs` + `boxes/mod.rs` + Cargo `[[bin]]` — serde-структури
  (manifest nodes/edges/layers + feed) та реєстрація боксу.
- **PH-S1730** manifest wire: read `GSV/docs/vision/manifest.json` → `GSV/data/gsv_manifest.json`;
  `GET /api/vision/manifest` (nodes/edges/layers).
- **PH-S1731** feed wire: `GSV/docs/vision/feed.json` → `GSV/data/gsv_feed.json`; `GET /api/vision/feed`.
- **PH-S1732** `GSV/src/bin/gsv_vision_sync.rs` — mirror + `--check` drift gate (source parse +
  revision parity). Live: rev 458, 1218 nodes, 535 edges, 12 feed items.
- **PH-S1733** Vision UI card (`ui/index.html`): summary + sprint feed ticker; ratio-safe (без 161 KB legacy JS).
- **PH-S1734** `tests/gsv_vision_contracts.rs` — 7 integration contracts.
- **PH-S1735** `GSV/docs/VISION.md` + `GSV_MIGRATION.md` rows ✅ + MEMORY mark.
- **PH-S1736** poolAI vision parity (`GSV/docs/vision/README.md` + cross-check).
- **PH-S1737** Ratio hold advisory (`gsv-loc-audit --min-ratio 0.95 --advisory`).
- **PH-S1738** Band close: ratio hold, fmt, clippy, cargo test, docs canon, vision-sync, push.

### Band 110 (PH-S1739…S1748, ✅ 2026-08-05) — Vision map UI (svg + map wire)
- **PH-S1739** `boxes/vision.rs` `map_report`/`wire_map` — легкий map-звіт (layers L0..L5 z-sorted,
  `node_count`/`edges_from`, `edge_kinds` tally, totals); `GET /api/vision/map`.
- **PH-S1740** `GSV/ui/vision.svg` (порт `GSV/docs/vision/vision.svg`, include_str!) + `GET /assets/vision.svg`
  (`image/svg+xml`). `.svg` = audit Ignored → ratio-neutral.
- **PH-S1741** Vision Map card у `ui/index.html`: per-layer chips + edge kinds + посилання на svg.
- **PH-S1742** `tests/gsv_vision_contracts.rs` +3 → **10** (map endpoint: 6 layers z-sorted, layer_sum;
  feed `?status=closed`; `/assets/vision.svg` 200 + content-type).
- **PH-S1743** `GET /api/vision/feed?status=closed|open|all` — серверний фільтр.
- **PH-S1744** `GSV/docs/VISION.md` (map/feed-filter/svg) + `GSV_MIGRATION.md` rows ✅ (svg, map, feed filter).
- **PH-S1745** poolAI vision parity + `GSV_TECH_ROADMAP.md` band 110 row.
- **PH-S1746** Ratio hold advisory (`gsv-loc-audit --min-ratio 0.95 --advisory`).
- **PH-S1747** vision-sync close: `gsv-vision-sync` refresh + poolAI vision rev **461**.
- **PH-S1748** Band close: ratio hold, fmt, clippy, cargo test, docs canon, vision-sync, push.

### Band 111 (PH-S1749…S1758, ✅ 2026-08-05) — sprint-map + doc-preview (Vision UI логіка)
- **PH-S1749** `boxes/vision.rs` `SprintMapReport`/`SprintNode` structs + `sprint_map_report`/`wire_sprint_map` —
  sprint-queue map (scoping/tracking edges: `sprint-scope`+`queue`+`session-tracks` → links з `NodeRef`,
  per-node targets tally → `modules`, kinds/layers stats); `GET /api/vision/sprint-map`.
- **PH-S1750** `DocPreviewReport`/`LinkTarget` structs + `doc_preview`/`wire_doc_preview` — node + 1-hop
  neighbors (`links_out`/`links_in`); `GET /api/vision/doc-preview?id=<node>` (missing → `ok:false` + error).
- **PH-S1751** `tests/gsv_vision_contracts.rs` sprint-map contracts (endpoint kinds ⊆
  {sprint-scope,queue,session-tracks}, real-workspace report + module ids) → 12.
- **PH-S1752** doc-preview contracts (endpoint node/link_count, missing+empty params, real-workspace
  1-hop read) → **14**.
- **PH-S1753** Sprint Map card у `ui/index.html`: modules/kinds/links (details), rev/next/last header.
- **PH-S1754** Doc Preview card: node id input (`galaxy_grid` default) + out/in links + sections/path.
- **PH-S1755** `GSV/docs/VISION.md` (sprint-map/doc-preview API) + `MEMORY.md` band 111 + HANDOFF/NEXT_SESSION.
- **PH-S1756** poolAI parity: `GSV/docs/gsv/GSV_MIGRATION.md` row 21 ✅, `GSV/docs/vision/README.md`,
  `GSV_TECH_ROADMAP.md` band 111.
- **PH-S1757** Ratio hold advisory (`gsv-loc-audit --min-ratio 0.95 --advisory`) + poolAI parity hold.
- **PH-S1758** Band close: ratio hold, fmt, clippy, cargo test, docs canon, vision-sync, push.

### Band 112 (PH-S1759…S1768, ✅ 2026-08-05) — vision auto-sync + sprint-queue planning
- **PH-S1759** `boxes/vision.rs` `Extensions` struct (active_sprint/revision/ui_version/updated_at +
  opaque `scopes` map) + `read_extensions`/`save_extensions`/`load_extensions`/`source_extensions` +
  `extensions_source`/`extensions_target` paths; `sync()` також мірорить `gsv_extensions.json`;
  `SyncReport` + extensions_source/target; `gsv-vision-sync` bin друкує extensions target;
  `collect_drift` парсить extensions. `wire_extensions` → `GET /api/vision/extensions`
  (active_sprint, revision, ui_version, scope_count + sorted scopes).
- **PH-S1760** `wire_sync` → `GET /api/vision/sync` — auto-sync: re-mirror canon у знімки +
  drift gate у відповіді (`drift: []` = зелено); route у `server/mod.rs`.
- **PH-S1761** `SprintQueueReport`/`sprint_queue_report`/`wire_sprint_queue` → `GET /api/vision/sprint-queue` —
  manifest.sprint_queue → `entries`/`open_count`, extensions.active_sprint → `active_sprint`,
  `planned` = entries ∪ активний спринт.
- **PH-S1762** `tests/gsv_vision_contracts.rs` — extensions contracts (real-workspace read:
  revision>0, scopes present, active == manifest.next; sync snapshot now also asserts
  `gsv_extensions.json` + extensions revision parity) → 17.
- **PH-S1763** sprint-queue + sync contracts (`/api/vision/sync` ok + empty drift + synced_at;
  `/api/vision/sprint-queue` ok + active == next + planned includes active; real-workspace
  `sprint_queue_report`) → **19**.
- **PH-S1764** UI cards у `ui/index.html`: **Vision Sync card** (Resync snapshot button + drift status)
  та **Sprint Queue card** (rev/next/last + active/open + planned details).
- **PH-S1765** `GSV/docs/VISION.md` (sync/extensions/sprint-queue API + sync док-секції) +
  `MEMORY.md` band 112 + HANDOFF/NEXT_SESSION.
- **PH-S1766** poolAI parity: `GSV/docs/gsv/GSV_MIGRATION.md` rows ✅, `GSV/docs/vision/README.md`,
  `GSV_TECH_ROADMAP.md` band 112, FM §5.93, poolAI HANDOFF/NEXT.
- **PH-S1767** Ratio hold advisory (`gsv-loc-audit --min-ratio 0.95 --advisory` → 95.56%) +
  poolAI parity hold.
- **PH-S1768** Band close: ratio hold, fmt, clippy, cargo test (118), docs canon, vision-sync, push.

### Band 113 (PH-S1769…S1778, ✅ 2026-08-05) — Galaxy UI: node search + interactive map
- **PH-S1769** `boxes/vision.rs` `NodeSearchReport`/`NodeSearchResult` structs + `node_search`/
  `wire_node_search` — case-insensitive match по id/label/path/sections, `top-N 25`
  (`NODE_SEARCH_LIMIT`) layer-z-sorted, `links_out`/`links_in` tallies;
  `GET /api/vision/node-search?q=&layer=` route + handler у `server/mod.rs`.
- **PH-S1770** `tests/gsv_vision_contracts.rs` node-search contracts (real-workspace
  id/label/path/links + layer-z sort, layer filter, no-match empty + cap) → 22.
- **PH-S1771** `tests/gsv_server_contracts.rs` node-search endpoint contract
  (ok + revision + results + links_out/in u64; empty `q` → ok true) → 19.
- **PH-S1772** Vision Map card рендерить **inline** `assets/vision.svg` (`<img>` через
  `GET /assets/vision.svg`) + chips/kinds.
- **PH-S1773** Layer filter + search UX у `ui/index.html`: клікабельні layer chips
  (active filter → `toggleMapLayer`), node-search input + results table
  (`searchVisionNodes`) → deep-link у Doc Preview (`openSearchNode`).
- **PH-S1774** `GSV/docs/VISION.md` (node-search API + інтерактивна мапа) + `MEMORY.md` band 113 +
  HANDOFF/NEXT_SESSION.
- **PH-S1775** poolAI parity: `GSV/docs/gsv/GSV_MIGRATION.md` row ✅, `GSV/docs/vision/README.md`,
  `GSV_TECH_ROADMAP.md` band 113, FM §5.94.
- **PH-S1776** Ratio hold advisory (`gsv-loc-audit --min-ratio 0.95 --advisory`) + poolAI parity hold.
- **PH-S1777** vision-sync close: `gsv-vision-sync` refresh + poolAI vision rev++.
- **PH-S1778** Band close: ratio hold, fmt, clippy, cargo test, docs canon, vision-sync, push.

### Band 114 (PH-S1779…S1788, ✅ 2026-08-05) — GSV Sprint-board + progress UI
- **PH-S1779** `boxes/vision.rs` `SprintBoardReport`/`SprintBoardColumn` structs +
  `sprint_board_report`/`wire_sprint_board` — доска зі спільного `planned` queue:
  columns open/closed/planned (active або `open` → open; `closed`/`done` → closed; решта → planned),
  counts + `progress_pct` = closed/total; `GET /api/vision/sprint-board` route + handler
  у `server/mod.rs`.
- **PH-S1780** `SprintProgressReport`/`SprintLayerProgress` structs +
  `sprint_progress_report`/`wire_sprint_progress` — status counts + per-layer розподіл
  (`node_count`/`linked_count` проти чергових спринтів, z-ascending);
  `GET /api/vision/sprint-progress` route + handler.
- **PH-S1781** `tests/gsv_vision_contracts.rs` sprint-board contracts (grouping, progress pct
  formula, column order, active in open, unique across columns, closed-only-done, revision parity,
  wire ok) → 30.
- **PH-S1782** sprint-progress contracts (layers match manifest + node sums, statuses sum,
  z-ordered, linked reflects queue sprints, planned formula, wire ok) → **38**.
- **PH-S1783** `tests/gsv_server_contracts.rs` sprint-board + sprint-progress endpoint contracts
  (ok + status sums + columns/layers shape) → 21.
- **PH-S1784** Sprint Board card у `ui/index.html`: progress bar + open/closed/planned
  колонки-details (`bar()` helper).
- **PH-S1785** Sprint Progress card: progress bar + per-layer таблиця nodes/linked.
- **PH-S1786** `GSV/docs/VISION.md` (sprint-board/sprint-progress API + band 114 section) +
  `MEMORY.md` band 114 + HANDOFF/NEXT_SESSION.
- **PH-S1787** poolAI parity: `GSV/docs/gsv/GSV_MIGRATION.md` rows ✅, `GSV/docs/vision/README.md`,
  `GSV_TECH_ROADMAP.md` band 114, FM §5.95.
- **PH-S1788** Band close: ratio hold (**95.02%**), fmt, clippy 0, cargo test (140), docs canon,
  vision-sync rev 467, push.

### Band 115 (PH-S1789…S1798, ✅ 2026-08-07) — GSV migration completion (legacy vision supersession)
- **PH-S1789** `GSV/docs/LEGACY_PARITY.md` — parity audit: кожна legacy-панель
  (`GSV/docs/vision/index.html`: layers/queue/map/speeds/rust/links/preview + chrome) → GSV
  endpoint+card / superseded / out-of-scope. Єдині прогалини: Speeds + Rust diagnostics.
- **PH-S1790** `SpeedIndexReport`/`SpeedIndexLatest` structs + `read_speed_index`/
  `save_speed_index`/`load_speed_index`/`source_speed_index` (live → snapshot → empty default) +
  `wire_speed_index` → `GET /api/vision/speeds` (route + handler у `server/mod.rs`).
- **PH-S1791** `RustDiagnosticsReport`/`RustDiagLatest` + `read_rust_diagnostics`/
  `save_rust_diagnostics`/`load_rust_diagnostics`/`source_rust_diagnostics` +
  `wire_rust_diagnostics` → `GET /api/vision/rust-diagnostics`.
- **PH-S1792** contracts: `tests/gsv_vision_contracts.rs` (real-workspace speed_index/
  rust_diagnostics reads + wire shapes) + `tests/gsv_server_contracts.rs`
  (`/api/vision/speeds` + `/api/vision/rust-diagnostics` 200/ok/present/shape).
- **PH-S1793** Speed Index card + Rust Diagnostics card у `ui/index.html` (present/empty
  states, latest metrics, top clippy codes).
- **PH-S1794** `GSV/docs/gsv/GSV_MIGRATION.md` rows ✅ (speed_index/rust_diagnostics moved;
  `vision.js`/`vision.css` superseded) + `GSV/docs/gsv/GSV_TECH_ROADMAP.md` band 115.
- **PH-S1795** `GSV/docs/VISION.md` +band 115 endpoints/section; `MEMORY.md` band 115;
  HANDOFF/NEXT band 115.
- **PH-S1796** poolAI parity: FM §5.12 §5.96, HANDOFF/NEXT band 115, `GSV/docs/vision/` canon.
- **PH-S1797** ratio hold advisory: `gsv-loc-audit` **95.04%**; legacy JS не переносимо.
- **PH-S1798** Band close: ratio hold (**95.04%**), fmt, clippy 0, cargo test (150), docs canon,
  vision-sync rev 468, push.

### Band 116 (PH-S1799…S1808, ✅ 2026-08-07) — GSV history charts (speed/rust analytics)
- **PH-S1799** FM §5.97 queue (band 116) + manifest sync (10 open).
- **PH-S1800** typed `SpeedTestCiRecord`/`SpeedBenchRecord` + `test_ci_history`/`bench_history`
  у `SpeedIndexReport`; `read_speed_index`/`SpeedIndexFile` carry history (source fallback unchanged).
- **PH-S1801** typed `RustDiagRecord` + `history` у `RustDiagnosticsReport`; `read_rust_diagnostics`
  carry history (source fallback unchanged).
- **PH-S1802** vision tests 20 → **23**: `history_records_parse_typed_fields`,
  `speed_chart_svg_renders_bars_and_empty_state`, `rust_chart_svg_renders_bars_and_empty_state`
  (+ `data_dir_of` helper).
- **PH-S1803** `speed_index_chart_svg` + `/api/vision/speeds.svg` (Rust-rendered SVG: test-ci
  wall bars green ok / red fail, ≤24 runs, footer latest bench) + `<img id="i-speed-chart">`.
- **PH-S1804** `rust_diagnostics_chart_svg` + `/api/vision/rust-diagnostics.svg` (warnings
  orange + errors red grouped bars, command footer) + `<img id="i-rust-chart">`.
- **PH-S1805** stand smoke: `/api/vision/speeds.svg` + `/api/vision/rust-diagnostics.svg` →
  200 `image/svg+xml`; `poolai-ui-wasm` defer row у `GSV_MIGRATION.md` + roadmap.
- **PH-S1806** `GSV/docs/VISION.md` +band 116 section/endpoints; MEMORY band 116; HANDOFF/NEXT band 116.
- **PH-S1807** poolAI parity: `GSV/docs/vision/README.md`; FM §5.12 §5.97; `GSV_TECH_ROADMAP.md` band 116;
  poolAI HANDOFF/NEXT band 116.
- **PH-S1808** Band close: ratio hold (**95.26%**), fmt, clippy 0, cargo test (153), docs canon,
  vision-sync rev 469, push.

### Band 117 (PH-S1809…S1818, ✅ 2026-08-07) — GSV legacy vision deactivation
- **PH-S1809** FM §5.98 queue (band 117) + manifest sync (10 open, rev 469).
- **PH-S1810** `GSV/docs/vision/index.html` → minimal GSV pointer page (no `vision.js`/`vision.css` refs).
- **PH-S1811** `vision.js`/`vision.css` DEACTIVATED banner (band 117); `GSV/docs/vision/README.md` deactivation note.
- **PH-S1812** live link retarget: poolai-vision-sync feed links → `http://127.0.0.1:8891/#b-sprint-board`;
  GSV `vision.rs` sample feed links; RUN_LOCAL/GSV_SERVER/docs-gsv README/SPEED_INDEX/RUST_DIAGNOSTICS → GSV.
- **PH-S1813** legacy test retirement: `poolai_vision_sync.rs` unit ×4 + `galaxy_horizon_s1011/s1019/s1039`
  → deactivated pointer state; e2e `vision.spec.ts`/`a11y.spec.ts` pointer assertions; `VISION_MAP_BAND40_ROWS` markers.
- **PH-S1814** `LEGACY_PARITY.md` + `GSV_MIGRATION.md` band 117 (index/JS/CSS deactivated).
- **PH-S1815** `VISION.md`/`MEMORY.md`/HANDOFF/NEXT band 117.
- **PH-S1816** poolAI parity: `GSV/docs/vision/README.md`; FM §5.12 §5.98; `GSV_TECH_ROADMAP.md` band 117;
  poolAI HANDOFF/NEXT band 117.
- **PH-S1817** ratio hold advisory (**95.26%**) + vision-sync rev 470 (poolai + gsv + --check).
- **PH-S1818** Band close: ratio hold, fmt, clippy 0, cargo test (poolAI test-ci + GSV 153), docs canon,
  vision-sync rev 470, push.

### Band 118 (PH-S1819…S1828, ✅ 2026-08-08) — GSV sprint UI migration (theme + focus map)
- **PH-S1819** FM §5.99 queue (band 118 sprint UI migration) + §5.12 header (master horizon).
- **PH-S1820** `SprintThemeReport`/`SprintPillTheme`/`SprintChipTheme`/`SprintQueueStateTheme`/
  `SprintLayerColor`/`SprintEdgeKindColor` structs + `sprint_theme_report`/`wire_sprint_theme` →
  `GET /api/vision/sprint-theme` (sprint `#a78bfa`/next `#c4b5fd`, pill/chip/queue colors,
  layer L0–L5 + edge-kind palettes, revision/git_head/active/next).
- **PH-S1821** `sprint_token_matches`/`path_matches_glob`/`nodes_for_sprint` +
  `sprint_focus_svg` → `GET /api/vision/sprint-focus.svg?sprint=` (sprint-dim: in-scope accent,
  out-of-scope opacity 0.22/text 0.28, edges tinted; default active sprint; empty-state).
- **PH-S1822** contracts: `gsv_vision_contracts` (theme real-workspace + wire shapes + focus svg
  highlight/dim/empty) → 44; `gsv_server_contracts` (theme + focus endpoints, `get_text` helper) → 25.
- **PH-S1823** `GSV/ui/index.html`: `--sprint*` CSS-змінні + sprint-pill/queue-state chips у
  Sprint Queue/Board cards; Sprint Focus card (input + button + `<img id="i-sprint-focus">`);
  `loadSprintTheme` apply + `loadSprintFocus` ре-запит svg.
- **PH-S1824** `GSV/docs/VISION.md` +band 118 (theme/focus endpoints + section); `MEMORY.md` band 118;
  GSV HANDOFF/NEXT band 118.
- **PH-S1825** poolAI parity: `GSV/docs/vision/README.md`; FM §5.12 §5.99; `GSV_TECH_ROADMAP.md` band 118.
- **PH-S1826** Ratio hold advisory: `gsv-loc-audit --min-ratio 0.95 --advisory` → **95.35%** +
  poolAI ratio96 advisory hold.
- **PH-S1827** vision-sync close: `poolai-vision-sync` rev **471**; `--check` ok; feed/manifest updated.
- **PH-S1828** Band close: ratio hold (95.35%), fmt, clippy 0, cargo test (163), docs canon,
  vision-sync rev 471, push.

### Band 119 (PH-S1829…S1838, ✅ 2026-08-08) — GSV Galaxy UI full parity (colors + box behaviors)
- **PH-S1829** `GSV/docs/gsv/GSV_TECH_ROADMAP.md` band 119 (PH-S1829…S1838): full `vision.css`
  `:root` palette + header chrome (ticker, GPU modes, power menu) + panel dock/collapse/
  fullscreen + starfield/galaxy backdrop scope.
- **PH-S1830** `GalaxyPalette` struct (bg-deep/bg/panel/panel-solid/border/border-bright/
  text/muted/accent/accent-2/glow/sidebar-w, `layers`+`layers_dim` L0–L5, `edge_docs`/
  `edge_code`/`edge_toml`, `ext_md`/`ext_rs`/`ext_json`/`ext_toml`, `sprint`, `bg_tone`,
  `galaxy_bg_opacity`) + `wire_palette` → `GET /api/vision/palette` (exact legacy `:root`
  values + `ok`/`revision`).
- **PH-S1831** `StarfieldMode`/`starfield_svg` (deterministic LCG per mode; eco sparse/static,
  fx dense+glow, ms medium) → `GET /api/vision/starfield.svg?mode=eco|fx|ms` (`image/svg+xml`).
- **PH-S1832** `galaxy_svg` (radial nebula gradients + spiral-arm ellipses) →
  `GET /api/vision/galaxy.svg` (`image/svg+xml`).
- **PH-S1833** header chrome: RSS ticker (`loadRssTicker` → `/api/vision/feed?status=all`,
  duplicated track), GPU mode button (`btnGpu` Eco/FX/Ms cycle → `body.vision-(eco|fx|ms)` +
  starfield re-request), power menu (`powerSoft` → `/api/vision/sync`, `powerReload` → resync,
  `powerOffline` → forced offline), meta-rev/meta-trail.
- **PH-S1834** panel dock + Esc-fullscreen: card `–` collapse → `syncDock()` chips (restore);
  `□` fullscreen + `Esc` exits; `.galaxy-backdrop` `<img>` + `#starfield` fixed backdrops
  (ratio-safe `.svg`).
- **PH-S1835** contracts: `gsv_vision_contracts` (palette == legacy `:root`, starfield/galaxy
  svg shape + mode variance, empty-state) → **50**; `gsv_server_contracts` (palette +
  starfield + galaxy 200 + `image/svg+xml`) → **29**.
- **PH-S1836** `GSV/docs/VISION.md` +band 119 (palette/starfield/galaxy/header UI) +
  `LEGACY_PARITY.md` rows migrated; MEMORY band 119; HANDOFF/NEXT band 119.
- **PH-S1837** Ratio hold advisory: `gsv-loc-audit` **95.18%** (UI delta компенсовано Rust
  tests; JS compact); vision-sync rev **472** (poolai + gsv + `--check`).
- **PH-S1838** Band close: ratio hold (95.18%), fmt, clippy 0, cargo test (**183**), docs canon,
  vision-sync rev 472, push.

### Band 120 (PH-S1839…S1848, ✅ 2026-08-08) — GSV Ratio 96% stretch
- **PH-S1839** `GSV/docs/gsv/GSV_TECH_ROADMAP.md` band 120 (PH-S1839…S1848): ratio **95.18% → ≥96%**
  via `gsv-loc-audit --stretch-96` advisory + server-rendered UI card fragments
  (`GET /api/ui/card/:name`, Rust HTML renderers) + compact UI (JS/CSS).
- **PH-S1840** `--stretch-96` advisory: `ratio.rs` `STRETCH_96_TARGET = 0.96` + `AuditConfig.stretch_96`
  + `RustRatioReport.stretch_target`/`meets_stretch_96` (`#[serde(default)]` — старий `rust_ratio.json`
  читається); `gsv_loc_audit.rs` `--stretch-96` flag → advisory (exit 0).
- **PH-S1841** `gsv_ratio_contracts` — roundtrip + JSON shape + wire stretch fields.
- **PH-S1842** `boxes/ui.rs`: `esc`/`tab`/`bar` helpers + 12 Rust renderers (tracker/sli/toolchain/
  ratio/hooks-tests/hooks-bench/sprint-map/sprint-queue/sprint-progress/sprint-board/speed-index/
  rust-diagnostics) + `render_card` dispatch + `CARD_NAMES` (12).
- **PH-S1843** `GET /api/ui/card/{name}` в `server/mod.rs` (`api_ui_card` handler; 404 unknown).
- **PH-S1844** `ui/index.html` thin glue: `getText(card)` → `api/ui/card/:name` + `rustCards` (12);
  8 JS renderers видалено.
- **PH-S1845** `gsv_ui_contracts` (6) + `gsv_server_contracts` (**30**, `ui_card_endpoint_renders_fragment_and_rejects_unknown`).
- **PH-S1846** Ratio 96% measurement: `gsv-loc-audit --stretch-96` → **96.51%** (rust 10027 / product 10390) ✅.
- **PH-S1847** GSV docs canon: MEMORY band 120; HANDOFF/NEXT band 120; `GSV_TECH_ROADMAP.md` band 120.
- **PH-S1848** Band close: ratio **≥96%**; fmt/clippy 0; cargo test (**204**); docs canon; vision-sync rev bump; push.

### Band 121 (PH-S1849…S1855, ✅ 2026-08-10) — GSV OmniRouter box parity
- **PH-S1849** `GSV/docs/gsv/GSV_TECH_ROADMAP.md` band 121 (PH-S1849…S1855): port the last hand-rolled
  JS card renderer (`renderOmni`) to the Rust UI fragment box — `GET /api/ui/card/omni`, `CARD_NAMES` 13.
- **PH-S1850** `boxes/ui.rs`: `render_omni` (summary/routing + recommended + providers table +
  models table) + `format_number` (grouping) + `render_card`/`CARD_NAMES` 13 + 2 unit tests.
- **PH-S1851** `server/mod.rs` `api_ui_card`: `"omni"` → `boxes::omni::wire`; `ui/index.html`:
  `renderOmni` JS видалено, `rustCards` 13, `resync()` url drop (test control залишено).
- **PH-S1852** `gsv_ui_contracts` (7: `ui_card_omni_renders_summary_providers_models`) +
  `gsv_server_contracts` (omni card endpoint 200 + markers).
- **PH-S1853** Ratio hold: `gsv-loc-audit --stretch-96` → **96.73%** (rust 10191 / product 10536) ✅;
  cargo test (**207**); clippy 0; fmt clean.
- **PH-S1854** GSV docs canon: MEMORY band 121; HANDOFF/NEXT band 121; VISION.md omni card section;
  `GSV_TECH_ROADMAP.md` band 121.
- **PH-S1855** Band close: ratio **≥96%**; fmt/clippy 0; cargo test (**207**); docs canon; vision-sync rev bump; push.

### Band 125 (PH-S1889…S1898, ✅ 2026-08-15) — GSV Vision/UI polish (a11y/error/offline/stand contracts)
- **PH-S1889** Scope + queue: FM §5.106 band 125 (PH-S1889…S1898) + §5.12 header (master horizon) + roadmap band 125 rows.
- **PH-S1890** `boxes/ui.rs`: 13 renderers error/empty-state HTML маркери (`err_html`/`empty_html`/`not_ok`,
  `<span class='err'>` + «— no data», no panic) + 5 unit tests.
- **PH-S1891** `gsv_ui_contracts` (12): `RUST_CARDS` 13 + stand contracts (error/empty markers for all renderers).
- **PH-S1892** `server/mod.rs`: canonical JSON error shape `{ok:false,error}` (`err_json` →
  preview/ui-card/ui-path/data-file/error-response/omni-test/spawn_cargo); `api_error_response` BAD_REQUEST.
- **PH-S1893** `gsv_server_contracts` (32): `error_responses_share_canonical_json_shape` +
  `post_errors_share_canonical_json_shape`.
- **PH-S1894** `ui/index.html`: a11y markers (role=status, aria-live/aria-label/alt/aria-haspopup) + a11y contracts.
- **PH-S1895** `ui/index.html`: offline-stable cards — `data-card` hooks, `getText` keep-last-good +
  `.card-status` badge on fetch fail; offline-stability contract.
- **PH-S1896** `boxes/vision.rs`: `wire_summary` empty-tolerant (`degraded` flag, error only on fallback) +
  consistent `ok`/`error` across `/api/vision*`; `gsv_vision_contracts` wire-shape contracts (52).
- **PH-S1897** Ratio hold: `gsv-loc-audit --stretch-96` → **96.87%** (rust 11176 / product 11537) ✅;
  cargo test (**221**); clippy 0; fmt clean.
- **PH-S1898** GSV docs canon: MEMORY/HANDOFF/NEXT/VISION band 125; FM §5.106 ✅ + §5.12 header (0 open);
  `GSV_TECH_ROADMAP.md` band 125 ✅; vision-sync rev **492**; push.

### Band 126 (PH-S1899…S1908, ✅ 2026-08-16) — GSV stand smoke + ops canon
- **PH-S1899** Scope + queue: FM §5.107 band 126 (PH-S1899…S1908) + §5.12 header (master horizon).
- **PH-S1900** `src/bin/gsv_http_stand_smoke.rs` (мірор poolAI `poolai-http-stand-smoke`): CLI
  `--base-url`/`--json`, `SmokeCaseResult`/`SmokeReport`, `check_ok`/`check_json`/`check_status`/
  `check_card`, `CARDS` (20), exit 1 при FAIL + 3 unit tests; `Cargo.toml` `[[bin]] gsv-http-stand-smoke`.
- **PH-S1901** 48 live checks проти запущеного сервера: health/tracker/sli/toolchain/update/ratio/
  omni-status (check_json для struct-wire) + vision* ok-gate (15) + SVG status (5) + ui cards (20).
- **PH-S1902** `tests/gsv_stand_smoke_contracts.rs` (6): vision ok-gate (15), struct-wire JSON (5),
  status-only 200 (5), cards render ok+html, report shape, card-list matches registry.
- **PH-S1903** GSV docs canon: `GSV_SERVER.md` stand-smoke section · `GSV_BOXES.md` row ·
  README (tests 230 / structure / endpoints / status) · `GSV_TECH_ROADMAP.md` band 126.
- **PH-S1904** Ratio hold: `gsv-loc-audit --stretch-96` → **96.87%** (rust 11176 / product 11537) ✅;
  cargo test (**230**); clippy 0; fmt clean.
- **PH-S1905** GSV vision docs canon: MEMORY band 126 + HANDOFF/NEXT band 126 + VISION.md stand smoke.
- **PH-S1906** poolAI parity: FM §5.12 §5.107 + `GSV/docs/vision/README.md` + poolAI HANDOFF/NEXT.
- **PH-S1907** vision-sync close: `poolai-vision-sync` rev **493**; `--check` ok.
- **PH-S1908** Band close: Speeds/Rust panel; один commit; `git push` + самарі; gsv-server restart.

## Важливі факти (не забувати)

1. **GSV — окремий Rust-проєкт** у `S:\rust\poolAI\GSV` (own workspace, own `target/`).
2. **Ratio аудит іде по git-tracked файлах** репо poolAI під префіксом `GSV/` (не `GSV/target/`, не `data/`).
   git-топ має MSYS-стиль `/s/rust/poolAI` — нормалізуємо в `S:/rust/poolAI` (`normalize_git_root`).
3. **Canon listener is `target/live/gsv-server.exe`** (`cargo xtask live`). `cargo test`/`build` may overwrite `target/debug/`. Do **not** kill the live copy. Only stop `target/debug/gsv-server.exe` if *that* file is the listener (os error 5).
4. **Data dir:** `GSV/data/*` gitignored (омні-конфіг, rust_ratio.json, трекер). Запуск:
   `--repo-root S:/rust/poolAI --data-dir S:/rust/poolAI/GSV/data --port 8891`.
5. **Збірка:** terminal MSYS2 bash; PATH префікс `C:\Users\plati\.cargo\bin`.
6. **OmniRouter** прокидає через OpenAI-сумісний proxy; dry-run заголовок `X-Omni-Dry-Run: 1` —
   жодного реального мережевого запиту в тестах.
7. **UI канон:** тонкий JS/DOM glue; якщо ratio падає <95% — **compact UI/CSS**, не Rust-обхід.
