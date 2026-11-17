# Dispatch Research 2026-11-18 — як проєкти оркеструють MoE / distributed inference (і що з цього дістається нашому флоту)

**Date:** 2026-11-18 · **Author:** GSV drain session · **Audience:** owner + future agents
**Status:** R1+R2+R3+R7 implemented & deployed live (`3ba2353`, `9691c7c`), A/B-verified via dispatch `g-199`; R3 refined (content-addressed chunks ⇒ no invalidation needed, only persistence).
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
| Battery/network aware | phoniics при 4g ~1.5 Mbps → upload 16 MiB/tensor ≈ 90 s; **ніколи не upload'ити повторно** — persistent peer cache обов'язковий (hub-side у нас L: mirror 46→86 chunks; peer-side — лише per-session memory, див. R3) |
| FF-PC freeze (27 tasks) | industry fix = health probe + LB de-register. У нас `last_seen_age_secs=1` при 0 claim — означає page живий, JS-цикл зупинений; причина: WebGPU suspend/resume після tab freeze — поза нашим контролем |

### 2.4 Verification

Industry: жоден фреймворк **не верифікує** числової коректності (довіряє fused kernels). Ми робимо **golden-x + max_err_model** кожним shard — це **краще**, ніж продакшн, і варто зберегти як readiness + correctness proof одночасно. (У MoE-Infinity аналог — EAM trace stats; не замінник.)

### 2.5 Шо є Non-Ideal у нашій реалізації (чесно)

1. ~~**Router**: `route_peers=[35,38,37,36]` — фіксований порядок~~ → **FIXED 2026-11-18 (R1/R2, `3ba2353`)**: EWMA-tier ranking; FF-PC ghost виключено; evidence у §3.1.
2. ~~**FF-PC ghost**: `ready=true proofs=25 tasks_done=26→25`~~ → **FIXED 2026-11-18 (R2)**: `fresh_proofs` gate; FF-PC тепер `proofs_fresh=0` у /sys і **виключений з heavyweight route pool** до свіжого proofs.
3. **No routed-expert EP**: найбільший perf-direct для 35B — 160 routed-expert tensors/blk (256 experts) лишаються un-distributed; при вазі ~48 MiB/blk×40 ≈ 1.9 GB, це viable лише через **precision-aware pre-load**, не per-token.
4. ~~**L: mirror vs peer cache**~~ → **R3 refined (див. §3)**: peer-side cache — **per-session only** (`const CACHE={}` у page-JS, первинний процес page-reload на кожну сесію); persistence відсутній; staleness неможлива by construction (chunks = SHA-256 content-addressed) — бракує лише persistence+eviction.
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

