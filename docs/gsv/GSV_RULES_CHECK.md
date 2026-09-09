# GSV Rules Check — Gravity harness

One drain command that confirms the kit rulebases still describe the shipped
truth. Mirrors the existing per-domain gates and reports drift in a single
pass; **hard** gates flip the exit code, **soft** checks only flag advisory
drift.

```bash
cargo xtask rules-check                # repo-state gates + live :9999 lockstep probe
cargo xtask rules-check --no-live      # skip the live probe (still advisory)
GSV_LIVE_URL=http://127.0.0.1:9999/api/watchdog cargo xtask rules-check
```

Exit code: `0` when every **hard** check passes, `1` otherwise. Soft-drift
rows are printed with `[soft]` and never flip the exit code. MCP read-only
task (`gsv_xtask {task:rules}`) exposes the same report without a live probe.

Report fields: `ok` (sum of hard gates), `at`, `git_head`, `doc`, `checks[]`
(`id`, `ok`, `gate`, `detail`).

## Checks

| id | gate | enforces | source |
|----|------|----------|--------|
| `git` | hard | working tree resolves a HEAD (`rev-parse --short HEAD` ≠ `unknown`) | `src/vision.rs` → `git_head_short` / `git_head` |
| `fingerprint` | hard | this crate version has a fingerprinted drain row (product=`gsv`) | `src/boxes/fingerprint.rs` → `latest` + `fingerprints.jsonl` (`docs/gsv/fingerprints.jsonl`) |
| `vision` | hard | vision snapshot has no source↔persisted drift | `src/boxes/vision.rs` → `collect_drift` (revision + feed + extensions + persisted mirror) |
| `registry` | soft | every registered product row keeps HANDOFF+NEXT (PRODUCTS.md ↔ discovery reality) | `src/boxes/products.rs` → `discover` + `scan` |
| `ratio` | hard | persisted Rust ratio ≥ formal band 95% (96% stretch advisory) | `src/boxes/ratio.rs` → `audit`/`save`/`load` → `data/rust_ratio.json` |
| `sli` | soft | SLI catalog used/unused counts (unused = new-SLI candidate pool) | `src/boxes/sli.rs` → `wire`/`SliCatalog::scan` |
| `bench` | soft | speed index fresh (≤ 31 days; rerun `cargo xtask record-speed`) | `src/boxes/vision.rs` → `read_speed_index` → `docs/vision/speed_index.json` |
| `live` | soft | running server (`/api/watchdog`) version == crate version (lockstep) | `src/boxes/keep_live.rs` → `probe_http_blocking` |

## Harness map (what already kept rules pinned)

Every row below is an existing gate that `rules-check` aggregates — it adds no
new sub-harness, only the report surface (box `src/boxes/rules.rs`, CLI arm in
`src/bin/gsv_xtask.rs`, MCP arm `rules` in `src/boxes/xtask.rs`).

| Gate | run as | source artifact | doc |
|------|--------|-----------------|-----|
| Rust 95–100% ratio | `cargo run --bin gsv-loc-audit -- --stretch-96` | `data/rust_ratio.json` | AGENTS.md «Ratio canon» |
| Vision snapshot drift | `cargo xtask sync --check` | `docs/vision/{manifest,feed,extensions}.json` + `data/` mirror | AGENTS.md «Speeds + Rust panel» |
| Fingerprints integrity | `cargo xtask fingerprint-recheck` / `-dedup` | `docs/gsv/fingerprints.jsonl` | AGENTS.md drain Step 6 |
| Product registry | `cargo xtask products` | `docs/gsv/PRODUCTS.md` + workspace/sibling discovery | AGENTS.md «Session» |
| Speed index | `cargo xtask record-speed` | `docs/vision/speed_index.json` | AGENTS.md «Speeds + Rust panel» |
| Rust diagnostics | `cargo xtask record-rust` | `docs/vision/rust_diagnostics.json` | AGENTS.md «Speeds + Rust panel» |
| SLI catalog | `cargo xtask products` / `/api/sli` | `src/bin/*.rs` first-doc-line + `/api/xtask` tasks | `src/boxes/sli.rs` |
| Live lockstep | `cargo xtask live` / watchdog | `target/live/gsv-server.exe` copy + `/api/watchdog` | AGENTS.md «Defaults» |
| Disk S0 | `cargo xtask disk` | `target/` scan + volume free | AGENTS.md «Defaults» |
| HTTP stand | `cargo xtask disk` / `gsv-http-stand-smoke` | `tests/*` + `/api/health` probes | `src/bin/gsv_http_stand_smoke.rs` |

## When to run

- After a drain's tests, before bump/fingerprint: confirms nothing drifted and
  the version you are about to close is fingerprinted.
- After `cargo xtask bump --band N` and fingerprinting `0.<N>.0`: the
  `fingerprint` hard gate turns green for the new band.
- Historical trend: `docs/vision/speed_index.json` + `rust_diagnostics.json`.