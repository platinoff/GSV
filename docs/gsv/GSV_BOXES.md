# GSV Boxes â€” Ð¿Ð°Ð½ÐµÐ»Ñ–/Ð¼Ð¾Ð¶Ð»Ð¸Ð²Ð¾ÑÑ‚Ñ– Â«Galaxy StarWalker VisionÂ»

Ð¡Ð¿ÐµÑ†Ð¸Ñ„Ñ–ÐºÐ°Ñ†Ñ–Ñ Ð±Ð¾ÐºÑÑ–Ð² ÑÐµÑ€Ð²ÐµÑ€Ð° GSV. ÐšÐ¾Ð¶ÐµÐ½ Ð±Ð¾ÐºÑ â€” Ð¿Ð°Ð½ÐµÐ»ÑŒ UI + Rust-Ð¼Ð¾Ð´ÑƒÐ»ÑŒ.

## 1. Tracker (Ñ‚ÐµÑ…Ð½Ñ–Ñ‡Ð½Ñ– Ð¿Ð°Ñ€Ð°Ð¼ÐµÑ‚Ñ€Ð¸ workflow)

**Ð Ð¾Ð»ÑŒ:** Ð¿Ð¾ÐºÐ°Ð·ÑƒÑ” Ñ‚ÐµÑ…Ð½Ñ–Ñ‡Ð½Ñ– Ð¿Ð°Ñ€Ð°Ð¼ÐµÑ‚Ñ€Ð¸ Ð²Ð¸ÐºÐ¾Ð½Ð°Ð½Ð¾Ð³Ð¾ Ð²Ð¾Ñ€ÐºÑ„Ð»Ð¾Ñƒ (Ñ‰Ð¾ Ñ€ÐµÐ°Ð»ÑŒÐ½Ð¾ Ð²Ð¸ÐºÐ¾Ð½ÑƒÐ²Ð°Ð»Ð¾ÑÑŒ).

Ð”Ð°Ð½Ñ–: ÑÐ¿Ñ€Ð¸Ð½Ñ‚Ð¸ (PH-S*), ÐºÐ¾Ð¼Ð°Ð½Ð´Ð¸, Ñ‡Ð°ÑÐ¾Ð²Ñ– Ð¼Ñ–Ñ‚ÐºÐ¸, ÑÑ‚Ð°Ñ‚ÑƒÑÐ¸, ÐºÑ–Ð»ÑŒÐºÑ–ÑÑ‚ÑŒ Ñ„Ð°Ð¹Ð»Ñ–Ð²/LOC, wall-clock.

| ÐŸÐ¾Ð»Ðµ | Ð”Ð¶ÐµÑ€ÐµÐ»Ð¾ |
|------|---------|
| Sprint id / band | FM Â§5.12 |
| Ð’Ð¸ÐºÐ¾Ð½Ð°Ð½Ñ– ÐºÐ¾Ð¼Ð°Ð½Ð´Ð¸ | shell history / logs |
| Ð¢Ñ€Ð¸Ð²Ð°Ð»Ñ–ÑÑ‚ÑŒ ÐºÑ€Ð¾ÐºÑ–Ð² | timestamps |
| LOC / files | `gsv-loc-audit` |
| Ð¡Ñ‚Ð°Ñ‚ÑƒÑ / âœ… | FM Â§5.12 |

Rust Ð¼Ð¾Ð´ÑƒÐ»ÑŒ: `tracker/` â†’ `gsv_tracker.json`.

## 2. SLI console (ÐºÐ¾Ð¼Ð°Ð½Ð´Ð¸ + SLI-Ñ„ÑƒÐ½ÐºÑ†Ñ–Ñ—)

**Ð Ð¾Ð»ÑŒ:** Ð±Ð°Ñ‡Ð¸Ñ‚Ð¸, ÑÐºÑ– ÐºÐ¾Ð¼Ð°Ð½Ð´Ð¸ Ð²Ð¸ÐºÐ¾Ñ€Ð¸ÑÑ‚Ð¾Ð²ÑƒÑŽÑ‚ÑŒÑÑ, Ñ‚Ð° **Ð²ÑÑ– SLI-Ñ„ÑƒÐ½ÐºÑ†Ñ–Ñ—, ÑÐºÑ– Ð¼Ð¾Ð¶Ð½Ð° ÑÑ‚Ð²Ð¾Ñ€Ð¸Ñ‚Ð¸ Ð· Ð½Ð°ÑÐ²Ð½Ð¸Ñ… ÑÐºÑ€Ð¸Ð¿Ñ‚Ñ–Ð²** (+ Ð½Ð¾Ð²Ñ–).

