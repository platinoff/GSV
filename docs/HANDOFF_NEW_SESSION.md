# Передача контексту новій сесії (GSV)

**Оновлено:** 2026-08-17 (band 143 ✅ · **band 144 queued** — always-on live copy)

**Наступна сесія:** відкрити Cursor на **`S:\rust\GSV`** (або `gsv.code-workspace`) →
**`абракадабра` / `abrakadabra`** → `scripts/list-vdt-products.sh` → **AskQuestion на проєкти з environment**
(не `gsv | poolai` з голови) → S0 диск/git → project scan (warnings first) →
drain **band 144** (`PH-S2079…S2088`, live copy + apply) → Speeds + Rust panel → vision-sync → **один commit** → **`git push` + самарі**.

Якщо вибір **gsv:** drain [`GSV_TECH_ROADMAP.md`](gsv/GSV_TECH_ROADMAP.md) **band 144** only
(spec [`gsv/GSV_ALWAYS_ON_UI.md`](gsv/GSV_ALWAYS_ON_UI.md), plan
[`superpowers/plans/2026-08-17-always-on-galaxy.md`](superpowers/plans/2026-08-17-always-on-galaxy.md)).
Не починати 145 (products) в тій же сесії, якщо 144 не закритий.
MCP canon: [`gsv/GSV_MCP_OPENBOT.md`](gsv/GSV_MCP_OPENBOT.md).
Канон ролей: [`GSV_ROLES.md`](GSV_ROLES.md). Реєстр: [`gsv/PRODUCTS.md`](gsv/PRODUCTS.md).

## Стан зараз

- **GSV** — окремий Rust-first проєкт (`S:\rust\GSV`), bands 102 · 108–121 · 125–142 · **143 ✅**.
- **Horizon 144–147 (queued):** always-on live copy; UI offline only during binary swap; VDT product picker + open folder + auto-parse; patch version per commit; fingerprint JSONL (IDE / bot / model / agent / time).
- **Band 143:** Galaxy chrome + type/chart — power menu `z-index:80` above workspace (header ≥ 40; no shared `z-index:2`); exclusive fullscreen (`data-action='card-fs'`, `exitFullscreen()`, Esc); collapsed cards leave the grid (`display:none` + dock restore); `--fs-ui/card/meta/chart` scale; speed/rust SVG height 168, font-size 11, ui-monospace. Live copy still band 144 — debug exe still locks `cargo test`.
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
  Discover: `scripts/list-vdt-products.sh` (не hardcoded `gsv | poolai`).
- **Ratio / тести:** `gsv-loc-audit --stretch-96` → **96.22%** (rust 17444 / product 18129) · **324** green · clippy 0 · fmt clean.
- **Сервер:** canon порт **9999** (`DEFAULT_PORT`; 8765 — Hyper-V reserved range).
- **Vision rev:** **510** (band 143 `gsv-vision-sync`).
- **Live UI** — `gsv-server` → `http://127.0.0.1:9999/`. MCP stdio — `cargo run --quiet --bin gsv-mcp`.
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

1. `df -h /s | tail -1` → `bash scripts/check_target_disk.sh` → `cargo clean` якщо <5G (12G дешево).
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

**⚠️ Перед build/test зупинити `gsv-server`** (блокує `gsv-server.exe`, os error 5); після — перезапустити.

## Git (кінець сесії)

- Один commit (код + docs + FM/HANDOFF/NEXT). Не `git add -A` — тільки файли спринту.
- **`git push` + самарі** — обов'язково останній крок.