| # | Рекомендація | Зусилля | Ефект | Статус |
|---|---|---|---|---|
| R1 | **Predicted-latency EWMA routing** in `route_peers`: tier-0 = fresh EWMA of recent `weights_matvec` samples (< 3600 s horizon, α=0.3), followed by fresh-proofed → stale-proofed → unproven probation | S | −60..80% makespan for blk dispatch | ✅ **DONE `3ba2353`** — live A/B `g-199` |
| R2 | **Stale-proof TTL**: heavyweight gate requires ≥1 fresh proof (< `PROOF_FRESH_SECS=3600`) while any fresh peer is alive; falls back to all-time proofs when nothing is fresh fleet-wide (no lockout). `ready`/`ready_proofs` telemetry unchanged; new `proofs_fresh` field | S | FF-PC ghost excluded from MiniCPM dispatch | ✅ **DONE `3ba2353`** (+ `peer_poll` + enqueue gate, `proof_gate()`; telemetry in `/api/sys/metrics`) |
| R3 | **Persistent peer chunk cache** (IndexedDB/OPFS): peer-JS kept chunks in memory only (`const CACHE={}`, per-page-session) → every fresh page load re-downloaded tensors. Chunks are **SHA-256 content-addressed** (`chunk?key=` + `crypto.subtle.digest` verify) ⇒ staleness impossible by construction; no invalidation needed, only LRU quota. Implemented: `gsv-mesh-weights` IDB, `IDB_CAP=128 MiB`, read-before-fetch, all failures fall back silently | M | −16 MiB × tensor re-download per reload; phone wake latency ↓↓ | ✅ **DONE `9691c7c`** — live `/wgpu` markers verified |
| R4 | **Multi-tensor block op** `weights_matvec_block`: one task per shard carries ALL shexp items for that block (gate/up/down). Hub verifies each item per-item (golden-x max_err_model). | M | Env-per-layer wake reduction (3 wakes → 1) | ✅ **DEPLOYED `0da304f` (code) — activation gated by phone page reload**; first live A/B `g-282` returned 4× `status=failed` (peer JS still on cached build); fallback `weights_matvec` verified green `g-283` (err ≤ 6e-7). Unit coverage added. |
| R5 | **Replicated-shexp local-inference mode** (16/32/48 MiB/peer): якщо peer може тримати 1 blk — виконує без mesh round-trip | M–L | Найбільший speedup для phones | 🅿️ **PARKED — design note** [`R5_DESIGN_NOTE_2026-11-18.md`](R5_DESIGN_NOTE_2026-11-18.md): R3 already covered the cheap 80 %; full local-inference conflicts with per-task `max_err_model` verification + phone hot-swap model; candidate subsumed by R4 |
| R6 | **EP exploration for 35B routed experts** — only after R4/R5 | L + owner gate | Диспетчіалізація 1.9 GB | OPEN |
| R7 | **Keep max_err_model golden-x** as dispatch correctness gate, plus **p50/p99** report in benchmark | S | Краще SLO visibility | ✅ **DONE `9691c7c`** — MESH_BENCHMARK_2026-11-18.md §R7 addendum (fleet n=150; bimodal; per-peer p50/p90/p99) |
| R8 | **FF-PC root-cause**: `about:config` WebGPU suspend log; or accept FF-PC as QA-tier | S owner | Повернути RDNA2 як eager-tier | OPEN — owner |

### 3.1 Live A/B evidence — dispatch `g-199` (2026-11-18, post `3ba2353`)

Dispatched `weights_matvec blk.12.ffn_gate_shexp.weight` shards=4 via `/api/wgpu/task/enqueue`:

```json
{"group_id":"g-199","route_peers":[35,38,36],"task_ids":["mt-199-0","mt-199-1","mt-199-2","mt-199-3"]}
```

**FF-PC (37) is absent** from the heavyweight route pool — `proofs_fresh=0` (all 25 proofs older than the 3600 s horizon), while valhall (18) and bifrost (21) are fresh. Claims followed route hint + sticky window with any-alive fallback:

| task | peer | elapsed | max_err_model |
|---|---|---:|---:|
| mt-199-0 | #35 valhall | **22 ms** | 4.47e-7 |
| mt-199-1 | #36 Pi4 (fallback claim) | 1668 ms | 6.57e-7 |
| mt-199-2 | #38 bifrost | **13.7 ms** | 4.52e-7 |
| mt-199-3 | #38 bifrost | **176 ms** | 3.66e-7 |

Phones claimed instantly; Pi4 took one shard via fallback — exactly the intended traffic shaping (before R1, the fixed `[35,38,37,37,36]` handed FF-PC up to half the shards with zero completions). `/api/sys/metrics` now carries `proofs_fresh` next to `ready_proofs`.

---

## 4. Що конкретно робити ДАЛІ (практично)

1. ~~**R1 + R2**~~ — **DONE** (`3ba2353`; 509/509 lib, clippy 0, fmt clean; live take-over via `/api/update/apply`; autonomous HKCU vbs respawn verified, outage ≈10 s).
2. ~~**R7**~~ — **DONE** (`9691c7c`; MESH_BENCHMARK_2026-11-18.md §R7 addendum: n=150, fleet p50=1693.7 ms / p99=12702.5 ms; per-peer honest view: valhall p50=17.9 ms, bifrost p50=123.0 ms, Pi4 p99=18.8 s).
3. ~~**R3**~~ — **DONE** (`9691c7c`; page-JS `gsv-mesh-weights` IDB, read-before-fetch verified live at `/wgpu`; 510/510 lib, clippy 0; SW is network-first so peers pick it up on next page load).
4. **R4** — `weights_matvec_block` deployed as `0da304f`; smoke ran `g-282` (4/4 `status=failed` — peers still on cached page). Fallback tested green (`g-283`). **Activation requires one page reload per phone**, then re-dispatch `blk.31` via the same envelope.
5. Document remains discovery/decision canon — owner gates unchanged.

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
