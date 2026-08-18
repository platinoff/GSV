# Always-on Galaxy UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep `gsv-server` reachable on `:9999` during builds, send the page offline only while swapping the binary, fix Galaxy chrome (power menu, collapse, fullscreen), balance type/charts, then add product picker, version bump, and fingerprints.

**Architecture:** Run a **live copy** of the exe (`target/live/gsv-server.exe`) so `cargo` can overwrite `target/debug/`. Chrome and typography stay thin HTML/CSS in `ui/index.html` plus Rust SVG. Products reuse the same discovery merge as `scripts/list-vdt-products.sh`, implemented in Rust. Version is `CARGO_PKG_VERSION`. Fingerprints are append-only JSONL under `docs/gsv/`.

**Tech Stack:** Rust 2021, axum, tokio, serde_json, thin `ui/index.html`, MSYS2 bash supervisor.

**Spec:** [`docs/gsv/GSV_ALWAYS_ON_UI.md`](../../gsv/GSV_ALWAYS_ON_UI.md)

## Global Constraints

- Ratio: `cargo run --bin gsv-loc-audit -- --stretch-96` ≥ 96%; no Python; no `vision.js` port.
- Bind: `127.0.0.1:9999` default; no LAN widen.
- POSTs: CSRF gate (band 133); body cap 256 KiB (band 134).
- Shell: `C:\msys64\usr\bin\bash.exe -lc '…'`; one `cargo` at a time; GSV `unset CARGO_TARGET_DIR`.
- Live process must **not** be `target/debug/gsv-server.exe` after band 144.
- Tests that currently `assert_eq!(w.version, "0.1.0")` must use `env!("CARGO_PKG_VERSION")` as soon as version bump lands (band 146) — do this in the same band as the bump.
- Never stage `data/*`, `.env*`, `*.pem`, `comitmsg/*.txt`.
- Drain: ≤10 PH-S* per session; band 143 first.

## File map

| File | Role |
|------|------|
| `ui/index.html` | Chrome CSS (z-index, collapse, FS), type scale, Update apply, products glue |
| `src/boxes/ui.rs` | `CARD_NAMES` / `UI_GROUPS` / header / new `products` + `fingerprints` renderers |
| `src/boxes/vision.rs` | SVG chart font-size + plot height |
| `src/boxes/update.rs` | `apply` wire; live vs debug binary paths |
| `src/boxes/products.rs` | **Create** — discover / select / open / scan |
| `src/boxes/fingerprint.rs` | **Create** — JSONL append + latest-N wire |
| `src/boxes/mod.rs` | `pub mod products;` `pub mod fingerprint;` |
| `src/server/mod.rs` | Routes for products, update/apply, fingerprints |
| `src/state.rs` | `product_selection`; optional live-bin flag |
| `scripts/gsv-live.sh` | **Create** — copy debug → live, exec, restart on exit |
| `scripts/gsv-bump-version.sh` | **Create** — patch bump `Cargo.toml` |
| `scripts/gsv-fingerprint.sh` | **Create** — append JSONL + suggest trailers |
| `.gitignore` | `target/live/` |
| `tests/gsv_ui_contracts.rs` | Power menu z-index, card-fs, type vars |
| `tests/gsv_update_flow.rs` | Apply emits offline; version from pkg |
| `tests/gsv_products_contracts.rs` | **Create** |
| `tests/gsv_fingerprint_contracts.rs` | **Create** |
| `docs/gsv/GSV_TECH_ROADMAP.md` | Bands 143–147 |
| `docs/HANDOFF_NEW_SESSION.md` / `docs/NEXT_SESSION_PROMPT.md` | Next drain = 143 then 144… |

---

# Band 143 — Chrome bugs + type/chart scale (PH-S2069…S2078) ✅

Shipped 2026-08-17. Next `абракадабра` on gsv is **band 144**.

### Task 1: Power menu above cards

**Files:**
- Modify: `ui/index.html` (style block ~L11, L65, L70)
- Modify: `src/boxes/ui.rs` `render_header` if it emits a duplicate `#powerMenu`
- Test: `tests/gsv_ui_contracts.rs`

