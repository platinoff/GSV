# GSV_FUNCTION_MANAGEMENT.md — функційний реєстр екосистеми (симбіоз)

**Status:** Draft for owner review (agent-drafted 2026-11-17, опирається на живий знімок репо) · **Owner:** platinoff
**Companions:** [`GSV_ECOSYSTEM_SYMBIOSIS.md`](./GSV_ECOSYSTEM_SYMBIOSIS.md) (тато/мама) · [`PRODUCTS.md`](./PRODUCTS.md) (реєстр) · [`GSV_AGI_PATH.md`](./GSV_AGI_PATH.md)
**Rule:** poolAI FM (`poolAI/docs/catalog/FUNCTION_MANAGEMENT.md`) очолює цю матрицю в категорії «мама»; цей документ — екосистемна зводка, не заміна per-project FM.

## 0. Правило класифікації

| Тег | Значення |
|-----|----------|
| **Implemented** | Є в коді + згадка в README/HANDOFF/тестах |
| **Partial** | Є API/модуль; wire неповний або без тестів |
| **Planned** | Відкритий рядок у NEXT/плані |
| **Concept-only** | Описано в концепті; цільової реалізації немає |
| **Dup** | Та сама функція в ≥2 репо — позначені рядки нижче |

Кожен рядок: **функція → де живе → статус → шар симбіозу** (тато=GSV · мама=poolAI · plugin=власне дерево).

## 1. Зводка по репо (знято 2026-11-17)

| Репо | Мова/стек | HEAD @ main | Ключові функції (джерело: Cargo/package.json + src/ + README) | Шар |
|------|-----------|-------------|-------------------------------------------------------------|-----|
| **GSV** | Rust (axum) v0.248.0 | 28753cf | vision server + boxes (Tracker, SLI, Toolchain, IDE, Update, Preview, Terminal, Hooks) · MCP 60-тул / resources / prompts / sessions · tickets board (claim/done/error/reclaim, lease) · keep-live + watchdog · mesh WASM dispatch + `/api/wgpu/*` · edge proxy (VDC §6) · Galaxy UI (tree cards) · usage/fingerprints · loc-audit · APK bridge · hosts `cargo xtask` (products/disk/sync/intel/rules-check) | **тато** |
| **poolAI** | Rust v0.2.2 (workspace) | e123ef05d | AI Mining Pool core + grid/orchestration · FM журнал (FUNCTION_MANAGEMENT.md) · WSP план (MiniCPM-35B кандидат) · enterprise/cloud HTTP API · ml/monitoring/memory/job engine · solana-adapter (Deferred) · FUNCTIONALITY_DIGEST · vision rev 499 (internal) | **мама** |
| **telenetis** | Rust v0.2.0 (axum, sqlx? ні — in-mem + GSV) | 21f7c9d | Telegram bot + Mini App (initData HMAC, snapshot/UI) · **commutation plugin**: bus poll/forward, tickets claim/done/error/reclaim через GSV, ranks UI pointer · `telenetis_live` binary · Docker/systemd deploy | plugin (commutation) |
| **llama-rs** | Rust v0.1.0 (workspace: xtask) | 84ec9b2 | llama.cpp-backed serving (HTTP + OpenAI-compatible `/v1/chat/completions`) · edge worker (`llama_edge`) · local provider `bunke-rock` + `llama_heartbeat.json` · Pi 4 build (bindgen/libclang, llama-cpp-sys-2 =0.1.154) · rolling logs | plugin (engine) |
| **rusty-engine** | Rust v0.1.0 (workspace: xtask) | 06f8063 | Pure-Rust MoE inference (GGUF), MiniCPM-V 4.7 target · serve (OpenAI-compatible) · chatml · activation-audit (INKLING candidates) · mesh-orchestrated expert distribution | plugin (engine) |
| **mesh-node** | Rust (workspace: crates/mesh-node) | 449726a | WASM/native matvec executor для Pi 4; `selftest`; agent-Pi bench ops (layernorm/softmax/mlpblock/minilm); Pi↔hub dispatch | plugin (mesh agent; **unregistered**) |
| **GSV-WGPU** | Rust v0.1.0 | c381a4f | Universal WebGPU plugin: device-resource grants, browser mesh ring (join/poll/result), `matvec` op (g-72 verified) · `GpuBroker` scopes · provider catalog | plugin (mesh/browser) |
| **LinFS** | Rust v1.0.0 (workspace: linfs-core/block/fs/mount/chroot/terminal/cli/gui + xtask) | 141aab5 | ext4/xfs/btrfs/f2fs mount on Windows · chroot · Linux terminal · linfs-gui · linfs-loc-audit stretch-96 | plugin |
| **rebook** | Rust v0.1.0 (edition 2024) | 6424975 | EPUB 3.2 + KDP pipeline (markdown→EPUB) · coloring raster engine (art check/brief/draft/KDP PNG→JPEG) · book tool (`build/` outputs) | plugin (private-ish) |
| **ORR_DESKTOP** | Rust v0.1.0 (edition 2024) | 40298ab | tray screen recorder: WGC/GDI capture, OpenH264 encode, MP4 mux, zero external processes | plugin (private) |
| **CoreBoard_Labs_shop** | Rust v0.1.0 (axum, tera/leptos? — local web UI) | 91480c2 | dropshipping PC-parts shop: compatibility engine, account-health, amazon_bridge, listing_sync, FNsku, analytics | plugin (private) |
| **sport-forcast** | Rust v0.1.0 | 3d8341e | Forecast journal (Poland football first) · private, binary releases · port `:9741` | plugin (private) |
| **omniroute** | Node/TS v3.8.50 | e3d202c | Unified LLM router (290 providers) · OpenAI-compatible API · MCP/A2A · RTK+Caveman compression · PWA/desktop | plugin (node, registered band 149) |
| **platinoff-profile** | static (html/svg) | 5d81430 | Galaxy Mission Control profile page | outside (no manifest; unregistered) |
| **freebeat.ai** | scaffold (images, 17 files) | 87daf9e | private plugin start (no code yet) | outside (unregistered) |

