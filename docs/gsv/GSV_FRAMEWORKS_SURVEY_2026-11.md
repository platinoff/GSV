# GSV Ecosystem Frameworks Survey + Faster-Candidate Research — 2026-11

ECO outcome of the owner ask 2026-11-06 ("пройдись по всіх проектах, збери фреймворки,
ресерч швидших, заміни grep на ripgrep"). Companion to `GSV_SPEED_RESEARCH_2026-11.md`.

## 1. Frameworks inventory (per project, from Cargo.toml + scripts)

| Project | Runtime/API framework | Notable frameworks / crates |
|---|---|---|
| `gsv` (GSV) | axum 0.8 + tower-http 0.7 (compression-gzip) | tokio 1.53, reqwest 0.13 (native-tls), tower 0.5, httparse 1 (finite HTTP/1), mimalloc 0.1 (global allocator), rayon 1.10 (parallel scans), tracing 0.1, tokio-stream, serde/serde_json/toml 1.1, chrono |
| `poolAI` | axum 0.8 (ws) + tower-http 0.6 | tokio 1.49 (full), criterion 0.8, mockito 1.4, rusqlite 0.32 (bundled), teloxide 0.13 (opt), aws/azure SDKs (opt cloud), wide 0.7 (opt SIMD), ed25519-dalek, windows-sys 0.61 (opt), opentelemetry/prometheus (opt) |
| `GSV-WGPU` | wgpu 30 (`noop` CI fallback) | futures 0.3 — browser GPU-compute lanes |
| `LinFS` | axum 0.8 + tower-http (fs, cors) | windows 0.62, clap 4, tokio (full), crc32fast — mount/mutate ext4/xfs/btrfs/f2fs |
| `llama-rs` | llama-cpp-2 `=0.1.154` pin (six patches) | clap 4.6, sysinfo, criterion 0.5, ed25519-dalek — llama.cpp-backed inference |
| ORR_DESKTOP | winit 0.30 (tray app) | muda 0.19, tray-icon 0.24, cpal 0.18, openh264 0.9, muxide 0.2, windows 0.62 — zero external processes |
| `rebook` | axum 0.8 + tokio 1.53 | bookbinder 0.1, genpdf 0.2, hypher, png/jpeg-decoder, zip 8.6 |
| `rusty-engine` | std-only (memmap2 0.9.11) | MoE GGUF engine, SSE/AVX/NEON dispatch markers, fat-LTO |
| `sport-forcast` | axum 0.8 + tokio | serde + tower/http-body-util (dev) |
| `telenetis` | axum 0.8 (ws) + tower-http 0.7 | reqwest 0.13, tokio-tungstenite 0.29 (dev), hmac/sha1/sha2 |
| `Tokenomica` | (placeholder, no deps yet) | — |
| CoreBoard_Labs_shop | axum 0.8 + tower-http (fs, limit) | reqwest 0.13 (opt, native-tls), tokio |
| omniroute | Node/Next runtime (own tree; TS-first product) | panes: AGENTBRIDGE / MCP-SERVER / WEBHOOKS docs; Rust-first rule keeps it out of the Rust audit |

Common eco shape: **axum 0.8 for every HTTP surface** (server, poolAI, LinFS, rebook,
sport-forcast, telenetis, CoreBoard), std/memmap2 for the low-level engines.

## 2. Faster-candidate research (2026-02..2026-11 community + TechEmpower reality check)

| Candidate | Measured claim | Rust 1.99.1 + eco formats compat | Verdict |
|---|---|---|---|
| actix-web 4.12.1 | +10–15% RPS vs axum 0.8 in synthetic benches (TechEmpower-class); p99 ~2ms | ✔ MSRV ≤1.8x; no new file formats. But own routing/middleware model — port cost for tower layers, SSE/MCP chunk filters (gsv `/mcp`), ws surfaces | **Candidate only** — not adopted; eco keeps axum (single stack, tower-http compression, MCP/SSE plumbing already battle-tested) |
| Rocket 0.5.1 / Warp 0.4.1 / Salvo 0.89.1 | slower than or comparable to axum | ✔ | Not adopted (slower or marginal) |
| tokio 1.53 → newest | monotonic dep bumps inside 1.x | ✔ | Keep; align poolAI 1.49 → 1.53 in a future band (poolAI currently parked 9-red) |
| mimalloc global allocator | lower latency tail vs system allocator | ✔ already in gsv bins | Keep (done in band 231) |
| ripgrep 15.2.0 vs GNU grep | multithreaded, SIMD (+SSE2/+AVX2 runtime), gitignore-aware | ✔ pure Rust binary, `.rs/.md/.mdc/.json/.js/.sh` sources — no new file format | **Adopted** owner ask 2026-11-06: installed `rg.exe` at `~/.cargo/bin` (cargo install ripgrep --locked, v15.2.0); all owned `.sh` scripts migrated grep→rg (see §4) |

Decision: **no HTTP framework swap.** The measurable gains already live in the
allocator + parallel-scan + ripgrep lanes; framework churn would burn the
tower-http/compression and MCP/SSE work without a real-workload win.

## 3. Compatibility notes (Rust 1.99.1 + allowed formats)

- Toolchain: rustc `stable-x86_64-pc-windows-gnu` 1.99.1 (ECO-1/ECO-5 final pin).
  All inventory crates compile on it; actix-web 4.12 would, if ever adopted.
- Source formats stay in the allow-list: `.rs`, `.md`, `.mdc`, `.json`, `.js`,
  `.wasm`, `.sh` (eco standard from ECO-6.7). No framework introduced a new
  file format.

## 4. grep → ripgrep migration (ECO-6.8 close-out)

- Installed: ripgrep 15.2.0 (`cargo install ripgrep --locked`) →
  `C:\Users\plati\.cargo\bin\rg.exe`, features: `-pcre2` absent (Rust-regex only,
  sufficient for every migrated pattern), SIMD SSE2/AVX2 runtime enabled.
- Migrated 30 owned scripts (grep → rg exit-code-compatible):
  - poolAI: `.cursor/hooks/post-push-ph-s-notify.sh` (self-test OK),
    `bin/{commit-ph-s44,verify-dev-stand,verify-lan-prep}.sh`,
    `comitmsg/{fix-file-list,fix-file-list2,_band80_gate}.sh`,
    `scripts/{check_system,generate-ph-s-completion-extension,install_gcc,QUICK_FIX_RUST_PATH,setup_rust_path,verify_build}.sh`.
  - rebook: `scripts/*.sh` + `workspace/books/*/*.sh` (netstat listener checks +
    KDP PRICING extract) + `~/.cargo/bin` PATH fixes in 4 scripts.
  - telenetis: `scripts/telenetis-boot-verify.sh` (SSE content-type check).
  - omniroute: `bin/rollback.sh`, `scripts/ci/should-promote-latest.sh`,
    `scripts/quality/test-scoped.sh`, `scripts/release/merge-train.sh`,
    `tests/e2e-tool-calls.sh` (incl. fixture `rg pattern /var`),
    `tests/security/test-docker-hardening.sh` (rg-only, grep fallback removed),
    `tests/_run_dns_guard_test.sh`.
- Intentional keeps (documented, not grep-binary): poolAI `bin/e2e-a11y-hc.sh`
  (`--grep` is a Playwright filter flag), omniroute `scripts/ops/runner-janitor.sh`
  (`pgrep -fc`), poolAI `comitmsg/move-vision.sh` (`git grep` + git-native
  pathspec exclusions — follow-up ticket to migrate to rg globs).
- POSIX-class rewrites: `[[:space:]]` → `\s` (Rust-regex), `[[:space:]]*` → `\s*`.

## Status

- Owner-gated leftovers remain: token rotation, pushes, ECO-4 owner dirs
  (`poolAI_concept.txt` keep/move), infra keep-live.
- open-source eco law respected: one commit per product repo; no push mid-drain.