**Interfaces:**
- Consumes: existing `#btnPower` / `#powerMenu` / `data-action='power-toggle'`
- Produces: CSS so header stacking context > workspace; menu `z-index: 80`

- [ ] **Step 1: Write the failing test**

In `tests/gsv_ui_contracts.rs` add:

```rust
#[tokio::test]
async fn ui_index_power_menu_stacks_above_workspace() {
    let (app, _state) = app();
    let html = get_index_html(&app).await;
    assert!(
        html.contains("z-index:80") || html.contains("z-index: 80"),
        "power menu must stack above cards: {html}"
    );
    assert!(
        html.contains(".power-menu") && html.contains("z-index"),
        "power-menu rule missing"
    );
    // Regression: later rule must not pin body>header to the same z-index as .workspace.
    assert!(
        !html.contains("body>header,.workspace{position:relative;z-index:2}"),
        "header must not share z-index 2 with workspace"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd /s/rust/GSV && cargo test --test gsv_ui_contracts ui_index_power_menu_stacks_above_workspace -- --nocapture
```

Expected: FAIL — assertion on `body>header,.workspace{position:relative;z-index:2}` or missing `z-index:80`.

- [ ] **Step 3: Minimal CSS fix**

In `ui/index.html`:

1. Keep `header{...position:sticky;top:0;z-index:40;overflow:visible}`.
2. Replace `body>header,.workspace{position:relative;z-index:2}` with:

```css
.workspace{position:relative;z-index:2}
body>header{position:sticky;top:0;z-index:40;overflow:visible}
.power-menu{z-index:80}
```

3. `.header-actions{position:relative}` so `absolute` menu is anchored to the button cluster.

- [ ] **Step 4: Run test to verify it passes**

Same `cargo test` as step 2. Expected: PASS.

- [ ] **Step 5: Do not commit yet** — wait for band close (one commit per drain).

### Task 2: Exclusive fullscreen + named Esc target

**Files:**
- Modify: `ui/index.html` (card action JS ~L137–157, CSS `.card.fullscreen`)
- Test: `tests/gsv_ui_contracts.rs` (markers in served HTML)

**Interfaces:**
- Consumes: `.card`, `.actions`
- Produces: `data-action="card-min"` / `data-action="card-fs"` on the injected buttons; `exitFullscreen()` helper

- [ ] **Step 1: Failing test**

```rust
#[tokio::test]
async fn ui_index_card_actions_use_data_action() {
    let (app, _state) = app();
    let html = get_index_html(&app).await;
    assert!(html.contains("data-action=\"card-fs\"") || html.contains("data-action='card-fs'"));
    assert!(html.contains("data-action=\"card-min\"") || html.contains("data-action='card-min'"));
    assert!(html.contains("function exitFullscreen"));
}
```

- [ ] **Step 2: Run — expect FAIL** (`function exitFullscreen` missing).

- [ ] **Step 3: Implement**

Replace the DOMContentLoaded injector with:

```javascript
function exitFullscreen() {
  document.querySelectorAll(".card.fullscreen").forEach((c) => {
    c.classList.remove("fullscreen");
    const b = c.querySelector("[data-action='card-fs']");
    if (b) b.textContent = "□";
  });
  document.body.classList.remove("panel-fs-active");
}
function collapseCard(card) {
  exitFullscreen();
  card.classList.add("collapsed");
  const b = card.querySelector("[data-action='card-min']");
  if (b) b.textContent = "+";
  syncDock();
}
function restoreCard(card) {
  card.classList.remove("collapsed");
  const b = card.querySelector("[data-action='card-min']");
  if (b) b.textContent = "_";
  syncDock();
}
document.addEventListener("DOMContentLoaded", () => {
  document.querySelectorAll(".card").forEach((card) => {
    const h2 = card.querySelector("h2");
    if (!h2) return;
    const actions = document.createElement("span");
    actions.className = "actions";
    const minBtn = document.createElement("button");
    minBtn.textContent = "_";
    minBtn.title = "Minimize / Restore";
    minBtn.setAttribute("aria-label", "Minimize / Restore card");
    minBtn.setAttribute("data-action", "card-min");
    minBtn.onclick = (e) => {
      e.stopPropagation();
      if (card.classList.contains("collapsed")) restoreCard(card);
      else collapseCard(card);
    };
    const maxBtn = document.createElement("button");
    maxBtn.textContent = "□";
    maxBtn.title = "Fullscreen Toggle (Esc)";
    maxBtn.setAttribute("aria-label", "Fullscreen toggle card");
    maxBtn.setAttribute("data-action", "card-fs");
    maxBtn.onclick = (e) => {
      e.stopPropagation();
      const turningOn = !card.classList.contains("fullscreen");
      exitFullscreen();
      if (turningOn) {
        card.classList.add("fullscreen");
        maxBtn.textContent = "×";
        document.body.classList.add("panel-fs-active");
      }
    };
    actions.appendChild(minBtn);
    actions.appendChild(maxBtn);
    h2.appendChild(actions);
  });
  document.addEventListener("keydown", (e) => {
    if (e.key !== "Escape") return;
    exitFullscreen();
    const pm = $("powerMenu");
    if (pm) pm.classList.remove("open");
    $("btnPower").setAttribute("aria-expanded", "false");
  });
});
```

