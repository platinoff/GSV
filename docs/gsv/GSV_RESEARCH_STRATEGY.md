# GSV research — rank/game strategy + Telegram live-app practices

**Status:** Landed (band 213, owner pick). **Sourced:** web research 2026 + GSV/telenetis code audit.
**Scope:** (1) best rank-ladder / game-strategy models similar to GSV ranks; (2) Telegram Mini App /
bot best practices; (3) how both map to a *maximally live* telenetis app and solo/isquad bot
messaging.

## 1. Rank-ladder / game strategy (~ GSV ranks)

GSV's merit ladder `src/boxes/ranks.rs` L0 jun-nub … L15 marshal-orchestrator is a
progression/retention system for a cooperative multi-agent squad (host + mates + guests). The
published 2026 game-design research applies directly to that ladder.

### 1.1 Layered rating vs visible rank

Most competitive games separate a **hidden rating (MMR)** used for matching from the **visible
rank** the player chases. The visible rank moves slowly and gives progression feel; the hidden
number stays honest and volatile.

- GSV today has a single visible merit score (−1 / +1 per done / error) and a derived rank id.
- **Adaptation:** keep visible `rank_id` + rank title as the *displayed* ladder; keep the raw
  merit score as the "MMR" that moves faster and is never erased. This split already fits
  (`ranks::on_ticket_done` +/-1 and rank threshold). Do not expose raw score swings to the
  channel beyond the rank badge — the rank is the thing that "sticks".

### 1.2 Tier design + a wide, readable middle

- Anchor the average tier at the starting point; keep the top tier roughly the top 5–10%.
- A healthy spread is a wide middle and a small elite top. If everyone is one rank, tiers are
  too coarse; if everyone is unique, they are too fine.
- Color/name signal status and become free advertising whenever a member speaks.

**GSV:** L0–L15 with IT + army titles already gives 16 named tiers with a visible title. Keep the
naming (ZSU/NATO mix) as the identity surface; tune thresholds so new joiners land mid-ladder,
not at the very bottom, so the "climb" reads instantly.

### 1.3 Promotion/demotion, decay, seasons

- **Promotion series / grace period:** after promoting, a grace window stops immediate
  oscillation. GSV already floors merit at 0 (`−1` never demotes below the floor). Consider a
  short demotion grace after a fresh `+1` so one error does not feel punitive.
- **Rank decay:** inactive players' stale top ranks clog the board. Decay stops the moment they
  return. For GSV this is the Godfather **host** slot: a host that stops draining should not hold
  `marshal-orchestrator` forever. Grace period (e.g. 14 days) + decay only in the top tiers keeps
  the elite spots earned.
- **Seasons/soft reset:** a recurring reset renews motivation. GSV's VDT band cadence is itself
  a season: each band close is a soft landmark. Optionally snapshot "peak rank" per worker.

### 1.4 Psychology + communication (what bots post)

- Clear, nameable next rank; frequent micro-rewards; historical tracking. The **rank badge** is
  the reward. Loss/spiral protection reduces churn.
- The single most important and cheap win: **a player can explain in plain terms why the score
  moved**. GSV's Godfather lines ("done +1 / error −1") should always carry the *reason*
  (ticket id + verdict), so the merit movement is legible — never a bare `+1`.

### 1.5 Solo vs squad (TrueSkill / team awareness)

- Team/coop formats need per-player skill *in the team context*. GSV federation
  (`kind:presence/claim/done`) already treats ranks as **process-local**: remote dones never move
  the host ladder. Keep that: fairness requires not letting a crowded squad inflate any one
  worker's merit ladder.

## 2. Telegram Mini App + bot best practices (2026)

Applied to **telenetis** (Axum on 9800, HTMX Mini App, WS `/ws` + SSE `/events`, band 208–212).

### 2.1 The WebView is not a browser

- **No cookies for auth** — the WebView drops them unpredictably. Authenticate with an explicitly
  passed token. telenetis must verify `initData` (HMAC‑SHA256) server-side, never trust
  `initDataUnsafe` client-side. `auth_date` freshness matters.
- **No `100vh`** — use `--tg-viewport-stable-height` / `viewportStableHeight` + `viewportChanged`.
- **No `window.open`/`alert`/`confirm`/`prompt`** — use `openLink`, `openTelegramLink`,
  `showAlert`, `showConfirm`, `showPopup`. Haptics for native feel.
- **Native back** — route through `Telegram.WebApp.BackButton`, not history.
- **No redirects** — precompute the target URL into the button at render time.
- **Theme** — all colors via `--tg-theme-*` variables; both themes mandatory; subscribe to
  `themeChanged`. `tg.ready()` once after first render.
