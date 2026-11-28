# Ticket-board logic reconciliation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring the 40 active GSV board rows into agreement with owner decisions / HANDOFF / roadmap, so every row is either actionable, honestly blocked, or closed with evidence — no stale, duplicated, or procedural rows remain in the active set.

**Architecture:** Truth order is **owner decision > HANDOFF/roadmap > plan > ticket body**. Board transitions go through `gsv_tickets_*` MCP only (`claim` → evidence → `done`/`error`); doc re-scopes go through one commit per product repo. Owner-gated rows keep their gate verbatim and are never executed implicitly. A re-scoped row is closed with a note that points at the new authoritative doc, not silently rewritten in `tickets.jsonl`.

**Tech Stack:** GSV MCP (`gsv_tickets`, `gsv_tickets_claim`, `gsv_tickets_done`, `gsv_tickets_error`), `docs/gsv/tickets.jsonl`, `docs/gsv/GSV_TECH_ROADMAP.md`, `rusty-engine/docs/roadmap.md`, MSYS2 bash + `cargo xtask`.

**Spec:** `docs/HANDOFF_NEW_SESSION.md` (owner steady state, PH-A5 permanently deferred), `docs/gsv/GSV_TECH_ROADMAP.md` (BENCH-2 / G13 / G19 rows), `docs/superpowers/plans/2026-11-21-ecosystem-next-tasks.md` (procedural-ticket rule, N12/O7), `docs/superpowers/plans/2026-11-15-minicpm-mesh-wsp.md` (PH-404 / PH-1206), `docs/gsv/research/VERSUS_WORKING_MODEL_PLAN_2026-11-27.md` (A1 expert-range loader), `docs/gsv/research/ECO4_READONLY_AUDIT_2026-11-27.md`.

## Global Constraints

- Shell: **only** `C:\msys64\usr\bin\bash.exe -lc '…'`. Never PowerShell.
- Env (MSYS2): `export PATH="/c/Users/plati/.cargo/bin:$HOME/.cargo/bin:/ucrt64/bin:/usr/bin:$PATH"`; `unset RUSTUP_TOOLCHAIN`; `cd /s/rust/GSV`; `unset CARGO_TARGET_DIR`.
- **Never** kill `target/live/gsv-server.exe`; do not start a second `cargo` on the same `target/`.
- No `git add -A`; stage only files named in a task. Never stage `data/*` (except `.gitkeep`), `.env*`, `*.pem`/`*.key`, weights, rebook workspace.
- One meaningful commit **per product repo**; GSV docs commit and rusty-engine docs commit are separate. Never mix products in one commit. Push + short summary as the last step of each repo's task.
- Board status fields change **only** via MCP tools; never hand-edit `status` in `docs/gsv/tickets.jsonl`.
- Owner-gated items are recorded, never executed: G13, G19, PH-1206, ECO-8.4 (`:8091`), ECO-7.7 (rebook), Telenetis token rotate, WSP Phase 2 go, plugin remotes, rebook RB-* claims.
- Evidence note format: `VERDICT/CANON: <command or doc path> → <result>`. No vague notes.
- Rust-first ratio 95–100% / wasm 0–5% unchanged; no Python, no npm/Node, no new file formats.

---

### Task 1: Freeze the evidence baseline

**Files:**
- Create: `docs/gsv/reports/TICKET_BOARD_LOGIC_RECONCILIATION_2026-11-27.md`
- Modify: `docs/superpowers/plans/2026-11-21-ecosystem-next-tasks.md` (prepend a superseded banner)

**Interfaces:**
- Consumes: live board snapshot from `gsv_tickets`.
- Produces: the evidence table that Tasks 2–6 cite by ticket id.

- [ ] **Step 1: Capture the baseline**

Run in Code Mode / MCP:

```js
const r = await tools.gsv_mcp_openbot.gsv_tickets({});
const active = r.tickets.filter(t => t.status !== "done");
const byStatus = {}, byProduct = {};
for (const t of active) { byStatus[t.status] = (byStatus[t.status]||0)+1; byProduct[t.product] = (byProduct[t.product]||0)+1; }
return { total: r.tickets.length, active: active.length, byStatus, byProduct };
```

