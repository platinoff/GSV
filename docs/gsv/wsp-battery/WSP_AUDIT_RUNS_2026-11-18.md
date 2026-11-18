# WSP battery v1 — baseline audit runs (2026-11-18)

**Model:** MiniCPM-V-4_7-1B-Q4_K_M.gguf (runner35, qwen35-hybrid: 18 recurrent/GDN + 6 full-attn layers 3, 7, 11, 15, 19, 23)
**Tool:** `rusty audit -m … -p "…"` (rusty-engine `47f66ad`; ECO-9.2 phase 9.3a) — **prefill residual stats only; no weight edits; no behavioral metrics**
**Battery:** `docs/gsv/wsp-battery/battery_v1.json` (sha256 `334f9b0e700a8323380bee70adee86dd452613dca9b9150fd76ab2e018560977`)
**Evidence:** `docs/gsv/wsp-battery/audit_runs_2026-11-18.jsonl` (7 probes, all exit 0, 2–6 s each, RAM peak dropped 89→61 % during the run)

## Per-probe delta_rms_max (4 decimals) — L6 (recurrent) / L7 (full attn) / L14 (recurrent) / L15 (full attn) / L23 (full attn final)

| id | L6 | L7 | L14 | L15 | L23 | max over 24 layers |
|---|---:|---:|---:|---:|---:|---:|
| uk-fact-1 (Kyiv — passes) | 0.2100 | 0.1532 | 0.2770 | 0.2690 | 0.3486 | 0.3486 |
| uk-fact-2 (Pushkin — fails) | 0.1838 | 0.1419 | 0.2068 | 0.1660 | **0.4538** | **0.4538** |
| math-1 (17·23) | 0.1815 | 0.1470 | 0.2392 | 0.2066 | 0.3607 | 0.3607 |
| math-2 (123+456) | 0.1815 | 0.1470 | 0.2392 | 0.2066 | 0.3607 | 0.3607 |
| codegen-1 (Rust print 391) | 0.1820 | 0.1394 | 0.1904 | 0.1483 | 0.4381 | 0.4381 |
| ifeval-1 (one sentence) | 0.1896 | 0.1365 | 0.1721 | 0.1246 | 0.4124 | 0.4124 |
| loop-1 (17·23 verbose) | 0.1751 | 0.1424 | 0.1963 | 0.1550 | 0.4306 | 0.4306 |
| ua-golden-1..5 | — | — | — | — | — | pending owner golden |

## What the data supports (and what it doesn't)

- **Confirmed vs 2026-11-15 baseline:** uk-fact-2 (known failing UA factual recall) shows
  the final full-attention layer **23** with `delta_rms_max = 0.4538` — clearly above every
  other probe (next highest: codegen-1 0.4381, loop-1 0.4306, ifeval-1 0.4124, math-1/2 0.3607,
  uk-fact-1 0.3486). Same shape as the 2026-11-15 reading (0.45 vs 0.33).
- **Not established:** causation. The signal is **layer-23 full-attn output columns**, consistent with a
  "single direction / few columns" candidate à la Astra Exp.8 — but identification of *which* columns,
  and whether subtracting a direction actually unlocks the fact, stays **speculative** until:
  (a) Uk-golden battery exists (owner to supply/verify 5 fixtures), (b) an editorial budget (10/25/50
  forward passes) is run via `rusty serve` + `rusty audit` A/B, (c) regression gates pass.
- math-1 ≈ math-2 (token-near-identical prefill; deltas equal to 4 decimals in 5 probe columns) — sanity
  check ok, probe design works.
- **Loop behavior is NOT measurable via `rusty audit`** (prefill RMS only). Token-limit statistics must be
  captured via serve (`POST /v1/chat/completions` with `max_tokens` + count answers hitting the cap).
- ifeval-1 / codegen-1 included for reference, not gated.

## Next steps (owner-gated)

1. Owner supplies/verifies `ua-golden-1..5` (uk_golden placeholders) → battery sha256 recomputed →
   this directory stays the single battery source (no edits sealed without re-hash).
2. Owner picks patch target hypothesis: (a) layer-23 full-attn output columns (Astra-Exp.8 analog),
   (b) FFN MLP direction subtraction, (c) stop at audit+ExpertCache (no weight edits).
3. Budget approval: GLM 5.3 diagnostic sessions (Zen free) — expected cost ~10–25 calls per hypothesis.
4. Any patch: `patched_*.gguf` copies only, Uk-golden gate = auto-rollback, token-limit share must not rise.

## Non-goals of this slice

- No edits to the original GGUF; no in-place requantization; no serve changes.
- No claim that layer-23 = "the" cause; battery is n=7 prompts, single model, single decode path.
