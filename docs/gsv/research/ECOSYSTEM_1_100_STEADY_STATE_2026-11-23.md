# Ecosystem steady state + Rust 1.100 standardization research (2026-11-23)

**Scope:** read-only research across the registered GSV environment. Sources: `gsv_products`, `cargo xtask products --state`, `gsv_sli`, `gsv_health`, `rustc +stable --version`, product `rust-toolchain.toml`, GSV docs (`FUNCTION_MANAGEMENT.md`, `GSV_TECH_ROADMAP.md`, `PRODUCTS.md`), `docs/vision/`, GASV G15–G24 evidence.

## 1. Toolchain (fact)

- Installed/active: `rustc 1.100.0 (0ef1bc386 2026-11-09)`, `cargo 1.100.0 (3acc79c0c 2026-11-05)`.
- GSV pin: `stable-x86_64-pc-windows-gnu` + targets `x86_64-pc-windows-gnu`, components `rustfmt`, `clippy`. Comment: floats with stable (1.99.1 → 1.100 → …), owner 2026-11-04. **No `RUSTUP_TOOLCHAIN` override** (AGENTS.md and HANDOFF).
- 11 products pin `stable-x86_64-pc-windows-gnu` (GSV, poolAI, LinFS, telenetis, llama-rs, rebook, ORR_DESKTOP, rusty-engine, GSV-WGPU, sport-forcast, CoreBoard_Labs_shop).
- 2 products pin host-agnostic `stable` for cross-platform CI: `adoreble_wifi`, `mesh-node`.
- 1 product has no pin: `omniroute` (policy-down, not started — do not change).
- poolAI `.cargo/config.toml` has `test-ci` alias (full gates = GitHub Actions authority, local gate `fmt + clippy + check` due to RAM/EDR).

## 2. Products / git state (`cargo xtask products --state`, 2026-11-23)

| id | HEAD | branch | dirty | registered | remote |
|---|---|---|---|---|---|
| gsv | `42b74c6` | main | 0 | yes | origin |
| poolai | `52eb60dbd` | main | 0 | yes | origin |
| linfs | `95d4e23` | main | 0 | yes | origin |
| telenetis | `bc81a69` | main | 0 | yes | origin |
| llama-rs | `c5ef64c` | master | 0 | yes | origin |
| rebook | `869bef8` | master | 0 | yes | origin |
| orr_desktop | `337b2b1` | master | 0 | yes | origin |
| omniroute | `a2e1e9f` | release/v3.8.50 | 0 | yes | origin (policy-down) |
| rusty-engine | `f40b942` | main | 0 | yes | origin |
| gsv-wgpu | `9c65f59` | main | 0 | yes | **no remote** (owner gate) |
| sport-forcast | `c7213ea` | main | 0 | yes | origin |
| coreboard_labs_shop | `85d6e8f` | main | 0 | yes | origin |
| adoreble_wifi | `c1c8ff4` | main | 0 | yes | origin |
| mesh-node | `08cf81b` | main | 0 | yes | **no remote** (owner gate) |
| freebeat.ai | `87daf9e` | main | 0 | no | — |
| platinoff-profile | `5d81430` | main | 0 | no | — |

All registered products clean, no ahead/behind drift. Host = GSV; others = portable plugins (own `target/`, no kit copy).

**Wiring check (2026-11-23, `cargo xtask products --wiring`):** 14 registered rows — GSV `na` (host) + 13 plugins all `wiring=ok` (including **mesh-node `ok`**). 2 unregistered (`freebeat.ai`, `platinoff-profile`) `na`. The old P6.5 note "wiring ok ×14 + mesh-node missing — owner exception" is **stale**: mesh-node wiring is now `ok`; only its git **remote** is still absent (owner gate), which is a separate matter from kit wiring. HANDOFF P6.5 row updated accordingly.

## 3. GSV capability delta since FM snapshot (FM was at `28753cf`, now `42b74c6`)

