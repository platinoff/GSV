# Research: SSD/mirror offload · WiFi/WGPU залізо · autonomy ladder (2026-11-26)

**Статус:** research only, жодного коду. Owner ask: «що можливо не зроблено у Rust, фокус на загрузці/вигрузці з SSD (у нас — дзеркало L:), хто як на якому мінімальному залізі запускав через WiFi/WGPU, які tuning-и, порівняти з нашою екосистемою Rust 1.100, і схеми автономної роботи hub → 255 пірів».

Джерела: 4 паралельні read-only агенти (primary docs/PR/papers + локальний `file:line` інвентар). Попередній канон: [`GASV_ENGINE_LANDSCAPE_MOE_ADAPTATION_2026-11-26.md`](GASV_ENGINE_LANDSCAPE_MOE_ADAPTATION_2026-11-26.md), [`GASV_STREAMING_MOE_PLAN_2026-11-26.md`](GASV_STREAMING_MOE_PLAN_2026-11-26.md).

Позначки: **[P]** primary source з URL; **[L]** локальний вимір/код; **[I]** інференс, не вимір.

## 1. Що вже є в нашому Rust (не писати з нуля)

| Примітив | Де | Стан |
|---|---|---|
| Artifact range-read + sha256 fail-closed | `weight_store.rs:103-119` (private) | Є, але синхронний; кожен виклик перечитує весь artifact-sources JSON і linearly сканує |
| Chunk store 4 МБ, atomic, audit, LRU by `stored_ts` | `weight_store.rs:29-43,364-386,472-499` | **`read_chunk` перезаписує весь `index.json` + JSONL на кожне читання** — hot-path tax |
| Q4_K/Q6_K streaming dequant + golden | `wgpu_dequant.rs:77-128,165-215,269-284` | Математика solid; **без memo**, range йде через важкий `read_chunk` |
| Hub golden-x verify | `wgpu.rs:791-834,1489-1771` | Наш «proof» сильніший за всі P2P системи (ті взагалі не верифікують) |
| Per-expert addressing | `moe_forward.rs:61-133` (589 824 B gate/up, 860 160 B down) | Є; `manifest_offsets` щоразу спавнить `rusty manifest`, `expert_key` 3×/експерт речитає sources JSON |
| Routing Gini/hot-set/prefetch helpers | `moe_forward.rs:487-615` (`should_prefetch`, `HotExpertSet`, `concurrency_decision`) | **НЕ підключені до `report_one_layer`** (`:677-880`) — forward серійний, 2 c poll, concurrency=1 |
| Peer IndexedDB 128 МБ LRU, content-addressed | `wgpu.rs:3009-3068` | Phones **завжди bypass IDB** (`:3419`), бо немає `crypto.subtle` на insecure LAN |
| Chunk GET як `{data_b64}` | `server/mod.rs:1022-1036` | ~33% inflation + `atob`/`flat` копії (`wgpu.rs:3423,3433`) |

**Ключ:** ми маємо *transport*, але майже не маємо *storage machinery* серйозних двигунів: browsing-кожен-шматок, без cache counters, без coalescing, без prefetch у циклі.

## 2. Хто як вантажить з диска — порівняльна таблиця [P]

