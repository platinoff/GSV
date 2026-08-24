# Flow, Sync & Multi-Agent Lessons & Logic Flows (ORR_DESKTOP & GSV Audit)

## 1. ORR_DESKTOP Workflow Mistake Analysis & Count

During the creation and development of `orr_desktop` (pure-Rust native recording pipeline), several workflow and synchronization anti-patterns were observed and logged across session interactions:

1. **Silent Tool Invocation without Telegram/GSV Sync (Count: 14 instances):**
   - *Mistake:* Executing file edits, writes, or builds without broadcasting `kind:sync` or `kind:bus` envelopes to the Godfather channel or ticket board.
   - *Fix:* Enforce that every tool sequence that modifies project state or claims a task MUST trigger `gsv_telegram_bus_send` or `gsv_tickets_done` in lockstep.

2. **Premature Success Assertion without Verification (Count: 8 instances):**
   - *Mistake:* Declaring a feature complete or passing without running `cargo test` and `cargo xtask sync --check` first.
   - *Fix:* Enforce the `verification-before-completion` skill: evidence before assertions always. Run `cargo test` + stand smoke + sync checks before committing.

3. **Lease Expiry / Stale Reclaim Collisions in Squad Mode (Count: 5 instances):**
   - *Mistake:* Multiple agents claiming tickets without maintaining active presence heartbeats (`gsv_tickets_presence`), causing lease timeouts and redundant reclaims.
   - *Fix:* Active workers must heartbeat presence every session turn, respecting `squad_cap` (channel member count).

---

## 2. Best Logic Flows for MCP + FlowFS + Tickets + Squad + Solo & Obsidian Sync

```
[Inbound Godfather / Telegram Message]
       │
       ▼
[gsv_telegram_poll / decode] ──► [Classify Ticket / Bus / Presence]
       │
       ▼
[gsv_tickets_next] (Inbox / WIP / Open Row)
       │
       ├──► Claim: gsv_tickets_claim (leases 300s, renew on heartbeat)
       ├──► Work: Code edit / test / verify (cargo test + stand smoke)
       └──► Complete: gsv_tickets_done (+ rank award + Telegram sync message)
       │
       ▼
[Vision Sync & Obsidian Vault Mirroring]
       │
       ▼
[One Commit & Push to origin main]
```

---

## 3. Strategy Game Rules & Multi-Agent Scenario Optimization

For multi-agent bot gameplay and strategy scenarios (`abrakadabra-session`, `memory-disk-speed`, `squad_walk`):

- **Turn Loop & State Transitions:**
  1. *Presence Phase:* Workers heartbeat presence and verify `squad_cap`.
  2. *Claim Phase:* Workers poll `gsv_tickets_next` and claim open rows atomically.
  3. *Execution Phase:* Execute subtasks (disk check, clippy, test, bench).
  4. *Federated Close Phase:* Post `kind:claim`, `kind:done`, or `kind:reclaim` envelopes across peer boards.
- **Strategy Recommendations:**
  - Keep task scopes granular ($\le 10$ tickets per scenario band).
  - Use deterministic pseudo-random seed assignment (`pick_squad`) to prevent race conditions among concurrent squad agents.
  - Enforce dry-run safety for OmniRouter and Telegram bots (`X-Omni-Dry-Run: 1`, `X-Telegram-Dry-Run: 1`).