- [ ] **Step 4: `cargo test --test gsv_ui_contracts ui_index_card_actions_use_data_action` — PASS.**

### Task 3: Collapse removes card from grid

**Files:**
- Modify: `ui/index.html` CSS `.card.collapsed`

**Interfaces:**
- Consumes: `syncDock()` (already)
- Produces: `.card.collapsed{display:none!important}` so only dock chips remain

- [ ] **Step 1: Failing test**

```rust
#[tokio::test]
async fn ui_index_collapsed_card_leaves_grid() {
    let (app, _state) = app();
    let html = get_index_html(&app).await;
    assert!(
        html.contains(".card.collapsed{display:none")
            || html.contains(".card.collapsed { display: none"),
        "collapsed cards must leave the grid"
    );
}
```

- [ ] **Step 2: Run — FAIL** (today only `.card.collapsed .body{display:none}`).

- [ ] **Step 3: Replace**

```css
.card.collapsed{display:none!important}
```

Keep `.card.collapsed .body{display:none!important}` as defense in depth or drop it — either is fine if the card itself is `display:none`.

Update `syncDock` restore to call `restoreCard(card)` (from Task 2) instead of only toggling class + first action button.

- [ ] **Step 4: Test PASS.**

### Task 4: Type scale + chart SVG size

**Files:**
- Modify: `ui/index.html` `:root` + `body` + `.card .body` + chart `<img>` cards
- Modify: `src/boxes/vision.rs` `speed_index_chart_svg` / `rust_diagnostics_chart_svg` (title `font-size="12"` → `11`, footer `10` → `11`, viewBox height 140 → 168, bar plot y-scale)
- Test: `tests/gsv_ui_contracts.rs` + existing vision chart unit tests in `src/boxes/vision.rs`

**Interfaces:**
- Consumes: `speed_index_chart_svg` / `rust_diagnostics_chart_svg` string builders
- Produces: CSS `--fs-ui:13px;--fs-card:12px;--fs-meta:11px;--fs-chart:11px`; SVG height 168

- [ ] **Step 1: Failing tests**

```rust
#[tokio::test]
async fn ui_index_defines_type_scale() {
    let (app, _state) = app();
    let html = get_index_html(&app).await;
    assert!(html.contains("--fs-ui:13px"));
    assert!(html.contains("--fs-card:12px"));
    assert!(html.contains("--fs-meta:11px"));
    assert!(html.contains("--fs-chart:11px"));
}
```

In `src/boxes/vision.rs` chart tests, extend assertions:

```rust
assert!(svg.contains("font-size=\"11\""), "{svg}");
assert!(svg.contains("height=\"168\"") || svg.contains("viewBox=\"0 0") && svg.contains("168"), "{svg}");
```

Read the current SVG header (`<svg ... height="140"`) and match the real attributes when editing.

- [ ] **Step 2: Run — FAIL** (variables missing; height still 140).

- [ ] **Step 3: Implement**

`:root` add the four `--fs-*`. `body{font:var(--fs-ui)/1.5 ...}`. `.card .body{font-size:var(--fs-card);max-height:420px}`. `.card.fullscreen .body{max-height:none}`. `.meta,.badge{font-size:var(--fs-meta)}`.

