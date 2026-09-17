---
name: agi
description: >-
  Trigger word agi starts a VDT drain with keep-live of all hub services and
  the GSV ticket board. Same product discovery as абракадабра / abrakadabra.
  FIRST cargo xtask products, ask which environment project, THEN keep-live
  (GSV, Telenetis, llama-rs, poolAI grid; OmniRoute policy-down), tickets,
  drain, one commit + push. Use when the owner writes agi in a new session
  (Cursor, OpenCode, or Grok).
metadata:
  audience: gsv-vdt-kit
  clients: cursor-opencode-grok
---

# `agi` — VDT drain + keep-live + tickets

Works the same in **Cursor**, **OpenCode**, and **Grok Build**. Git canon:
**`S:\rust\GSV/.agents/skills/agi/`**. Client copies under `.cursor/skills/` and
`.opencode/skills/` must stay identical (`cargo xtask mirrors`). Grok reads
`.agents/skills` via `[skills] paths` in `.grok/config.toml`.

`agi` is **not** a different product loop. It is the `абракадабра` /
`abrakadabra` drain **plus** hub keep-live and the ticket board. After Step 0,
follow [`.agents/skills/abracadabra/SKILL.md`](../abracadabra/SKILL.md) for the
picked product.

Do **not** assume the product is GSV just because the window is GSV.

## Step 0 — Discover environment projects (ALWAYS first)

When the owner writes `agi` (standalone word; same session as `абракадабра` /
`abrakadabra`), **before anything else**:

```bash
C:\msys64\usr\bin\bash.exe -lc 'cd /s/rust/GSV && cargo xtask products'
```

Then AskQuestion / `question` / numbered list: **«Проєкти з цього середовища. З яким працюємо?»**

One option **per discovered row**. Do **not** hardcode `gsv | poolai`.

## Step 0d — Keep-live all hub services (before drain)

Call MCP (live `:9999`, 59 tools / 15 `gsv://`). Fail-open: a down peer does not
abort the session.

| Probe | Tool / check | Expected |
|-------|----------------|----------|
| GSV hub `:9999` | `gsv_health` + `gsv_watchdog` | `ok`, `version` = crate, `version_lag=false` |
| Keep-live aggregate | `gsv_keep_live` | `gsv` + `telenetis` + `llama_rs` up when those processes exist |
| Telenetis `:9800` | `gsv_telenetis_health` | alive if the bot is running |
| poolAI grid `:8091` | `gsv_grid` | ALLBGP mirror; report nodes/workers |
| OmniRoute `:20128` | `gsv_keep_live` `omniroute` | **policy-down** (EDGE_PLAN). Probe only. **Do not start it.** |

Do **not** start a second `cargo xtask live`. If `version_lag` or
`server_debug_newer`, recopy via `POST /api/update/apply` (owner window) after
the drain build — not mid-scan. Do **not** kill `target/live/` before GSV tests.

`catalog_stale` / `listed_tool_count` ≠ `tool_count` → tell the owner to restart
Cursor (agent refresh does not re-list tools).

## Step 0e — Tickets

1. `gsv_tickets_presence` (heartbeat).
2. `gsv_tickets` — list open / in_progress. Do not claim owner-gated or other-product rows unless the pick is that product.
3. `gsv_tickets_create` for this session work (product = owner pick). Scenario `agi-session` places the keep-live + drain band.
4. Claim → drain → `gsv_tickets_done` with a note. Optional: `gsv_tickets_hook` ≤10 PH-S* from the product roadmap.
5. `gsv_tickets_next` after presence if the board already has WIP.

Godfather / Telegram tokens: never echo. `gsv_settings` / `gsv_telegram*` are redacted reads.

## Then drain the picked product

Follow **abracadabra** for that row (S0 disk → warnings-first → ≤10 PH-S* →
product tests → one commit + push + summary). Hard rules are the same: MSYS2
bash, no `git add -A`, no mid-drain push, no parallel `cargo` on one `target/`.

## See also

- Drain skill: `.agents/skills/abracadabra/SKILL.md`
- Keep-live: `docs/gsv/GSV_EDGE_PLAN.md` · MCP `gsv_keep_live`
- Tickets: `docs/gsv/GSV_SETTINGS_TELEGRAM.md` · scenario `agi-session`