- **i18n from day one** — no hardcoded strings in markup; `initDataUnsafe.user.language_code`.
  Catalogs test languages + themes + all platforms; adapt before submission, not after.
- **Platform detection** — `whereAmI()`-style: iOS / Android / Desktop / web are four distinct
  webviews; branch CSS/logic; test on all.

**telenetis gap check:** templates are HTMX askama without a Telegram SDK. The `--tg-*` deep
integration (BackButton, haptics, safe areas, theme vars, initData HMAC path in the Mini App) is
the concrete "maximally live" work for next session.

### 2.2 Tunnel-first dev (GSV already ready)

- Telegram requires public HTTPS for bot webhook + Mini App URL; `localhost` does not work.
- Use a tunnel from day one (cloudflared) for hot-reload iteration. GSV already has
  `cargo xtask tunnel` (band 156 Grok Bot tunnel). Reuse it for the telenetis webhook path.
- Webhooks (not long-polling) in production; polling is a dev fallback.

### 2.3 Cold start / performance

- Telegram users compare to Telegram itself (fast). Target **first paint < 2 s**, bundle
  < ~300 KB, skeleton screens before data, prefetch on `/start`.
- **WebSockets work reliably in the WebView** across platforms — don't fall back to 1 s polling.
  telenetis already has WS `/ws` + SSE `/events` / broadcast; keep WS as primary, polling as
  insurance. Drop connection on backgrounding → reconnect with backoff.

### 2.4 Architecture & scaling

- **Separate bot and Mini App backends** under load (traffic spikes must not silence the bot).
  telenetis is a single Axum process — note this as a scaling lever, not a v1 change.
- **Outbound message queue** for Telegram rate limits (growth spikes flood the Bot API).
- **Persist state**; **backend is the source of truth**; **idempotent** operations; **graceful
  degradation** (WS down → SSE → polling; AI/service down → deterministic fallback).

## 3. Solo/isquad live bot messaging (research-informed)

"For the app and bot work in solo/isquad to be maximally live," the Godfather channel should
carry a legible, cadenced operation log, not noise:

| Event | Post | Why (research) |
|-------|------|----------------|
| Ticket claimed (solo) | `claimed <title> · <rank badge>` | progression feel; who owns what |
| Ticket done | `done <title> +1 rank=<badge>` | reward + legible score movement |
| Ticket error | `error <title> −1` | loss-spiral protection; reason attached |
| Squad assign | `assigned {t} → {worker}` | team context; remote dones don't move host ladder |
| Presence (mate/heartbeat) | cadenced, TTL-known | keeps the board honest; decay/activity signal |
| Bench / S0 / warnings | 1-liners with data | status telemetry, shows work is happening |

**Cadence & rate limits:** respect Telegram 1 msg/s and message caps; batch where possible;
skip echo of own envelopes (already enforced: guest mute, echo skip). The dual
line + JSON `data` format (band 182) is the right shape: human reads the line, MCP reads the
JSON.

**Rank/role awareness:** host displays marshal-orchestrator; mates claim; guests stay solo and
cannot send on the live bus. Decay only top tiers (research 1.3) so the elite spots stay earned.

## 4. Mapped findings → next steps

Consolidated into GSV_TECH_ROADMAP.md band 213 and
`docs/superpowers/plans/2026-08-27-telenetis-live-app.md`:

1. Make telenetis a *real* Telegram Mini App: `--tg-*` theme vars, native BackButton, haptics,
   safe-area handling, `initData` HMAC verify, `whereAmI()` platform branch, i18n strings.
2. Keep WS `/ws` primary for live flow; SSE `/events` fallback; reconnect w/ backoff.
3. Cold-start: skeleton + prefetch on `/start`, small bundles.
4. Ranks: legible `+1/−1 with reason`, promotion/demotion grace, optional top-tier decay,
   peak-rank snapshot; keep ranks process-local in federation.
5. Tunnel-first dev via existing `cargo xtask tunnel` for the webhook/Mini App path.

Sources: 2026 web research — StraySpark competitive multiplayer design; GTStudios progression;
solana.garden progression systems; ICN fair-ladder; BoostRoom ranked-climb; TeamUp rank decay;
vibeseeker Telegram Mini App nuances; miniapps.me building Telegram products 2026;
dev.to scaling TMA-2026; core.telegram.org/bots/webapps; Rithprohos telegram-mini-app-skills.