In both chart functions, bump canvas: `width="720" height="168"` (or current width × new height), stretch bar `y` math from the old 140 baseline to 168 (keep padding top 28 / footer 24). Title and footer `font-size="11"`. `font-family="ui-monospace, Cascadia Code, Consolas, monospace"`.

- [ ] **Step 4: `cargo test --test gsv_ui_contracts ui_index_defines_type_scale` and `cargo test speed_chart_svg rust_chart_svg` — PASS.**

### Task 5: Band 143 docs + contracts + close

**Files:**
- Modify: `docs/gsv/GSV_TECH_ROADMAP.md` (mark 143 rows ✅ as they complete)
- Modify: `docs/HANDOFF_NEW_SESSION.md`, `docs/NEXT_SESSION_PROMPT.md`, `docs/MEMORY.md`
- Modify: `docs/gsv/GSV_BOXES.md` (chrome behavior note)
- Test: full GSV gate

- [ ] **Step 1:** `cargo fmt -- --check && cargo clippy --all-targets -- -D warnings` (or repo’s usual clippy invocation without `-D` if that’s canon — match last band: `cargo clippy --all-targets`).
- [ ] **Step 2:** `cargo test` (live copy not required until 144; if debug exe is locked, this is the last session that may need to stop `target/debug/gsv-server.exe`).
- [ ] **Step 3:** `cargo run --bin gsv-loc-audit -- --stretch-96`
- [ ] **Step 4:** `bash bin/record-test-speed.sh --skip-run` if tests already ran; `bash bin/record-rust-diagnostics.sh`; `bash bin/gsv-vision-sync.sh` then `--check`
- [ ] **Step 5:** One commit, then `git push origin main`

Commit message:

```
feat(ui): PH-S2069-S2078 Galaxy chrome stack, collapse, fullscreen, type scale

Power menu above cards; exclusive fullscreen; collapsed cards leave the grid;
shared --fs-* scale and taller speed/rust SVG charts.
```

---

# Band 144 — Always-on live binary (PH-S2079…S2088)

Do **not** start this band in the same session as 143 unless 143 closed and the owner asks to continue.

### Task 6: Live copy supervisor

**Files:**
- Create: `scripts/gsv-live.sh`
- Modify: `.gitignore` (`/target/live/`)
- Modify: `docs/gsv/GSV_SERVER.md`, `AGENTS.md`, `docs/HANDOFF_NEW_SESSION.md` (stop “kill server before cargo test”)

**Interfaces:**
- Consumes: built `target/debug/gsv-server.exe`
- Produces: running `target/live/gsv-server.exe --host 127.0.0.1 --port 9999`

`scripts/gsv-live.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEBUG="$ROOT/target/debug/gsv-server.exe"
LIVE_DIR="$ROOT/target/live"
LIVE="$LIVE_DIR/gsv-server.exe"
HOST="${GSV_HOST:-127.0.0.1}"
PORT="${GSV_PORT:-9999}"
mkdir -p "$LIVE_DIR"
copy_live() {
  cp -f "$DEBUG" "$LIVE"
}
if [ ! -f "$DEBUG" ]; then
  echo "build debug first: cargo build --bin gsv-server" >&2
  exit 1
fi
copy_live
while true; do
  copy_live
  "$LIVE" --host "$HOST" --port "$PORT" || true
  echo "gsv-live: process exited, restarting in 1s" >&2
  sleep 1
done
```

Acceptance: with `gsv-live.sh` running, `cargo test` does not hit os error 5 on `gsv-server.exe`.

### Task 7: `POST /api/update/apply`

**Files:**
- Modify: `src/boxes/update.rs`, `src/server/mod.rs`, `src/state.rs`
- Modify: `ui/index.html` `doUpdate()`
- Test: `tests/gsv_update_flow.rs`, `tests/gsv_server_contracts.rs`

**Interfaces:**
- Consumes: CSRF-safe POST (same as `/api/update/notify`)
- Produces: JSON `{ok:true, applying:true}`; SSE `event: offline`; process `std::process::exit(0)` after a short flush (tokio spawn + 200ms sleep so the response is sent)