- ÐŸÐ°Ñ€ÑÐ¸Ð½Ð³ `src/bin/` + `cargo xtask` â†’ ÐºÐ°Ñ‚Ð°Ð»Ð¾Ð³ SLI-ÐºÐ¾Ð¼Ð°Ð½Ð´ (Ð½Ð°Ð·Ð²Ð°, Ð¾Ð¿Ð¸Ñ, Ð²Ñ…Ð¾Ð´Ð¸). No product `.sh`.
- Ð’Ð¸Ð²Ð¾Ð´Ð¸Ñ‚ÑŒ Ñ„Ð°ÐºÑ‚Ð¸Ñ‡Ð½Ð¾ Ð²Ð¸ÐºÐ¾Ñ€Ð¸ÑÑ‚Ð°Ð½Ñ– ÐºÐ¾Ð¼Ð°Ð½Ð´Ð¸ (Ð· Tracker/history).
- ÐŸÑ€Ð¾Ð¿Ð¾Ð½ÑƒÑ” **Ð½ÐµÐ·Ð°Ð´Ñ–ÑÐ½Ñ– ÑÐºÑ€Ð¸Ð¿Ñ‚Ð¸** â†’ Ð¿Ð¾Ñ‚ÐµÐ½Ñ†Ñ–Ð¹Ð½Ñ– Ð½Ð¾Ð²Ñ– SLI-Ñ„ÑƒÐ½ÐºÑ†Ñ–Ñ—.
- Ð’Ñ–Ð´ÐºÑ€Ð¸Ñ‚Ð¸Ð¹ Ñ€ÐµÑ”ÑÑ‚Ñ€ Ð´Ð»Ñ Ð½Ð¾Ð²Ð¸Ñ… Ñ„ÑƒÐ½ÐºÑ†Ñ–Ð¹.

Rust Ð¼Ð¾Ð´ÑƒÐ»ÑŒ: `sli/` â†’ `gsv_sli.json`.

## 3. Toolchain (ÑÐºÑ– Ñ‚ÑƒÐ»Ð¸ Ð²Ð¸ÐºÐ¾Ñ€Ð¸ÑÑ‚Ð¾Ð²ÑƒÑŽÑ‚ÑŒÑÑ)

**Ð Ð¾Ð»ÑŒ:** Ñ–Ð½Ð²ÐµÐ½Ñ‚Ð°Ñ€ Ñ‚ÑƒÐ»Ñ–Ð² Ð¿Ñ€Ð¾Ñ”ÐºÑ‚Ñƒ.

| Ð¢ÑƒÐ» | Ð’ÐµÑ€ÑÑ–Ñ | Ð”Ð¶ÐµÑ€ÐµÐ»Ð¾ |
|-----|--------|---------|
| rustc / cargo | 1.92.0 | `rust-toolchain.toml` |
| clippy / rustfmt | â€” | toolchain |
| MSYS2 bash | â€” | AGENTS.md |
| Node / Playwright | â€” | `e2e/` |
| Cursor / opencode | 3.16.29 | service (desktop `package.json`; toolchain `cursor` entry) |

Rust Ð¼Ð¾Ð´ÑƒÐ»ÑŒ: `toolchain/` â†’ `gsv_toolchain.json`.

## 4. IDE (opencode + cursor Ñ‡Ð°Ñ‚Ð¸; Ð²Ð¸Ð±Ñ–Ñ€, Ð· Ñ‡Ð¸Ð¼ Ð¿Ñ€Ð°Ñ†ÑŽÐ²Ð°Ñ‚Ð¸)

**Ð Ð¾Ð»ÑŒ:** Ð¿Ð¾Ñ€Ñ‚ÑƒÐ²Ð°Ñ‚Ð¸ opencode + cursor Ñ‡Ð°Ñ‚Ð¸; Ð¼Ð¾Ð¶Ð»Ð¸Ð²Ñ–ÑÑ‚ÑŒ Ð¾Ð±Ð¸Ñ€Ð°Ñ‚Ð¸, Ð· Ñ‡Ð¸Ð¼ Ð¿Ñ€Ð°Ñ†ÑŽÐ²Ð°Ñ‚Ð¸.

- Ð§Ð¸Ñ‚Ð°Ð½Ð½Ñ ÑÐµÑÑ–Ð¹/Ñ‡Ð°Ñ‚Ñ–Ð² opencode (`~/.local/share/opencode/`) Ñ‚Ð° cursor (`.cursor/`).
- Ð¡Ð¿Ð¸ÑÐ¾Ðº ÑÐµÑÑ–Ð¹ Ñƒ UI; Ð²Ð¸Ð±Ñ–Ñ€ Ð°ÐºÑ‚Ð¸Ð²Ð½Ð¾Ñ— â†’ **Ð¾ÑÑ‚Ð°Ð½Ð½Ñ– 8 Ð¿Ð¾Ð²Ñ–Ð´Ð¾Ð¼Ð»ÐµÐ½ÑŒ** (`preview_messages` jsonl).
- Ð’Ð¸Ð±Ñ–Ñ€ Ñ€Ð¾Ð±Ð¾Ñ‡Ð¾Ð³Ð¾ Ñ„Ð¾Ð»Ð´ÐµÑ€Ð°/ÑÐ¿Ñ€Ð¸Ð½Ñ‚Ñƒ.

Rust Ð¼Ð¾Ð´ÑƒÐ»ÑŒ: `ide/` (read-only).

## 5. Update (Ð¾Ð½Ð¾Ð²Ð»ÐµÐ½Ð½Ñ Ð±Ñ–Ð½Ð°Ñ€Ð½Ð¸ÐºÐ°; offline-ÑÑ‚Ñ–Ð¹ÐºÑ–ÑÑ‚ÑŒ)