Expected: `total: 984`, `active: 40`, `byStatus: {open:25, blocked:9, cancelled:4, skipped:2}`, `byProduct: {gsv:11, rusty-engine:8, llama-rs:8, rebook:5, poolai:4, telenetis:3, omniroute:1}`.

- [ ] **Step 2: Write the report**

Create `docs/gsv/reports/TICKET_BOARD_LOGIC_RECONCILIATION_2026-11-27.md` with these exact sections and rows (no placeholders):

```markdown
# Ticket-board logic reconciliation (2026-11-27)

Baseline: 984 rows / 40 active (25 open, 9 blocked, 4 cancelled, 2 skipped).
Products: gsv 11, rusty-engine 8, llama-rs 8, rebook 5, poolai 4, telenetis 3, omniroute 1.
Truth order: owner decision > HANDOFF/roadmap > plan > ticket body.

## Close with evidence
| id | verdict | evidence |
|---|---|---|
| t-1795273000000000013 (G11) | done | g-711/g-712 PASS 6.94e-18; lease reclaimed before scorecard — finalize here, do not re-run |
| t-1789742434759689700 (telenetis webhook) | done | owner 2026-11-18 deleteWebhook → getUpdates; GSV poller alive (HANDOFF "Mesh cycle-watch") |
| t-1793103116713530800, t-1793103116757230900, t-1793103116846827100, t-1793103116892891100 (AGI) | done | procedural scaffold, not product backlog (plan 2026-11-21 §Open tickets) |
| t-1795700063552630000 (BENCH-2) | done-rescoped | #44 withdrawn; live #43+#39; partial gate 130.98 / up 187.06 MFLOPS, 3.47e-18; down environmental |
| t-1791999083385121400 (PH-404) | done-superseded | PH-404 `[x]` + PH-404a 468a6f2 next-use/Belady; NVMe = PH-406 owner-gated |
| t-1789361515100705500 (ECO-8.2 0.1.156) | done-merged | duplicate of t-1794211220000000000; single target = 0.1.161, MTP eh_proj blocker |

## Honestly blocked (note added by MCP error)
PH-1206 (permanently owner-gated, PH-A5 closed), G19 (no G17 candidate), WSP-G1/WSP-3 (owner go, 55% hybrid only), MC-1/MC-5 (depend on PH-1206), 6 llama-rs ggml-rpc swarm rows (superseded by GSV WebGPU mesh), ECO-8.2 remaining (MTP GGML_ASSERT eh_proj).

## Re-scoped in docs, left open
PH-1202/PH-1203 (`[ ]` genuinely open), PH-1204/PH-1205 (`[~]`: plumbing landed, remaining = trained alignment + real image bytes + mtmd parity).

## Unchanged and consistent
ECO-4, token rotate, group reply, P6.2, ECO-8.4, ECO-7.7, RB-63..66 cancelled, ECO-7.1/7.6 skipped, poolai dep queue.
```

- [ ] **Step 3: Banner the old plan**

Prepend to `docs/superpowers/plans/2026-11-21-ecosystem-next-tasks.md`:

```markdown
> **SUPERSEDED 2026-11-27:** rows N12, N15 and O7 below are reconciled by
> `docs/superpowers/plans/2026-11-27-ticket-board-logic-reconciliation.md`
> and `docs/gsv/reports/TICKET_BOARD_LOGIC_RECONCILIATION_2026-11-27.md`.
> The AGI scaffold rows `t-17931031167*` are procedural, not product backlog.
```

- [ ] **Step 4: Verify + commit**

Run:

```bash
git diff --stat
git add docs/superpowers/plans/2026-11-21-ecosystem-next-tasks.md docs/gsv/reports/TICKET_BOARD_LOGIC_RECONCILIATION_2026-11-27.md
git commit -m "docs: freeze ticket-board logic reconciliation baseline" -m "Gsv-Actor: agent" -m "Gsv-Ide: board-truth-order" -m "Gsv-Model: opencode"
git push
```

Expected: one GSV commit, clean tree for the staged files, `git status -sb` shows no staged leftovers.

