# GASV-G17 — Astra-style candidate audit (2026-11-22)

**Ticket:** `t-1795349626080627400`  
**Scope:** AUDIT-ONLY. No weight writes, no pack-mint, no publish. Tool: `rusty audit -m <model> -p <prompt>` (activation audit, ECO-9.2 phase 9.3a).

## 1. MiniCPM-V-4_7-1B-Q4_K_M (qwen35 hybrid dense) — candidate #1 from prior research

Two prompts through the full 24-layer audit (entry_rms/delta_rms per layer):

| Prompt | top delta_rms_max layer | value |
|---|---|---|
| UK fact: `Хто написав Гамлета?` | **23** (full attn) | **0.3251** |
| EN control: `The capital of France is` | **23** (full attn) | **0.3762** |

Other high layers: UK → 22 (0.2088), 14 (0.2068), 6 (0.1838), 15 (0.1660); EN → 14 (0.2666), 22 (0.2480), 15 (0.2365), 6 (0.1990).

**Verdict: REJECT as Astra-patch candidate.** The layer-23 peak is *higher on the unrelated English control* than on the Ukrainian "fact" prompt. This independently confirms the earlier retraction (`WSP_SESSION_2026-11-18.md` §7): the layer-23 signal is a **generic final-integration-layer artifact**, not factuality. Combined with the frozen canary result (zero all 8192 blocks of `blk.23.attn_output.weight` changed decoding but fixed no fact), this model is an **ability gap, not an obstacle gap**. Gemma-law applies: patching can remove an obstacle hiding an ability, but cannot add an ability the model lacks. No targeted patch should be attempted here.

## 2. Granite-3.1-3B-A800M-Instruct-Q4_K_M (MoE) — audit-only candidate

`rusty audit` fails on this model:

```
rusty-engine error: textgen35: geometry invalid: missing required tensor 'output.weight'
```

Cause: the audit path uses qwen35 textgen geometry (requires `output.weight`); Granite is `granitemoe` with a different output/embedding layout. **Tool limitation, not evidence about the model.** Granite stays audit-only but is blocked until a granitemoe-aware audit lane exists (separate Rust ticket, owner-gated). No patch decision can be made from this run.

## 3. Overall G17 verdict

| Candidate | Class | Decision |
|---|---|---|
| 1B qwen35 dense | ability gap (layer-23 artifact confirmed by control) | **no patch** — do not proceed to G19 on this model |
| Granite A800M MoE | audit tool incompatible (granitemoe geometry missing) | blocked, needs audit-lane extension; no patch decision |

**No obstacle-gap candidate survived G17.** Therefore G19 (staged Q4_K patch) has no green candidate and must not start. Recommended next: focus the "best 4-bit MoE in ecosystem" goal on **G20** (artifact-backed WGPU weight lane so all 733 tensors are dispatchable in Min) rather than weight patching that prior + current evidence says has no safe target. Vision/mmproj feasibility remains **G18**, owner-gated and orthogonal.

- No Perl/Python, no weight writes, no git weight blobs, no quality/parity claim. G13 remains blocked.
