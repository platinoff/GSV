# MCP always-on catch-up Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose always-on boxes (products, scan, watchdog, SW, fingerprints) on `gsv_mcp_openbot` so the next `абракадабра` gsv drain has a real band instead of a blank owner pick.

**Architecture:** Add five tools and two `gsv://` resources in `src/mcp.rs` that call existing `boxes::*::wire` / `scan` functions. No new HTTP routes. No MCP open/apply. Discovery counts follow `TOOL_NAMES` / `RESOURCE_URIS`.

**Tech Stack:** Rust 2021, existing MCP JSON-RPC (`handle_value`), axum `/mcp`, tokio tests.

**Spec:** [`docs/gsv/GSV_POST_ALWAYS_ON.md`](../../gsv/GSV_POST_ALWAYS_ON.md)

## Global Constraints

- Ratio: `cargo run --bin gsv-loc-audit -- --stretch-96` ≥ 96%; no Python; no `vision.js` port.
- Bind: `127.0.0.1:9999` default; no LAN widen; no Grok Bot tunnel.
- MCP tools wrap boxes; they do not spawn Explorer or exit the server.
- Shell: `C:\msys64\usr\bin\bash.exe -lc '…'`; one `cargo` at a time; `unset CARGO_TARGET_DIR`.
- Do not kill `target/live/gsv-server.exe` before `cargo test`.
- Never stage `data/*`, `.env*`, `*.pem`, `comitmsg/*.txt`.
- Drain: ≤10 PH-S*; **one commit** at band close (`--band 151` + fingerprint). No mid-push.

## File map

| File | Role |
|------|------|
| `src/mcp.rs` | `TOOL_NAMES`, `tools_list`, `call_tool`, `RESOURCE_URIS`, `RESOURCES`, `PROMPTS` `gsv_drain` text, unit tests (`assert_eq!(names.len(), 31)`) |
| `tests/gsv_mcp_contracts.rs` | HTTP `/mcp` tools/list length 31; new tool calls; resource read; unknown product id |
| `src/boxes/ui.rs` | `render_mcp` already uses `tool_count` from wire — only touch if a test needs a marker |
| `docs/gsv/GSV_MCP_OPENBOT.md` | Tools table 31; resources 8; horizon 151 ✅ after close |
| `docs/gsv/GSV_BOXES.md` / `GSV_SERVER.md` / `GSV_ARCHITECTURE.md` | tool_count / watchdog row |
| `README.md` | MCP line **31 tools** + **8 resources** |
| `docs/HANDOFF_NEW_SESSION.md` / `NEXT_SESSION_PROMPT.md` / `MEMORY.md` | band 151 ✅ · next = 152 |
| `docs/gsv/GSV_TECH_ROADMAP.md` | PH-S2149…S2158 ✅ |
| `docs/VISION.md` | 26 → 31 if it still says 26 tools |

---

# Band 151 — MCP catch-up (PH-S2149…S2158)

Next `абракадабра` on **gsv**: S0 → clippy → **this band**. Do not skip to 152.

### Task 1: Scope + queue (PH-S2149)

**Files:**
- Modify: `docs/vision/extensions.json` `active_sprint` → `PH-S2149` (vision-sync at band close)
- Modify: `docs/gsv/GSV_TECH_ROADMAP.md` band 151 table (unchecked until each sprint lands)

**Interfaces:**
- Consumes: spec P0 table
- Produces: queued PH-S2149…S2158 visible in HANDOFF (already queued this session)

- [ ] **Step 1:** Confirm spec still matches code (`products::scan`, `watchdog::wire`, `sw::wire`, `fingerprint::wire` / `clamp_limit`).
- [ ] **Step 2:** Set `active_sprint` only when implementation starts (not in the spec-queue commit).
- [ ] **Step 3:** Do not commit yet — wait for band close.

### Task 2: `gsv_products` + `gsv_products_scan` (PH-S2150)

