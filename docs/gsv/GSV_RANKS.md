# GSV ranks — merit ladder (IT + army)

**Status:** Band **192**.  
**Date:** 2026-08-20  
**Store:** `data/gsv_ranks.json` (gitignored — Telegram ids never land in git)

## Research

Game ranks are a **monotonic integer** plus a **display title**. Military tables mix enlisted (OR) and officers (OF). Ukrainian ЗСУ uses рядовий → сержантський корпус → офіцери → генерали; NATO uses Private → NCO → Lieutenant → General. IT careers use Intern → Junior → Middle → Senior → Staff/Lead → Principal → Architect → Distinguished → Fellow.

GSV mixes those lists into **16 rungs (L0–L15)**. The names are playful on purpose:

| L | Mix title | IT | Army |
|---|-----------|----|------|
| 0 | **Jun-nub** | Intern | Рядовий / Private |
| 1 | Intern-private | Intern | Солдат |
| 2 | Trainee-soldier | Trainee | Старший солдат / PFC |
| 3 | Junior-corporal | Junior | Капрал |
| 4 | Associate-sergeant | Associate | Молодший сержант |
| 5 | Middle-staff | Middle | Сержант / Staff Sergeant |
| 6 | Senior-NCO | Senior | Старший сержант |
| 7 | Lead-warrant | Lead | Головний сержант / Warrant |
| 8 | Staff-lieutenant | Staff | Молодший лейтенант / 2LT |
| 9 | Senior-lieutenant | Senior+ | Лейтенант / 1LT |
| 10 | Principal-captain | Principal | Капітан |
| 11 | Architect-major | Architect | Майор |
| 12 | Distinguished-ltcol | Distinguished | Підполковник |
| 13 | Fellow-colonel | Fellow | Полковник |
| 14 | General-fellow | Distinguished Engineer | Генерал |
| 15 | **Marshal-orchestrator** | Orchestrator | Маршал |

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