---

### Task 2: Close resolved and procedural rows

**Files:**
- Modify: `docs/gsv/tickets.jsonl` (via MCP only)

**Interfaces:**
- Consumes: ticket ids from Task 1 report.
- Produces: 11 fewer active rows; events carry the evidence notes.

- [ ] **Step 1: Close G11 with finalized scorecard**

```js
await tools.gsv_mcp_openbot.gsv_tickets_claim({ id: "t-1795273000000000013" });
await tools.gsv_mcp_openbot.gsv_tickets_done({
  id: "t-1795273000000000013",
  note: "VERDICT: g-711/g-712 PASS max_err_model=6.94e-18 (<1e-4) on blk.3/blk.11.attn_output. Ticket was reclaimed on lease expiry 2026-11-22; scorecard finalized in docs/gsv/reports/TICKET_BOARD_LOGIC_RECONCILIATION_2026-11-27.md. Do NOT re-run canaries; G15 references this close."
});
```

Expected: `ok:true`, status `done`, event note present.

- [ ] **Step 2: Close the Telenetis webhook conflict**

```js
await tools.gsv_mcp_openbot.gsv_tickets_claim({ id: "t-1789742434759689700" });
await tools.gsv_mcp_openbot.gsv_tickets_done({
  id: "t-1789742434759689700",
  note: "RESOLVED: owner 2026-11-18 approved deleteWebhook on @GsvOfficialBot → bot on getUpdates; GSV poller (poll=true) inbound alive again. Evidence: docs/HANDOFF_NEW_SESSION.md row 'Mesh cycle-watch-install 2026-11-19'. Group-ownership follow-up stays in t-1789606667836911200."
});
```

- [ ] **Step 3: Close the four AGI procedural scaffolds**

For each of `t-1793103116713530800`, `t-1793103116757230900`, `t-1793103116846827100`, `t-1793103116892891100`:

```js
await tools.gsv_mcp_openbot.gsv_tickets_claim({ id });
await tools.gsv_mcp_openbot.gsv_tickets_done({
  id,
  note: "PROCEDURAL: scenario scaffold from agi-session, not product backlog (docs/superpowers/plans/2026-11-21-ecosystem-next-tasks.md §Open tickets). Real work is tracked by product tickets; do not re-open as a band."
});
```

- [ ] **Step 4: Verify**

```js
const r = await tools.gsv_mcp_openbot.gsv_tickets({});
return r.tickets.filter(t => ["t-1795273000000000013","t-1789742434759689700","t-1793103116713530800","t-1793103116757230900","t-1793103116846827100","t-1793103116892891100"].includes(t.id)).map(t => [t.id, t.status]);
```

Expected: all six `done`.

---

### Task 3: Re-scope stale GSV rows (BENCH-2, WSP-G1, G19, ECO-8.2 duplicate)

**Files:**
- Modify: `docs/gsv/tickets.jsonl` (via MCP only)
- Modify: `docs/gsv/GSV_TECH_ROADMAP.md:2398` (BENCH-2 row)

**Interfaces:**
- Consumes: Task 1 report.
- Produces: one authoritative target for ECO-8.2; BENCH-2 truthful about fleet.

- [ ] **Step 1: Close BENCH-2 as re-scoped**

```js
await tools.gsv_mcp_openbot.gsv_tickets_claim({ id: "t-1795700063552630000" });
await tools.gsv_mcp_openbot.gsv_tickets_done({
  id: "t-1795700063552630000",
  note: "RESCOPED: the 3-device definition (#43/#39/#44) is retired — #44 withdrawn by owner, live fleet #43+#39. Already measured: gate g-7363 130.98 MFLOPS, up g-7364 187.06 MFLOPS, max_err 3.47e-18; down blocked by peer offline (environmental, not logic). New scope lives in docs/gsv/GSV_TECH_ROADMAP.md addendum; repeat only when both peers stable."
});
```

- [ ] **Step 2: Replace the roadmap BENCH-2 row**

Edit `docs/gsv/GSV_TECH_ROADMAP.md`, replace line 2398:

