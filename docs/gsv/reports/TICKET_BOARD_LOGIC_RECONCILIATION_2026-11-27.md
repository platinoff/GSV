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
| t-1789331514784733600, t-1789331514816905400, t-1789331514847571700, t-1789331774800895100, t-1789334171870418600, t-1789411991572414400 | blocked (already) | ARCHITECTURALLY SUPERSEDED: ggml-rpc fan-out (Pi f5b9bd39 / A54 Termux / browser tensor peer) replaced by GSV WebGPU mesh at :9999 with peer proofs. Owner closed the node list 2026-11-17 (PH-A5 permanently deferred). Keep as blocked history; do not spend Pi/A54 on rpc. |

## Re-scoped in docs, left open
PH-1202/PH-1203 (`[ ]` genuinely open), PH-1204/PH-1205 (`[~]`: plumbing landed, remaining = trained alignment + real image bytes + mtmd parity).

## Unchanged and consistent
ECO-4, token rotate, group reply, P6.2, ECO-8.4, ECO-7.7, RB-63..66 cancelled, ECO-7.1/7.6 skipped, poolai dep queue.

## Dead dependency chain (named, not hidden)
PH-A5 closed → PH-406 and PH-1206 permanently owner-gated → MC-1 (needs 35B top-k=2) → MC-5 (needs MC-1).
A1 expert-range loader (no node) is the only executable lane; GSV `moe-forward --pack` is the numerics baseline.

## Gate rule under keep-live (permanent for future sessions)
The hub, watchdog and `cycle-watch` daemon are **hot-live by design**. Do not kill any of them to free `target/debug/gsv-xtask.exe` for a full multi-target `cargo test` rebuild — keeping the ecosystem alive outranks rebuilding a dev binary.

Authoritative gate set in this steady state (all green on 2026-11-28, HEAD `4ca1d8f`):
- `cargo fmt --all -- --check` → 0
- `cargo clippy --all-targets -- -D warnings` → 0 warnings
- `cargo test --lib` → 609 passed / 0 failed / 5 ignored, exit 0 (canonical `target/`; does not rebuild the locked bin)
- `cargo run --bin gsv-loc-audit -- --stretch-96` → rust 99.68%
- `cargo xtask sync` → exit 0 (also verified with an isolated `CARGO_TARGET_DIR`, rev 516, next PH-S3129)

**Known environment caveat, not a regression:** the full `cargo test` that rebuilds all targets fails with `Access is denied (os error 5)` while the resident daemon holds the bin. An isolated `CARGO_TARGET_DIR` bypasses the lock for `xtask sync`, but two watchdog tests — `boxes::watchdog::tests::telenetis_debug_exe_prefers_workspace_target` and `boxes::watchdog::tests::debug_newer_telenetis_sees_workspace_debug` — hard-code the canonical workspace `target/` path and therefore fail there (607 passed, 2 failed). Never "fix" this by killing processes or by changing those tests just to make a number green. If a full all-target suite is genuinely required, the owner schedules a controlled keep-live pause; agents never improvise it.
