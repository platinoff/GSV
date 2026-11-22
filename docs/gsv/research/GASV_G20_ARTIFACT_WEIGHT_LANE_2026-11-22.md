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
