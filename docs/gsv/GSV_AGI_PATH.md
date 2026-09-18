# GSV_AGI_PATH.md — open ecosystem, super MCP hub, AGI pathing

**Status:** Accepted (owner 2026-09-17)  
**Deciders:** owner  
**MCP:** `gsv://docs/agi-path`  
**Companions:** [`GSV_VDC.md`](./GSV_VDC.md) (fleet must-haves) · [`GSV_EDGE_PLAN.md`](./GSV_EDGE_PLAN.md) (ports / env) · [`GSV_MCP_OPENBOT.md`](./GSV_MCP_OPENBOT.md) (client wiring) · [`GSV_VDT_KIT.md`](./GSV_VDT_KIT.md) (agent kit) · [`GSV_RUST_DEV.md`](./GSV_RUST_DEV.md) (Rust-first)

This is the **ecosystem setup canon**. GSV is not “one more product next to Telenetis.”
It is the **super MCP hub**: one always-on brain (`:9999` + `gsv_mcp_openbot`) that
paths AGI work across an **open** set of portable plugins under a Rust folder.

Owner one-liner: **The Rust folder is the host. Everything else is a plugin. Environment security first. Maximum Rust architecture. `agi` is the session rule.**

## Decision

1. **Rust folder is the host.** The only required tree for someone to run this
   ecosystem is a **Rust directory** that contains the GSV crate (kit +
   `gsv-server`). On this box that is `S:/rust/GSV` inside `S:/rust`. For a
   replica: clone GSV into `<rust>/GSV` and open **that** folder. No GSV → no
   hub, no MCP, no `agi` drain. The rust folder is the security boundary of the
   environment, not a convenience path.
2. **Everything else is a portable plugin.** poolAI, llama-rs, Telenetis, APK,
   OmniRoute, rebook, LinFS, ORR, and any future git tree are **plugins**: own
   repo, own `target/`, own product rules. They plug in by sitting next to the
   hub (sibling under `<rust>/`) or as a discovered workspace folder. They do
   **not** copy the kit. They do **not** become a second `:9999`. Unplug =
   fail-open; the hub still runs.
3. **Open discovery, not a closed club.** `cargo xtask products` / `gsv_products`
   lists what the environment can see. [`PRODUCTS.md`](./PRODUCTS.md)
   **enriches** registered drains (HANDOFF, tests, ratio); it is not a closed
   allowlist of existence. Discovered-but-unregistered is still a valid pick.
4. **Super MCP hub.** One server id `gsv_mcp_openbot`. Cursor / OpenCode / Grok
   are **clients**. They attach. Folder MCP for Cursor is
   `http://127.0.0.1:9999/mcp` (never User-scope). Stdio live copy
   `target/live/gsv-mcp.exe` for OpenCode/Grok. MCP sandbox is **this GSV
   crate**, not the plugin trees.
5. **AGI pathing.** Orchestration, tickets, Omni, grid, edge proxy, keep-live
   live **only** in GSV. Plugins **execute**. Telenetis is a Telegram-shell
   plugin. A phone APK with WiFi debug is a `virtual_node` plugin via
   `/api/edge`. Trigger `agi` follows kit rules: discover → keep-live → tickets
   → drain the **picked** plugin (window = GSV does not pick GSV).
6. **Environment security is the first constraint.** Discovery does not grant
   hub privilege. Plugins cannot widen MCP sandbox, User-scope MCP, `:8091`
   off-box, or secret echo. Allowlists, tokens, redaction, bind/CSRF, git
   hygiene, and “work stays in repo + `target/`” are not optional extras.

## Plugin contract (portable)

Preferred layout:

```
<rust>/                 # environment root (this box: S:/rust)
  GSV/                  # HOST — kit + gsv-server. Not a plugin.
  poolAI/               # plugin
  llama-rs/             # plugin
  <name>/               # any other git tree = plugin
```