## 2. Межі шарів (що де живе)

| Функція | GSV (тато) | poolAI (мама) | plugin |
|---------|-----------|----------------|--------|
| UI Galaxy / cards / tickets board | ✅ host | — | — |
| FM журнал / band queue `PH-S*` | ❌ (лише GM→FM дзеркало id) | ✅ мама | — |
| WSP pipeline (MiniCPM-35B) | ❌ | ✅ мама (план ECO-9.x) | — |
| Grid/discovery/enterprise API | ❌ | ✅ мама | — |
| Keep-live / watchdog / heartbeat | ✅ host | ❌ (нікуди не реєструє сервіси) | llama-rs/rusty пишуть heartbeat-файл |
| Inference HTTP API (OpenAI-compat) | ❌ **ніколи** | ❌ **ніколи** | llama-rs / rusty-engine — єдині місця |
| LLM provider routing (external) | ✅ `OmniRouter` каталог + quota | ❌ | omniroute = parallel implementation (owner-opt-in) |
| Mesh dispatch (WASM/peers) | ✅ host (`/api/wgpu/*`, plates) | ❌ | GSV-WGPU = browser lane; mesh-node = Pi lane |
| Telegram bus / Godfather | ✅ host (bus, tickets, ranks) | ❌ | telenetis = **commutation** only |
| Компільовані origins / neutral host | GSV sandbox (MCP) | — | плагіни own their runtime/ports/target/ |

## 3. Дублі (§3 з GSV_ECOSYSTEM_SYMBIOSIS.md у функційній формі)

