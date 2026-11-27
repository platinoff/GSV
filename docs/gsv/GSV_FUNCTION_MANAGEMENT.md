# GSV_FUNCTION_MANAGEMENT.md — функційний реєстр екосистеми (симбіоз)

**Status:** Canon, owner-ratified 2026-11-17 (агент-драфт, owner підтвердив ключові розподіли); matrix snapshot 2026-11-27, **current heads: GSV `git log -1` (snapshot `6adec96`, MCP 61 tool), rusty-engine `25b1d4e`, LinFS `5666918`, adoreble_wifi `141ede0`** · **Owner:** platinoff
**Companions:** [`GSV_ECOSYSTEM_SYMBIOSIS.md`](./GSV_ECOSYSTEM_SYMBIOSIS.md) (мама/тато) · [`PRODUCTS.md`](./PRODUCTS.md) (реєстр) · [`GSV_AGI_PATH.md`](./GSV_AGI_PATH.md)
**Rule:** poolAI FM (`poolAI/docs/catalog/FUNCTION_MANAGEMENT.md`) очолює цю матрицю в категорії «тато»; цей документ — екосистемна зводка, не заміна per-project FM.

## 0. Owner decisions 2026-11-17 (ratified)

| Рішення | Висновок |
|---------|----------|
| Метафора | **poolAI = тато** (генетичний фонт коду — перший проєкт власниці); **GSV = мама** (хаб — керує екосистемою, народжує плагіни, тримає все живе). Фактичний розподіл праці не міняється. |
| Глибина FM | **Повні FM лише для GSV + poolAI**; плагіни — скелет за потребою |
| mesh-node | **Зареєстровано** в PRODUCTS.md як Pi-агент (remote owner-gated) |
| OmniRoute lane split | **D2 — канон наміреного паралелізму** (див. §3): GSV OmniRouter = mesh/free lanes; omniroute = external-provider gateway |

## 1. Зводка по репо (знято 2026-11-17)

| Репо | Мова/стек | HEAD @ main (на момент зняття) | Ключові функції | Шар |
|------|-----------|-------------|----------------|-----|
| **poolAI** | Rust v0.2.2 (workspace) | e123ef05d | AI Mining Pool core + grid/orchestration · FM журнал (FUNCTION_MANAGEMENT.md) · WSP план (MiniCPM-35B) · enterprise/cloud HTTP API · ml/monitoring/memory/job engine · solana-adapter (Deferred) · FUNCTIONALITY_DIGEST · vision rev 499 (internal) | **тато** |
| **GSV** | Rust (axum) v0.248.0 | 51b1512 | vision server + boxes · MCP 61 tool · tickets board · keep-live/watchdog · mesh WASM dispatch · edge proxy · Galaxy UI · usage/fingerprints · loc-audit · hosts `cargo xtask` · 4-bit MoE numerics forward / expert-major pack / LOAD lane · ecosystem repair canon | **мама** |
| **telenetis** | Rust v0.2.0 (axum) | 21f7c9d | Telegram bot + Mini App · initData HMAC · snapshot/UI · ticket actions · **commutation plugin** | plugin |
| **llama-rs** | Rust v0.1.0 (workspace) | 84ec9b2 | llama.cpp-backed serve · edge worker · `bunke-rock` provider · Pi 4 build | plugin (engine) |
| **rusty-engine** | Rust v0.1.0 (workspace) | 06f8063 | Pure-Rust MoE inference (GGUF) · MiniCPM-V 4.7 target · OpenAI-compat serve · chatml · rollout logs | plugin (engine) |
| **mesh-node** | Rust (workspace) | 449726a | Pi 4 mesh executor (native matvec/layernorm/softmax/mlpblock/minilm) · `selftest`; **зареєстрований 2026-11-17** | plugin (mesh agent) |
| **GSV-WGPU** | Rust v0.1.0 | c381a4f | Universal WebGPU plugin: device-resource grants, browser mesh ring, `matvec` op · `GpuBroker` | plugin (mesh/browser) |
| **LinFS** | Rust v1.0.0 (workspace) | 141aab5 | ext4/xfs/btrfs/f2fs mount on Windows · chroot · Linux terminal · linfs-gui | plugin |
| **rebook** | Rust v0.1.0 | 6424975 | EPUB 3.2 + KDP pipeline (markdown→EPUB) · coloring raster engine | plugin |
| **ORR_DESKTOP** | Rust v0.1.0 (ed 2024) | 40298ab | tray screen recorder (WGC/GDI, OpenH264, MP4) | plugin (private) |
| **CoreBoard_Labs_shop** | Rust v0.1.0 (axum) | 91480c2 | dropshipping PC-parts shop · compat engine · listing_sync | plugin (private) |
| **sport-forcast** | Rust v0.1.0 | 3d8341e | Forecast journal (PL football first) · `:9741` | plugin (private) |
| **omniroute** | Node/TS v3.8.50 | e3d202c | Unified LLM router (290 providers) · OpenAI-compat API · MCP/A2A · PWA | plugin (node, D2) |
| **platinoff-profile** | static (html/svg) | 5d81430 | Galaxy Mission Control profile page | outside (no FM) |
| **freebeat.ai** | scaffold (images) | 87daf9e | private plugin start (code absent) | outside (no FM) |

## 2. Межі шарів (що де живе)

