# GSV ranks — merit ladder (IT + army)

**Status:** Band **220**.  
**Date:** 2026-08-28  
**Store:** `data/gsv_ranks.json` (gitignored — Telegram ids never land in git)

## Research

Game ranks are a **monotonic integer** plus a **display title**. Military tables mix enlisted (OR) and officers (OF). Ukrainian ЗСУ uses рядовий → сержантський корпус → офіцери → генерали; NATO uses Private → NCO → Lieutenant → General. IT careers use Intern → Junior → Middle → Senior → Staff/Lead → Principal → Architect → Distinguished → Fellow.

GSV mixes those lists into **16 rungs (L0–L15)**. The names are playful on purpose:

| L | Mix title | IT | Army |
|---|-----------|----|------|
| 0 | **Jun-nub** | Intern | Рядовий / Private |
| 1 | Intern-private | Intern | Солдат / Private+ |
| 2 | Trainee-soldier | Trainee | Старший солдат / PFC |
| 3 | Junior-corporal | Junior | Капрал / Corporal |
| 4 | Associate-sergeant | Associate | Молодший сержант / Junior Sergeant |
| 5 | Middle-staff | Middle | Сержант / Staff Sergeant |
| 6 | Senior-NCO | Senior | Старший сержант / SFC |
| 7 | Lead-warrant | Lead | Головний сержант / Sergeant Major |
| 8 | Staff-lieutenant | Staff | Молодший лейтенант / 2LT |
| 9 | Senior-lieutenant | Senior+ | Лейтенант / 1LT |
| 10 | Principal-captain | Principal | Капітан / Captain |
| 11 | Architect-major | Architect | Майор / Major |
| 12 | Distinguished-ltcol | Distinguished | Підполковник / LtCol |
| 13 | Fellow-colonel | Fellow | Полковник / Colonel |
| 14 | General-fellow | Distinguished Engineer | Генерал / General |
| 15 | **Marshal-orchestrator** | Orchestrator | Маршал / Marshal |

**Floor is 0.** A demote at Jun-nub stays Jun-nub. **Cap is 15.**

## Channel host vs earned marshal

The **bot admin / host** of the Godfather channel **displays** `marshal-orchestrator` (the chief) even when their *earned* merit is lower. Mates and guests climb L0–L15 from completed tickets. Guest still starts at Jun-nub.

## Rewards

- Ticket **done** → **+1** level (`done_ok`). Identity = fingerprint `actor|ide|agent` and optional Telegram `from.id` (`telegram_id` on MCP/HTTP, or `GSV_TELEGRAM_USER_ID`).
- Ticket **error** (blocked) → **−1** (`done_bad`).
- Recorded **`cargo test` failed after a commit** (`docs/vision/speed_index.json` `latest.test_ci_ok=false`) → **−1** once per `git_head`, attributed to the matching fingerprint row (and Telegram id when set). Galaxy **review failed tests** / MCP `gsv_ranks action=review`.

Telegram ids are stored only under `data/` and redacted on the wire to a 4-character **tail**.

## API

- `GET /api/ranks` — ladder + redacted roster
- `POST /api/ranks` `{action:list|award|demote|review,...}`
- MCP `gsv_ranks` (same body)
- Galaxy ops card `ranks` (`CARD_NAMES` 42)
- Resource `gsv://docs/ranks` → this file

Band **193:** host/mate heartbeats include the redacted rank badge on Godfather `kind:presence` (`rank_id` / `rank_title`). Guest does not post.

Band **194:** army rank titles corrected — L4 "Sergeant" → "Junior Sergeant" (Молодший сержант), L7 "Warrant" → "Sergeant Major" (Головний сержант). Full NATO equivalents filled in the table.

## Band 220 — reason-bearing lines, grace, decay, peak

- **Never bare `+1`.** Godfather lines for a done ticket now carry the merit reason: `done … +1` (or `−1` for an error/blocked ticket). A demote that is **grace-held** reads `(grace-held)` instead of `−1`. Federation (`done_remote` / `reclaim_remote`) is process-local and never touches ranks, so no reason line is invented for remote closes.
- **Demotion grace.** A `−1` within `GRACE_SECS` (1 h) of the last move is **held** — the row keeps its level (event kind `grace-held`) instead of losing a rung. Prevents a fresh ticket victory being instantly clawed back.
- **Top-tier host decay.** A host row at level ≥ `TOP_DECAY_LEVEL` (14) with no move for `HOST_DECAY_SECS` (14 days) is decayed −1 on `wire()`. Only the host row decays; earned marshal does not.
- **Peak snapshot.** Each `RosterRow` tracks `peak` (high-water mark of the played ladder) and `last_move_ts` (RFC3339). `peak` survives a later drop, so the roster shows both the all-time high and the current `level`. `redact_row` emits `peak`; wire roster rows stay redacted (Telegram ids → 4-char tail).
- Return value: `award` / `demote` / `on_ticket_done` / `on_ticket_error` / `on_apply` now return a `RankMove { row, delta, held }`; the walk/done/error handlers thread `delta` / `held` into the Godfather line and into `WalkStep.rank_delta` / `rank_held`.