`GSV/telenetis` is a **nested exception**, not the pattern to copy. New plugins
are sibling git roots under `<rust>/`.

| Plugin must | Plugin must not |
|---|---|
| Own git root + own `target/` | Copy `.agents/skills/` / VDT kit into itself |
| Be drop-in: clone/move/remove without rewriting GSV | Run a second `gsv-server` / User-scope MCP |
| Talk to the hub (`/mcp`, `/api/edge`, keep-live) | Call poolAI `:8091` off-box |
| Keep product rules in **its** tree | Write product rules into `S:\rust\GSV\AGENTS.md` |
| Fail-open when down | Abort the hub or steal Godfather tokens |

Registered drain (optional `PRODUCTS.md` row): HANDOFF + test command + ratio
gate. Unregistered sibling is still a plugin — S0 + git only, no PH-S* until
registered.

APK / Mini App / Telegram client = **edge plugins** (token + path allowlist),
not git-siblings. Same rule: execute, do not orchestrate.

## Environment security (first)

The **environment** is the rust folder + live hub process. Security is the
architecture, not a later pass.

| Gate | Rule |
|---|---|
| MCP sandbox | Preview / terminal / vision / xtask stay inside the GSV crate. `gsv_products_*` is an allowlist to **name** plugins — no `products/open`, no tunnel, no User-scope Cursor MCP (leaks into plugin windows). |
| Bind | Live may listen LAN (`0.0.0.0` + `--allow-lan`); mutating POSTs from non-local Origin are rejected. CSP / nosniff / DENY / no-store. POST body cap 256 KiB. |
| Edge | `GSV_EDGE_TOKEN` (never on the wire). Path allowlist. 20 req/s. JSON key redact. No `login` / `vm` / `users`. |
| Grid | poolAI `:8091` **on-box only**. Clients use hub `/api/edge`. |
| Secrets | Never echo Godfather / edge / `.env` / `*.pem`. Do not stage `data/*` (except `.gitkeep`), `.env*`, keys. No `git add -A`. |
| Workdir | Work stays in the open repo + `target/` (live under `target/live/`). No Windows temp/tmp for scratch, downloads, or tooling. |
| Terminal | HTTP SLI allowlist (no extra shell, no `git push` via MCP). |
| Godfather | `telegram-relay` + `allowed_user_ids`. Tokens redacted on `gsv_settings` / `gsv_telegram*`. |
| OmniRoute | Policy-down (`:20128`). Probe only. Do not start it from `agi`. |

A plugin that is missing, dirty, or Node-based does **not** lower these gates.

## Maximum Rust architecture (`agi` rules)

Hub and every **Rust plugin** follow the kit:

- Runtime / API / boxes / tests / benches / scripts = **Rust** (`src/`,
  `tests/`, `benches/`, `src/bin/`, `cargo xtask`). No Python product files. No
  Java. No Node as the hub. Thin HTML/CSS/JS glue only — never port large
  legacy JS (GSV: never copy `docs/vision/vision.js`).
- Ratio: GSV **95–100%** (`gsv-loc-audit --stretch-96`); other Rust plugins per
  their `PRODUCTS.md` row (typically 90–100%). Wasm 0–5% horizon.
- Allowed source formats in the kit: `.rs`, `.md`, `.mdc`, `.json`, `.js`,
  `.wasm`. Everything else is production output of a registered pipeline, not
  a new hub language.
- Shell for agents: **MSYS2 bash**, not PowerShell. Separate `target/` per
  plugin. Do not kill `target/live/` before GSV `cargo test` / `cargo build`.
- Session: `agi` / `абракадабра` / `abrakadabra` — discover rust-folder
  projects, ask, then drain **that** tree. One commit in the **plugin** repo
  (or GSV if that was the pick). No mid-drain push.

A Node plugin (OmniRoute today) may exist as a plugin; it is **not** absorbed
into the hub and is not the AGI control plane.

