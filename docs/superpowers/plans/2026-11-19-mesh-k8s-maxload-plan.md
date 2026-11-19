# Plan: максимальне навантаження mesh-кластера через wgpu з docker/k8s (2026-11-19)

**Owner ask (через чат):** «тест кластер через wgpu з зеркала взяти docker образ та ваги для кожного піра визначені з NanoJev та потужностями кластера і хабу, з докерконтейнерами — максимальне навантаження сплануй. докер кубернетіс.»

**Статус:** план (не виконання). Ключові виміри зроблено; рішення власника потрібні до фази 1.

---

## 1. Що є зараз (знято живцем, 2026-11-19)

| Складова | Стан |
|---|---|
| Хаб `gsv-server` | живий, `:9999`, RAM ~90 %, TTFB 2–3 s, CPU ~25–97 % |
| Піри | `#35 valhall` (phone, offline periodically), `#36 Pi4` (**тепер на `GSV-Peers-6442`, 192.168.137.222**), `#37 FF-PC` (Windows), `#38 bifrost` (phone) |
| 48/48 shard-групи 35B | розселено й перевірено (`max_err ≤ 3.5e-6`) |
| Mesh-роутинг | R1 EWMA + R2 stale-proof, routing через hub (не pods) |
| Mirror `L:/gsv_mirror` | ~24 GB вільно (blk5/blk7..12 shexp f32 ~4 MB + `wgpu_weights/`) |
| Mirror `L:/qwen4lama_rs` | Qwen3.8-27B-UD-IQ2_XXS.gguf (6.8 GB) |
| **NanoJev canary** | **відсутній на диску** (GSV/poolAI `data/models/nanojev/` — порожньо; WSP-прогони були ephemeral) |
| **docker / podman / WSL2 / k8s** | **немає ніде** (host: no docker/podman; Pi: no docker; host Windows 11, WSL2 not installed) |
| poolAI docker image | `S:/rust/poolAI/docker/Dockerfile`: multi-stage `rust:1.87-slim` → `debian:bookworm-slim`; binary=`poolai`; ports 8080/8443; **x86_64-only pin; не перевірявся для ARM64; Rust 1.87 прибитий (зараз 1.100)** |
| poolAI k8s support | **consumer-only**: `cloud-sdk` (`k8s-openapi` client — deployer для existing k8s), без k3s/k3d/kind/helm у репо; деплой на наявний кластер, не власноручний |

## 2. Розрив між запитом і реальністю

| Запит | Реальність |
|---|---|
| «взяти docker образ з зеркала» | Зеркало (`L:/gsv_mirror`) містить вагові shards, **не docker image**. Docker-образів ні на диску, ні в реєстрі немає |
| «ваги для кожного піра визначені з NanoJev» | NanoJev GGUF відсутній на диску — треба або скачати (~MB), або замінити canary на golden-tensor перевірку наявних shards (рекомендовано) |
| «з докерконтейнерами» / «кубернетіс» | На жодному вузлі немає docker/k8s. Хост Windows 11 (RAM ~90 %, hub RAM-tight), єдина Linux-нода — Pi4 (3.8 GB RAM, 4 ядра, 2.6 GB SD вільно) |
| «максимальне навантаження» | Тобто k8s-плани вмикають розклад по pods; pods можна запустити лише після встановлення k3s (або Docker Desktop+WSL2 на host) |

## 3. Варіанти розміщення кластера

| Варіант | Деталі | Реальність |
|---|---|---|
| A. Docker Desktop + k3d на host | найпотужніший single-node k8s (Ryzen/AMD, 32 GB?, 4+ ядра) | великий крок; WSL2 + Hyper-V; RAM конфліктує з хабом; x86_64 (ок) |
| B. k3s на Pi4 single-node | після `curl -sfL https://get.k3s.io | sh -`, 1 node + embedded etcd/sqlite, RAM ~1 GB, SD ~1 GB | **ARM64** — образ poolAI не запуститься (x86_64 binary); потрібен ARM64 build або мінімальний busy-образ |
| C. Docker smoke без k8s (Pi) | `docker run busy-image` → container доmesh-runner → self-test golden tensor | найменший ризик; перевіряє pipeline image→container→GPU/CPU; k8s переносимо в фазу 3 |
| D. GitHub-hosted агенти + kind кластер | k8s на GH Actions хмарі; 35B не влазить; weights-heavy → повільно/дорого | не підходить під «максимальне навантаження reallie» |

**Рекомендовано: C → B** (поетапно), host докер не чіпати (RAM-хисткий хаб).

## 4. Планові фази

### Phase 0 — owner decisions (до будь-яких змін)
Рекомендовано: Pi (B/C) + ARM64 busy-образ замість poolAI-образу + golden-shard canary замість NanoJev + mirror як golden cache (без push до registry).

### Phase 1 — Docker smoke на Pi (без k8s)
- встановити Docker CE ARM64 на Pi (apt: docker.io, ~100 MB, ~30 s — але SD 2.6 GB вільно!; якщо не тісно — docker.io + hello-world)
- зібрати `gsv-mesh-runner:latest` (user-owned образ, вміст: debian bookworm-slim + mesh-node binary @ /usr/local/bin + golden shard blk5 f32 (4 MB) + self-check script)
  - **ARM64** обов'язково — x86_64 image не запуститься на Pi