**Ð Ð¾Ð»ÑŒ:** ÑÐºÑ‰Ð¾ Ð¾Ð½Ð¾Ð²Ð»ÑŽÑ”Ð¼Ð¾/Ð´ÐµÐ±Ð°Ð¶Ð¸Ð¼Ð¾ vision Rust-ÐºÐ¾Ð´Ð±Ð°Ð·Ñƒ Ñ– Ð·Ð°Ð¿ÑƒÑ‰ÐµÐ½Ð° bin-Ð²ÐµÑ€ÑÑ–Ñ â€” ÑÐµÑ€Ð²ÐµÑ€ Ð¿Ñ€Ð¸Ð¹Ð¼Ð°Ñ” **Ð¿Ð¾Ð²Ñ–Ð´Ð¾Ð¼Ð»ÐµÐ½Ð½Ñ Ð¿Ñ€Ð¾ Ð°Ð¿Ð´ÐµÐ¹Ñ‚**; Ð²ÐµÐ±ÑÑ‚Ð¾Ñ€Ñ–Ð½ÐºÐ° Ð½Ðµ Ð¿Ð°Ð´Ð°Ñ” Ð¿Ñ€Ð¸ Ð¾Ñ„Ð»Ð°Ð¹Ð½.

ÐŸÐ¾Ð²ÐµÐ´Ñ–Ð½ÐºÐ°:
1. ÐŸÐµÑ€ÐµÐºÐ¾Ð¼Ð¿Ñ–Ð»ÑÑ†Ñ–Ñ â†’ Ð½Ð¾Ð²Ð¸Ð¹ Ð±Ñ–Ð½Ð°Ñ€Ð½Ð¸Ðº (`target/debug/`).
2. Canon listener â€” **live copy** `cargo xtask live` â†’ `target/live/gsv-server.exe`.
3. UI: **Â«UpdateÂ»** â†’ `POST /api/update/apply` (SSE `offline`, process exit).
4. Ð¡Ñ‚Ð¾Ñ€Ñ–Ð½ÐºÐ° Ð½Ðµ Ð¿Ð°Ð´Ð°Ñ” â€” Â«offlineÂ» Ð»Ð¸ÑˆÐµ Ð¿Ñ–Ð´ Ñ‡Ð°Ñ swap; SSE `onopen` â†’ resync.

Ð”ÐµÑ‚Ð°Ð»Ñ–: [`GSV_SERVER.md`](./GSV_SERVER.md) (endpoints `/api/update`, `/api/update/apply`, `/events`, live copy).

## 6. Box preview (Rust-ÐºÐ¾Ð»ÑŒÐ¾Ñ€Ð¸ Ð²Ñ–Ð´Ð¿Ð¾Ð²Ñ–Ð´Ð½Ð¾ Ð´Ð¾ ÑÐ¸Ð½Ñ‚Ð°ÐºÑÐ¸ÑÑƒ)

**Ð Ð¾Ð»ÑŒ:** Ð¿Ñ€ÐµÐ²Ê¼ÑŽ Ñ„Ð°Ð¹Ð»Ñ–Ð², Ð´Ðµ **Rust-ÐºÐ¾Ð»ÑŒÐ¾Ñ€Ð¸ Ð²Ñ–Ð´Ð¿Ð¾Ð²Ñ–Ð´Ð°ÑŽÑ‚ÑŒ ÑÐ¸Ð½Ñ‚Ð°ÐºÑÐ¸ÑÑƒ** (Ð²Ð¸ÑÐ²Ñ–Ñ‚Ð»ÐµÐ½Ð½Ñ ÑÐ¸Ð½Ñ‚Ð°ÐºÑÐ¸ÑÑƒ Rust).

- `GET /api/preview?file=â€¦` â†’ HTML Ð· Ñ‚Ð¾ÐºÐµÐ½-Ð²Ð¸ÑÐ²Ñ–Ñ‚Ð»ÐµÐ½Ð½ÑÐ¼ (Rust-Ð¿Ð°Ð»Ñ–Ñ‚Ñ€Ð°).
- ÐŸÑ–Ð´Ñ‚Ñ€Ð¸Ð¼ÐºÐ° `.rs`, `.toml`, `.md`, `.js`, `.css`.
- Ð¨Ð»ÑÑ… Ð»Ð¸ÑˆÐµ repo-relative: `ParentDir` / absolute â†’ reject; canonicalize Ð¿Ñ–Ð´ `repo_root`.

## 7. SLI terminal (AI â†’ ÐºÐ¾Ð¼Ð°Ð½Ð´Ð¸)

**Ð Ð¾Ð»ÑŒ:** Ñ‰Ð¾Ð± AI (Ð¨Ð†) Ð¼Ñ–Ð³ Ð¿Ð¾ÑÐ¸Ð»Ð°Ñ‚Ð¸ ÐºÐ¾Ð¼Ð°Ð½Ð´Ð¸ Ð½Ð° ÑÐµÑ€Ð²ÐµÑ€.