**Files:**
- Modify: `src/mcp.rs` (`TOOL_NAMES`, `tools_list`, `call_tool`)
- Test: `src/mcp.rs` unit + `tests/gsv_mcp_contracts.rs`

**Interfaces:**
- Consumes: `crate::boxes::products::{wire, scan}`; `AppState.repo_root`; `product_selected` is **not** required for scan (id arg required)
- Produces: tools `gsv_products`, `gsv_products_scan`

- [ ] **Step 1: Write the failing test**

In `src/mcp.rs` `tools_list_covers_box_wraps`, add `"gsv_products"` and `"gsv_products_scan"` to the array and change:

```rust
assert_eq!(names.len(), 31);
```

In `tests/gsv_mcp_contracts.rs` (same file as the current `assert_eq!(names.len(), 26)`):

```rust
assert!(names.contains(&"gsv_products"));
assert!(names.contains(&"gsv_products_scan"));
assert_eq!(names.len(), 31);

#[tokio::test]
async fn products_scan_unknown_id_is_tool_error() {
    let app = app();
    let (status, body) = mcp_post(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 80,
            "method": "tools/call",
            "params": { "name": "gsv_products_scan", "arguments": { "id": "nope" } }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"]["isError"], true);
}

#[tokio::test]
async fn products_scan_gsv_ok() {
    let app = app();
    let (status, body) = mcp_post(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 81,
            "method": "tools/call",
            "params": { "name": "gsv_products_scan", "arguments": { "id": "gsv" } }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"]["isError"], false);
    let text = body["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert!(text.contains("\"ok\":true") || text.contains("\"ok\": true"), "{text}");
    assert!(text.contains("git_head"), "{text}");
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd /s/rust/GSV && cargo test --lib tools_list_covers_box_wraps -- --nocapture
cd /s/rust/GSV && cargo test --test gsv_mcp_contracts products_scan -- --nocapture
```

Expected: FAIL — `names.len()` still 26; unknown tool `gsv_products_scan`.

- [ ] **Step 3: Minimal implementation**

Append to `TOOL_NAMES` (keep existing order, add at end):

```rust
    "gsv_preview",
    "gsv_products",
    "gsv_products_scan",
    "gsv_watchdog",
    "gsv_sw",
    "gsv_fingerprints",
];
```

(Watchdog/sw/fingerprints land in the next tasks; adding names now without `call_tool` arms will fail `unknown tool` on call tests — add names **together with** their `call_tool` arms in tasks 2–4, or add all five names in this task and stub arms that `tool_err("not implemented")` only if a test would call them. Prefer: add **all five names + all five arms** in this task if cheaper, then tasks 3–4 are contracts. Executor: one `call_tool` match update for all five is OK as long as tests are red first.)

`tools_list()`: add

```rust
        tool("gsv_products", "VDT environment products (workspace ∪ sibling git ∪ kit).", object_schema()),
        tool(
            "gsv_products_scan",
            "Scan one discovered product (git HEAD, HANDOFF/NEXT, kind). id required.",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Product id from gsv_products (e.g. gsv)." }
                },
                "required": ["id"]
            }),
        ),
```

`call_tool`:

```rust
        "gsv_products" => {
            let sel = state.product_selected.lock().ok().and_then(|g| g.clone());
            tool_ok(crate::boxes::products::wire(&state.repo_root, sel.as_deref()))
        }
        "gsv_products_scan" => {
            let id = arg_str(&args, "id");
            if id.is_empty() {
                tool_err("id required")
            } else {
                match crate::boxes::products::scan(&state.repo_root, &id) {
                    Ok(s) => tool_ok(serde_json::to_value(&s).unwrap_or_else(|_| json!({"ok":false}))),
                    Err(e) => tool_err(e),
                }
            }
        }
```

Use the same `product_selected` access pattern as `api_products_select` (`state.product_selected.lock()`). If the field name differs, match `src/server/mod.rs`.

