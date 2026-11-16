# Екосистем ревʼю 2026-11-16 — стани продуктів · ресерч подібних екосистем · адаптовані наступні дії

**Дата:** 2026-11-16 · **Хто:** OpenCode free-lane (GLM) · **Контекст:** MiniCPM mesh + ECO-7.4 + `/sys` live (GSV crate 0.248.0, HEAD `24ff669`)

## 1. Стани продуктів (скан 2026-11-16, S0: disk 72 GiB free / 58%)

| Репо | HEAD (local) | dirty | ahead/behind vs origin | Стан |
|------|--------------|-------|------------------------|------|
| GSV | `24ff669` 2026-11-15 | 0 | 0/0 | crate 0.248.0 · /sys live · 1049 tests |
| GSV-WGPU | `c381a4f` 2026-11-15 | 0 | 0/0 | 35B план g-72..g-76 |
| rusty-engine | `d7eb58a` 2026-11-15 | 0 | 0/0 | qwen35moe validate✅ · runner35 · serve dogfood |
| poolAI | `964628ee9` 2026-11-09 | 0 · CRLF-шум відкочено | 0/0 | POOL row clean · WSP brief untouched |
| telenetis | `7efad09` 2026-11-08 | 0 | 0/0 | up на :9800 |
| llama-rs | `f8d927a` 2026-11-06 | 0 · CRLF-шум відкочено | 0/0 | **ECO-8.2** (llama-cpp-sys-2 bump) відкритий |
| LinFS | `141aab5` 2026-11-09 | 0 · CRLF-шум відкочено | 0/0 | ECO-8.3 ✅ |
| ORR_DESKTOP | `40298ab` 2026-11-09 | 0 · CRLF-шум відкочено | 0/0 | ECO-8.3 ✅ |
| sport-forcast | `3d8341e` 2026-11-09 | 0 | 0/0 | ECO-8.3 ✅ |
| **CoreBoard_Labs_shop** | `91480c2` 2026-11-09 | 0 | **ahead 1** | ECO-8.3 коміт не запушено — **потрібне рішення owner** |
| rebook | `6424975` 2026-11-08 | 0 · CRLF-шум відкочено | 0/0 | ECO-7.7 owner-gated |
| omniroute | `e3d202c` 2026-11-06 | 0 | 0/0 | policy-down |
| platinoff-profile | `5d81430` 2026-10-29 | 0 | 0/0 | clean |
| freebeat.ai | `87daf9e` 2026-10-29 | 0 | 0/0 | clean |

CRLF-шум (`AGENTS.md` ×4, WSP brief) відкочено за owner-правилом «CRLF-шум → `git checkout -- <file>`»; вміст не змінювався (`git diff --ignore-cr-at-eol` = порожньо).

## 2. Ресерч подібних екосистем (2026-11-16)

### 2.1 Розподілений інференс по пристроях — exo / Mesh LLM / llama.cpp RPC / Distributed Llama

- **exo** (archived; наступник — **Mesh LLM**): LAN auto-discovery (mDNS), **кільцеве розбиття ваг ∝ RAM** кожної машини (memory-weighted ring), re-partition 10–60 s при join/leave, опційний cloud peer для важких шарів.
- **Mesh LLM**: QUIC/iroh mesh, capability-gossip, **per-model election**, пакетна поставка GGUF-шарів (layer packages), **«процес, що просто народився, не routable»** — readiness-gate перед маршрутизацією; Skippy stage splits; session-affinity для префікс-кешу.
- **llama.cpp RPC**: явні хости, `--tensor-split`, потрібно <5 ms LAN — інакше per-op мережевий RTT убиває декод; для Wi-Fi-телефонів tensor-parallel **не життєздатний**. Це підтверджує наш шлях **грубих shard-оффлоадів** (повні тензори `weights_matvec`, активації-sized payload), а не per-token all-to-all.
- **vLLM / Codeflare / MeshRouter** дають control-plane канон: readiness 200 **до** видачі трафіку; session-affinity pinning; explicit pin hints + strict lane pin з fail-loud (не silent fallback); routing знаходиться **у control-plane**, не всередині моделі.
- **Висновок для MiniCPM mesh:**
  1. Partition по **RAM-weighted ring** лишається каноном, але для телефонів це означає «шари ffn_shexp group g-73…» на кожен alive+ready пір — вже доведено (g-72..g-76, `max_err ≈ 1e-6`).
  2. **Readiness-gate перед розміщенням** — канон (Codeflare/vLLM), у нас його немає: пір стає candidate тільки після mt-тестів з `max_err < 1e-4`.
  3. Документ-джерело істини = `wgpu_peers/tasks.jsonl` — вже є; бракує **placement API**, який детерміновано видає групу peer→shard.

### 2.2 MoE-оффлоадінг — MoE-Infinity (Expert Activation Matrix)

- **Суть:** full Q4 35B міститься в host RAM; hot experts resident на GPU; **trace-based prefetch** ховає I/O; **початкові шари пріоритетніші в кеші**; <5 % експертів багаторазово активуються на sequence (>100-expert); **expert parallelism round-robin across servers** + host-staging fallback.
- **Висновок:** наш PH-404 (ExpertCache) — той самий вигляд. Брак активаційних лічильників — **EAM-подібний trace** потрібен **до** eviction-евристики; першим шарам вищу кеш-пріоритетність; host-staging fallback замість hard fail при miss.

### 2.3 Ведення документації/планів у подібних екосистемах