Do **not** grow Telenetis into “the app that handles everything” (board-as-OS,
tensor, KVM, swarm). Mini App WebGPU (VDC §3 path b) is a **probe**, not the
control plane. No Termux.

## Layers (pathing)

| Layer | Who | Wire | AGI role |
|---|---|---|---|
| Host (not a plugin) | `gsv-server` + MCP | `:9999` · `/mcp` | tickets, Omni, grid, `/api/edge`, keep-live, VDT |
| Grid plugin | poolAI | `:8091` **only on-box**; clients use hub `/api/edge` | seats, jobs, virtual-nodes |
| Tensor plugin | llama-rs | `:8080` deep · `:8082` fast | local model tiers |
| Telegram-shell plugin | Telenetis | `:9800` | identity, Mini App chrome, LAN when Telegram is down |
| Phone-edge plugin | APK + WiFi debug | hub `/api/edge/{*path}` | execute slices / tasks; never clone Telenetis |
| Cloud Omni (hub box) | GSV OmniRouter | `data/omni.toml` | free-tier chain; **OmniRoute `:20128` stays policy-down** |
| Agent kit | this repo | `.agents/skills/` | `абракадабра` / `abrakadabra` / `agi` |

## Настройка екосистеми (operator)

### Replica (another person, same ecosystem)

1. Have a **Rust folder**. Clone this repo into `<rust>/GSV`. Open that folder
   (or a workspace with GSV first).
2. `cargo xtask live` + `cargo xtask watchdog`. Cursor folder MCP →
   `http://127.0.0.1:9999/mcp`. Never User-scope.
3. Drop other git trees as **sibling plugins** under `<rust>/`. Next `agi`
   lists them. Optional `PRODUCTS.md` row only for a registered drain.
4. Do not copy the kit into plugins. Do not expose `:8091`. Do not start
   OmniRoute. Follow `agi` — that is the session law.

### Hub (always on)

```
cargo xtask live      # target/live/gsv-server.exe :9999  (do not kill before cargo test)
cargo xtask watchdog  # respawn; lockstep-off during drains
```

Cursor (this kit folder only):

```json
{ "mcpServers": { "gsv_mcp_openbot": { "type": "http", "url": "http://127.0.0.1:9999/mcp" } } }
```

OpenCode / Grok: stdio `target/live/gsv-mcp.exe` (copied by `cargo xtask live`).
`GET /mcp` → `tool_count` 59, `gsv://` resources include **`gsv://docs/agi-path`**.
If `version_lag` or `catalog_stale`: owner `POST /api/update/apply` / restart Cursor.

Keep-live (`gsv_keep_live`): GSV + Telenetis + llama-rs **up**; OmniRoute **down**
(fail-open). Grid: `gsv_grid` (ALLBGP mirror). Disk: `cargo xtask disk`
(`--clean` keeps `target/live/`).

### Godfather (owner channel)

- Channel `@GSV_OFFICIAL`, workflow `telegram-relay` (enables the inbound loop).
- Allowlist includes owner Telegram id + `platinofff` + session words.
- Inbound classifies `/ticket`, hook phrases, bus JSON, and leftover chat.
  Allowlisted non-bot text becomes a ticket (`freetext_ok`). Empty allowlist
  still skips plain chat so the channel cannot flood the board. `/ticket gsv …`
  still works.

### Edge plugins (Telenetis / APK)

- Token: `GSV_EDGE_TOKEN` (env wins) or `settings.edge.token`. Never on the wire.
- Base: `http://<lan>:9999/api/edge/{*path}` — topology / workers / discovery /
  virtual-nodes / grid / jobs / health GET. Not `login` / `vm` / `users`.
- Telenetis `GSV_URL=:9999`. Grid calls through the hub, not `POOLAI_URL=:8091`
  off-box. Service account is the edge token (`gsv-apk service-account`):
  `GSV_EDGE_TOKEN` / `TELENETIS_EDGE_TOKEN`. poolAI `admin/admin123` is not a
  client credential.

