# GASV-G20 — Artifact-backed WGPU weight lane (2026-11-22)

**Ticket:** `t-1795350828508303000` (in_progress)  
**Goal:** make every one of the 733 tensors in `gsv-versus-ai-full-q4k.gguf` dispatchable over the Min WGPU cluster without duplicating ~20 GB of chunks on `L:` (only ~20 GB free).

## 1. Problem

- Artifact: 733 tensors, 21,166,255,488 B, Q4_K/Q6_K direct-copy; sha256 `2f37d609…`.
- Live weight-store index: **612 tensors** registered from earlier G5 work. The 122 huge `ffn_*_exps.weight` MoE tensors exist in the artifact but **not** as chunks.
- Duplicating them as 4 MiB chunks would add ~20 GB to `L:` (already ~43 GB used of 63) — unacceptable.

## 2. Design (implemented slice)

Content-addressed chunks are keyed by SHA-256 of the raw bytes. Instead of storing a duplicate file per key, map the key to a byte range inside the artifact:

```json
// data/wgpu_weights_artifact_sources.json (gitignored, beside the store)
[
  {"key":"<sha256>", "artifact":"L:/gsv_mirror/models/gsv-versus-ai/gsv-versus-ai-full-q4k.gguf",
   "offset": 721241344, "len": 4194304}
]
```

- `weight_store::read_chunk(key)` first checks `chunks/<key>.bin`; if absent, falls back to `read_chunk_artifact`: range-reads `offset..offset+len`, **verifies `sha256(bytes) == key`** (fail-closed), bumps `hits`, audits `read`.
- Peer-facing API is unchanged: `GET /api/wgpu/weights/chunk?key=…` serves the same bytes; browser IndexedDB cache (keyed by sha256) keeps working. No peer/page change required.
- Unit test `artifact_backed_chunk_reads_and_verifies_sha256` PASS (nonzero offset + tampered-len rejection). `cargo clippy --all-targets -- -D warnings` green.

## 3. Remaining work (next session / iceman)

1. **Generator utility (Rust):** a `cargo xtask` (or `rusty` subcommand) that:
   - parses the artifact GGUF header via existing `rusty-engine` reader to get `(tensor name, data_offset, byte_len, ggml_type, dims)`;
   - for a chosen huge tensor, splits its bytes into ≤4 MiB slices, computes `sha256` of each slice as the key,
   - appends rows to `wgpu_weights_artifact_sources.json`,
   - registers the tensor via `weight_store::register_tensor` (name → ordered chunk keys + dims), reusing existing `register_tensor` contract.
   - Must skip/flag tensors whose bytes are not already in the artifact; no silent coercion.