Failing test (oneshot cannot observe process exit — test the handler’s **pre-exit** wire via a `apply_update(state) -> Value` that sets a flag and emits, and unit-test that; integration test checks route 200 + `applying`):

```rust
#[tokio::test]
async fn post_update_apply_emits_offline_and_ok() {
    let (app, state) = state();
    let mut rx = state.events.subscribe();
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/update/apply")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let payload = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("sse")
        .expect("msg");
    assert!(payload.contains("offline") || payload.contains("update_apply"));
}
```

In tests, **do not** `process::exit`. Gate exit behind:

```rust
pub fn apply_should_exit() -> bool {
    std::env::var("GSV_UPDATE_APPLY_EXIT").ok().as_deref() != Some("0")
}
```

`AppState::new` for tests sets `GSV_UPDATE_APPLY_EXIT=0` in `cfg(test)` or the test sets it before the request.

UI:

```javascript
async function doUpdate() {
  setOffline(true);
  toast("applying update…");
  try {
    await fetch("api/update/apply", { method: "POST", headers: { "Content-Type": "application/json" }, body: "{}" });
  } catch (_) { /* server is exiting — expected */ }
}
```

SSE `onopen` already `setOffline(false); resync();`.

### Task 8: Band 144 close

Same gate as Task 5. Commit:

```
feat(server): PH-S2079-S2088 live copy + update apply

Run target/live/gsv-server.exe; cargo may rebuild debug; UI goes offline until SSE reconnects.
```

---

# Band 145 — Products picker (PH-S2089…S2098)

### Task 9: Discovery module

**Files:**
- Create: `src/boxes/products.rs`
- Modify: `src/boxes/mod.rs`
- Test: `tests/gsv_products_contracts.rs`

**Interfaces:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductRow {
    pub id: String,
    pub name: String,
    pub path: String,
    pub kind: String,       // rust | node | git | folder
    pub registered: bool,
    pub source: String,     // workspace | sibling | kit
    pub git: bool,
    pub cargo: bool,
}

pub fn discover(kit_root: &Path) -> Vec<ProductRow>;
```

`discover` must:

1. Read `gsv.code-workspace` folders if present.
2. Scan `kit_root.parent()` for directories with `.git`.
3. Always include `kit_root`.
4. Dedup by canonical path.
5. `registered` iff `docs/gsv/PRODUCTS.md` has `| **{id}**`.

Mirror `scripts/list-vdt-products.sh` (do not shell out). Paths as `S:/rust/...` (`/` not `\`).

Failing test:

```rust
#[test]
fn discover_includes_gsv_kit() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rows = gsv::boxes::products::discover(&root);
    assert!(rows.iter().any(|r| r.id == "gsv" && r.kind == "rust" && r.registered));
}
```

### Task 10: HTTP + open confine

**Files:**
- Modify: `src/server/mod.rs`, `src/state.rs` (`Mutex<Option<String>>` selected id)
- Modify: `src/boxes/ui.rs` — `render_products`, add `"products"` to `CARD_NAMES` and `UI_GROUPS` ops (after health)
- Modify: `ui/index.html` — card section + `data-action` select/open
- Test: server contracts 200 + unknown id 404 `{ok:false}`

**Interfaces:**

```
GET  /api/products            { ok, products: [...], selected: Option<id> }
POST /api/products/select     { id } → { ok, selected }
POST /api/products/open       { id } → { ok, opened, how: "explorer"|"cursor" }
GET  /api/products/scan       uses selected; { ok, git_head, git_status_short, kind, registered, handoff_exists, next_exists, cargo_name }
```

Open confine:

```rust
pub fn open_folder(kit_root: &Path, id: &str) -> Result<String, String> {
    let rows = discover(kit_root);
    let row = rows.iter().find(|r| r.id == id).ok_or("unknown product")?;
    // spawn explorer.exe or cursor — never user-supplied argv beyond the confined path
}
```

Unknown id → 404 `{ok:false,error}`. Path traversal impossible because lookup is by id in the discovered set.

Scan parsers (no `cargo test`):

- `git -C path rev-parse --short HEAD` and `git status -sb` via `std::process::Command` (already used elsewhere) **or** read `.git/HEAD` + skip status if you want zero git spawn — prefer `Command` with `git` allowlisted like terminal.rs.
- `handoff_exists`: `{path}/docs/HANDOFF_NEW_SESSION.md` or `{path}/docs/development/HANDOFF_NEW_SESSION.md` (poolAI).
- `cargo_name`: parse `name =` from `Cargo.toml` first `[package]` if `cargo`.

UI glue: buttons `data-action="product-select" data-product-id="gsv"` etc. from Rust HTML.

### Task 11: Band 145 close

Gate + commit `feat(ui): PH-S2089-S2098 VDT product picker + folder open + scan`.

---

# Band 146 — Version bump + fingerprints (PH-S2099…S2108)

### Task 12: Version from crate; stop hardcoding `0.1.0`

**Files:**
- Modify: `tests/gsv_update_flow.rs` (`assert_eq!(w.version, env!("CARGO_PKG_VERSION"))`)
- Create: `scripts/gsv-bump-version.sh` — increment patch in `Cargo.toml` `[package] version`
- Modify: drain close in HANDOFF to run the bump **before** the commit

```bash
# scripts/gsv-bump-version.sh — python-free: awk/sed
# read version = "X.Y.Z" under [package], Z+=1, rewrite file
```

Acceptance: after bump, `cargo test` still green because tests use `CARGO_PKG_VERSION`.

### Task 13: Fingerprint JSONL + card

**Files:**
- Create: `src/boxes/fingerprint.rs`
- Create: `docs/gsv/fingerprints.jsonl` (may start empty or with a header comment **not** valid JSONL — **do not** put `#` comments; start empty file or one seed object)
- Create: `scripts/gsv-fingerprint.sh`
- Modify: `src/boxes/ui.rs` `render_fingerprints`, `CARD_NAMES`, ops group
- Test: `tests/gsv_fingerprint_contracts.rs`

