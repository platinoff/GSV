# AGENTS.md — GSV (Galaxy StarWalker Vision)

Standalone Rust-first vision server **and VDT entry workspace** for registered
Rust products. **Root:** `S:\rust\GSV` (sibling of PoolAI, not a subfolder).

**Kit (rules / skills / `абракадабра`):** open **this** folder in Cursor. Canon —
[`docs/gsv/GSV_VDT_KIT.md`](docs/gsv/GSV_VDT_KIT.md). Product code may live in
another tree (`S:\rust\poolAI`, …); the window being GSV does **not** mean the
drain target is GSV — the skill asks first.

## Stack

- Runtime / API / boxes: **Rust** (`edition 2021`, `tokio`, `axum`) in `src/`, `tests/`, `src/bin/`.
- UI: thin HTML/CSS/JS glue in `ui/` (no Python, no Java).
- Ratio canon: Rust **95–100%** / wasm 0–5% — `cargo run --bin gsv-loc-audit -- --stretch-96`.
- Terminal: **MSYS2 bash**, not PowerShell.

```bash
export PATH="/c/Users/${USER:-${USERNAME}}/.cargo/bin:$HOME/.cargo/bin:/ucrt64/bin:/usr/bin:$PATH"
export RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-stable-x86_64-pc-windows-gnu}"
cd /s/rust/GSV || cd "S:/rust/GSV"
unset CARGO_TARGET_DIR
```

## Session (`абракадабра` — спочатку вибір продукту)

Trigger skill: `.agents/skills/abracadabra/`. Вікно GSV ≠ автоматично drain GSV.

1. `bash scripts/list-vdt-products.sh` → AskQuestion / `question` on **those** environment projects (not a hardcoded `gsv | poolai` pair).
2. S0 disk for **that** product → `git fetch` → its HANDOFF.
3. Drain next band (GSV: `docs/gsv/GSV_TECH_ROADMAP.md`; PoolAI: FM §5.12).
4. If **gsv:** stop `gsv-server` before `cargo test` / `cargo build` (locks `target/debug/gsv-server.exe`).
5. `cargo fmt --all` → product tests (`cargo test` here; `cargo test-ci` in PoolAI).
6. One commit **in the product repo**. GitHub remote: `origin` → `https://github.com/platinoff/GSV` (create if missing, then `git push`).

## OpenCode (Windows)

OpenCode defaults to PowerShell — **this repo forbids that**. All `cargo` / `git` / scripts:

```
C:\msys64\usr\bin\bash.exe -lc 'команда'
```

`абракадабра` in OpenCode uses the `question` tool (not Cursor AskQuestion). Skills: `.agents/skills/` (`opencode.json` → `skills.paths`). Plugin host: `.opencode/package.json` (`@opencode-ai/plugin`). **Do not auto-generate a product UI** — live Galaxy UI is `gsv-server` at `http://127.0.0.1:9999/`.

Cursor ↔ OpenCode: Cursor `AskQuestion` = OpenCode `question`. Shared kit git-canon is this repo; copy (not symlink) to `.cursor/skills/` and `.opencode/skills/` via `bash scripts/sync-vdt-skill-mirrors.sh`.

## Speeds + Rust panel (GSV drain)

After tests: `bash bin/record-test-speed.sh` (or `--skip-run`) and `bash bin/record-rust-diagnostics.sh`. Writers: `gsv-speed-index` / `gsv-rust-diagnostics` → `docs/vision/*.json`. Then `bash bin/gsv-vision-sync.sh`.

## Defaults

| Flag | Default |
|------|---------|
| `--repo-root` | this crate (`S:/rust/GSV`) |
| `--data-dir` | `{repo-root}/data` |
| `--port` | **9999** (8765 is Hyper-V reserved) |
| Vision sources | `docs/vision/{manifest,feed,extensions,speed_index,rust_diagnostics}.json` |

Optional: `--repo-root S:/rust/poolAI` to scan PoolAI FM / `bin/` / `scripts/` from this server.

PoolAI vision canon (after the split) lives in **PoolAI** at `docs/vision/`. This repo keeps its own `docs/vision/` snapshot.

## Do not

- Stage `data/*` (except `.gitkeep`), `.env*`, `*.pem` / `*.key`.
- Run `git add -A`.
- Add Python product files.
- Push mid-drain.