```markdown
| BENCH-2 | rescoped 2026-11-27 — sweep shards 2/4/8 × gate/up/down on **#43 + #39 only** (#44 withdrawn by owner); partial result gate 130.98 / up 187.06 MFLOPS, max_err 3.47e-18; down pending stable peers. Old 3-device ticket t-1795700063552630000 closed with that evidence. |
```

- [ ] **Step 3: Mark WSP-G1 owner-gated**

```js
await tools.gsv_mcp_openbot.gsv_tickets_claim({ id: "t-1794500787830346100" });
await tools.gsv_mcp_openbot.gsv_tickets_error({
  id: "t-1794500787830346100",
  note: "OWNER-GATED: WSP Phase 2 requires explicit go. ECO-9.1 verdict = 20–25% direct RL→WSP, 55% hybrid only; MC-6 CLOSED with ROLLBACK; patch-canary exhausted single-direction on MiniCPM-1B. Bridge may be wired only with owner go or as part of A1 expert-range loader (VERSUS_WORKING_MODEL_PLAN_2026-11-27)."
});
```

- [ ] **Step 4: Mark G19 and WSP-3 owner-gated**

```js
await tools.gsv_mcp_openbot.gsv_tickets_claim({ id: "t-1795349626351586200" });
await tools.gsv_mcp_openbot.gsv_tickets_error({
  id: "t-1795349626351586200",
  note: "NOT STARTED: no safe candidate per G17; roadmap says 'G19 Open, no safe patch candidate'. Owner-gated. Do not mint a patch until G17 selects an obstacle-gap candidate."
});
await tools.gsv_mcp_openbot.gsv_tickets_claim({ id: "t-1794500787998945000" });
await tools.gsv_mcp_openbot.gsv_tickets_error({
  id: "t-1794500787998945000",
  note: "OWNER-GATED: WSP Phase 2 needs explicit go. ECO-9.1 verdict = 20–25% direct RL→WSP, 55% hybrid only; MC-6 ROLLBACK and the patch-canary exhausted single-direction on MiniCPM-1B. Cloud-oracle is optional, not a blocker. Do not start cloud delta-patch pack without owner go."
});
```

- [ ] **Step 5: Merge the ECO-8.2 duplicate (corrected 2026-11-27)**

**MCP surface fact:** `gsv_tickets_claim` only transitions `open → in_progress`. There is no tool for `blocked → open`, so a ticket that is already `blocked` cannot be marked `done`. `t-1789361515100705500` (0.1.156) is **already blocked**, so it stays blocked and the merge is recorded in the report instead. Do not hand-edit `status` (Global Constraints).

```js
await tools.gsv_mcp_openbot.gsv_tickets_claim({ id: "t-1794211220000000000" });
await tools.gsv_mcp_openbot.gsv_tickets_error({
  id: "t-1794211220000000000",
  note: "OWNER-GATED / SINGLE TARGET: 0.1.154 → 0.1.161 lockstep with llama-cpp-2; re-verify 6 Windows-GNU patches + MTP common/log.cpp + vendored ggml-rpc f5b9bd39. Blocker = MTP-draft GGML_ASSERT eh_proj (owner decision 2026-11-27). Duplicate 0.1.156 row t-1789361515100705500 remains blocked and is superseded by this row; do not work both. One band only — never two cargo on one target."
});
```

Also edit `docs/gsv/reports/TICKET_BOARD_LOGIC_RECONCILIATION_2026-11-27.md`: move the ECO-8.2 row out of `Close with evidence` into `Honestly blocked`, with the line:

```markdown
| t-1789361515100705500 (ECO-8.2 0.1.156) | blocked (already) | Superseded by t-1794211220000000000 (single target 0.1.161). MCP cannot transition blocked → done; recorded here, status unchanged. |
```

- [ ] **Step 6: Verify + commit GSV docs**

```js
const r = await tools.gsv_mcp_openbot.gsv_tickets({});
return ["t-1795700063552630000","t-1789361515100705500","t-1794500787830346100","t-1795349626351586200","t-1794500787998945000","t-1794211220000000000"].map(id => [id, r.tickets.find(t => t.id === id).status]);
```