**Interfaces:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fingerprint {
    pub ts: String,
    pub actor: String,   // owner | agent
    pub ide: String,     // cursor | opencode | unknown
    pub model: String,   // grok-4.6 | unknown
    pub agent: String,   // orchestrator | explore | ...
    pub version: String,
    pub git_head: Option<String>,
    pub band: Option<String>,
    pub summary: String,
}

pub fn append(path: &Path, fp: &Fingerprint) -> std::io::Result<()>;
pub fn latest(path: &Path, n: usize) -> Vec<Fingerprint>;
```

Path: `{repo}/docs/gsv/fingerprints.jsonl` (git-tracked). Never `data/`.

`GET /api/fingerprints?limit=20`

Commit trailers (drain close):

```
Gsv-Actor: agent
Gsv-Ide: cursor
Gsv-Model: grok-4.6
```

### Task 14: Band 146 close

Run bump + fingerprint append **in the same commit** as the band. Gate + push.

---

# Band 147 — README-level polish leftovers (PH-S2109…S2118)

Only what 143–146 did not absorb:

- Header density vs presentation shots (padding, RSS ticker always visible if feed non-empty — already loadRssTicker).
- Card `border-radius` / gap matching `docs/assets/presentations/gsv-galaxy-ui.png` (visual, not pixel-perfect).
- Docs index rows for Always-on spec; `GSV_ARCHITECTURE.md` live-copy note; README Quick start: `bash scripts/gsv-live.sh` instead of raw `cargo run` if 144 shipped.
- Stand-smoke: new cards `products` / `fingerprints` in `CARDS` list.
- Ratio hold.

---

## Self-review (spec coverage)

| Spec P0 | Task |
|---------|------|
| Live copy / cargo test without os error 5 | Task 6 |
| Offline during apply + SSE resync | Task 7 |
| Power menu z-index | Task 1 |
| Collapse leaves grid | Task 3 |
| Exclusive fullscreen + Esc | Task 2 |
| Type + chart scale | Task 4 |
| Products list/select/open/scan | Tasks 9–10 |
| Version bump per commit | Task 12 |
| Fingerprints | Task 13 |
| README polish remainder | Band 147 |

No TBD placeholders. Band 143 is the only work for the next `абракадабра` gsv drain.