- запустити 1 контейнер: `docker run --rm gsv-mesh-runner sh /self-check.sh`
  - self-check: читає shard, бере x-vector, рахує matvec (f32), порівнює з golden row з L:/gsv_mirror/blk5/ (скачати через scp)
- критерій фазу завершено: max_err_model < 1e-6 на Pi; контейнер самоочищується; SD не падає

### Phase 2 — k3s single-node на Pi
- `curl -sfL https://get.k3s.io | sh -s - --write-kubeconfig-mode 640` (~60 s)
- namespace `gsv`; ConfigMap: topology `#36 = Pi4 @ GSV-Peers-6442`; pods:
  - `mesh-runner` × 1 (requests 250m/512Mi, limits 1/1Gi) — golden matvec loop
  - `metrics-exporter` × 1 (prometheus scrape через mesh telemetry)
  - `weights-loader` × 1 (initContainer: скачує shard з L:/gsv_mirror через scp прямо з host соба Pi)
- Service ClusterIP; probe: liveness = self-check через 5 s
- перевірити: `k3s kubectl get pods -n gsv -o wide` — 3/3 Running; telemetry `/api/sys/metrics` показує Pi з новим iface/process

### Phase 3 — Max-load dispatch (через k8s)
- що має сенс робити: перезапуск мешевого dispatcher'а в контейнері, що витягує shards через HTTP і робить matvec на Pi
- та в 2-3 верстці для стресу плюс паралельний run на host-мешу для порівняння (pods vs bare-metal Pi)
- виміри: wall-time для 2, 4, 8, 16 shards; max_err_model (golden); hub/pod RAM; CPU pod; переваги:
  - bare-metal Pi self-test result == (тензор, результат) Pod result == однаковий bitwise?
  - 10 roundів підряд → перевіряємо чи нема memory-leak у pod
- відкат: `kubectl delete ns gsv; systemctl stop k3s; docker rmi gsv-mesh-runner`

### Phase 4 — звітування

---

## 5. Canary (заміна NanoJev)

NanoJev GGUF відсутній на диску. Замість нього — **golden-shard canary**:

- golden: `L:/gsv_mirror/blk5.ffn_up_shexp.weight` (4 MB f32) + x-vector (з `xDet()` у mesh-node) + recorded `max_err_model ≈ 2.384185791015625e-07`
- у контейнері/pod рахується `matvec(x, shard)`; якщо `|result - golden| < 1e-6` — пір пройшов; сума=peer-class capability registry

Це те саме, що робить мешева гейт-система (`proof_gate`), але всередині контейнера/під k8s.

## 6. Критерії успіху

| # | Критерій | Гейт |
|---|---|---|
| 1 | k3s + 3 pods на Pi в namespace `gsv` | `kubectl get pods -n gsv` → 3/3 Running |
| 2 | Golden matvec у pod == bare-metal результат | `max_err_model < 1e-6` |
| 3 | Max-load dispatch: 2/4/8/16 shards через pods | wall time записано; без OOM; fail=0 |
| 4 | SD не падає нижче 1.5 GB вільно | `df -h /` |
| 5 | Pi mesh agent — still alive (пережив docker+k3s на тому ж вузлі) | `/api/wgpu/mesh` → peer #36 alive |

## 7. Ризики

| Ризик | Митигація |
|---|---|
| Pi OOM при docker+k3s (3.8 GB total, ~3.5 GB available, hub вже ~90 % host RAM) | Phase 0 mini-benchmark: `free` до/після; якщо k3s не влізе — C only, k8s cancel |
| SD картка (2.6 GB) — docker images можуть бути важкі | busy-образ ≤ 120 MB; docker.io само 100+ MB — перевіряти на кожній фазі |
| π agent — WS/indexable вихід за NAT | pod не може піти сам; допоможе HostToPoD-net (metrics-exporter → hub HTTP) |
| poolAI образ x86_64 не запуститься на Pi | ARM64 busy-образ (рекомендація); ARM64 poolAI — під як окремий owner-гейт |
| Wi-Fi RTT 38 ms | Pod overhead ≈ 38 ms; для max-load dispatch це не критично (net to hub вже 2–3 ms hub-side) |
| хаб RAM | навряд торкнеться (dispatch робиться на Pi), але вимірюємо |

## 8. Owner decisions (без цього план не починається)

1. **Де k3s/docker?** (рекомендую: Pi single-node; host не чіпати RAM-tight)
2. **Що в контейнері?** (рекомендую: ARM64 busy-образ + self-check; x86_64 poolAI не піде на Pi)
3. **Canary без NanoJev?** (рекомендую: golden blk5-shexp; NanoJev завантажити окремо, якщо потрібен)
4. **Mirror semantics?** (рекомендую: L:/gsv_mirror як golden warm cache, без registry push)
5. **Чи треба migration mesh-агента в pod?** (рекомендую: ні; mesh-node залишити в systemd; pod-раннер — лише для max-load виміру)

## 9. Що виміряти

- wall-time для 2, 4, 8, 16 shards; kernel peak RAM per pod; node CPU; pod restart count; golden matvec 10 rounds поспіль
- Hub metrics: TTFB, RAM hold, hub mesh alive/ready після dispatch
- SD: вільне місце до/після

## 10. Out-of-scope

- poolAI cloud-sdk deploy до existing k8s — це інший тикет (cloud-sdk déjà є, але наш use case локальний)
- 35B forward на Pi (ні при якому сценарії)
- телефони в k8s (неможливо — web-піри)
- повна migration mesh на pods