Expected: `done, blocked, blocked, blocked, blocked, blocked` — the 0.1.156 duplicate stays `blocked` (MCP has no `blocked → done` transition).

```bash
git add docs/gsv/tickets.jsonl docs/gsv/GSV_TECH_ROADMAP.md docs/gsv/reports/TICKET_BOARD_LOGIC_RECONCILIATION_2026-11-27.md docs/superpowers/plans/2026-11-27-ticket-board-logic-reconciliation.md
git commit -m "docs: reconcile board rows with canon and rescope BENCH-2" -m "Gsv-Actor: agent" -m "Gsv-Ide: board-truth-order" -m "Gsv-Model: opencode"
git push
```

`docs/gsv/tickets.jsonl` is included because it already holds the uncommitted Task 2 closes plus Task 3 changes; the plan file is untracked and must be preserved in git.

---

### Task 4: Re-scope rusty-engine vision and MoE rows in the product roadmap

**Files:**
- Modify: `S:/rust/rusty-engine/docs/roadmap.md` (lines 293–297)

**Interfaces:**
- Consumes: Task 1 report; `rusty-engine/docs/HANDOFF_NEW_SESSION.md` (plumbing commits `61bb0ba`, `62550df`).
- Produces: truthful `[ ]` vs `[~]` statuses that the board notes point to.

- [ ] **Step 1: Edit the B12 table rows**

In `S:/rust/rusty-engine/docs/roadmap.md`, replace the four status cells and append the clarifying clause to PH-1204/PH-1205:

```markdown
| PH-1202 | **vision tower у Rust** ... | `[ ]` | ...FileSystem: `src/vit.rs` + unit + `#[ignore]` parity — genuinely open, no plumbing yet |
| PH-1203 | **`v.vit_merger` ...** | `[ ]` | ... genuinely open, no plumbing yet |
| PH-1204 | **image-token prefix у prefill** — plumbing landed 2026-11-24 (`61bb0ba`), `[~]` | `[~]` | ... **Remaining scope (2026-11-27): trained alignment of 1024-dim vision vector to MiniCPM embedding space + real image bytes from GSV `vision-chat` + mtmd greedy parity. Text-only PH-405 golden must stay green. No image→answer claim until G13.** |
| PH-1205 | **serve мультимодальний** — wire landed `62550df`, `[~]` | `[~]` | ... **Remaining scope (2026-11-27): real image bytes encode, E2E mtmd parity, provider-row `rusty-vision`, trained alignment. Re-scoped, not re-implemented.** |
| PH-1206 | **MiniCPM-V 4.7-35B-A3B** through `ExpertCache` ... | `[ ]` **PERMANENTLY OWNER-GATED** | ... PH-A5 dedicated ≥24 GB node closed forever by owner 2026-11-17; this row is a recorded gate, not scheduled work. A1 expert-range loader (no node) is the only executable lane. |
```

- [ ] **Step 2: Add the PH-404 reconciliation note**

Append under the B4 section (near the PH-404/PH-404b rows):

```markdown
> **2026-11-27 reconciliation:** PH-404 expert-cache LRU + byte-budget + pin + prefetch is `[x]`; PH-404a `468a6f2` added next-use/Belady eviction. NVMe offload belongs to PH-406 and is owner-gated on a node that will not exist. Board ticket `t-1791999083385121400` closed as superseded — do not re-open.
```

- [ ] **Step 2b: Close the PH-404 board ticket (GSV MCP)**

`gsv_tickets_claim({id:"t-1791999083385121400"})` then `gsv_tickets_done` with:

```js
note: "SUPERSEDED: rusty-engine roadmap PH-404 is [x] (expert-cache LRU + byte-budget + pin + prefetch) and PH-404a 468a6f2 added next-use/Belady eviction. NVMe offload belongs to PH-406, owner-gated on the permanently deferred PH-A5 node. No further work under this id."
```

- [ ] **Step 3: Gate + commit in rusty-engine (separate repo)**

```bash
export PATH="/c/Users/plati/.cargo/bin:$HOME/.cargo/bin:/ucrt64/bin:/usr/bin:$PATH"
unset RUSTUP_TOOLCHAIN
cd /s/rust/rusty-engine
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
git add docs/roadmap.md
git commit -m "docs: reconcile PH-1204/1205 scope and PH-404/PH-1206 gates" -m "Gsv-Actor: agent" -m "Gsv-Ide: board-truth-order" -m "Gsv-Model: opencode"
git push
```

Expected: fmt/clippy/test exit 0; one rusty-engine commit; no GSV files staged in this repo.

---

### Task 5: Block superseded llama-rs swarm rows and re-scope MC-1/MC-5

**Files:**
- Modify: `docs/gsv/tickets.jsonl` (via MCP only)
- Modify: `docs/gsv/reports/TICKET_BOARD_LOGIC_RECONCILIATION_2026-11-27.md` (append the dependency chain)

**Interfaces:**
- Consumes: Task 1 report; HANDOFF «next tasks» item 5 (PH-A5 deferred forever).
- Produces: honest blocked reasons; no dead dependency chains left open.

- [ ] **Step 1: Record the 6 ggml-rpc swarm rows as superseded (no status change)**

All six ids are **already `blocked`** in the baseline, and MCP has no `blocked → in_progress` transition. Do NOT call `claim` on them — it fails. Record the reason in the report only:

```markdown
| t-1789331514784733600, t-1789331514816905400, t-1789331514847571700, t-1789331774800895100, t-1789334171870418600, t-1789411991572414400 | blocked (already) | ARCHITECTURALLY SUPERSEDED: ggml-rpc fan-out (Pi f5b9bd39 / A54 Termux / browser tensor peer) replaced by GSV WebGPU mesh at :9999 with peer proofs. Owner closed the node list 2026-11-17 (PH-A5 permanently deferred). Keep as blocked history; do not spend Pi/A54 on rpc. |
```

Add that line to the `Honestly blocked` section of `docs/gsv/reports/TICKET_BOARD_LOGIC_RECONCILIATION_2026-11-27.md`.

- [ ] **Step 2: Block PH-1206, MC-1 and MC-5 with the dead dependency named**

```js
const chain = [
  ["t-1792860702696858700", "PERMANENTLY OWNER-GATED: needs dedicated ≥24 GB node (PH-A5 closed forever by owner 2026-11-17). Only executable lane = A1 expert-range loader in docs/gsv/research/VERSUS_WORKING_MODEL_PLAN_2026-11-27.md."],
  ["t-1794758435872050000", "BLOCKED on PH-1206: real top-k=2 expert dispatch belongs to 35B-A3B (MiniCPM-1B is dense). MC-2 and WSP-G1-rusty are done, but the 35B node does not exist. Re-baseline on GSV streaming 4-bit MoE (moe-forward --pack) or keep blocked."],
  ["t-1794758436045763000", "BLOCKED on MC-1: hybrid prefill/decode split cannot be benchmarked until a real expert dispatch exists. No fallback measurement without an MoE target."]
];
for (const [id, note] of chain) {
  await tools.gsv_mcp_openbot.gsv_tickets_claim({ id });
  await tools.gsv_mcp_openbot.gsv_tickets_error({ id, note });
}
```

- [ ] **Step 3: Append the chain to the report**

Append to `docs/gsv/reports/TICKET_BOARD_LOGIC_RECONCILIATION_2026-11-27.md`:

```markdown
## Dead dependency chain (named, not hidden)
PH-A5 closed → PH-406 and PH-1206 permanently owner-gated → MC-1 (needs 35B top-k=2) → MC-5 (needs MC-1).
A1 expert-range loader (no node) is the only executable lane; GSV `moe-forward --pack` is the numerics baseline.
```

- [ ] **Step 4: Verify**

```js
const r = await tools.gsv_mcp_openbot.gsv_tickets({});
const ids = ["t-1789331514784733600","t-1789331514816905400","t-1789331514847571700","t-1789331774800895100","t-1789334171870418600","t-1789411991572414400","t-1792860702696858700","t-1794758435872050000","t-1794758436045763000"];
return ids.map(id => [id, r.tickets.find(t => t.id === id).status]);
```

Expected: all `blocked` (the six rpc rows were already blocked; only PH-1206/MC-1/MC-5 changed from `open` to `blocked`).

- [ ] **Step 5: Commit the report update**

```bash
git add docs/gsv/tickets.jsonl docs/gsv/reports/TICKET_BOARD_LOGIC_RECONCILIATION_2026-11-27.md
git commit -m "docs: name the dead PH-A5 dependency chain on the board" -m "Gsv-Actor: agent" -m "Gsv-Ide: board-truth-order" -m "Gsv-Model: opencode"
git push
```

---

### Task 6: Final gate — board and docs consistency

**Files:**
- Read-only verification; no new files.

**Interfaces:**
- Consumes: Tasks 1–5 outputs.
- Produces: the "definition of done" numbers quoted in the session summary.

- [ ] **Step 1: Re-count the active board**

```js
const r = await tools.gsv_mcp_openbot.gsv_tickets({});
const active = r.tickets.filter(t => t.status !== "done");
const byStatus = {}, byProduct = {};
for (const t of active) { byStatus[t.status] = (byStatus[t.status]||0)+1; byProduct[t.product] = (byProduct[t.product]||0)+1; }
return { total: r.tickets.length, active: active.length, byStatus, byProduct };
```

Expected: `active: 32` (was 40), `byStatus: {open:10, blocked:16, cancelled:4, skipped:2}`. Arithmetic: `open` starts at 25. Eight become `done` (G11, webhook, 4 AGI, BENCH-2, PH-404) → −8. Seven become `blocked` (WSP-G1, WSP-3, G19, ECO-8.2 remaining, PH-1206, MC-1, MC-5) → −7. So `open` = 25 − 8 − 7 = 10 and `blocked` = 9 + 7 = 16. The already-blocked ECO-8.2 0.1.156 and the six rpc rows are unchanged. Active = 10 + 16 + 4 + 2 = 32. Remaining open rows are exactly: ECO-4, Telenetis token rotate, Telenetis group reply, P6.2, ECO-8.4, ECO-7.7, PH-1202, PH-1203, PH-1204, PH-1205 (10). All other blocked/cancelled/skipped rows are the unchanged consistent rows listed in the report.

- [ ] **Step 2: Confirm no owner-gated row is `in_progress`**

```js
const r = await tools.gsv_mcp_openbot.gsv_tickets({});
return r.tickets.filter(t => t.status === "in_progress").map(t => [t.id, t.title]);
```

Expected: `[]` — every claim above was followed by `done` or `error`, no leaked lease.

- [ ] **Step 3: GSV gates**

```bash
cd /s/rust/GSV
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo run --bin gsv-loc-audit -- --stretch-96
cargo xtask sync
git status -sb
```

Expected: exit 0 everywhere; `sync` reports no drift; ratio ≥ 99%; clean tree except intended local untracked logs.

- [ ] **Step 4: Summary**

Report in chat: `product · ticket ids closed/blocked · HEAD per repo · gates · next open ticket`. Push is the last action of each repo task and must already be green before this summary.

---

## Self-review (spec coverage)

| Spec requirement | Task |
|---|---|
| G11 finalized, not re-run | Task 2 Step 1 |
| Telenetis webhook resolved by owner | Task 2 Step 2 |
| AGI procedural rows not backlog | Task 1 Step 3, Task 2 Step 3 |
| BENCH-2 truthful fleet | Task 3 Steps 1–2 |
| ECO-8.2 single target (0.1.156 stays blocked — MCP has no blocked→done) | Task 3 Step 5 |
| PH-404 board ticket closed as superseded | Task 4 Step 2b |
| PH-1206 permanently gated | Task 4 Steps 1–2, Task 5 Step 2 |
| PH-1204/1205 `[~]` alignment gap | Task 4 Step 1 |
| llama-rs rpc superseded | Task 5 Step 1 |
| Owner gates untouched | Global Constraints + Task 3/5 notes |
| One commit per repo, push, gates | Tasks 1/3/4 + Task 6 |

No placeholders: every step names exact ids, paths, commands, and expected results.
