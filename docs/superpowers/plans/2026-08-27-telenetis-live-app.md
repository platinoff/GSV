# Plan — telenetis live Mini App + solo/isquad live bot (band 213)

**Goal:** make telenetis a *maximally live* Telegram Mini App and tune the solo/isquad bot
messaging per research (`docs/gsv/GSV_RESEARCH_STRATEGY.md`). This band is **research +
roadmap/plan adaptation** — the code-landing work is scheduled as the next telenetis code bands.
Everything here is grounded in the 2026 Telegram Mini App + rank-strategy research.

## Why

- Telegram WebViews are not browsers: cookies drop, no redirects, no `100vh`, no
  `window.open`/`alert`. telenetis currently ships HTMX templates without the Telegram SDK deep
  integration (`--tg-*` theme, BackButton, haptics, safe areas, `initData` HMAC).
- Live feel = WS-first (works in WebView) + cold-start < 2 s + native back/haptics + theme sync.
- Ranks/coordination need *legible* merit movement and decay so the Godfather channel stays a
  living operation log rather than noise.

## What (planned next-code bands)

1. **Mini App Telegram-native layer**
   - `initData` HMAC-SHA256 server-side verify + `auth_date` freshness (telenetis `security/`).
   - `whereAmI()` platform/module detection; branch CSS/logic per iOS / Android / Desktop / web.
   - Theme via `--tg-theme-*` CSS variables; both light + dark; `themeChanged`; `tg.ready()`.
   - Native `BackButton` navigation; haptics on claim/done/error actions.
   - Safe-area sizing via `--tg-viewport-stable-height` (+ `viewportChanged`), never `100vh`.
   - i18n strings (no hardcoded UI text) keyed by `initDataUnsafe.user.language_code`.

2. **Live stream primacy**
   - Keep WS `/ws` as the primary live channel (broadcast FlowEvent); SSE `/events` fallback.
   - Client reconnect with exponential backoff; server keep-alive; drop-tolerant.

3. **Cold start** — ✅ **landed (band 217)**.
   - Skeleton screens (shimmer rows per `data-area`, `aria-busy`) in all five
     templates — first paint is never blank.
   - `GET /api/snapshot?lang=` consolidated bundle (status + tickets + flows +
     workers + i18n + live config) prefetched in one round-trip; `<link rel="preload">`
     hint in every template.
   - `app.js` `bootstrap()` opens the WS immediately (early upgrade, parallel with
     the snapshot) and hydrates board/flows/roles/status from the snapshot.
   - `/start` server-side prefetch (`warm_start` in `bot/webhook.rs`) syncs the
     board before the Mini App opens; offline fallbacks keep skeletons + SSE live.
   - Remaining P-scope (Mini App board actions + i18n + offline empty states) —
     ✅ **landed (bands 218 + 219)**.

4. **Ranks + messaging polish (GSV box, not telenetis)**
   - Alive Godfather lines always attach a reason: `done <t> +1` / `error <t> −1`.
   - Promotion/demotion grace; optional top-tier-only decay; peak-rank snapshot.
   - Keep ranks process-local in federation (remote dones never move the host ladder).

## Route through the flow

- Registered scenario `gsv-live-app-research` (band 213) — walk via
  `POST /api/tickets/walk {scenario_id, create:true}` posts claim/done + Telegram `kind:sync`
  to Godfather (this session).
- Subsequent telenetis code bands land the Telegram-native layer above (P0 → P4).

## Acceptance

- Research + plan + roadmap band 213 tables committed; scenario registered; warn-in
  telenetis clippy fixed; fmt/clippy/test green; band bumped; one commit + push.