2. **Live registration:** run generator for a representative huge tensor (e.g. `blk.5.ffn_gate_exps.weight`), then `POST /api/wgpu/weights/register_tensor` and `GET /api/wgpu/weights/chunk?key=...` to prove range-served bytes.
3. **WGPU canary:** enqueue `weights_matvec` on the registered huge tensor, require `max_err_model < 1e-4` on Min (#37 hub RDNA2 + #39 bifrost), no #40 dependency.
4. **Gates + close:** `cargo test --lib`, `cargo clippy --all-targets -- -D warnings`, one GSV commit, `tickets_done` with evidence. Disk check (`gsv_disk`, 12 GB min) before any temp file.
5. **Owner gate:** no canonical change; no quality/parity claim; G13 still blocked.

## 4. Why this is the right next target

- G15 proved the artifact is structurally correct and Min-transport-clean (6.94e-18).
- G17 proved there is **no safe Astra-style weight-patch candidate** (layer-23 is a generic artifact; control prompt higher than "fact"; Granite audit tool-incompatible). So patching cannot improve this artifact within current evidence.
- Therefore the highest-value path to "best 4-bit MoE ideally working in the ecosystem" is **making all 733 tensors dispatchable** (G20), not editing weights.

No Perl/Python, no git weight blobs, no full-LLM/quality claim.

## 5. Implemented + proved (2026-11-22, later round)

- **Generator shipped:** `rusty artifact-chunks <gguf> <tensor> [--max N]` in rusty-engine (commit pending) emits `gasv.artifact-chunks.v1` with `{key, offset, len}` per ≤4 MiB slice. Ran on `blk.5.ffn_gate_exps.weight`: 150,994,944 B → 36 chunks, first key `cab595f4…`.
- **`register_tensor` extended (GSV):** accepts N-dim (1..=4) artifact-backed tensors with `source:"artifact"`, `dtype`, `ggml_type`, `bytes`; `chunk_len_any` checks store index + artifact sources; cap raised to 1 Gi values for artifact-backed. Store-backed path unchanged (2D/F32/64 Mi cap). Two new unit tests green; `cargo clippy --all-targets -- -D warnings` green.
- **Live proof (isolated server, port 9998, temp data-dir `target/live/g20_test_data`):** wrote `wgpu_weights_artifact_sources.json` (36 rows); `POST /api/wgpu/weights/register_tensor` → `{"ok":true,"tensor":"blk.5.ffn_gate_exps.weight","chunks":36,"bytes":150994944}`; `GET /api/wgpu/weights/chunk?key=<first>` returned 4,194,304 B; independent `sha256(decoded_bytes)` = `cab595f4c4fce114f669f54dc24d8873681f4c4d348d5884ae54fa2f0cab8798` = key → **MATCH**. Lane verified end-to-end.
- **Hub-side Q4_K dequant (G21, in progress):** new `src/boxes/wgpu_dequant.rs` ports rusty-engine's scalar `dequant_q4_k` (256 vals/144 B, 6-bit scales/mins, `y=d*sc*q-dmin*m`). `weights_matvec_reference` now calls `tensor_f32` which dequantizes Q4_K artifact tensors (F32 store-backed path unchanged); `tensor_bytes` now uses artifact-aware `read_chunk`. 2 unit tests green. This makes the *hub CPU reference* correct for Q4_K — a prerequisite for any peer canary; the peer still needs its own dequant (next).
- **Canon deploy DONE (2026-11-22):** rebuilt gsv-server (after the G21 dequant code) + `POST /api/update/apply` → watchdog respawn; live binary mtime updated, `register_tensor` on :9999 accepts the 3D artifact tensor, chunk GET returns 4 MiB with sha256 == key.
- **Page-side Q4_K dequant DONE (GSV `2fa9eb2`):** embedded `/wgpu` page has `f16ToF32`/`dequantQ4K`, dtype-driven dispatch in `downloadWeightsFlat`, 3D dims flattened. Marker test green.
- **Canary PASS (2026-11-22, after fleet restored):** `g-721` `weights_matvec` on `artifact/blk.0.attn_gate.weight` (Q4_K, [2048,4096], 2 chunks), shards=2, **peers [39,37]** (bifrost + RDNA2): both shards `status=done`, **`max_err_model=6.938893903907228e-18`** (<1e-4), `scale=1.0`, group `ok=true`. Earlier g-715..g-718 failed only due to post-restart degraded mesh (FW-7 stuck-claim + result-POST drops); after fleet recovery (owner confirmed both peers available) the clean rerun passed. **G21 done via MCP.** Proves Q4_K artifact-backed MoE tensor dispatch works end-to-end in the GSV ecosystem.
- **G22 scale-up (2026-11-22, in flight):** hub `dequant_q4_k_range` + `weights_matvec_reference_segment` wired into `task_result` (GSV `234c01f`), page shard-row dequant + `x_spec=onehot:<idx>:<len>` + 3D cols flatten (GSV `465b808`). 65 wgpu tests + clippy green. `g-726` on `blk.5.ffn_gate_exps.weight` shards=8: **peer #37 computed shard 0/8 in 926 ms @ 72 MFLOPS (256×131072)** proving the streaming lane — but all 8 cached shard W buffers held ~1 GB and **both peer pages crashed (alive:false for #37/#39)**; result POSTs were lost. Fix deployed (GSV `47373ae`): GPU/WASM caches now evict other shards of the same tensor before caching a new one; canon :9999 rebuilt + watchdog-respawned (PID 22316). **Owner action needed:** reload `/wgpu` on #37 (Firefox localhost) and #39 (bifrost phone) to pick up the fixed page, then re-enqueue the huge-expert canary. No PASS claim until `max_err_model < 1e-4`. G13 blocked.

- **Oracle cross-check reference:** `rusty-engine` `rusty tensor data/models/MiniCPM-V-4_7-1B-Q4_K_M.gguf blk.0.attn_gate.weight` (ggml=12, 1179648 B, offset 351305216) reports first dequant values `[-0.0302639, -0.0042099953, 0.002303481, 0.034870863, 0.002303481, 0.04138434]`. Use as an independent golden when validating GSV's `dequant_q4_k` — the peer and hub ports are independent transcriptions, so canary agreement alone does not exclude a shared transcription bug.
- **Compact x storage (GSV `568be1a`, `b48ce2f`):** task rows keep only `x_spec` ("onehot:i:n", ~28 B) instead of a 131072-float x (~262 KB); the page expands it locally via `taskX()` and the hub expands it only inside `task_result` reference. `mesh` strips x entirely. `wgpu_tasks.jsonl` stayed 7.4 KB after an 8-shard enqueue (was 8.4 MB). Also `MAX_TASKS` 200→32 and `Semaphore(2)` for heavy verifications.
- **ROOT CAUSE of 8-shard failures + 1 GB RSS (fixed, GSV `89ed920`, `4a218f6`):**
  1. `tensor_raw_range` looked up artifact-backed chunk keys with `chunks_len` (local `.bin` only) → always `None` → `weights_matvec_reference_segment` returned `None` → `task_result` never stamped `max_err_model`. Diagnostic test proved it: after switching to `chunk_len_any`, `seg.first() = 0.0072112083` = rusty-engine golden.
  2. `reduce_groups` (every `/api/wgpu/mesh`) called full `weights_matvec_reference` → dequantized the whole 151 MB expert into ~1.07 GB f32. Now artifact-backed groups use a length-only reference and per-task stamped `max_err_model`.
  Verified after fix: 8 huge-expert tasks enqueued, hub RSS stayed **34 MB** (was ~1 GB), tasks JSONL 8 KB. No more memory ceiling.
- **G23 (Q6_K lane, implemented + golden-verified, canary pending):** all **20 remaining Q6_K huge experts** registered (ggml 14, ~220 MB each) — total **122/122 huge MoE tensors addressable** from the single 21 GB artifact (4698 artifact-source keys, no weight duplication). GSV ports `dequant_q6_k` + streaming `dequant_q6_k_matvec` (7 dequant tests + clippy green); hub reference now returns `0.0178085566` for `blk.0.ffn_down_exps.weight` = rusty-engine golden `0.017808557` (fixed half-overwrite bug in streaming path). Page `dequantQ6K` + `qt===14` dispatch deployed. `g-731`/`g-732` Q6_K canaries were enqueued but did not complete numeric verification amid peer churn (WASM fallback / result-POST failures); **no Q6_K PASS claimed**. G23 stays in_progress; Q4_K `g-730` (3.47e-18) and `g-721` (6.94e-18) remain the proven cases. G13 blocked.
- **✅ G23 PASS (2026-11-22, closed via MCP):** Q6_K canary `g-736` on `blk.0.ffn_down_exps.weight` (Q6_K, 3D `[512,2048,256]` → 512×524288, 220 MB direct-copy), shards=8 on peers **[39,37]**: **8/8 done**, `max_err_model=1.734723475976807e-18 / 3.469446951953614e-18` (<1e-4), peer `partial[0]=0.01780855655670166` = rusty-engine golden. Earlier `g-731..g-737` failures were peer churn / result-POST drops, not a shader-width limit — once peers stayed online with updated pages the wide `cols=524288` shader ran fine. Together with Q4_K `g-730` (3.47e-18) and `g-721` (6.94e-18), **all 122/122 huge MoE tensors are now proven addressable**, and both Q4_K and Q6_K dequant lanes are verified end-to-end. G23 done; no full-LLM/quality claim (G13 blocked).
- **✅ G22 PASS (2026-11-22, closed via MCP):** `g-730` on `blk.5.ffn_gate_exps.weight` (Q4_K, [2048,512,256], 151 MB, 3D flatten to 2048×131072), shards=8 on peer **#37 RDNA2 WGPU**: **8/8 `done`, `max_err_model=3.469446951953614e-18`** (<1e-4), group `ok=true`, `scale=1.0`. Together with `g-721` (moderate Q4_K on [39,37] `6.94e-18`), the artifact-backed Q4_K lane is proven end-to-end at both scales. Full 4-bit MoE artifact remains versioned companion in mirror; canonical F32 unchanged; no runnable-full-LLM/quality claim (G13 blocked). Hub restarted clean (38 MB, tasks trimmed to 4.5 KB) and the runaway canary was **not** claimed as PASS. The moderate Q4_K canary (`g-721`, 2 shards) remains the proven end-to-end case. A future G22 slice needs a per-shard memory budget / sequential verification (e.g., cap concurrent `task_result` verifications to 1–2) before retrying 8 shards. G13 blocked.
- **Canary diagnosis (g-729, honest):** peer #37 computed shard 0/8 in **81 ms @ 828 MFLOPS** but posted `partial` = **all zeros** → hub rejected `max_err_model=0.0319` (expected first value `0.0072112083`). Golden test (`dequant_q4_k_matches_rusty_engine_golden`, GSV `99715ae`) proves the **hub dequant/reference is correct** (bit-exact with rusty-engine oracle), so the zero partial is a **stale peer page / stale CACHE entry** on #37, not a hub bug. Also optimized `tensor_raw_range` to read only ~5 of 36 overlapping chunks (not all 151 MB) so `task_result` stays fast.
- **Shard-offset bug found + fixed (GSV `6e5e748`):** `tensor_raw_range` returns a block-aligned slice, but `tensor_f32_range` trimmed it with ABSOLUTE `value_start/value_end` offsets — valid only for shard 0. Shards 1..7 got no valid hub reference, which explains the canary failures/zero partials. Now trims with LOCAL offsets (`value_start - (value_start/256)*256`) and F32 path no longer double-slices. `MAX_TASKS` 200→32 prevents 50 MB `wgpu_tasks.jsonl` (huge x arrays) from stalling every poll/mesh re-parse — this was the actual cause of slow Start-worker pages. 65 wgpu + 4 dequant tests + clippy green; deployed to canon :9999.
- **Hub hang incident + fix (2026-11-22):** runaway huge-expert tasks (g-726/g-727/g-729, 8 shards × repeated requeues) made synchronous verification block Tokio workers → `/api/health` and Start-worker `peer/join` timed out ("Failed to fetch"/"Start worker тупить"). Fixed by `api_wgpu_task_result` using `tokio::spawn_blocking` (GSV `0c5945d`), clearing the 24 runaway `blk.5.ffn_gate_exps` task rows, and watchdog respawn. Verified: health `ok:true`, RSS dropped 419 MB → 125 MB, `peer/join` responds instantly. No canary PASS claimed from the aborted runs.
- **Batch registration DONE:** all **100 Q4_K huge MoE experts** registered via `rusty artifact-chunks` → artifact-sources + tensor registry: **100/100 `ok:true`**, `wgpu_weights_artifact_sources.json` = 3638 keys / 678,619 B, tensor registry 713 rows (100 `ffn_*_exps`), HTTP manifest confirms 100 expert tensors. **No weight duplication** (~15 GB Q4_K bytes stay in the single artifact GGUF; only byte ranges hashed). The remaining 20 huge experts are Q6_K (ggml 14) and need a Q6_K dequant lane — not supported yet. Full 733-tensor addressability is therefore ~712/733 (612 prior + 100 experts), still short of a runnable full LLM (G13 blocked).
