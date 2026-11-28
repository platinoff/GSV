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

## Honestly blocked (note added by MCP error)
PH-1206 (permanently owner-gated, PH-A5 closed), G19 (no G17 candidate), WSP-G1/WSP-3 (owner go, 55% hybrid only), MC-1/MC-5 (depend on PH-1206), 6 llama-rs ggml-rpc swarm rows (superseded by GSV WebGPU mesh), ECO-8.2 remaining (MTP GGML_ASSERT eh_proj).

| id | verdict | evidence |
|---|---|---|
| t-1789361515100705500 (ECO-8.2 0.1.156) | blocked (already) | Superseded by t-1794211220000000000 (single target 0.1.161). MCP cannot transition blocked → done; recorded here, status unchanged. |

## Re-scoped in docs, left open
PH-1202/PH-1203 (`[ ]` genuinely open), PH-1204/PH-1205 (`[~]`: plumbing landed, remaining = trained alignment + real image bytes + mtmd parity).

## Unchanged and consistent
ECO-4, token rotate, group reply, P6.2, ECO-8.4, ECO-7.7, RB-63..66 cancelled, ECO-7.1/7.6 skipped, poolai dep queue.
