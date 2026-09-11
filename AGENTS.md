# AGENTS.md — GSV (Galaxy StarWalker Vision)

Standalone Rust-first vision server **and VDT entry workspace** for registered
Rust products. **Root:** `S:\rust\GSV` (sibling of PoolAI, not a subfolder).

**Kit (rules / skills / `абракадабра` / `abrakadabra`):** open **this** folder in Cursor / OpenCode / Grok Build. Canon —
[`docs/gsv/GSV_VDT_KIT.md`](docs/gsv/GSV_VDT_KIT.md). Product code may live in
another tree (`S:\rust\poolAI`, …); the window being GSV does **not** mean the
drain target is GSV — the skill asks first.

## Stack

- Runtime / API / boxes: **Rust** (`edition 2021`, `tokio`, `axum`) in `src/`, `tests/`, `benches/`, `src/bin/`. Product tests/benches/scripts are `.rs` (`cargo xtask`).
- UI: thin HTML/CSS/JS glue in `ui/` (no Python, no Java).
- Ratio canon: Rust **95–100%** / wasm 0–5% — `cargo run --bin gsv-loc-audit -- --stretch-96`.
- Terminal: **MSYS2 bash**, not PowerShell.

```bash
export PATH="/c/Users/${USER:-${USERNAME}}/.cargo/bin:$HOME/.cargo/bin:/ucrt64/bin:/usr/bin:$PATH"
export RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-stable-x86_64-pc-windows-gnu}"
cd /s/rust/GSV || cd "S:/rust/GSV"
unset CARGO_TARGET_DIR
```

## Session (`абракадабра` / `abrakadabra` — спочатку вибір продукту)

Trigger skill: `.agents/skills/abracadabra/`. Either spelling starts the same drain. Вікно GSV ≠ автоматично drain GSV.

1. `cargo xtask products` → AskQuestion (Cursor) / `question` (OpenCode) / numbered plain-text list (Grok) on **those** environment projects (not a hardcoded `gsv | poolai` pair).
2. S0 disk for **that** product → `git fetch` → its HANDOFF.
3. Drain next band (GSV: `docs/gsv/GSV_TECH_ROADMAP.md`; PoolAI: FM §5.12).
4. If **gsv:** do **not** kill `target/live/gsv-server.exe` before `cargo test` / `cargo build`. Only stop `target/debug/gsv-server.exe` if that file is the listener.
5. `cargo fmt --all` → product tests (`cargo test` here; `cargo test-ci` in PoolAI).
6. One commit **in the product repo**. GitHub remote: `origin` → `https://github.com/platinoff/GSV` (create if missing, then `git push`).

## OpenCode (Windows)

OpenCode defaults to PowerShell — **this repo forbids that**. All `cargo` / `git` / scripts:

```
C:\msys64\usr\bin\bash.exe -lc 'команда'
```

`абракадабра` / `abrakadabra` in OpenCode uses the `question` tool (not Cursor AskQuestion). Skills: `.agents/skills/` (`opencode.json` → `skills.paths`). Plugin host: `.opencode/package.json` (`@opencode-ai/plugin`). **Do not auto-generate a product UI** — live Galaxy UI is `gsv-server` at `http://127.0.0.1:9999/`.

Cursor ↔ OpenCode ↔ Grok: Cursor `AskQuestion` = OpenCode `question`; Grok asks in plain text (numbered list, no question tool). Shared kit git-canon is this repo; copy (not symlink) to `.cursor/skills/` and `.opencode/skills/` via `cargo xtask mirrors`; Grok reads the `.agents/skills/` canon directly (`[skills] paths` in `.grok/config.toml`, no copy).

Grok Build: reads `AGENTS.md` + root `.mcp.json` natively; stdio MCP + skills paths live in `.grok/config.toml`; verify discovery with `grok inspect`. Same MSYS2 bash rule — never PowerShell.

## MCP (`gsv_mcp_openbot`)

One MCP server serves Cursor / OpenCode / Grok — same tools, one surface. Canon: [`docs/gsv/GSV_MCP_OPENBOT.md`](docs/gsv/GSV_MCP_OPENBOT.md); live server state: `GET /mcp` (if `version_lag` > 0, recopy: `cargo xtask live`).

