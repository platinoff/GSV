# Dispatch Research 2026-11-18 — як проєкти оркеструють MoE / distributed inference (і що з цього дістається нашому флоту)

**Date:** 2026-11-18 · **Author:** GSV drain session · **Audience:** owner + future agents
**Question (owner):** «проресерч як у кого в проектах релізовано логічно найкраще» — тобто як vLLM / Ray Serve / llm-d / Megatron-Core / KubeRay релізують паралельне виконання MoE, routing/scheduling, shared-expert placement, і що з цього відтворюється на нашому канвасі (fleet: Pi4 #36 cortex-a72, FF-PC #37 RDNA-2/Firefox-WebGPU, phones #35 valhall / #38 bifrost, усі за `https://LAN:9999`, hot-swap, Wi-Fi/4g, fleet замкнений).

**Scope boundary:** код не змінюється; це документ-синтез + практичні висновки.

---

## 1. Як це роблять інші (що ліцензують джерела)

### 1.1 vLLM — Expert Parallelism (EP)

Canon [Expert Parallel Deployment](https://docs.vllm.ai/en/latest/serving/expert_parallel_deployment/) + [AMD MoE Playbook](https://rocm.blogs.amd.com/software-tools-optimization/vllm-moe-guide/README.html):

| Факт | Значення |
|---|---|
| EP — **не самостійна стратегія**, а прапор `--enable-expert-parallel` | Активна лише якщо `TP × DP > 1` |
| Routed experts distributed | `determine_expert_map` → різні експерти на різні GPU |
| Routing experts без EP | Усі експерти локально, ваги шардовані (`flatten_tp_across_dp`) + AllReduce |
| Shared experts | **Always active, dense MLP**; шардовані як звичайний linear; реплікуються тільки при EP + TP>1 + DP>1 + all2all backend (`allgather_reducescatter`/deepep) |
| DP+EP спілкування | AllToAll (DP attention, KV-cache partition by request) |
| TP+EP спілкування | AllReduce (ніякого AllToAll при `dp_size=1`) |
| Crossover TP↔DP | ~256–512 concurrent для MI300X MI300X; TP краще latency при ≤128, DP краще throughput при ≥512 |
| Activation density | <1% → EP=0; >3% → EP=1. Llama-4-Maverick (0.78%): EP гірший на 7–12%; DeepSeek-R1 (3.13%) і Qwen3-235B (6.25%): EP=1 |
| MLA exception | DeepSeek MLA **потребує** DP+EP (без цього повне копіювання KV ×8 і 38% failure rate) |

**Crossover таблиця (DeepSeek-R1, MI300X×8):**

| Конкурентність | Переможець |
|---|---|
| 64 req | TP+EP (+52% throughput, −80% TTFT) |
| 1024 req | DP+EP (+47% throughput) |

### 1.2 Megatron-Core MoE (NVIDIA)

[Megatron-Core MoE docs](https://docs.nvidia.com/megatron-core/developer-guide/0.15.0/api-guide/moe.html): EP сумісна з **TP + DP + SP + PP**; sequence parallelism **обов'язкова** при EP×TP; токени, що йдуть на той самий експерт, **пакуються разом** у GroupGEMM (один fused kernel); expert per rank = TotalExperts / EP.

### 1.3 Ray Serve LLM (anyscale)

[Anyscale blog 2026-06](https://www.anyscale.com/blog/high-performance-distributed-inference-ray-serve-llm-vllm-google-kubernetes-gke):

- **HAProxy (C) ingress** + **Direct Streaming**: routing control-plane відокремлений від data-plane; HAProxy запитує `ingress request router` → отримує рішення → напряму проксіює SSE-стрім клієнту (обхід event-loop bottleneck).
- **vLLM RayExecutorV2** (default у vLLM 0.21+): `mp` backend + async scheduling roll-in.
- **Consistent hashing** на session ID → stickiness для багатоходових агентів (KV-cache reuse).
- **P/D disaggregation + WideEP**: 4P4D конфігурація з WideEP (Phi-tiny-MoE) обігнала vllm-router (13.6 ms vs 14.8 ms TPOT @256 concurrency).
- Ефект: **4.4× throughput prefill-heavy, 24× decode-heavy** vs Ray Serve LLM baseline.

### 1.4 llm-d (CNCF Sandbox) — wide expert parallelism

[llm-d Wide-EP guide](https://llm-d.ai/docs/well-lit-paths/foundations/wide-expert-parallelism):

1. `DisaggregatedSet`/`LeaderWorkerSet` — версіонований юніт для prefill+decode реплік (Kubernetes-native).
2. **Дистрибуція не по pod, а по DP rank**: router `InferencePool` таргетить **усі DP rank ports кожного пода** (`rank0..rank7`), тобто router обирає ранг, а не pod.
3. **DeepEP (NVLink/RDMA)** для all-to-all між вузлами (32×H200 NVL72; NIXL для KV-transfer), AMD — MoRI; Intel XPU — `allgather_reducescatter`.
4. **Router policies**: predicted-latency routing (well-lit path «Route Requests by Predicted Latency») та `prefix-cache-aware routing`.
5. Кожен DP rank = окремий API-сервер з власним портом; `LeaderWorkerSet` керує multipart deployment; upgrade/downgrade без розриву.
6. Зразок: DeepSeek-R1-0528, 32×H200, prefill DP16 + decode DP16.

### 1.5 KubeRay / Kubernetes patterns

Node affinity/taints (GPU nodes notfound у tainted pool), PodDisruptionBudgets, DRA (DeviceResourceAPI), `LeaderWorkerSet` для multi-replica MoE. Failover = re-schedule; health probes.

### 1.6 exo / Mesh LLM (рамі-зважений ring)

RAM-weighted **ring partition** моделі поверх heterogenous девайсів; пере-partition при join/leave 10–60 s; readiness перед route. Not MoE-aware — plain tensor sharding.

### 1.7 llama.cpp RPC

Транспорт RPC server↔worker (TCP), модель шардиться на `--rpc` вузлах; без MoE-awareness, без verification, без session/KV-aware routing.

### 1.8 MoE-Infinity / activation traces

EAM (expert-activation memory) tracing → **hot-expert cache** (ранні шари пріоритетніші); overhead-орієнтований, а не throughput.

---

## 2. Що це означає саме для НАШОГО канвасу (GSV mesh fleet)

**Наші факти (MESH_BENCHMARK_2026-11-18.md):**
140 `weights_matvec` dispatches; mix: Pi4 83 tasks (4.54 s/task), phones ~45–172 ms/task, FF-PC завис на 25. Середня `max_err_model` = 8.8e-7. Router `route_peers=[35,38,37,36]` — по суті round-robin по ready peers. `ready=true` ⌊lагging proof count⌋.

### 2.1 Shared vs routed experts у MiniCPM-35B-A3B

| Тип експерта | Наш стан | Industry поведінка |
|---|---|---|
| **Shared expert** (`ffn_*_shexp`, always-active, dense) | Доступний, 3 tensors/block × 40 blocks; ми шардуємо 2048×512 / 512×2048 (~4 MiB кожен) | Sharded як dense linear **або replicated** (якщо EP і >1 TP×DP і all2all); **ніхто не робить EP для shared experts** |
| **Routed experts** (256 experts, top-8, MiniCPM-35B-A3B) | **Не дистрибутовані.** Ніхто не просить ваги routed experts (blk.N.ffn_gate_exps etc.) | EP — тільки тут. AllToAll помер би на Wi-Fi/4g |

**Ключовий висновок №1**: ми дистрибутуємо саме той клас тензорів, який у продакшні завжди-active щільний — правильно було б **реплікація на кожному ready peer** (16 MiB × 3 × blk ≤ 48 MiB; 40 blks ≈ 1.9 GB max worst-case, цілком реально для Pi4/FF-PC/SD-карт phones), а не 4–8-way partial matvec + reassembly. Multi-block resident cache на peer: **локальний inference замість remoteness** там, де peer тягне ваги локально; mesh — тільки як fallback / overflow.

### 2.2 Математика reduce — що ми маємо і що ні

- Gate/Up (column-parallel, K=512 in → 2048 out): кожен peer рахує рядки `[start,end)` → host **конкатенує** — це TP column-parallel **without collective** (vLLM bit-wise behaviour з `flatten_tp_across_dp`, DP=1 → AllReduce там не треба).
- Down (`ffn_down_shexp` 512×2048): якщо ми ріжемо по виходу (K=2048), тобто інпут вздовж K — тоді host мусить **AllReduce-suma**, інакше результат неправильний у ч.с. U Her.refilled.
- Наш code path (`max_err_model ≈ 8.8e-7` проти golden-x) підтверджує, що на практиці down_projection **не розбивалась по K** у наших dispatch (ряди 512 різались, K=2048 повний на peer). Інакше err був би ~O(1). Тонка деталь, варта задокументувати.

### 2.3 Routing / scheduling — що робити замість round-robin

Industry standard — **predicted-latency routing** (llm-d) або **session-affinity consistent hashing** (Ray Serve LLM). У нашому випадку:

| Критерій | Що взяти |
|---|---|
| Per-peer latency EWMA | `elapsed_ms` з кожного done task → EMA (α=0.3) в hub registry; enqueue обирає peer з найменшою EWMA, а не фіксований порядок |
| Stale ready peers | `ready=true` з `last_seen_age_secs > TTL` → пропускати (аналог llm-d predicted-latency health) |
| Phones sleep/wake | lease + requeue (у нас є claim/lease, треба гарантувати re-route при sleeper, а не failed) |
| Battery/network aware | phoniics при 4g ~1.5 Mbps → upload 16 MiB/tensor ≈ 90 s; **ніколи не upload'ити повторно** — peer cache обов'язковий (у нас chunk registry на peer-фланзі вже відбувся: 46→86 chunks) |
| FF-PC freeze (27 tasks) | industry fix = health probe + LB de-register. У нас `last_seen_age_secs=1` при 0 claim — означає page живий, JS-цикл зупинений; причина: WebGPU suspend/resume після tab freeze — поза нашим контролем |

### 2.4 Verification

Industry: жоден фреймворк **не верифікує** числової коректності (довіряє fused kernels). Ми робимо **golden-x + max_err_model** кожним shard — це **краще**, ніж продакшн, і варто зберегти як readiness + correctness proof одночасно. (У MoE-Infinity аналог — EAM trace stats; не замінник.)

### 2.5 Шо є Non-Ideal у нашій реалізації (чесно)

1. **Router**: `route_peers=[35,38,37,36]` — фіксований порядок; Pi4 (4.5 s/shard) отримав 83/140 завдань проти phones (10× швидше) — waste. Industry fix: predicted-latency weighting.
2. **FF-PC ghost**: `ready=true proofs=25 tasks_done=26→25` — хтось де-дублює/переоблічує; подозра — enqueued task упав і був reclaimed, `ready_proofs` lagging. Діагностувати в `wgpu.rs` enqueue filter.
3. **No routed-expert EP**: найбільший perf-direct для 35B — 160 routed-expert tensors/blk (256 experts) лишаються un-distributed; при вазі ~48 MiB/blk×40 ≈ 1.9 GB, це viable лише через **precision-aware pre-load**, не per-token.
4. **L: mirror vs peer cache**: L: (180 MB) — джерело істини для upload; peer-side частина повторної роботи при падінні peer — потребує peer-cache invalidation (у нас зараз chunk registry тільки hub-side; peer просто мовчки скіпнув старий chunk при rejoin через gate фільтр).
5. **No collective-protocol support** (AllToAll): будь-яке майбутнє EP/P-D впровадження потребує транспортний шар; LoC висока, вигоди сумнівна при Wi-Fi.

### 2.6 Що НЕ переноситься на наш канвас (і чому)

| Технологія | Чому не переноситься |
|---|---|
| NIXL/UCX/RDMA KV-transfer | Немає KV-cache streaming між peers |
| P/D disaggregation | Немає довгих генерацій з prompt/decode фаз на різних вузлах |
| DP Attention | Немає batched request pool — одиниці dispatch-ів, не QPS |
| NVL72/GB200-scale all-to-all | Немає NVLink; Wi-Fi 1.5 Mbps/phone |
| Consistent-hash session affinity | Операції idempotent, без state; скорійше peer-affinity важить для cache chunks |
| FP8/FP4 GroupGEMM | Phones/ARM Mali WebGPU — FP16 (fp32) лише; dense compute обмежений |
| Prefix-cache routing | Немає prefix cache на dispatch path |

---

## 3. Прикладні рекомендації для GSV mesh (за зростанням зусиль)

| # | Рекомендація | Зусилля | Ефект | Де |
|---|---|---|---|---|
| R1 | **Predicted-latency EWMA routing** у `route_peers` selection: шардувати 3:1 phones:Pi4 при рівній готовності; Pi4 — лише bulk fallback | S (1–2 години, `weight_store`/`wgpu.rs`) | −60..80% makespan для blk dispatch | `src/boxes/wgpu.rs` enqueue route |
| R2 | **Stale-proof TTL**: `ready_proofs` свіжістю < 300 s; інакше de-register з route pool (аналог llm-d predicted-latency) | S | FF-PC-подібні ghosts не плутатимуться | `wgpu.rs` ready filter |
| R3 | **Peer-side chunk cache invalidation note** при `weights/manifest` bump — GSV hub уже віддає manifest; peer JS мусить re-fetch при delta | S (peer code поза GSV; doc + ticket) | Безпечні hot updates без 4 MiB re-upload-шторм | peer agent, spec |
| R4 | **Multi-tensor task envelope**: `op=weights_matvec_block` env with 3 tensors in one task → один HTTP round-trip на block | M | −66% wake overhead на phones | `wgpu.rs` op design + peer |
| R5 | **Replicated-shexp local-inference mode** (16/32/48 MiB/peer): якщо peer може тримати 1 blk — виконує без mesh round-trip | M–L | Найбільший speedup для phones | new mode flag |
| R6 | **EP exploration для 35B routed experts** — тільки після R4/R5; попередньо model-side аналіз (кількість routed tensors в 35B, їх точність, можливість FP4) | L + owner gate | Диспетчіалізація 1.9 GB | research ticket |
| R7 | **Keep max_err_model golden-x** як dispatch correctness gate, але додати **percentile p50/p99** report у benchmark (pi-writer) | S | Краще SLO visibility | `MESH_BENCHMARK_*.md` harness |
| R8 | **FF-PC root-cause**: diagnostic read `about:config` WebGPU suspend log; власний трек | S owner | Повернути RDNA2 як eager-tier | owner / FF |

---

## 4. Що конкретно робити ДАЛІ (практично)

1. **Цей резервний документ** — підшитий у канон (посилання з HANDOFF/NEXT).
2. **R1 + R2** — фактично це одна зміна в `wgpu.rs`: `route_peers` selection на основі `(ready, proofs-fresh, EWMA(elapsed_ms))` замість фіксованого `[35,38,37,36]`. Гейти: `cargo test --lib`, stand-smoke 86/86, одна commit.
3. **R4** — обговорити власника: extension op `weights_matvec_block` (один request → 3 done directives, один wake). Це **єдина** зміна в wire protocol, потрібна для phones-sleep.
4. **R6** — покласти в backlog доки не буде consumer; нічого не будувати.
5. **Документ не потребує власника** — discovery-only; виконання R1/R2/R4/R8 — окремі тікети.

---

## 5. Джерела (validated 2026-11-18)

1. vLLM — [Expert Parallel Deployment](https://docs.vllm.ai/en/latest/serving/expert_parallel_deployment/)
2. AMD ROCm — [The vLLM MoE Playbook (TP/DP/PP/EP)](https://rocm.blogs.amd.com/software-tools-optimization/vllm-moe-guide/README.html)
3. NVIDIA — [Megatron-Core MoE Parallelism](https://docs.nvidia.com/megatron-core/developer-guide/0.15.0/api-guide/moe.html)
4. Anyscale/Ray — [High Performance Distributed Inference with Ray Serve LLM + vLLM](https://www.anyscale.com/blog/high-performance-distributed-inference-ray-serve-llm-vllm-google-kubernetes-gke) (2026-06-18)
5. llm-d — [Scale MoE Models with Wide Expert Parallelism](https://llm-d.ai/docs/well-lit-paths/foundations/wide-expert-parallelism) (CNCF Sandbox, v1.0)
6. NVIDIA — [Scaling Large MoE Models with Wide-EP on GB200 NVL72](https://developer.nvidia.com/blog/scaling-large-moe-models-with-wide-expert-parallelism-on-nvl72-rack-scale-systems/)
7. Red Hat — [Scaling DeepSeek-style MoEs with vLLM + llm-d using Wide EP](https://developers.redhat.com/articles/2025/09/08/scaling-deepseek-style-moes-vllm-and-llm-d-using-wide-ep)
8. Kubernetes — [LeaderWorkerSet](https://lws.sigs.k8s.io/docs/concepts/disaggregatedset/)

---

## 6. Нереалізовані шматки / відкриті питання (owner gates)

- **Vision rev mirror** (GSV 516 vs poolAI 499) — власник ще не вирішив.
- **ECO-8.2** (llama-cpp-sys bump, MTP eh_proj assert) — власник.
- **mesh-node remote** `github.com/platinoff/mesh-node` — власник.
- **Qwen3.8-27B GDN/SSM** — модель не сумісна з поточним qwen35moe-вью; потрібен новий арх-patch (власник).
- **36** — це Pi4 #36 tasks_done=92/93 (розрахунки плавають при reclaim); benchmark claim'и скориговано.
- **FF-PC** — власник: ручна `about:config` зміна або прийняти FF-PC як **observer/QA tier**, що не бере ваги.
