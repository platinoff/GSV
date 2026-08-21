# Solo / squad / jail — federated GSV MCP

**Status:** Bands **186–196**. **196** federated `kind:reclaim`. **195** federated `kind:done`. **194** federated `kind:claim`. **193** federated `kind:presence`. **192** ranks + no CMD flash. **191** host/mate/guest/local + GitHub origin lockstep. **187** live `getChatMemberCount`. Bands **166–185 ✅**.  
**Date:** 2026-08-20  
**Deciders:** owner  
**Owner ask:** update + security check; Git workflow research; align solo vs squad; environment checks for a joiner who installed their own `gsv-server`; how that MCP joins a squad; host bot-admin on a channel vs own channel + own bot + invite others; squad bot cap = channel user cap; apps built inside a per-server MCP jail.

**Telegram / tickets:** [`GSV_SETTINGS_TELEGRAM.md`](./GSV_SETTINGS_TELEGRAM.md)  
**MCP sandbox:** [`GSV_MCP_OPENBOT.md`](./GSV_MCP_OPENBOT.md)  
**Plan:** [`docs/superpowers/plans/2026-08-20-gsv-solo-squad-jail.md`](../superpowers/plans/2026-08-20-gsv-solo-squad-jail.md)

## Problem

v1 squad is **several MCP clients on one `gsv-server` jail** (process-local presence). That is enough for Cursor + OpenCode on the owner machine. It does **not** explain:

1. A person clones GSV, runs **their** live server, and wants to join **your** Godfather squad.
2. You already have a Telegram **bot admin** on a channel.
3. Someone else wants **their** channel + **their** bot and to invite other people who also run GSV + Telegram bots.
4. How big the squad may be.
5. Where application code is allowed to live (security jail).

Cost of leaving it: joiners paste `bot_token` into chat, point Cursor MCP at a tunneled `/mcp`, or share one working tree so two agents overwrite files.

## Research

### Git (2026 multi-agent practice)

Production parallel-agent setups isolate each worker with a **git worktree** (own index + directory, shared object DB), assign **one ticket per worktree**, and **merge sequentially** (PR / merge queue). MCP is agent-to-tool; the ticket board is agent-to-agent (A2A). Git is the merge queue, not the chat bus.

| Mode | Git | Cargo / `target/` | Who pushes |
|------|-----|-------------------|------------|
| **Solo** | Trunk on `main`. One worktree. One commit at drain close (`cargo xtask git`). No mid-drain push. | One `target/` (GSV). Do not kill `target/live/` before tests. | The one jail |
| **Same-machine squad** | Still one clone. Sequential cargo (file lock). Tickets.jsonl is the shared board. | Same jail `target/`. Two MCP *clients* (Cursor HTTP + OpenCode stdio) against **one** live server. | One closer commit |
| **Federated squad** | GitHub Flow: branch per ticket, PR, sequential merge. Each jail has its own clone/worktree. Do **not** treat peer `tickets.jsonl` as merge source of truth (host board + Godfather envelopes are). | Separate `target/` per machine. PoolAI uses `CARGO_TARGET_DIR=/s/rust/poolAI/target` only when draining poolai. | Each jail pushes **its** branch; host merges |

`cargo xtask git` (status / log / fetch / commit `--file comitmsg/*.md` / push) stays the VDT closer. Terminal MCP still cannot `git push` (SLI allowlist).

### Telegram ceilings (published 2026)

Used as **BotFather bot slots**, not as the MCP jail-worker cap.

| Chat | People | Admins (incl. bots) | Bots in the chat |
|------|--------|---------------------|------------------|
| Basic group | 200 (then upgrades) | 50 | **20** |
| Supergroup | 200 000 | 50 | **20** |
| Channel | subscribers uncapped (broadcast) | **50** (a bot joins a channel only as admin) | admin slots |