| # | Функція | Де існує | Вердикт (симбіоз) |
|---|---------|----------|-------------------|
| D1 | OpenAI-compat HTTP serve (LLM API) | llama-rs, rusty-engine | **намірені паралельні engine-плагіни** — кожен власний; GSV не хостить жодного; omniroute — routing-шар над ними (не inference) |
| D2 | LLM routing / fallback | GSV `OmniRouter` (mesh+free lanes), omniroute (290 providers, node) | **намірено різні**; GSV = mesh/peers, omniroute = external-provider gateway; жоден не має стати "офіційним роутером винятково" |
| D3 | Keep-live watchdog | GSV watchdog | **єдиний** — у poolAI/плагінах не дублювати |
| D4 | FM/management журнал | poolAI FM | **єдиний канон у poolAI**; GSV бере лише `t-*` id для дзеркал; жодних копій FM в GSV |
| D5 | Telegram (бот/апи) | GSV `boxes/telegram` (Godfather bus), telenetis (bot + Mini App) | **Розподілена пара**: bus/state ∈ GSV; UI/commutation ∈ telenetis; більше ніде |
| D6 | Ratio/loc-audit tooling | GSV, poolAI, LinFS, llama-rs (xtask) | **намірено в кожному** (свій формальний gate ≥95/96); не об'єднувати |
| D7 | Logging rolling appender | GSV (tracing-appender), rusty-engine (std-only), llama-rs, telenetis (journald/docker) | **намірено в кожному**; формат різний, призначення однакове — консистентності не потребує |
| D8 | mesh executor | GSV `crates/mesh-wasm` (hub-side), GSV-WGPU (browser), mesh-node (Pi) | **одна функція, три lane-и** (hub / browser / Pi) — це один програм, не дублікати; GL: mesh-node **unregistered** — owner fix needed before full drain |
| D9 | 35B-A3B валідація/навантаження | rusty-engine (serve) + GSV mesh (`weights_matvec` dispatch) | GSV dispatch, engine execute — **норм** |
| D10 | EPUB/KDP build | rebook only | — |
| D11 | Screen capture | ORR_DESKTOP only | — |
| D12 | Filesystem mount | LinFS only | — |
| D13 | Shop/compat engine | CoreBoard only | — |
| D14 | Forecast journal | sport-forcast only | — |
| D15 | Solana adapter | poolAI (Deferred) | concept-only, не торкатись |

## 4. Що з цього випливає для симбіозу

- **тато (GSV) строго операційний**: UI, tickets, keep-live, mesh-dispatch, MCP sandbox, edge proxy, OmniRouter-каталог. Будь-яка спроба додати сюди inference або FM-журнал = порушення межі.
- **мама (poolAI) строго management**: FM, grid, enterprise API, WSP. GSV НЕ копіює FM; пулиНЕ тягає Galaxy.
- **plugins містять product runtime**: engines = інференс; LinFS = FS; rebook = EPUB; ORR = capture; etc. Плагін каже хабу «я живий/ готовий» через heartbeat/bridge — не більш.
- **Намерені пари/лаїни** (D1, D2, D5, D6, D7, D8) — це **features, not bugs**; симбіоз = дисципліна меж, не уніфікація коду.

## 5. Окремі розрив на функційному рівні (звірено з §3 symbiosis)

1. **mesh-node не у реєстрі** (`PRODUCTS.md` його не містить) — поки не зареєстрований, drain не ведеться. Owner: додати рядок або залишити як Pi-agent тільки-для-mesh.
2. **omniroute** — зареєстрований як node, але `npm test` і **ratio n/a**; паралельна implementation D2 з GSV OmniRouter; policy-down до owner-go (§keep-live AGENTS).
3. **freebeat.ai / platinoff-profile** — поза периметром: scaffold / static; FM не потрібен; лишити поза симбіоз-догляду (mark unknown, no FM).
4. **poolAI CI (`test-ci`)** — RAM/EDR блок; до тих пір poolAI = "declared funcs unverified in CI".

## 6. FM по кожному проєкту — пропозиція

**Принцип:** один документ `docs/FUNCTION_MANAGEMENT.md` на репо, легкий (не poolAI-тяжкий). Скелет:

```markdown
# <repo> FUNCTION_MANAGEMENT (симбіоз)
**Companion:** GSV docs/gsv/GSV_FUNCTION_MANAGEMENT.md · **Шар:** тато/мама/plugin
## 1. Оголошені функції (джерело: README / Cargo.desc / src/)
| Функція | Статус | Джерело |
## 2. Звірка (що підтверджено тестами/CI)
| Функція | Тест/CI | Дата |
## 3. Дублі з іншим репо (посилання на майстер §3)
## 4. Owner-рішення / наступні кроки
```

**Rollout:** 13 зареєстрованих + 1 unregistered (mesh-node) — по одному файлу на репо. Рекомендація: довірити **пилоту на 2 репо** (telenetis + rusty-engine — обидва маленькі і в роботі) з наступним розкоченням за досвідом. Це ~3–4 сесії роботи, не одна.

## 7. Owner-рішення (потрібні)

1. **Глибина FM**: легкий § скелет вище для всіх 14, чи повноцінний FM у poolAI-форматі лише для GSV + 2 пілотів?
2. **mesh-node**: зареєструвати (рядок у PRODUCTS.md) чи лишити як Pi-agent?
3. **omniroute**: підтвердити D2-розподіл (GSV OmniRouter = mesh/free, omniroute = external gateway) чи ні.
4. **freebeat.ai / platinoff-profile**: лишити поза або включити (тоді їм потрібен FM)?