- [ ] **Step 4: Run tests to verify they pass** (products tests only; watchdog tests still fail until task 3).
- [ ] **Step 5:** Do not commit yet.

### Task 3: `gsv_watchdog` + `gsv_sw` (PH-S2151)

**Files:**
- Modify: `src/mcp.rs`
- Test: `tests/gsv_mcp_contracts.rs`

**Interfaces:**
- Consumes: `watchdog::wire(&state.repo_root)`, `sw::wire()`
- Produces: tools `gsv_watchdog`, `gsv_sw`

- [ ] **Step 1: Failing tests**

```rust
assert!(names.contains(&"gsv_watchdog"));
assert!(names.contains(&"gsv_sw"));

#[tokio::test]
async fn watchdog_and_sw_tools_ok() {
    let app = app();
    for (id, name, needle) in [
        (82u64, "gsv_watchdog", "alive"),
        (83, "gsv_sw", "gsv-shell-v1"),
    ] {
        let (status, body) = mcp_post(
            &app,
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "tools/call",
                "params": { "name": name, "arguments": {} }
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{name}");
        assert_eq!(body["result"]["isError"], false, "{name}");
        let text = body["result"]["content"][0]["text"].as_str().unwrap_or("");
        assert!(text.contains(needle), "{name} {text}");
    }
}
```

- [ ] **Step 2:** `cargo test --test gsv_mcp_contracts watchdog_and_sw -- --nocapture` → FAIL.
- [ ] **Step 3: Implementation**

```rust
        tool("gsv_watchdog", "Live watchdog heartbeat (target/live/watchdog.json).", object_schema()),
        tool("gsv_sw", "Service Worker shell cache discovery (cache name + precache urls).", object_schema()),
```

```rust
        "gsv_watchdog" => tool_ok(crate::boxes::watchdog::wire(&state.repo_root)),
        "gsv_sw" => tool_ok(crate::boxes::sw::wire()),
```

- [ ] **Step 4:** Same test PASS.
- [ ] **Step 5:** Do not commit yet.

### Task 4: `gsv_fingerprints` (PH-S2152)

**Files:**
- Modify: `src/mcp.rs`
- Test: `tests/gsv_mcp_contracts.rs`

**Interfaces:**
- Consumes: `fingerprint::clamp_limit`, `fingerprint::wire`
- Produces: tool `gsv_fingerprints`

- [ ] **Step 1: Failing test**

```rust
assert!(names.contains(&"gsv_fingerprints"));

#[tokio::test]
async fn fingerprints_tool_ok() {
    let app = app();
    let (status, body) = mcp_post(
        &app,
        json!({
            "jsonrpc": "2.0",
            "id": 84,
            "method": "tools/call",
            "params": { "name": "gsv_fingerprints", "arguments": { "limit": 3 } }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"]["isError"], false);
    let text = body["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert!(text.contains("fingerprints.jsonl"), "{text}");
}
```

- [ ] **Step 2:** Run → FAIL.
- [ ] **Step 3:**

```rust
        tool(
            "gsv_fingerprints",
            "Drain fingerprints JSONL (actor / IDE / model / time).",
            json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Latest N rows (default 20, cap 100)." }
                }
            }),
        ),
```

```rust
        "gsv_fingerprints" => {
            let limit = crate::boxes::fingerprint::clamp_limit(
                args.get("limit").and_then(|v| v.as_u64()).map(|n| n as usize),
            );
            tool_ok(crate::boxes::fingerprint::wire(&state.repo_root, limit))
        }
```

- [ ] **Step 4:** PASS.
- [ ] **Step 5:** Do not commit yet.

### Task 5: Resources (PH-S2153)

**Files:**
- Modify: `src/mcp.rs` `RESOURCE_URIS` + `RESOURCES`
- Test: existing resources tests + new read