- **Transport per client:** Cursor = Streamable HTTP `http://127.0.0.1:9999/mcp` (folder-scoped `.cursor/mcp.json`, never User MCP); OpenCode = stdio `opencode.json → mcp.gsv_mcp_openbot`; Grok = stdio `.grok/config.toml` (live copy `target/live/gsv-mcp.exe`). Do **not** `cargo run --bin gsv-mcp` (slow, cargo lock, second AppState).
- **Sandbox = this GSV repo:** preview/terminal/vision/xtask stay inside it; `gsv_products_*` is VDT-product allowlist only — no `products/open`, no tunnel, no User-scope Cursor MCP.
- **Resources:** `gsv://vision/{manifest,feed,extensions}` + `gsv://docs/{mcp-openbot,handoff,next,fingerprints,post-always-on,rust-dev,omni-catalog,settings-telegram,solo-squad-jail,ranks,rules-check}` (allowlist; `..` / `file://` → `-32602`). `gsv_vision_sync` notifies every subscribed `gsv://` URI.
- **Prompts:** `gsv_status` · `gsv_vision_brief` · `gsv_drain`.
- **Read rules:** `gsv_xtask {task:sync}` is `--check`-only drift; terminal = HTTP SLI allowlist; `gsv_settings`/`gsv_telegram*` redact `token_set`/`bot_token` — never echo secrets.
- `абракадабра` / `abrakadabra` in OpenCode/Cursor: product discovery via `cargo xtask products` **or** `gsv_products` → `question`/AskQuestion.

## Speeds + Rust panel (GSV drain)

After tests: `cargo xtask record-speed` (or `--skip-run`) and `cargo xtask record-rust`. Writers: `gsv-speed-index` / `gsv-rust-diagnostics` → `docs/vision/*.json`. Then `cargo xtask sync`.

## Defaults

| Flag | Default |
|------|---------|
| `--repo-root` | this crate (`S:/rust/GSV`) |
| `--data-dir` | `{repo-root}/data` |
| `--port` | **9999** (8765 is Hyper-V reserved) |
| Vision sources | `docs/vision/{manifest,feed,extensions,speed_index,rust_diagnostics}.json` |

Optional: `--repo-root S:/rust/poolAI` to scan PoolAI FM / `bin/` / `scripts/` from this server.

PoolAI vision canon (after the split) lives in **PoolAI** at `docs/vision/`. This repo keeps its own `docs/vision/` snapshot.

## Formats + local rules (global, session-independent)

These globals hold in **every** session and **every** registered product, whatever the IDE/model:

- Global scope is the GSV kit only: MCP toolchains, environment flows (terminal, `rustup`, env), IDE/Cursor/OpenCode/Grok lockstep, model life & communications with connected release apps (`gsv-server` live). Product code and its local rules stay in the product repo — GSV never absorbs them.
- Allowed source formats: `.rs`, `.md`, `.mdc`, `.json`, `.js`, `.wasm`. Everything else is **not** allowed except production output of a registered pipeline. Ratio stays Rust-first (Rust **95–100%** / wasm 0–5%); `.wasm`/`.js`/`.json`/`.md` count toward the audited ratio per product row.
- Rust toolchain only for what is needed in this kit. **npm / Node is not installed and not used here** — anything needed is done with Rust (`.rs`, `cargo`, `cargo xtask`, `cargo run --bin …`) and MSYS2 bash. npm-based product hooks/gates belong to the product repo, not to the global kit.
- **Work stays inside the Rust environment (the repo tree + `target/`).** No roaming to Windows temp/tmp (`C:\...\temp`, `C:\tmp`, `/tmp`, `%TEMP%`, `%TMP%`) for scratch files, downloads, or intermediate tooling. Anything extra needed (docs, references, drafts) lives inside the open repo or under `target/live/` — never outside the workspace.
- Local rules (`AGENTS.md` / `CLAUDE.md` / `.cursorrules` / `rules/`) live **in each registered product tree** and must not affect globals. `S:\rust\GSV\AGENTS.md` is the only global rule file; no other product's rules are placed inside the GSV tree.

## Do not

- Stage `data/*` (except `.gitkeep`), `.env*`, `*.pem` / `*.key`.
- Run `git add -A`.
- Add Python product files.
- Push mid-drain.