- exo / Mesh LLM / llama.cpp тримають дизайн **у репо** (RFC/архдоки), gates на тегах, стан системи — через heartbeat/logs, **не через документ**.
- GSV kit уже робить це (HANDOFF/NEXT/roadmap/tickets/fingerprints). **Виявлена щілина:** стани продуктів ніде не агрегуються одним файлом — цей документ фіксує їх вручну; канон відсьогодні — рядок `products --state` (див. PH-A4).

## 3. Адаптовані наступні дії

### 3.1 Негайні (owner)
- **CoreBoard_Labs_shop**: ahead 1 (`91480c2` ECO-8.3) — запушити (owner) чи залишити?
- **RAM**: 86–91 % завантаження, 2+ смерті хаба за 15.11 — звільнити до наступних збірок; інакше `/sys`-дані будуть недостовірні для placement-рішень.

### 3.2 Документація (depутале цього ревю)
- цей план; `docs/NEXT_SESSION_PROMPT.md` banner (2026-11-16 review); `docs/HANDOFF_NEW_SESSION.md` review row; ECO-10 у `docs/gsv/GSV_TECH_ROADMAP.md` — дослідне node.

### 3.3 Кандидати бендів
- **PH-A1** ✅ **shipped 2026-11-16** (commit `24ff669`+1, deploy `target/live/gsv-server.exe`): пір отримує `ready` лише за ≥1 proof-задачі з `max_err_model < 1e-4` / `max_err < 1e-6` (з `wgpu_tasks.jsonl`); `/api/sys/metrics` — `ready`/`ready_proofs`/`ready_alive`/`shard_gb_ready_est`; `/sys` — ready-колонка + Σ ready; shard-est для pc-wgpu #28 невідомий (adapter не репортить `navigator.deviceMemory`) → 0, чесно. **Живий стан:** alive 3 (#28 pc-wgpu ready=2 proofs; #33/#34 phones not ready — 0 proofs, не брати в placement). Route-гейтинг — у PH-A2 за дизайном.
- **PH-A1** Readiness gate: пір стає `ready` лише після mt-тестів з `max_err < 1e-4` (за `wgpu_tasks.jsonl`); `/sys` колонка ready; mesh-route виключає unready.
- **PH-A2** ✅ **shipped 2026-11-16** (deploy `target/live/gsv-server.exe`): `GET /api/mesh/placement?model=<profile>` — детермінований greedy placement, fail-loud excluded-reasons, `ok:false` + `known_models` на невідомий профіль. Профілі: `minicpm-1b-blk0` (1 MiB), `minicpm-35b-shexp-blk0` (3× 4 MiB blk.0 ffn gate/up/down shared-expert, ≈ спостережувані розміри), `minicpm-35b-full` (завжди shortfall → PH-A5 hint). Кандидати: alive + proof ≥1; peers без reported memory → емпіричний cap 32 MiB (доведені ≤4.2 МіБ shardи g-73..g-76). **Live:** #28 pc-wgpu → 3/3 shardи placed; #33/#34 phones виключені (0 proofs); #26/#27 stale (не alive). Route-гейтінг (виклик placement з mesh-шляху) — лишився для наступного зрізу.
- **Автопіринг пірів ✅ shipped 2026-11-16**: `device_key` = SHA-256(adapter|UA|class|cores|device_memory, length-prefixed); той самий пристрій, перезавантажившись, переюзовує `peer_id` (кріс в mixed стані реєстру). Auto-prune у `peer_join`: stale (lease expired > 24 h) + без proofs → row дропдається; живі й доказові зберігаються. Одноразове live-прибирання виконано: 34 → 30 rows (dropped 5/6/10/13), backup `target/live/wgpu_peers.bak-*.jsonl`. Після релізу реальні телефони отримають стабільні id з device_key при наступному reload.
- **PH-A3** Activation trace: EAM-подібні лічильники експертів у JSONL (наприклад `data/expert_activations.jsonl`) → feeds PH-404 (ExpertCache prioritization).
- **PH-A4** ✅ **shipped 2026-11-16**: `cargo xtask products --state` — one-liner per repo (HEAD / branch_line / dirty / ahead / behind / last_commit_age_secs / crate_version / registered); `--json` для raw JSON. Запущена наживо 2026-11-16: 14 продуктів, всі чисті крім `gsv dirty=3` (саме ця робота) і `coreboard_labs_shop [ahead 1]` (owner decision). Замінює кустарні bash-огляди.
- **PH-A5** 35B forward на виділеному вузлі (owner decision; потрібен ≥ 24 GB RAM node; PH-401..406).

### 3.4 Двигуни / rusty-engine (незмінно)
- PH-1202 (35B weight load у runner35) → PH-1206 (MoE core + ExpertCache) → PH-404/405. Serve-догфуд на 1B живий; chat-template gap — тікет `t-1794778713443485500`.

### 3.5 Екосистема (без змін за пріоритетом)
- **ECO-8.2** llama-rs `llama-cpp-sys-2` bump — окремий бенд; ECO-8.3/9/7.4 закрито ✅.

### 3.6 WSP
- MC-6 ROLLBACK; **не рушати**; рішення owner за Phase 2 (гібрид RL+WSP) — лише після явного go.

## 4. Залишки ризиків (чесно)
- Telegram tests flake під паралельним cargo test (`dry_run_queue_round_trips_two_messages`) — ізольовано зелено; моніторити.
- Phone battery/VRAM/true CPU% недоступні з браузера — потрібен `gsv-apk` (нативний) для повних метрик.
- Хаб die-silent з невідомої зовнішньої причини (≥ 6 смертей 15.11) — EDR/антивірус або RAM; owner-scoped.
- 35B full forward **неможливий** на цьому боксі (7.6 GB RAM) — лише dedicated node.

## 5. Коміти цього ревю
- docs-only: цей план + 3 файли адаптації (див. git log).