**Interfaces:**
- Consumes: `preview::resolve` (unchanged confine)
- Produces: `gsv://docs/fingerprints`, `gsv://docs/post-always-on`

- [ ] **Step 1: Failing test** in `src/mcp.rs` (next to `assert_eq!(RESOURCES.len(), RESOURCE_URIS.len())`):

```rust
assert_eq!(RESOURCE_URIS.len(), 8);
assert!(RESOURCE_URIS.contains(&"gsv://docs/fingerprints"));
assert!(RESOURCE_URIS.contains(&"gsv://docs/post-always-on"));
```

Add a `resources/read` call for `gsv://docs/post-always-on` that expects markdown containing `band 151`. Traversal `gsv://docs/../` still `-32602`.

- [ ] **Step 2:** Run → FAIL (len still 6).
- [ ] **Step 3:** Append resource specs:

```rust
    ResourceSpec {
        uri: "gsv://docs/fingerprints",
        name: "Drain fingerprints",
        description: "Append-only drain fingerprint JSONL.",
        mime: "application/jsonl",
        rel: "docs/gsv/fingerprints.jsonl",
    },
    ResourceSpec {
        uri: "gsv://docs/post-always-on",
        name: "Post always-on spec",
        description: "MCP catch-up conception for band 151+.",
        mime: "text/markdown",
        rel: "docs/gsv/GSV_POST_ALWAYS_ON.md",
    },
```

Keep `RESOURCE_URIS` in the same order.

- [ ] **Step 4:** PASS.
- [ ] **Step 5:** Do not commit yet.

### Task 6: `gsv_drain` prompt (PH-S2154)

**Files:**
- Modify: `src/mcp.rs` `PROMPTS` entry `gsv_drain`
- Test: `prompts/get` for `gsv_drain`

**Interfaces:**
- Consumes: prompt name `gsv_drain` (unchanged)
- Produces: new `text` body

- [ ] **Step 1: Failing test**

```rust
#[tokio::test]
async fn drain_prompt_names_always_on_tools() {
    let s = state();
    let got = rpc(&s, 90, "prompts/get", json!({ "name": "gsv_drain" })).await;
    let text = got["result"]["messages"][0]["content"]["text"]
        .as_str()
        .unwrap_or("");
    assert!(text.contains("gsv_products"), "{text}");
    assert!(text.contains("gsv_products_scan"), "{text}");
    assert!(text.contains("gsv_watchdog"), "{text}");
    assert!(text.contains("gsv://docs/next"), "{text}");
    assert!(text.contains("mid-drain"), "{text}");
}
```

- [ ] **Step 2:** FAIL (old text has `gsv_vision_queue` only).
- [ ] **Step 3:** Replace `gsv_drain` `text` with:

```text
Start a GSV VDT drain. Read gsv://docs/next and gsv://docs/post-always-on. Call gsv_products, gsv_products_scan with id=gsv (or the owner pick), and gsv_watchdog. Propose the next ≤10 PH-S* after the last closed band. Do not push mid-drain. Shell is MSYS2 bash.
```

- [ ] **Step 4:** PASS.
- [ ] **Step 5:** Do not commit yet.

### Task 7: Discovery + Galaxy card (PH-S2155)

**Files:**
- Modify: none if `http_info` already uses `TOOL_NAMES.len()` (it does)
- Test: `tests/gsv_mcp_contracts.rs` `assert_eq!(names.len(), 31)`; stand-smoke still hits `GET /mcp`

**Interfaces:**
- Consumes: `http_info` / `render_mcp`
- Produces: Galaxy card `tools 31 · resources 8`

- [ ] **Step 1:** Grep for hardcoded `26` in `src/mcp.rs` and `tests/gsv_mcp_contracts.rs` — replace with `31` or `TOOL_NAMES.len()`.
- [ ] **Step 2:** `cargo test --test gsv_mcp_contracts -- --nocapture` PASS.
- [ ] **Step 3:** Do not commit yet.