- `POST /api/terminal {command}` â€” Ð²Ð¸ÐºÐ¾Ð½Ð°Ñ‚Ð¸ SLI-ÐºÐ¾Ð¼Ð°Ð½Ð´Ñƒ.
- ÐÑƒÐ´Ð¸Ñ‚ Ñƒ Tracker; Ñ€ÐµÐ·ÑƒÐ»ÑŒÑ‚Ð°Ñ‚ â€” JSON/stdout.
- ÐžÐ±Ð¼ÐµÐ¶ÐµÐ½Ð½Ñ: whitelist SLI-ÐºÐ°Ñ‚Ð°Ð»Ð¾Ð³Ñƒ (Ð±ÐµÐ· `bash`/`node`/`npm`/`cat`), cargo/git subcommand allowlist, sandbox (Ð±ÐµÐ· `..` / shell metacharacters).
- Mutating POST Ð· Ð½Ðµ-loopback `Origin` Ð°Ð±Ð¾ `Sec-Fetch-Site: cross-site` â†’ 403.
- POST body > 256 KiB â†’ 413 `{ok:false}`. Responses include CSP / nosniff / `Cache-Control: no-store`.

## 8. Rust tests / benchmarks hook (Ð±ÐµÐ· Ð¿ÐµÑ€ÐµÐºÐ¾Ð¼Ð¿Ñ–Ð»ÑÑ†Ñ–Ñ—)

**Ð Ð¾Ð»ÑŒ:** Ð·Ð°Ð¿ÑƒÑÐº Ñ‚ÐµÑÑ‚Ñ–Ð²/Ð±ÐµÐ½Ñ‡Ð¼Ð°Ñ€ÐºÑ–Ð² **Ð±ÐµÐ· Ð¿ÐµÑ€ÐµÐºÐ¾Ð¼Ð¿Ñ–Ð»ÑÑ†Ñ–Ñ—** (read-only hook).

- `GET /api/hooks/tests` â†’ ÑÑ‚Ð°Ñ‚ÑƒÑ + Ñ€ÐµÐ·ÑƒÐ»ÑŒÑ‚Ð°Ñ‚Ð¸ Ð· `target/` (deps, `test-*` bins) Ð±ÐµÐ· `cargo build`.
- `GET /api/hooks/bench` â†’ Criterion medians (read `target/criterion/`).
- Ð”Ð°Ð½Ñ– Ð½Ðµ Ð¿ÐµÑ€ÐµÐ±ÑƒÐ´Ð¾Ð²ÑƒÑŽÑ‚ÑŒ Ð¿Ñ€Ð¾Ñ”ÐºÑ‚ â€” Ð»Ð¸ÑˆÐµ Ð·Ñ‡Ð¸Ñ‚ÑƒÑŽÑ‚ÑŒ Ð½Ð°ÑÐ²Ð½Ñ– Ð°Ñ€Ñ‚ÐµÑ„Ð°ÐºÑ‚Ð¸.

## 9. OmniRouter (Rust AI-Ð¿Ñ€Ð¾ÐºÑÑ–/Ñ€Ð¾ÑƒÑ‚ÐµÑ€)

**Ð Ð¾Ð»ÑŒ:** Rust-Ñ€Ð¾ÑƒÑ‚ÐµÑ€ Ð¿Ð¾ AI-Ð¿Ñ€Ð¾Ð²Ð°Ð¹Ð´ÐµÑ€Ð°Ñ… (researched 2026-08-18) Ð´Ð»Ñ **Rust + web** Ð½Ð° OmniRouter, Cursor, OpenCode Ñ– Grok. Ð ÐµÐºÐ¾Ð¼ÐµÐ½Ð´Ð¾Ð²Ð°Ð½Ñ–: Grok 4.6, GPT-5.2 Codex, Claude Sonnet 4.6, Gemini 3 Pro, Kimi K2.7 Code, GPT-5.3 Codex. ÐšÐ¾Ð¶ÐµÐ½ Ð¿Ñ€Ð¾Ð²Ð°Ð¹Ð´ÐµÑ€ Ð¼Ð°Ñ” `quota.reset_secs` â€” MCP `gsv_omni_route` Ð¿Ñ€Ð¾Ð¿ÑƒÑÐºÐ°Ñ” host Ñƒ cooldown.

**Ð”Ð°Ð½Ñ–:**

| ÐŸÐ¾Ð»Ðµ | Ð”Ð¶ÐµÑ€ÐµÐ»Ð¾ |
|------|---------|
| ÐšÐ°Ñ‚Ð°Ð»Ð¾Ð³ Ð¿Ñ€Ð¾Ð²Ð°Ð¹Ð´ÐµÑ€Ñ–Ð² | `catalog.rs` (19 providers, incl. xAI + Cursor) |
| ÐšÐ°Ñ‚Ð°Ð»Ð¾Ð³ Ð¼Ð¾Ð´ÐµÐ»ÐµÐ¹ (ctx / rust / web / clients) | `catalog.rs` |
| ÐšÐ²Ð¾Ñ‚Ð¸ / Ñ‚Ð°Ð¹Ð¼ÐµÑ€Ð¸ | `quota.rs` â†’ `data/omni_quota.json` (Ð½Ðµ git) |
| ÐšÐ¾Ð½Ñ„Ñ–Ð³ / Ñ‚ÑŽÐ½Ñ–Ð½Ð³ | `GSV/data/omni.toml` + env `OMNI_<PROVIDER>_API_KEY` / `_BASE_URL` |
| Ð ÐµÐºÐ¾Ð¼ÐµÐ½Ð´Ð¾Ð²Ð°Ð½Ð¸Ð¹ ÑÐ¿Ð¸ÑÐ¾Ðº | rust+web research 2026-08-18 (6 Ð¼Ð¾Ð´ÐµÐ»ÐµÐ¹) |
| Canon notes | [`GSV_OMNI_CATALOG.md`](./GSV_OMNI_CATALOG.md) |

