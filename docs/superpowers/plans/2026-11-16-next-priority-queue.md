# Next-priority queue (owner-driven order, post 2026-11-16 drain)

**Date:** 2026-11-16 (після logging ecosystem pass + WASM LAN + ChatML + MC-2 close) · **Owner steer:** «постав по логічності в плани і продовжуємо» · **Канон:** цей документ + [`2026-11-16-ecosystem-review.md`](2026-11-16-ecosystem-review.md)

## 1. Goal

Зафіксувати чергу, яка **послідовно і фізично можлива** саме тепер — після того, як майже всі пункти з ECO drain (2026-11-15) закриті/зрушені. Порядок має враховувати залежності, а не просто «починаємо зверху».

## 2. Поточний стан питань (чинна симбіотика 2026-11-16 23:44)

**✅ Закрито в цьому заході:**
- Logging ecosystem pass — `GSV → daily rolling telenetis parity` (commit `a277be5`); rusty-engine std-only rolling (`fe74d37`); llama-rs rolling default (`823956e`, rolling log applied — Pi build BLOCKED on bindgen).
- `/sys` conky FULL telemetry — UA Client Hints/Model/Platform/Arch/Bitness/Battery/saveData/GPU buf/InvocationsInWorkgroup/UA-MIA-divmetrics/Noise/busy/Link/all pulled into live page with auto-expand on peer revive + join-feed (`405b3fe`).
- WASM SIMD lane — `crates/mesh-wasm` (cdylib ~26 KB) → `/assets/mesh-wasm.wasm`; `matvec_shard` + `weights_matvec` partial kernels with SIMD128; live err 0 on blk.1.ffn_gate_shexp (`8108cc3`, `ed024ee`, `527a524`).
- `/sys` Resources card — dual-polyline canvas + summary (cpu avg 27.5% / ram 86.3% / free 75 GB) (`1fd42cf`).
- ChatML fix — `src/chatml.rs` (new, std-only, no serde), `RUSTY_CHAT_TEMPLATE=raw` revert flag; live HTTP 200 on Qwen/MiniCPM; t-1794778713443485500 closed (`8a5a819` board `368da0e`).
- MC-2 — real-weight Q8 shards across 2+ devices (g-98/99 Pi4 + g-84/g-85/g-86 fleet); max_err_model ≤ 1e-6; ticket t-1794758435803909000 closed (`4e6c15b`/`a251344`).
- ECO-7.4 — Rust 1.100 re-gate board-closed with fresh evidence (`d74099e`).
- Python-purge — 0 `.py`/`.pyc` у GSV і mesh-node (те цього дня).
- mesh-node agent full ops — layernorm/softmax/mlpblock/minilm executor + selftest; peer #36 (`5f0bfe8`, `194b6e9`).
- Pi SSH — `owner_session.key` (ed25519, пароль у `images/gen_owner_session.sh`); `ssh_agent bash -lc`+`askpass` works (PH-2 skipped → built-in scripts, no external key needed anymore).
- `cargo xtask sys-history` — ring cap 1440 (24 h); `/api/sys/history?limit=` live feed; 3 samples observed; live serve echoes `/api/sys/metrics` for peers.
- Hub redeployed multiple times — own faults from raw `taskkill //F` redeploys (RAM 500–750 MB free / wave); going for `/api/update/apply` path going forward.

**⛔ Owner-gated (те, що не може виконати агент без власника):**
1. RAM/EDR стабільність хаба (>10 випадків падіння; RAM 750–840 MB вільних під навантаженням → використання набір build/test jobs поза хабом; не торкає хаб фізіологічний).
2. mesh-node репо remote — потрібен `github.com/platinoff/mesh-node`. Без цього образ Pi4 + код агента лише на цьому боксі.
3. PH-A5 dedicated ≥ 24 GB node — блокує повний 35B forward + PH-A3 (EAM) + PH-1206 (35B MoE).
4. WSP Phase 2 — тільки після explicit go (попередній ROLLBACK усе ще в силі: MC-6 не на очах).