Sources: [Telegram Limits](https://limits.tginfo.me/en/), [group/channel FAQ summaries](https://www.usecarly.com/blog/telegram-group-limit/). Bot API still rate-limits `sendMessage` (GSV live walk is 1/s).

### MCP vs jail

Band **160** already named the crate path `sandbox` on `GET /mcp`. A **jail** in this spec is that sandbox **plus** identity: one live `gsv-server`, one folder MCP, one `data/gsv_settings.json`, no User-scope MCP, no `update/apply` / tunnel on MCP. Application code (MDS, product bins) is built **inside** that jail in both solo and squad.

## Vocabulary

| Term | Meaning |
|------|---------|
| **Jail** | One installed `gsv-server` + its loopback MCP + git working tree. Preview/terminal/`gsv://` cannot leave the crate. `jail.id` is a public nickname (default `local`). |
| **MCP worker** | A client (Cursor / OpenCode / Grok) heartbeating `gsv_tickets_presence` against **that** jail. |
| **Godfather host** | The channel/chat + bot token bound in **one** jail’s settings. Tokens never leave that machine. |
| **Shared Godfather bot** | One BotFather bot (host). Many jails set `from=jail_id` on bus envelopes. **Preferred** when squad size > Telegram bot slots. |
| **Own bot** | Joiner creates `@BotFather` bot, adds it as **channel admin**. Consumes a Telegram admin/bot slot. |
| **Squad cap** | Max **MCP jail workers** online. Owner policy: **equals Godfather `member_count`** (channel users / group members). Hard clamp `200_000` (supergroup ceiling) so presence stays bounded. |
| **Bot slot cap** | Max **BotFather bots** on the chat: 50 (channel admins) or 20 (group/supergroup bots). Independent of squad cap. |
| **host** | This jail’s bot is `creator`/`administrator` on the Godfather chat (or owner override). Poll, post, hook GitHub, run squad. |
| **mate** | Channel **member**, not admin. Heartbeat + claim. Do not share the host token. Shared bot uses `from=jail_id`. |
| **guest** | Not a member yet. Local work stays **solo**. Live bus send is refused. GitHub origin update still applies. |
| **local** | No channel bound. Same-machine Cursor+OpenCode squad still allowed. |

## How a joiner connects MCP to a squad

**Do not** point Cursor/OpenCode at someone else’s `/mcp` over the internet. That exports the host jail (preview, terminal allowlist, tickets). Tunnel stays owner CLI opt-in (`cargo xtask tunnel`), never an MCP tool.

### Path A — you already have a bot admin on a channel (host Godfather)

1. Joiner installs GSV, `cargo xtask live` on **their** loopback `:9999`.
2. Joiner wires **folder** MCP to `http://127.0.0.1:9999/mcp` (Cursor) or `target/live/gsv-mcp.exe` (OpenCode/Grok). Never User MCP.
3. Environment check (`GET /api/tickets` → `env`, also inside MCP `gsv_tickets`): `crate_version`, `sandbox` is **their** path, `token_set` / `channel_set`, `squad_full`, `loopback_mcp`.
4. Joiner is added as a **human** on the channel (this occupies a member slot → raises `member_count` → raises squad cap).
5. Their MCP does **not** use the host `bot_token`. They heartbeat presence on **their** jail for local Cursor+OpenCode, and talk to the host squad over Godfather bus `{v:1,kind:bus,from:<jail.id>}`. Federated presence on the **host** board is `kind:presence` (band **193**): remote jails show under `federation`; they do **not** consume this jail’s `squad_cap`. Federated **claim** is `kind:claim` (band **194**): a remote jail claims an *open* row on **this** jail’s `tickets.jsonl`; the remote still keeps its own JSONL. Federated **done** is `kind:done` (band **195**): the jail that finished a claimed row posts the close and the host board transitions it `in_progress` → `done`; ranks stay process-local, so remote dones never move the host merit ladder. Federated **reclaim** is `kind:reclaim` (band **196**): lease expiry or an explicit release posts the reclaim and peer boards transition their copy back to `open` (+ `kind:reclaimed`) — the federation lifecycle (presence → claim → done → reclaim) is complete. Guest mute. Echo of this jail is ignored.
6. Optional: host adds the joiner’s bot as a second admin (consumes `bot_slot_cap`). Skip this when using the shared host bot.

### Path B — joiner wants their own channel and bot, then invites others

1. Create a Telegram **channel** (or group). BotFather bot → **admin**. Store token only in **that** jail’s `data/gsv_settings.json` (or `GSV_TELEGRAM_BOT_TOKEN`). Enable `ticket-claim`, `telegram-relay`, `ticket-squad`.
2. Set `tickets.chat_kind` (`channel` / `group` / `supergroup`) and `tickets.member_count` (Settings card or POST). `squad_cap` defaults to `member_count`.
3. Invite people (member slots). Each invited human may run a GSV jail. Extra MCP workers beyond `bot_slot_cap` **share this Godfather bot** (`from=jail_id`).
4. Git: each jail clones `origin`, works a ticket branch, opens a PR. Host (or CODEOWNERS) merges sequentially. One commit per jail drain; no mid-drain push.

### Environment checklist (fail closed)

| Check | Pass |
|-------|------|
| Disk | `gsv_disk` / health `disk_ok` (S0; `<12 GiB` → `disk --clean` keeps live) |
| Live crate | `gsv_update` `version_lag=false`; recopy `target/live/` after bump |
| MCP catalog | `catalog_stale=false`; Cursor may need a **full restart** (agent refresh does not re-list) |
| Sandbox | `GET /mcp` `sandbox` is the joiner’s crate, not the host’s |
| Secrets | `token_set` true on **their** settings; wire never contains `bot_token` |
| Channel | `channel_set`; joiners on the allowlist if `allowed_user_ids` is non-empty |
| Squad room | `online < squad_cap`; heartbeat `accepted` |
| Transport | MCP URL is loopback (or owner-opt-in tunnel, documented risk) |

## Security check (band 186)

| Finding | Severity | Rule |
|---------|----------|------|
| Remote MCP join | **High** if misused | Default: each jail’s MCP is loopback-only. Do not publish `/mcp`. Tunnel is CLI-only. |
| Shared `bot_token` | **High** | Never paste tokens in Godfather, tickets.jsonl, or git. Each host jail stores its own secret. |
| User-scope Cursor MCP | **High** | Folder GSV only (band 160). User MCP leaks into PoolAI windows. |
| Presence is process-local | **Medium** | Two machines do not share `PresenceStore`. Federated squad uses Telegram bus + git, not a shared HashMap. On one jail, **all** heartbeat paths (`presence` / `next` / claim / walk) use `heartbeat_capped`. |
| `tickets.jsonl` git merge | **Medium** | Host board is canonical. Federated clones must not blindly merge peer JSONL. |
| POST `/mcp` skips browser CSRF | **Low** (documented) | Bots are not Galaxy UI; body cap still applies. Other POSTs stay CSRF+loopback. |
| Catalog freeze | **Low** (ops) | `catalog_stale` → restart Cursor before claiming squad work. |
| Jail escape via preview | **Low** (mitigated) | `preview::resolve` + `gsv://` allowlist; no `../poolAI`. |
| App build in jail | Policy | Product code, tests, benches stay in **that** crate. Squad coordinates via tickets + git PRs, not by writing into another jail’s tree. |

## Settings schema (landed 186)

`data/gsv_settings.json` (gitignored):

```json
{
  "jail": { "id": "alice-gsv" },
  "tickets": {
    "mode": "squad",
    "lease_secs": 300,
    "squad_cap": 0,
    "member_count": 12,
    "chat_kind": "channel"
  }
}
```

- `jail.id` empty → wire `local`.
- `tickets.squad_cap` `0` → use `member_count` if set, else `bot_slot_cap` (50 channel / 20 group).
- `member_count` is filled from Telegram `getChatMemberCount` on live bind/poll (band **187**). Owner POST still works; a failed probe does not wipe a stored count. Dry-run stub is `3` and does not persist.
- Redacted GET/MCP includes these fields; never `bot_token`.

Presence: `POST /api/tickets/presence` / `gsv_tickets_presence` / `gsv_tickets_next` / claim / walk all use `heartbeat_capped`. A **new** worker is refused when `online >= squad_cap`. Renewing an existing heartbeat still works.

`GET /api/tickets` adds `jail_id`, `squad_cap`, `bot_slot_cap`, `member_count`, `chat_kind`, `env` (join checklist), `federation` (remote `kind:presence` rows). MCP `gsv_tickets` is the same wire (**56** tools, **13** `gsv://` resources).

## Scenarios

Catalog ids: `federated-join` · `own-channel` · `jail-app` · `federated-presence` · `federated-claim` · `federated-done` · `federated-reclaim` (plus existing `squad-dev` / `telegram-solo` / `abrakadabra-session` / `rank-ladder`).

## Non-goals (186–196)

- Public `/mcp` mesh / Cloudflare on MCP.
- Sharing one `bot_token` as a “join code”.
- Raising Telegram’s own 20/50 bot-admin ceilings.
- Auto-assign (`try_dispatch`) to remote federation workers — still process-local.
- Remote dones moving the host rank ladder — ranks stay process-local (band 195).
- Remote reclaims moving the host rank ladder — a `kind:reclaimed` release is rank-free too (band 196).

## Success metrics

- Settings round-trip jail id + squad_cap/member_count/chat_kind; wire has no `bot_token`.
- Presence rejects an extra distinct worker when cap is 1; the holder still renews.
- Tickets/Galaxy/MCP show jail + `online/squad_cap` + env hint.
- Scenarios `federated-join` / `own-channel` / `jail-app` / `federated-reclaim` exist.
- Resource `gsv://docs/solo-squad-jail` reads this file.
- `--stretch-96` ≥ 96%; fmt/clippy/test green.

## See also

- Co-workflows: `drain` · `ticket-claim` · `telegram-relay` · `ticket-squad`
- Always-on live copy: [`GSV_POST_ALWAYS_ON.md`](./GSV_POST_ALWAYS_ON.md)
- Localhost hardening: bands 133–134, 160