**Endpoints:**

- `GET /api/omni` â€” overview wire (providers, models, clients, quotas, recommended, routing).
- `GET /api/omni/route?task=rust|web&prefer_free=` â€” timer-aware next pick.
- `GET /api/omni/config` â€” ÐºÐ¾Ð½Ñ„Ñ–Ð³ **redacted** (Ð»Ð¸ÑˆÐµ `key_set`, Ð±ÐµÐ· ÐºÐ»ÑŽÑ‡Ñ–Ð²).
- `POST /api/omni/config` â€” Ñ‚ÑŽÐ½Ñ–Ð½Ð³ (base_url / api_key / enabled / priority / routing).
- `GET /api/omni/v1/models` â€” OpenAI-ÑÑƒÐ¼Ñ–ÑÐ½Ð¸Ð¹ ÑÐ¿Ð¸ÑÐ¾Ðº Ð¼Ð¾Ð´ÐµÐ»ÐµÐ¹.
- `POST /api/omni/v1/chat/completions` â€” OpenAI-ÑÑƒÐ¼Ñ–ÑÐ½Ð¸Ð¹ proxy (empty `model` auto-picks; 429 starts cooldown; dry-run Ñ‡ÐµÑ€ÐµÐ· `X-Omni-Dry-Run: 1`).
- `POST /api/omni/test {provider}` â€” connectivity check (`GET {base}/models`).

**Ð Ð¾ÑƒÑ‚Ð¸Ð½Ð³** (`proxy.rs::select_provider`): `X-Omni-Provider` header / `provider` Ñƒ Ñ‚Ñ–Ð»Ñ– â†’ Ð²Ð»Ð°ÑÐ½Ð¸Ðº Ð¼Ð¾Ð´ÐµÐ»Ñ– Ð· ÐºÐ°Ñ‚Ð°Ð»Ð¾Ð³Ñƒ (skip cooling) â†’ `free_fallback_order` â†’ `routing.default_provider` â†’ `routing.fallback_order` â†’ Ð½Ð°Ð¹Ð²Ð¸Ñ‰Ð¸Ð¹ Ð¿Ñ€Ñ–Ð¾Ñ€Ð¸Ñ‚ÐµÑ‚. `base_url` Ð¼Ð¾Ð¶Ðµ Ð²ÐºÐ°Ð·ÑƒÐ²Ð°Ñ‚Ð¸ Ð½Ð° OmniRoute (`http://127.0.0.1:20128/v1`).

Rust Ð¼Ð¾Ð´ÑƒÐ»ÑŒ: `omni/` (catalog.rs, config.rs, proxy.rs, quota.rs) â†’ `GSV/data/omni.toml`.

## Ð—Ð²ÐµÐ´ÐµÐ½Ð° Ñ‚Ð°Ð±Ð»Ð¸Ñ†Ñ