G15–G24 (GASV track) added/advanced:
- artifact-backed WGPU weight lane (`wgpu_dequant`, `wgpu_weights_artifact_sources.json`, 4698 keys) — 122/122 huge MoE experts addressable from one 21 GB artifact, no weight duplication.
- Q4_K + Q6_K streaming dequant with golden tests vs rusty-engine oracle; canaries `g-721` 6.94e-18, `g-730` 3.47e-18, `g-736` 1.73e-18 (all <1e-4).
- compact `x_spec` task storage + page `taskX`; `MAX_TASKS` 200→32; `spawn_blocking` + `Semaphore(2)` for heavy verification; `reduce_groups` no full dequant.
- durable device identity `wgpu_peer_ids.json` (device_key→peer_id), stable IDs across reconnect; metrics/logs contiguous.
- best 4-bit MoE companion published versioned in mirror `publish.json` (canonical F32 unchanged).
- G18 feasibility: vision/mmproj patch negative-for-now (BF16 UnsupportedType + no evaluator + G13).

**FM gap:** FUNCTION_MANAGEMENT.md §1 rows stop at #42 (fm-xlink); no rows for the artifact-backed lane, Q4_K/Q6_K dequant, durable peer identity, or 4-bit companion. §2 gates stale (test 567s, ratio 99.61%, HEAD `28753cf`). §4 owner queue stale (RAM/EDR, vision rev mirror, P4 federation) and lacks G13 (runnable full LLM) and plugin-remote gates.

## 4. Automation gap (SLI, 2026-11-23)

`gsv_sli` (2026-11-23): **40 unused**, **7 used** (`inventory`, `record-rust`, `record-speed`, `sync`, `eco-check` [new], plus 2 adopted during the 1.100 slice). Existing automation that is declared but not wired into a single gate:
- `cargo xtask adopt` — portable auto-wire, digest sibling git tree, write registration artifacts (dry-first).
- `cargo xtask mirrors --check` — skills/kits drift audit (non-zero on drift).
- `cargo xtask rules-check` — git/fingerprint/vision/registry/ratio drift (hard gates fail).
- `cargo xtask fm-xlink --strict` — FM↔tickets cross-link (advisory; strict fails on orphans).
- `cargo xtask products --state` / `--wiring` — host vs plugin + per-repo target dir.
- `cargo xtask disk`, `intel`, `record-scenario-bench`, `alive-watch`, `status`.

No single command verifies "Rust 1.100 pin + clean plugins + wiring + FM/tickets + vision sync". This is the standardization/automation gap.

## 5. Devices / metrics (steady state)

- Hub `gsv-server` 0.248.0 on `:9999`, watchdog alive, `version_lag=false`, disk 43G free, `crate_version 0.248.0`.
- Mesh peers: #37 pc-wgpu (RDNA2), #39 bifrost, #40 valhall opportunistic; #36 Pi4/#38 dead; #41 RDNA2 dead; #42 test/curl.
- Peer identity: `wgpu_peer_ids.json` durable map (4 device_keys → ids 37/39/40/42).
- Metrics surfaces: `/api/wgpu/mesh`, `/api/wgpu/capability`, `/api/wgpu/telemetry` (ring 100), `/api/wgpu/console` (ring), `/api/sys/metrics`, `data/sys_metrics_history.jsonl`, `wgpu_weights_audit.jsonl`, `gsv-rate/speed/rust` panels.
- Kit gates: `record-rust` warnings=0 errors=0; `record-speed --skip-run` ok (last full test 518s); `sync` vision rev 516, next PH-S3129.

## 6. Constraints for the adaptation

- Rust-first, MSYS2 bash only; no Python/PowerShell; no `git add -A`; one commit per repo; push + summary.
- Global rules: only `S:/rust/GSV/AGENTS.md`; plugin rules stay in plugin repo; no kit copy into plugins; folder-scoped MCP; no tunnel/User MCP.
- `git fetch` before any drain; S0 disk guard; do not kill `target/live/gsv-server.exe` before tests/builds.
- poolAI = dad/FM plan, GSV = mom/ops hub; PM/ops boundaries must stay in docs.
- "Maximum Rust architecture": runtime/API/tests/benches/scripts `.rs` via `cargo xtask`; allowed formats `.rs/.md/.mdc/.json/.js/.wasm`; no weights/secrets in git.