### Join a plugin (open ecosystem)

1. Put a git repo at `<rust>/<name>` (or add a workspace folder).
2. Next `agi` / `абракадабра` → `cargo xtask products` lists it.
3. Optional `PRODUCTS.md` row only if you want a registered drain (HANDOFF +
   test command + ratio). Discovered-but-unregistered is still a valid pick.

### Join a phone (APK path)

1. WiFi debug already exists on the device (configure, debug, disk, logcat).
2. The phone client is a **Rust-ratio APK** (95–100%), not Chrome and not a
   Telegram Mini App tensor worker. Mini App / WebView GGUF is a crutch/probe.
3. APK registers as `virtual_node` through hub `/api/edge` with the edge token.
   Host-side: `cargo run --bin gsv-apk -- join --json` (dry-run hops) or
   `join --live --json` (POST health + register-remote + pool/join). Origin
   `apk_edge`, class `edge`. Never `:8091`. Token header only, never printed.
   Native package: `cargo run --bin gsv-apk -- package --json` (`org.gsv.apk`,
   no WebView). `--write` emits `AndroidManifest.xml` under `target/live/apk/`
   (pipeline output, not product source).
   Disk/settings: `GET /api/apk` or `gsv-apk disk --json` (WiFi ADB plan,
   no screencap). `GSV_APK_CACHE` / `GSV_APK_ADB` optional.
4. Mini App / Chrome is **not** the phone worker. Hub `POST /api/edge` with
   origin `telegram_edge` / `chrome` / `webview` is **403**. Tasks go to
   `apk_edge` (`gsv-apk worker --json`). WebGPU stays probe-only. Telenetis
   freeze stays.
5. Telegram stays a **proxy/passthrough to the APK only** (forward auth/commands
   / open the APK). Hub policy: `cargo run --bin gsv-apk -- telegram auth`
   (also `command`, `open_apk`). Mini App chrome (`dashboard`/`board`) may stay
   a shell. `tensor` / `host-tests` / `kvm` / `start-worker` are rejected.
   Do not ship ticket board, bot, tensor, or KVM inside the APK.

## AGI session shape

Trigger `agi` in the GSV (rust-folder) workspace: discover plugins → keep-live
hub → tickets → S0 disk → warnings-first → drain the **picked** tree (not
“GSV because the window is GSV”) → one commit + push.

Board scenario `hub-agi-path` is APK peer, Telenetis freeze, service-account
edge token, and Godfather allowlisted free-text ingest. Scenario
`hub-agi-plugins` is the plugin-contract band
(host vs plugin scan, MCP sandbox, keep-live fail-open, kit-not-copied,
per-plugin `target/`).

## Non-goals

- Embedding Cursor / OpenCode / Grok inside `gsv-server`.
- A second MCP User-scope install (leaks into plugin windows).
- Telenetis or APK as orchestrator.
- Copying the VDT kit into every plugin.
- Absorbing plugin source or product rules into GSV.
- Starting OmniRoute (`:20128`).
- Termux ggml-rpc on phones.
- Node / Python / Java as the hub runtime.

## See also

- Fleet table + VDC must-haves: [`GSV_VDC.md`](./GSV_VDC.md)
- Ports / env matrix: [`GSV_EDGE_PLAN.md`](./GSV_EDGE_PLAN.md)
- MCP clients: [`GSV_MCP_OPENBOT.md`](./GSV_MCP_OPENBOT.md)
- VDT kit entry: [`GSV_VDT_KIT.md`](./GSV_VDT_KIT.md)
- Product / plugin registry: [`PRODUCTS.md`](./PRODUCTS.md)
- Tickets: scenarios `hub-agi-path` and `hub-agi-plugins` in [`ticket_scenarios.json`](./ticket_scenarios.json)