**🟡 In-flight (частково виконано, або рішення потрібно від власника на цьому кроці):**
5. ECO-8.2 — llama-rs `llama-cpp-sys-2` 0.1.154 → 0.1.161 — лише rolling log застосовано; збірка/валідація Pi BLOCKED на `bindgen-0.72.1`/`clang_sys::load()` → `Unable to find libclang` на aarch64 Debian 13. Можливі шляхи: (a) повний rebuild PC із `CARGO_BUILD_JOBS=1` та hub down; (b) CI хост на Pi з кращим llvm/libclang; (c) skip ECO-8.2 (лишається pinned 0.1.154 + patches).
6. llama-rs Pi build — owner рішення про #5.

**🔧 Agent-actionable (виконуємо далі):**
7. Auto-fill `/sys Resources · 24h` — запустити `cargo xtask sys-history` автоматично у watchdog tick (кожен 60 s) замість ручного/окремого виклику (тоді картка наповниться без окремої cron).
8. Допорожнити PLAN doc — цей документ (`2026-11-16-next-priority-queue.md`).

## 3. Логічний порядок

**Спочатку власник (незамінний спільний крок) → інженера (agent can continue) → власник (що він воліє).**
A. **Owner**: step 1 RAM/EDR → step 2 mesh-node remote → step 3 PH-A5 dedicated node (або Pi-fleet рішення).
B. **Agent**: step 7 watchdog autoload `/sys history` — свіжу картку/рис без ручного запуку; нагорода за це — live 24 h графік без зовнішнього cron.
C. **Owner + Agent**: step 5/6 llama-rs ECO-8.2 — рішення і виконання з відповідним гейтом.
D. **Owner**: step 4 WSP Phase 2 — go/no-go owner.

## 4. Acceptance per step

- **Step 1 (owner RAM/EDR):** `cat /proc/meminfo | grep -E "MemAvailable|MemFree"` ≥ 1500 MB за постійне навантаженням embed-runtime; EDR process list clean; хаб витримує `cargo test --release` без OOM-kill.
- **Step 2 (owner mesh-node remote):** `github.com/platinoff/mesh-node` створено, локальний HEAD `5f0bfe8` запушено; `cargo xtask products --state` показує remote; owner decision on `mesh-node/images/*` (gitignored обгортка зображення: repo only бачить код агента, не образ).
- **Step 3 (owner PH-A5 dedicated node):** ≥ 24 GB RAM node виділено (хост-VM/машина); `mesh/capability` shows it ready; 35B blk.1 forwarding — виконується з `max_err_model ≤ 1e-2`; PH-1206 (MoE core + ExpertCache) відкривається.
- **Step 4 (owner WSP Phase 2):** explicit go від власника; після того — MC-6 цикл (1B fixture sweep) + 2× nullings; без нього не робимо.
- **Step 5 (ECO-8.2 + llama-rs Pi):** either (PC full rebuild + OWNER-GATED tests + push) or (owner decision: skip; keep 0.1.154 pin with 6 patches in registry).
- **Step 7 (agent — watchdog auto-sys-history):** `sys-metrics-history.jsonl` поповнюється без ручного виклику; `/sys Resources` показує повний 24 h графік (min/max CPU, RAM, peers).

## 5. Non-goals

- Не класти GGUF / weights / Uk-golden fixtures у git.
- Не включати node / Python / npm — це Rust ecosystem.
- Не випускати жодного heavyweight weights_matvec на неверифікованих пірів (без ready_proofs).
- Не торкати `:8091` / rebook books / теги냗 не торкає хаб board.

## 6. Канон-посилання

- [`2026-11-16-ecosystem-review.md`](2026-11-16-ecosystem-review.md) — стани продуктів + ресерч;
- [`2026-11-15-minicpm-mesh-wsp.md`](2026-11-15-minicpm-mesh-wsp.md) — mesh архітектура та WSP плагіни;
- `docs/HANDOFF_NEW_SESSION.md` — фінальна нотатка сесії;
- `docs/NEXT_SESSION_PROMPT.md` — стани/черга.