### Task 8: Contracts sweep (PH-S2156)

**Files:**
- Test: `tests/gsv_mcp_contracts.rs` + `src/mcp.rs` `#[cfg(test)]`
- Optionally: `tests/gsv_stand_smoke_contracts.rs` if it pins tool_count 26

- [ ] **Step 1:** `rg "26 tools|tool_count.: 26|names.len\\(\\), 26" --glob '*.rs'`
- [ ] **Step 2:** Fix remaining Rust pins.
- [ ] **Step 3:**

```bash
cd /s/rust/GSV && cargo test --lib mcp -- --nocapture
cd /s/rust/GSV && cargo test --test gsv_mcp_contracts -- --nocapture
```

Expected: PASS.
- [ ] **Step 4:** Do not commit yet.

### Task 9: Docs canon (PH-S2157)

**Files:**
- Modify: `docs/gsv/GSV_MCP_OPENBOT.md` (26 → 31 tools, 6 → 8 resources; mark band 151 landed)
- Modify: `docs/gsv/GSV_BOXES.md` MCP row
- Modify: `docs/gsv/GSV_SERVER.md` `/mcp` row
- Modify: `docs/gsv/GSV_ARCHITECTURE.md` MCP sentence + `watchdog/` component row if still missing
- Modify: `README.md` MCP paragraph
- Modify: `docs/VISION.md` if it still says 26 tools
- Modify: `docs/HANDOFF_NEW_SESSION.md` / `NEXT_SESSION_PROMPT.md` / `MEMORY.md` / `GSV_TECH_ROADMAP.md` / `GSV_POST_ALWAYS_ON.md` status → band 151 ✅ · next = 152

**Interfaces:** none.

- [ ] **Step 1:** Update counts to 31 / 8.
- [ ] **Step 2:** HANDOFF next drain = band **152** (select), not 153.
- [ ] **Step 3:** Do not commit yet.

### Task 10: Ratio + band close (PH-S2158)

**Files:** `Cargo.toml` version via bump script; `docs/gsv/fingerprints.jsonl`

- [ ] **Step 1:**

```bash
export PATH="/c/Users/${USER:-${USERNAME}}/.cargo/bin:$HOME/.cargo/bin:/ucrt64/bin:/usr/bin:$PATH"
export RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-gnu
cd /s/rust/GSV
unset CARGO_TARGET_DIR
cargo fmt -- --check
cargo clippy --all-targets
cargo test
cargo run --bin gsv-loc-audit -- --stretch-96
```

Expected: fmt clean, clippy 0, tests green, stretch ≥ 96%.

- [ ] **Step 2:** `bash bin/record-test-speed.sh` and `bash bin/record-rust-diagnostics.sh` (or `--skip-run` if tests just ran). `bash bin/gsv-vision-sync.sh` then `--check`.
- [ ] **Step 3:** `bash scripts/gsv-bump-version.sh --band 151` then `bash scripts/gsv-fingerprint.sh`.
- [ ] **Step 4:** One commit (sprint files only — not `git add -A`, not `data/*`, not `comitmsg/*.txt`). Trailers `Gsv-Actor` / `Gsv-Ide` / `Gsv-Model`.
- [ ] **Step 5:** `git push origin main` + summary in chat.

---

# Band 152 — MCP products select (PH-S2159…S2168) ✅

Landed 2026-08-18. `gsv_products_select` `{id}` → same allowlist as `POST /api/products/select`.
`gsv_products_scan` may omit `id` when `AppState` has a selection.
Still **no** `gsv_products_open`, **no** `update/apply`.
Prompt `gsv_drain` uses selected id.
Close: `--band 152` + fingerprint; next = 153.

# Band 153 — Watchdog card + fingerprint model (later)

Owner pick. Dedicated ops card `watchdog`; optional fingerprint `model` from Cursor session. Grok Bot tunnel stays opt-in.
