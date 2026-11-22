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
- **Canary attempted, NOT passed (honest):** registered `artifact/blk.0.attn_gate.weight` [2048,4096] Q4_K (2 chunks) on :9999; g-715..g-718 enqueued. Peer 39 computed shards (283–316 MFLOPS) but `task result post FAILED after 3 attempts` each time; after the hub restart the mesh degraded (#37 `alive:false`, #39 stuck `claimed_now:1` on mt-715-1, `within_quota:false`, `route_peer:null` for new heavyweights). FW-7 stuck-claim requeue (~300 s) + a peer page reload are needed before re-running. **No `max_err_model` PASS claim.** G21 stays in_progress.
- **Next:** wait for FW-7 requeue / peer reconnect, re-probe (matmul64) to restore proof gate, re-enqueue `weights_matvec` on the Q4_K tensor, require `max_err_model < 1e-4` on Min. G13 still blocked.