| Box | Rust module | Endpoint | Ð”Ð¶ÐµÑ€ÐµÐ»Ð¾ Ð´Ð°Ð½Ð¸Ñ… |
|-----|-------------|----------|---------------|
| Tracker | `tracker/` | `/api/tracker` | FM Â§5.12, logs, loc-audit |
| SLI console | `sli/` | `/api/sli` | `bin/`, `scripts/`, `src/bin/` |
| Toolchain | `toolchain/` | `/api/toolchain` | toolchain, env; **band 164:** `cursor` from desktop `package.json` |
| IDE | `ide/` | `/api/ide/â€¦` | opencode/cursor ÑÐµÑÑ–Ñ— |
| Products | `products/` (`boxes/products.rs`) | `/api/products` Â· `/api/products/select` Â· `/api/products/open` Â· `/api/products/scan` | workspace âˆª sibling git âˆª kit; `registered` from PRODUCTS.md (`omniroute` band 149); scan HANDOFF fallback `AGENTS.md` / `docs/ROADMAP.md`; **band 225:** llama-rs scan enriches `heartbeat_path` / `heartbeat_alive` (keep-live file probe) |
| Fingerprints | `fingerprint/` (`boxes/fingerprint.rs`) | `/api/fingerprints` | append-only `docs/gsv/fingerprints.jsonl` (actor / IDE / model / agent / time); ops card `fingerprints`; **band 154:** `model` from `GSV_MODEL` else Cursor session (`CURSOR_MODEL` / `GSV_SESSION_FILE`); else latest Cursor `renderer.log` `catalogModelId`; default `unknown` |
| Ranks | `ranks/` (`boxes/ranks.rs`) | `GET`/`POST /api/ranks` | IT+army merit ladder L0 jun-nub â€¦ L15 marshal-orchestrator; host *displays* marshal; `data/gsv_ranks.json` (gitignored); MCP `gsv_ranks` â€” [`GSV_RANKS.md`](./GSV_RANKS.md) **band 192** |
| Service Worker | `sw/` (`boxes/sw.rs`) | `/sw.js` Â· `/api/sw` | Rust-rendered SW; Cache Storage `gsv-shell-v1`; precache `/` + live CSS + galaxy/vision svg; skip `/events` `/mcp`; ops card `sw` |
| Watchdog | `watchdog/` (`boxes/watchdog.rs`) | `/api/watchdog` Â· bin `gsv-watchdog` | probe `/api/health`; after 2 misses copy debugâ†’live and spawn detached; heartbeat `target/live/watchdog.json`; health row `watchdog_alive`; **band 154:** ops card `watchdog`; **band 162:** `debug_newer` + POST apply when debug is newer than live; **band 165:** live `gsv-watchdog` copy; `lockstep-fail` + `last_apply_status` / `lockstep_note`; oneshot apply if a peer is already running; health `version_lag` also locksteps; **band 172:** `bin_version` / crate `version_lag` on the wire; oneshot on lag; yield only if peer pid is alive; `lockstep-wait` during cooldown; successor hop when the running exe is stale; **band 180:** `hop_successor` each tick; `debug_newer_server` (do not POST apply because a locked watchdog exe is stale); `stop_peer_watchdog` on takeover; wire `server_debug_newer` / `watchdog_debug_newer`; **band 228:** multi-probe `telenetis_alive` (TCP `:9800`) + `telenetis_debug_newer` (Telenetis live-supervisor debugâ†’live parity) on `/api/watchdog` + watchdog card |
| Usage | `usage/` (`boxes/usage.rs`) | `/api/usage` | per-session token counts from OmniRouter completions (**band 156:** includes `stream:true` SSE) + MCP bot (`gsv_omni_chat` / `Mcp-Session-Id`) + fail-open OmniRoute `/api/usage/history`; persist `data/gsv_usage.json`; vision-sync snapshot; Galaxy studio card `usage` (**band 155**) |
| Keep-live | `keep_live/` (`boxes/keep_live.rs`) | `/api/keep-live` Â· `/api/health { keep_live }` Â· MCP `gsv_keep_live` Â· Galaxy card `keep-live` | aggregation-only supervision of the always-on kit (**band 223**): `KeepLiveReport { gsv, telenetis, llama_rs, omniroute }` each `{ alive, url, version?, lag?, latency_ms?, uptime_secs? }` (band 226: probe latency ms + uptime, server injects its own `uptime_secs`), probes 1s, **fail-open** â€” `ok` stays true when a peer is down (like `disk_ok` band 181) + `hint` summarizes up/down peers; no respawn; env overrides `GSV_KEEP_LIVE_GSV_URL` / `GSV_KEEP_LIVE_TELENETIS_URL` / `OMNIROUTE_URL` / `LLAMA_HEARTBEAT_PATH`; llama_rs alive = fresh `llama_heartbeat.json` (age â‰¤ 60s; llama-rs writes it when `GSV_LIVE=1`/`LLAMA_RS_HEARTBEAT=1` â€” band 225 âœ…); omniroute probe URL chain `OMNIROUTE_URL` â†’ `GSV_KEEP_LIVE_OMNIROUTE_URL` â†’ `GSV_OMNIROUTE_URL` â†’ default `http://127.0.0.1:20128` (real omniroute `.env PORT=20128`; usage-box parity, **band 227**); band 226: `GET /mcp` `keep_live_summary` (15s cache) + verify bin `gsv-keep-live-boot-verify` (4 probes, graceful peers, `--strict` â€” see `GSV_SERVER.md`) **band 226 âœ…**; `CARD_NAMES` **43** |
| Grid | `grid/` (`boxes/grid.rs`) | `/api/grid` · MCP `gsv_grid` | ALLBGP durable mirror of the poolAI :8091 fleet view (topology nodes / workers / virtual-nodes / telegram seats) + history ring in `data/gsv_grid.json`. gsv-server refresh loop 10s emits SSE `event: grid` on change; when poolAI is down the last-known topology is KEPT + `poolai_alive:false`/`stale` (hub keeps a working view). env `GSV_POOLAI_URL` default `http://127.0.0.1:8091/api/v1`. Canon [GSV_VDC.md](GSV_VDC.md). `spawn_grid_loop` in `gsv_server`. hub VDC band 2026-09-14 |
| Update | `update/` | `/api/update` Â· `/api/update/apply` Â· `/events` | live copy + Ð²ÐµÑ€ÑÑ–Ñ; **band 162:** `crate_version` / `version_lag`; **band 191:** `github_ahead` / `can_apply` (origin newer even when local `src/` is not); health uses the same `update_available` |
| Box preview | `preview/` | `/api/preview` | Ñ„Ð°Ð¹Ð»Ð¸ |
| SLI terminal | `terminal/` | `/api/terminal` | SLI-ÐºÐ°Ñ‚Ð°Ð»Ð¾Ð³ |
| Tests/bench hooks | `hooks/` | `/api/hooks/â€¦` | `target/` Ð°Ñ€Ñ‚ÐµÑ„Ð°ÐºÑ‚Ð¸ |
| OmniRouter | `omni/` (`boxes/omni/`) | `/api/omni/â€¦` | shared catalog + `omni.toml` + quota timers; **band 225:** local provider `bunke-rock` / `lama-2.8` (kind `local`, gated on `models/Qwen3.8-27B-UD-IQ2_XXS.gguf` exists via `catalog::host_ready`); wire `ProviderWire.kind`; route task=rust picks it when prefer_free |
| Vision | `vision/` (`boxes/vision.rs`) | `/api/vision*` Â· `/assets/vision.svg` | `GSV/docs/vision/{manifest,feed,extensions}.json` â†’ `GSV/data/gsv_*.json`; **band 163:** `cargo xtask bump --band N` locksteps `last_sprint_closed` / `next_sprint` / `active_sprint`; **band 173:** bump is **close of N** (last of N / first of N+1), not start of N; `/assets/vision.svg` is a static **L0â€“L5 legend** (live graph = Vision Map chips) |
| UI fragments | `ui/` (`boxes/ui.rs`) | `/api/ui/layout` Â· `/api/ui/card/:name` Â· `/api/ui/load-palette` Â· `/api/ui/load-theme` | dashboard `CARD_NAMES` **43** + chrome 8 + layout `html`/`header` + live `:root` CSS; **band 190:** About box + hover tips + distinct card icons; **band 181:** Galaxy glue `selectProduct` / `reclaimTicket` + health `disk_ok`; **band 168:** ops card `tickets`; **band 167:** ops card `telegram`; **band 166:** ops card `settings`; **band 156:** `.card.fullscreen img{max-height:none`; **band 155:** studio card `usage`; **band 154:** ops card `watchdog`; **band 148:** ops card `sw`; **band 146:** ops card `fingerprints`; **band 145:** ops card `products` (list/select/open/scan); **band 143:** power menu `z-index:80` above workspace, exclusive fullscreen (`data-action='card-fs'`), collapsed cards `display:none` (dock restore), `--fs-*` type scale, speed/rust SVG height 168; **band 223:** studio card `keep-live` |
| About | `guide/` (`boxes/guide.rs`) | `/api/ui/card/about` Â· `/api/ui/icon/:name` Â· `/api/ui/icons.svg` | English how-to; hover blurbs for every card; distinct SVG glyph per box; ratio ring; always-visible About card |
| Stand smoke | `src/bin/gsv_http_stand_smoke.rs` | live HTTP Ð¿ÐµÑ€ÐµÐ²Ñ–Ñ€ÐºÐ° | Ð²ÑÑ– boxes + `/api/vision*` + SVG + `/api/ui/card/:name` |
| **gsv_mcp_openbot** | `mcp.rs` + `gsv-mcp` bin | stdio live copy + `GET`/`POST`/`DELETE /mcp` + Galaxy card `/api/ui/card/mcp` | 58 box tools + 13 `gsv://` resources (band **226** `gsv_telenetis_health` + keep-live hint / `keep_live_summary` Â· **223** `gsv_keep_live` Â· **192** `gsv_ranks` + `gsv://docs/ranks` Â· **186** `gsv://docs/solo-squad-jail` Â· **185** `catalog_stale` / restart Cursor Â· **184** session catalog lockstep Â· **183** `gsv_tickets_next` + `tools/list_changed` Â· **182** `gsv_telegram_decode` Â· **179** `gsv_telegram_poll` Â· **178** `gsv_tickets_bench` Â· **177** `gsv_tickets_hook` Â· **175** `gsv_tickets_walk` + `gsv_mds` Â· **174** `gsv_telegram_ticket` Â· **173** `gsv_drain` close-lockstep Â· **171** `gsv_tickets_reclaim` Â· **170** `gsv_tickets_create` + `done` + `error` + `presence` Â· **169** `gsv_telegram_bus_send` + `gsv_telegram_bus_poll` Â· **168** `gsv_tickets` + `gsv_tickets_claim` Â· **167** `gsv_telegram` Â· **166** `gsv_settings` + `gsv://docs/settings-telegram` Â· **164** Cursor 3.16.29 kit lockstep Â· **160** GSV sandbox + no User MCP Â· **159** Cursor HTTP url + session SSE hold Â· **158** live stdio + sync check Â· **157** omni route) â€” [`GSV_OMNI_CATALOG.md`](./GSV_OMNI_CATALOG.md) Â· [`GSV_MCP_OPENBOT.md`](./GSV_MCP_OPENBOT.md) Â· [`GSV_SOLO_SQUAD_JAIL.md`](./GSV_SOLO_SQUAD_JAIL.md) |
| Settings | `settings/` (`boxes/settings.rs`) | `GET`/`POST /api/settings` | Godfather channel + redacted token + co-workflows + **band 189** labeled Galaxy form / `squad_cap_override` / dark scrollbars; **band 186** jail id / squad_cap / member_count; `data/gsv_settings.json` (gitignored; env `GSV_TELEGRAM_BOT_TOKEN` wins) â€” [`GSV_SETTINGS_TELEGRAM.md`](./GSV_SETTINGS_TELEGRAM.md) **band 166 âœ…** Â· [`GSV_SOLO_SQUAD_JAIL.md`](./GSV_SOLO_SQUAD_JAIL.md) **band 186 âœ…** |
| Telegram | `telegram/` (`boxes/telegram.rs`) | `GET /api/telegram` Â· `GET`/`POST /api/telegram/bus` Â· `POST /api/telegram/ticket` Â· `POST /api/telegram/poll` Â· `POST /api/telegram/decode` | Godfather bind (`getMe`+`getChat`+`getChatMemberCount`); dry-run stub under cargo test / `X-Telegram-Dry-Run: 1`; **band 187** live member_count persist; **band 179** inbound `getUpdates` loop in `gsv-server`; **band 182** dual session line + JSON `data` / MCP `gsv_telegram_decode`; never `bot_token`; band **174** ticket ingest; band **175** `kind:sync` on solo walk; band **176** session lines + live `sendMessage` 1/s â€” [`GSV_SETTINGS_TELEGRAM.md`](./GSV_SETTINGS_TELEGRAM.md) **band 167 âœ… Â· 174 âœ… Â· 175 âœ… Â· 176 âœ… Â· 179 âœ… Â· 182 Â· 187** |
| Tickets | `tickets/` (`boxes/tickets.rs`) | `GET`/`POST /api/tickets` Â· `POST /api/tickets/claim` Â· `/done` Â· `/error` Â· `/presence` Â· `/reclaim` Â· `/walk` Â· `/hook` Â· `GET`/`POST /api/tickets/bench` Â· `POST /api/tickets/next` | git JSONL board; scenario `tickets[]` bands; registered product; solo/squad; `lease_until` + stale reclaim; jail/`squad_cap` + join `env` (band **186**); MCP `gsv_tickets*` + `gsv_telegram_ticket` + `gsv_telegram_poll` + `gsv_telegram_decode` + `gsv_tickets_walk` + `gsv_tickets_hook` + `gsv_tickets_bench` + `gsv_tickets_next` + `gsv_ranks` + `gsv_keep_live` + `gsv_telenetis_health` (**58** tools); events in `ticket_claims.jsonl` â€” [`GSV_SETTINGS_TELEGRAM.md`](./GSV_SETTINGS_TELEGRAM.md) **band 168 âœ… Â· 170 âœ… Â· 171 âœ… Â· 174 âœ… Â· 175 âœ… Â· 176 âœ… Â· 177 âœ… Â· 178 âœ… Â· 179 âœ… Â· 182 âœ… Â· 183** Â· [`GSV_SOLO_SQUAD_JAIL.md`](./GSV_SOLO_SQUAD_JAIL.md) **186** |
| MDS | `mds/` (`boxes/mds.rs`) | `GET /api/mds` | light memory / disk / speed probe (`gsv-mds` bin) â€” **band 175 âœ…** |
| Telegram bus | same telegram box (**band 169 âœ…** Â· **193** `kind:presence` Â· **194** `kind:claim` Â· **195** `kind:done` Â· **196** `kind:reclaim`) | MCP `gsv_telegram_bus_send` / `gsv_telegram_bus_poll` Â· `GET`/`POST /api/telegram/bus` | MCP bots talk via Godfather channel envelopes (dry-run in-memory queue; no webhook; no Cloudflare). Band **193**: federated jail heartbeat `kind:presence`. Band **194**: federated ticket claim `kind:claim` on this jailâ€™s board. Band **195**: federated ticket close `kind:done` (`in_progress` â†’ `done`; ranks process-local). Band **196**: federated release `kind:reclaim` (lease sweep + explicit wire; peer boards back to `open`; ranks process-local). |
| Telegram ticket | same telegram box (**band 174 âœ…**) | MCP `gsv_telegram_ticket` Â· `POST /api/telegram/ticket` | `/ticket` or `{kind:ticket}` â†’ board row; solo MCP auto-claims when one worker is online |
| Telegram walk sync | same telegram box (**band 176 âœ… Â· 182**) | MCP `gsv_tickets_walk` Â· `POST /api/tickets/walk` | dual human line + JSON `data` (`hint` / `next` / disk / crate); live send 1/s; dry-run queue |
| Ticket next | tickets box (**band 183**) | MCP `gsv_tickets_next` Â· `POST /api/tickets/next` | A2A-style inbox: Godfather `hint` â†’ next tool; `initialize` `tools.listChanged` |
| MCP catalog | mcp (**band 185**) | GET `/mcp` `catalog_stale` / `catalog_hint` / `listed_tool_count` | Cursor agent refresh does not re-list; Galaxy warns **restart Cursor** when listed is 0 with sessions |
| Telegram / MCP hook | tickets + telegram (**band 177 âœ…**) | MCP `gsv_tickets_hook` Â· `POST /api/tickets/hook` Â· phrase on `gsv_telegram_ticket` | `run mcp bot hook up scenario <id|band N|plan stem> [walk]`; cap 10; Godfather `hook â€¦ n=` |
| Telegram poll | same telegram box (**band 179 âœ…**) | MCP `gsv_telegram_poll` Â· `POST /api/telegram/poll` | `gsv-server` `getUpdates` loop; classify `/ticket` / hook / bus; offset `data/telegram_offset.json` |