| Функція | poolAI (тато) | GSV (мама) | plugin |
|---------|---------------|------------|--------|
| FM журнал / band queue `PH-S*` · poolai-vision-sync | ✅ тато | ❌ (лише mirror id) | — |
| WSP pipeline (MiniCPM-35B) · grid/discovery · enterprise API | ✅ тато | ❌ | — |
| UI Galaxy / cards / tickets board | — | ✅ мама | — |
| Keep-live / watchdog / heartbeat | ❌ | ✅ мама | llama-rs/rusty пишуть heartbeat-файл |
| Inference HTTP API (OpenAI-compat) | ❌ **ніколи** | ❌ **ніколи** | llama-rs / rusty-engine — єдині місця |
| LLM provider routing | ❌ | ✅ `OmniRouter` (mesh/free lanes) | omniroute = D2 (external gateway) |
| Mesh dispatch (WASM/peers) | ❌ | ✅ мама (`/api/wgpu/*`) | GSV-WGPU = browser lane; mesh-node = Pi lane |
| Telegram bus / Godfather | ❌ | ✅ мама | telenetis = **commutation** only |
| Product runtime (ports/target/) | own tree | own tree | own tree |

## 3. Дублі (D-catalog, owner-вердійки 2026-11-17)

| # | Функція | Де існує | Вердикт |
|---|---------|----------|---------|
| D1 | OpenAI-compat HTTP serve | llama-rs, rusty-engine | **намірені паралельні engine-плагіни** — кожен власний; GSV не хостить жодного |
| D2 | LLM routing / fallback | GSV `OmniRouter` (mesh+free), omniroute (290 providers) | **КАНОН (owner 2026-11-17): намірені паралельні lane-и** — GSV = mesh/free, omniroute = external gateway; жоден не absorb'ить іншого |
| D3 | Keep-live watchdog | GSV | **єдиний** |
| D4 | FM/management журнал | poolAI | **єдиний канон у poolAI**; GSV бере лише `t-*` id |
| D5 | Telegram (бот/апи) | GSV (bus/state) + telenetis (UI/commutation) | **розподілена пара** — більше ніде |
| D6 | Ratio/loc-audit | GSV, poolAI, LinFS, llama-rs | **намірено в кожному** (свій gate ≥95/96) |
| D7 | Logging rolling appender | GSV, rusty-engine, llama-rs, telenetis | **намірено в кожному** |
| D8 | mesh executor | `crates/mesh-wasm` (hub) · GSV-WGPU (browser) · mesh-node (Pi) | **одна функція, три lane-и** — один програм, не дублікати; mesh-node **зареєстровано 2026-11-17** |
| D9 | 35B-A3B навантаження | rusty-engine (serve) + GSV mesh (`weights_matvec`) | GSV dispatch · engine execute — норм |

## 4. Що з цього випливає для симбіозу

- **мама (GSV) строго операційний**: UI, tickets, keep-live, mesh-dispatch, MCP sandbox, edge proxy, OmniRouter-каталог. Будь-яка спроба додати inference або FM-журнал = порушення межі.
- **тато (poolAI) строго management**: FM, grid, enterprise API, WSP. GSV НЕ копіює FM; poolAI не тягає Galaxy.
- **plugins містять product runtime**: engines = інференс; LinFS = FS; rebook = EPUB; ORR = capture. Плагін каже хабу «я живий/готовий» через heartbeat — не більш.
- **Намірені пари/лаїни** (D1, D2, D5, D6, D7, D8) — **features, not bugs**.

## 5. Окремі розрив на функційному рівні

1. ~~**mesh-node не у реєстрі**~~ — **вирішено 2026-11-17**: зареєстровано в PRODUCTS.md (Pi-агент; remote owner-gated).
2. **omniroute** — ratio n/a (node); policy-down до owner-go (keep-live AGENTS); D2 — канон.
3. **freebeat.ai / platinoff-profile** — поза периметром (scaffold/static; no FM, unknown declared).
4. **poolAI CI (`test-ci`)** — RAM/EDR блок; до тих пір poolAI = "declared funcs unverified in CI".
5. **vision rev** — GSV 516 (ecosystem canon) / poolAI 499 (internal); mirror rule — відкрите owner-питання (P6.1 phase 5).

## 6. FM по кожному проєкту — owner ratified 2026-11-17

- **GSV** — власний повний FM: [`FUNCTION_MANAGEMENT.md`](./FUNCTION_MANAGEMENT.md) (цей самий репо, мама-хаб).
- **poolAI** — власний повний FM: `poolAI/docs/catalog/FUNCTION_MANAGEMENT.md` (тато-план).
- **Плагіни** — скелет `docs/FUNCTION_MANAGEMENT.md` (оголошені функції → статус → звірка → дублі → owner-рішення) **тільки за окремим owner-запитом**; масового розгортання немає. Скелет форми:

```markdown
# <repo> FUNCTION_MANAGEMENT (симбіоз)
**Companion:** GSV docs/gsv/GSV_FUNCTION_MANAGEMENT.md · **Шар:** мама/тато/plugin
## 1. Оголошені функції (README / Cargo.desc / src/)
## 2. Звірка (що підтверджено тестами/CI)
## 3. Дублі з іншим репо (посилання на майстер §3)
## 4. Owner-рішення / наступні кроки
```