| Техніка | Хто / цифри | Потреби | Portable на Windows Rust 1.100 + L: mirror? |
|---|---|---|---|
| mmap lazy paging | llama.cpp `-lm auto` ≈ mmap; ≥10× повільніші повторні завантаження з O_DIRECT на гарячих даних (community #18758); 112 K page faults cold (ServerlessLLM); ≥30% prefill regression (discussion #29347, unresolved) | OS VM, модель ≤ RAM | **Частково** — `memmap2` мапить, але немає willneed/sequential/mincore/large pages; з 1.7 ГБ вільно → refault storm. **Не покладатись** |
| O_DIRECT / `FILE_FLAG_NO_BUFFERING` | ServerlessLLM 2.9 GB/s raw → 7.4–8.1 GB/s з pinned; ≥10× loss на гарячих даних; 4096-вирівнювання | aligned offset+len+buffer | **Умовно A/B** — наші 589 824 / 860 160 Б вже кратні 4096. Лише з власним bounded cache |
| io_uring / tokio-uring | Linux ≥5.10; у llama.cpp mainline не знайдено | Linux syscall | **Ні** — Windows 11 host |
| `posix_fadvise` / `MADV_WILLNEED/SEQUENTIAL` / `mincore` | Strata: `MADV_SEQUENTIAL` 65→23 c (RX 6900 XT, 29.9 ГБ experts.bin, 2× NVMe), faults 1.35M→~0.1M; fill 39 MB/s→3.2 GB/s | Unix | **Ні напряму** — лише слабкі Windows hints (`SEQUENTIAL_SCAN`), working-set trim — не correctness |
| Expert-contiguous pack + address manifest | SGLang SSD Expert Pack: 4096-B aligned `(layer,expert)` gate/up/down разом; 1418→~370 reads/token (mbolt); 3.6–8.2× cold load (ServerlessLLM OSDI'24) | offline conversion + JSON manifest | **Так** — чистий формат + JSON. **Найвища цінність** |
| Slot pool / bounded byte-stamped cache | HF `moe-expert-streaming` #50717: OLMoE 10.1→34.6 tok/s; ik PoC 64 slots = knee (39.9→26.5, −11.1 ГБ); SGLang hit 54% vs 17% (21 vs 5 ГБ cache) | `HashMap` + pin top-k | **Так** — std, без dep. **Найвища цінність** |
| Sorted + gap-merged multi-range `pread`, ~8 потоків | mbolt +13–14% end-to-end; HF 8 I/O threads; Strata `STRATA_IO_PF_THREADS=8` | router lookahead | **Так** — `seek_read` + rayon/tokio (вже dep); без `mincore` — свій cache index |
| 1–2 layer router lookahead prefetch | HF prefill 68→228 tok/s; HOBBIT top-1 next-layer ~96%, prefetch ~10% prefill; mbolt: predictive prefetch **не допоміг** layout-blind шляху — A/B | router до dispatch | **Так**, з watermark gate |
| Persisted hot profile → resident budget | Strata 4 GiB headroom; GSV FWD-7 HotExpertSet 15.6 МБ/256 МБ | вже є | **Частково зроблено** — треба wired у forward |
| Lossless exponent compression (FluxMoE) | 7.2× vLLM / −79% TPOT | GPU decompression | **Ні** — немає локального GPU |
| SGLang headline: прибрати page-cache→pinned H2D копію | 6–7× decode у них | локальний CUDA + pinned | **N/A** — ми шлемо байти пірам, H2D немає |

**Windows-обмеження [P]:** `io_uring`/`tokio-uring` Linux-only; O_DIRECT/fadvise/madvise/mincore — Unix; file-backed mmap views на Windows **ніколи не отримують large pages**; `SeLockMemoryPrivilege` + перелогін для pinning; Strata виміряв Task Scheduler trap: 1.42–1.52 GiB/s у терміналі vs 0.05 GiB/s у Task Scheduler (821 c!). Наші shard-розміри вже 4096-aligned → unbuffered A/B можливий, але **лише після** власного pack/cache.

## 3. Хто запускав на мінімальному залізі через WiFi/WGPU [P]

| Система | Мін. залізо / пам'ять | Мережа | Піри | tok/s (як опубліковано) | Tunings | Portable на GSV |
|---|---|---|---|---|---|---|
| **SwarmLLM/Pooled** | phone 0.5–1 GB pledge; 27B ~17 ГБ / MoE ~22.5 ГБ на кімнату; iPhone Safari ~1.5 ГБ soft limit; 256 МБ storage binding | WebRTC mesh, WiFi / cross-internet | 1, 2, 3 emulated; 16 topology; 64 tabs падає | MoE 40–46 plain / 65–84 spec solo GB10; Mac+iPhone **7.7**; 2 devs @20 ms 12.9/20.6; 3 @20 ms 9.9/17.5; Flash 125B 8→10–12 | f16 hidden 4/10 KB, 16-col frames ×6 in flight, K=3/5/7 speculation, 4.6 KB slices ×4 stripes, 30 s lap / 90 s verify timeout, 2.5 s/500 ms ping, ICE redial 15 s, 64 KB/8 MB range window, pledge caps | **Найвища** — blueprint протоколу; треба hub-relay або WebRTC + WGSL peer |
| **LlamaWeb** (arXiv 2605.20706) | iOS Safari tab **<500 МБ**; low-end mobile GPU only 4 smallest workloads | 1 пристрій | 1 | mobile 4–17; RTX 5080 ~100 decode; +45–69% decode, −21–51% prefill vs WebLLM | static arena, OPFS 4×1 MB stream, f32 accumulation (f16 ламає Apple M), tunable workgroups, shader cache 1–5 c, 23 quant formats | Kernel/loading discipline для майбутнього WASM peer |
| **wllama v4** | 4 ГБ WASM heap (compat), RPC workers по 4 ГБ | in-page workers | 1 | M5 Max CPU: 350M 277 tg, 1.2B 118, 3B 42 | OPFS sync handle, 4 parallel connections ≥64 МБ | Single WASM peer option |
| **WeInfer** (WWW'25) | single tab | — | 1 | до **3.76× WebLLM** | GPU buffer reuse, async pipelines | Dispatch-overhead lessons |
| **exo** | Apple silicon, TB5 RDMA full mesh (vendor) | LAN WiFi/Ethernet; RDMA TB5 | 1, 2, 3, 4 | 3B: 49.3 solo → 44.4(2)/39.7(3) single; 95.7(2)/108.8(3) multi; 235B 31.9 на 4×M3 Ultra (Jeff Geerling, third-party) | topology placement by memory + RTT; pipeline → TP 1.8×/2, 3.2×/4; <4 KB activations | Placement ideas; runtime Python/MLX |
| **llama.cpp RPC** | `-DGGML_RPC=ON`, **unauthenticated** | TCP :50052 LAN, optional RDMA | 1–4; один звіт 15 GPU | 2.5 GbE ~3 tok/s; 235B 4 nodes **15.2 vs 20.4** (regress); Ghostlink 30B works | memory-proportional split, `--tensor-split`, RDMA/TCP | Уникати sync request/response per tensor |
| **distributed-llama** | Pi 5 8 ГБ, q40+q80; **2^n пірів, ≤ #KV heads** | TCP :9999 / switch | 1, 2, 4 | Pi5 3B 5.95→10.33→13.68; Pi5 30B 13.04 (незалежно 15.143) | `--nthreads 4`, 0.6–1 МБ/токен sync | power-of-2 cap = погано для 255 |
| **Intel pipeline shards** (2608.19147) | 3× AI PC 32 ГБ, INT4 OpenVINO, **Windows 11 + 802.11ax WiFi** | static TCP daisy chain, 16 KB/hop, **6.5 ms RTT** | 1, 2, 3, 4 | 2 nodes 43.97 tok/s (1.79× mono, 2 users); 3 nodes 64.67 @33 streams | beam_idx Gather fusion, mask KV rewind, K=3 spec, TCP_NODELAY | **Найсильніший WiFi+weak-hardware доказ** |
| **Voltron** (2607.07046) | гетерогенні phones, mixed precision | Wi-Fi Direct | 2–4 | QoS TTFT 10 c / TPOT 400 мс | per-layer MP/TP вибір за RSSI (−60 → TP, −70 → MP) | Policy planner ideas |
| **Mesh LLM** | Rust + QUIC/iroh + Nostr | QUIC relays | not published | немає в README | stage splits, package fragments | Найближчий Rust-родич GSV |

**Головний закон [P]:** plain decode платить **один network lap на пристрій на токен**; speculation (K=3/5/7) і batched prefill (16-col ×6 in flight) — обов'язкові; додавання пірів купує **aggregate throughput, не швидкість одного стріму**. Наївний layer split на LAN може **зменшити** single-request (exo 49.3→44.4; RPC 20.4→15.2). Наші 0.59/0.86 МБ expert matvec — інша гранулярність, але straggler-закон той самий.

## 4. Autonomy ladder (аналіз, не код)

 наші фактичні межі [L]: `MAX_PEERS=50` (`wgpu.rs:340`, drain при join), `MAX_TASKS=32` (`:345`), `concurrent=1` (`:362`), `POLL_SECS=2` → ~127 poll/с при 255 пірах, кожен перезаписує JSONL під `mesh_lock`; `TASK_CLAIM_REQUEUE_SECS=300/1200`; `proof_gate` fail-open; `ranked_peers` 4-tier EWMA (наш EPLB-аналог).

| Tier | Що реально | Bottleneck | Versus capability | Очікуваний wall | Верифікація |
|---|---|---|---|---|---|
| **0. Hub only** | Планування: router/top8, ExpertAddr, range-read, golden reference, checkpoint. Heavy enqueue → `no_ready_peer`, не crash | 21 ГБ артефакт vs 1.7 ГБ вільно; немає qwen35moe runner | **Тільки numerics/planning, без токенів** | N/A | Хаб рахує всі reference, 0 shard dispatched |
| **1. Hub + 1 пір** | FWD-4 1-layer (24 matvec, `6.94e-18`); canary `3.47e-18`; gate/up PASS | `concurrent=1`: 8 shard → **270–282 c wall** при 2–4 c compute; 960 matvec/токен серійно [I] | Доведено 1 шар; 40 шарів ще ні | [I] хвилини/токен | 960 `max_err_model`/токен |
| **2. Hub + 2 піри** | round-robin fan-out по 2 готових пірах; ~238 MFLOPS сумарно | `MAX_TASKS=32`, 1 task/peer, 2 c poll, батарея/WiFi | 2-way shard parallelism для top-8 | [I] compute 10–15 c/токен, wall — десятки секунд | Ті самі 960 перевірок, розділені на 2 |
| **3. Hub + 3 піри** | 8-shard group по 3 пірах; один мертвий → requeue | straggler задає makespan (Pi 4.54 c vs phone 45–172 мс); EWMA пом'якшує, не прибирає | Meaningful parallel dispatch; 3 — емпіричний практичний максимум для малих hidden [P, Pooled] | [I] ~⅓ 2-peer wall якщо балансується | 960 перевірок; можна почати sampled verification |
| **10 пірів** | Реєстр ще <50; 20 poll/с; 32 tasks allow 10 concurrent якщо підняти env | **JSONL rewrite per poll під одним mutex**; 200-row/50 МБ інцидент уже в історії; exo broadcast storm ~10 nodes [P] | Hot-set 8–16/шар стає реально корисним | [I] orchestration-bound | Хаб стає bottleneck → VeriLLM sampling |
| **100 пірів** | **Недосяжно без змін** — join дренить до 50; 32-task ring; ~200 poll/с → stall; proof-window starvation | реєстр, task ring, JSONL I/O, WiFi airtime; Ray DP groups ≤128 **з RDMA** [P]; SwarmLLM 150–500 = DHT network size, не активні воркери [P] | Лише як **idle cache/admission pool**: ≤16 активних hot пірів, решта standby (Pooled «ask-only guests» [P]) | [I] жодного когерентного токену при 100 активних | Повна верифікація неможлива → sampling |
| **255+ пірів** | ID space u64 ок, але реєстр hard-cap 50 [L] | Code: `MAX_PEERS`, `MAX_TASKS=32`, concurrency 1, JSONL+`mesh_lock`; Physics: 255 браузерів на одному 2.4 ГГц AP, 0.6 ГБ/токен, без RDMA/NVLink; SGLang elastic EP важкий уже на 20→24 GPU на H100 [P] | **Жодного корисного inference на 255 активних.** Тільки: (a) admission-controlled fabric з 8–16 активними, (b) persistent peer-side expert cache, (c) batched verification | [I] хвилини–години/токен при примусі | sampled/crypto verification + trust |

**Один рядок рекомендації:** не думати «255 EP ranks». Досяжний максимум GSV — **hub + 8–16 активних перевірених пірів**, решта — cold standby. Що реально купує масштаб: (1) прибрати JSONL+mutex hot path, (2) підняти concurrency з 1, (3) залишити golden-x на sampled subset, (4) зберегти fail-open `proof_gate`, щоб churn не став group-wide outage як у Ray DP-group.

## 5. Що з цього реально адаптувати (рангований shortlist, без нових dep)

1. **Offline expert-major pack на L: + sidecar manifest.** Один 4096-aligned запис на `(layer,expert)`: gate/up/down поруч; manifest `file_index, offset, len, dtype, sha256`. Оригінальний 21 ГБ GGUF лишається immutable source; derived pack верифікується **один раз** при збірці, не re-hash на кожне читання. Це ServerlessLLM direct addressing + SGLang Expert Pack + mbolt layout в одному. **[P][L]** Передумова для 2–4.
2. **Bounded hub cache з явним admission/pin + counters.** `HashMap<key, Arc<Vec<u8>>>` у MiB (сотні МБ, не ГБ), LRU/LFU з захистом поточних top-k; `cache_hits/misses/evictions, pack_read_bytes, io_errors, file_mb, read_ms`. **Це найбільша поведінкова різниця між нашим стеком і кожним серйозним двигуном тут.** **[L]**
3. **Coalescing read scheduler.** Сортування за offset, gap-merge, positional `seek_read` на ~8 потоках у preallocated aligned буфери; cap per-layer reads (HF 64-at-a-time). Очікувано +13–14% end-to-end і менше syscall. **[P]**
4. **1–2 layer router lookahead prefetch.** Запускати router(и) наступного шару раніше, issue «guessed» read тільки коли SSD інакше іде; install у cache тільки якщо використано. A/B обов'язковий (mbolt: не допомогло). **[P]**
5. **Підключити вже написані helper'и до forward.** `should_prefetch`/`HotExpertSet`/`concurrency_decision` описані, але **не викликаються** з `report_one_layer`; forward серійний, 2 c poll. Env `GSV_WGPU_MAX_CONCURRENT_PER_PEER` уже є ([L] `wgpu.rs:396-402`). Найвищий expected impact: TARGET 270 c wall → ~10 c. **[L]**
6. **Memo artifact-sources JSON + прибрати index rewrite на Reads.** `read_artifact_sources` перечитується `read_chunk_artifact`, `chunk_len_any`, `expert_key` (3×/експерт); `read_chunk` переписує `index.json` + audit щодо кожного shard. `OnceLock`/mtime-keyed map + batch hit accounting — std + serde_json вже є. **[L]**
7. **Chunk GET як `application/octet-stream` + safe phone IDB reuse.** Замінити `{data_b64}` на axum `Bytes`; hash-check phone cache через hub-supplied digest (телефони не мають `crypto.subtle`). Прибирає ~33% inflation і `atob`/`flat` копії; телефони перестають ре-завантажувати всі chunks кожен task. **[L]**

## 6. Чесна межа

- Жоден measurement з цієї нотатки **не є GSV-виміром** крім позначених **[L]** (наші 130.98/187.06 MFLOPS, 270/279 c wall, `max_err 3.47e-18`, `6.94e-18`, hot-count 8, 15.6 МБ/256 МБ).
- Pooled/Strata/WeInfer/LlamaWeb — vendor/author-reported; latency emulation у Pooled **не реальний лінк** (page timer, один GPU); exo 1.8×/2, 3.2×/4 і «99% RDMA» — vendor; незалежні: Jeff Geerling 4× M3 Ultra 31.9/32.5, hellomatik 15.143 tok/s на 4× Pi, багаторазові RPC-регресії в discussion #9136.
- Наш conjectured envelope лишається **<1 tok/s** на 2 пірах (llama-rs 0.497 tok/s на 7.4 ГБ RAM, IQ2_XXS). Усі 50–90 tok/s у огляді вимагають 8–32 ГБ VRAM + NVLink/RDMA.
- G13 blocked — жодного image→answer/quality-claim. Цей документ про data plane і orchestration, не про якість.

## 7. Адаптований план (єдина черга, owner go 2026-11-26)

Рангований shortlist з §5 і локальний інвентар дали **дві різні першочерговості** — research радив pack спочатку, локальний вимір (`wall 270–282 c` vs `compute 2–4 c`, ratio ≈0.015 → `queue_claim`) каже спочатку прибрати hot-path накладні витрати й підключити вже написані helper'и. Поєднуємо чесно: **дешеві/memo-фікси → оркестрація → data-plane pack**.

| # | Тікет | Що | Чому саме тут | Очікуваний ефект | dep |
|---|---|---|---|---|---|
| 1 | **LOAD-0** | Memo artifact-sources JSON (mtime-keyed `OnceLock`/map) + прибрати index rewrite на кожне `read_chunk`; `sha256_hex` через `format!` ×2 | `read_artifact_sources` речитається `read_chunk_artifact`/`chunk_len_any`/`expert_key` 3× на експерт; `read_chunk` переписує `index.json`+JSONL на кожен shard — це синхронні дискові записи в hot verification path | −3 перечитування/експерт, −1 rewrite/shard; знімає JSONL tax до SCALE-1 | std + serde_json (вже) |
| 2 | **LOAD-4** | Wire `should_prefetch`/`HotExpertSet`/`concurrency_decision` у `report_one_layer`; env `GSV_WGPU_MAX_CONCURRENT_PER_PEER` уже є; overlap enqueue/collect; proof gate зберегти | Helpers описані в FWD-5…7, але **не викликаються**; forward серійний із 2 c poll sleep. Наш вимір: 270 c wall / 4 c compute | **Найвищий expected impact:** target 270 c → ~10 c; не вимагає нового формату | tokio Semaphore (вже) |
| 3 | **LOAD-2** | Bounded hub raw-chunk cache `HashMap<key, Arc<Vec<u8>>>` (сотні МБ) + counters `cache_hits/misses/evictions, pack_read_bytes, read_ms`; promote `read_chunk_artifact` → `pub(crate)` | Найбільша поведінкова різниця між нашим стеком і кожним серйозним двигуном: ми покладаємося на OS page cache без явного admission/pin | repeated verification hot shard не читає знову; телеметрія дає A/B основу | std |
| 4 | **LOAD-1** | Offline expert-major pack на L: (4096-aligned gate/up/down разом на `(layer,expert)`) + sidecar manifest; derived pack верифікується **один раз** | ServerlessLLM direct addressing + SGLang Expert Pack + mbolt layout; передумова для coalescing; 21 ГБ GGUF лишається immutable source | 1418→~370 reads/token [P, mbolt]; основа для LOAD-3 | std + sha2 (вже) |
| 5 | **LOAD-3** | Coalescing positional read scheduler: sort offsets, gap-merge, ~8 потоків `seek_read` у preallocated aligned буфери, cap 64 reads/layer | +13–14% end-to-end [P, mbolt]; на Windows — `seek_read`, не io_uring | менше syscall, стабільний cold I/O | rayon (вже dep) |
| 6 | **LOAD-5** | Chunk GET як `application/octet-stream` (axum `Bytes`) + phone IDB hash-check через hub digest | ~33% base64 inflation + `atob`/`flat` копії; телефони не мають `crypto.subtle`, тому bypass IDB | −33% wire, phone reuse без silent corruption | axum/bytes (вже) |
| 7 | **SCALE-1** | In-memory registry/task state замість JSONL+`mesh_lock`; `MAX_PEERS=50`, `MAX_TASKS=32` переглянути | передумова для >10 пірів; 200-row/50 МБ stall уже в історії; Parallax: routing math дешева (8 мс @256 GPU), наше збереження — ні | відкриває tier 10 пірів; 255 — тільки admission pool | std + tokio |

**Коректність пріоритету:** LOAD-0/4/2 — «оркестрація + hot-path», найдешевші й б'ють по виміряному 270 c wall. LOAD-1/3 — «data plane», потрібні для багатотокенного forward'у, але не виправляють чергу. LOAD-5 — transport для телефонів. SCALE-1 — окремий суттєвий шмат (не в цій серії).

**Статус:** research завершено; owner дав go «продовжуємо тікетами» → LOAD-0 ✅ (memo + batched accounting, 2 TDD green), LOAD-4 🟡 (parallel experts + skip-resident, TDD green, живий A/B у дорозі) → LOAD-2 → LOAD-1 → LOAD-3 → LOAD-5 → SCALE-1 далі окремими тікетами з TDD/гейтами/одним комітом. Жодного quality-claim (G13 blocked).
