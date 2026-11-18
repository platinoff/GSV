# R4 full-fleet block-op dispatch — 2026-11-18

**Scope:** enqueue `weights_matvec_block` (gate/up/down shared-expert tensors,
4 shards, x = deterministic `((k % 7) - 3) * 0.25`) for **all 40 layers** of the
35B-A3B model, routed browser-only (pi4-cpu/pc-cpu excluded by enqueue- and
claim-time guards, `7da54de` / `2dca721`).

- Start: 2026-11-18 09:35:42 · End: 2026-11-18 10:10:23 (~35 min, sequential)
- Prerequisite: 120 `ffn_{gate,up,down}_shexp.weight` tensors registered in the weight
  store (verified before dispatch: `all 120 present`)
- Mobile PWA reload by owner before session (valhall #35, bifrost #38)
- Hub: `f8fdffc` (GSV 0.248.0), L: mirror `/l/gsv_mirror/wgpu_weights` (127 chunks, ~520 MB)

## Result — 40/40 ok

| blk | group | route | max_err_model |
|-----|-------|-------|---------------|
| 0 | g-295 | [38,35] | 7.00e-7 |
| 1 | g-296 | [38,35] | 1.58e-6 |
| 2 | g-297 | [38,35] | 3.87e-7 |
| 3 | g-298 | [38,35] | 6.26e-7 |
| 4 | g-299 | [38,35] | 8.49e-7 |
| 5 | g-300 | [38,35] | 5.81e-7 |
| 6 | g-301 | [38,35] | 5.66e-7 |
| 7 | g-302 | [38,35] | 6.85e-7 |
| 8 | g-303 | [38,35] | 7.15e-7 |
| 9 | g-304 | [38,35] | 9.98e-7 |
| 10 | g-305 | [38,35] | 8.49e-7 |
| 11 | g-306 | [38,35] | 1.13e-6 |
| 12 | g-307 | [38,35] | 6.56e-7 |
| 13 | g-308 | [38,35] | 6.41e-7 |
| 14 | g-309 | [38,35] | 5.22e-7 |
| 15 | g-310 | [38,35] | 8.34e-7 |
| 16 | g-311 | [38,35] | 9.39e-7 |
| 17 | g-312 | [38,35] | 1.31e-6 |
| 18 | g-313 | [38,35] | 6.85e-7 |
| 19 | g-314 | [38,35] | 4.62e-7 |
| 20 | g-315 | [38,35] | 6.26e-7 |
| 21 | g-316 | [38,35] | 7.60e-7 |
| 22 | g-317 | [38,35] | 6.26e-7 |
| 23 | g-318 | [38,35] | 3.87e-7 |
| 24 | g-319 | [38,35] | 8.34e-7 |
| 25 | g-320 | [38,35] | 6.41e-7 |
| 26 | g-321 | [35] | 8.79e-7 |
| 27 | g-322 | [35] | 6.26e-7 |
| 28 | g-323 | [38,35] | 6.56e-7 |
| 29 | g-324 | [38,35] | 7.60e-7 |
| 30 | g-325 | [38,35] | 5.96e-7 |
| 31 | g-287 | [35,38,35,35] | 9.24e-7 (earlier session) |
| 32 | g-290 | [35,38] | 9.83e-7 (earlier session) |
| 33 | g-289 | [35,38,35,38] | 5.51e-7 (earlier session) |
| 34 | g-294 | [35] | 8.34e-7 (earlier session) |
| 35 | g-292 | [35] | 4.77e-7 (earlier session) |
| 36 | g-293 | [35] | 4.02e-7 (earlier session) |
| 37 | g-326 | [38,35] | 6.41e-7 |
| 38 | g-327 | [38,35] | 4.17e-7 |
| 39 | g-328 | [38,35] | 4.62e-7 |

- Every group: `done_shards=4`, `complete=true`, `ok=true`.
- `max_err_model` range: **3.87e-7 … 1.58e-6** (all ≪ REDUCE_TOL=1e-3 · scale).
- Route: `[38,35]` for most; `[35]` ×3 when #38 (bifrost) wasn't ready at enqueue time.
- blk.26/27 single-peer (bifrost proofs expired mid-run — ready-gate filtered it;
  fresh proof renewals happen on next successful result).
- pi4-cpu (#36) received **0** block shards (claim-time browser guard, `2dca721`).
- Recommended next: batch re-dispatch with both phones hot, full set again after
  any fleet change, or fold block-op into the default distribution path.
