# Mesh cluster stress test — 2026-11-19 (live, controlled)

**Методика:** контрольований burst-тест меша через хаб (`POST /api/wgpu/task/enqueue`), три фази, усі задачі пройшли через той самий dispatch-роутер (R1/R2 EWMA predicted-latency + stale-proof TTL), який використовується у production-блок-діспетчі. Hub load до тесту: RAM 89 %, CPU ~97 % (хаб і так напружений). Продуктивність меша НЕ порушувала нічого; піри продовжували нормальну роботу.

## Baseline (до тесту, 1775051420)

- hub: ram_load 89 %, cpu 96.7 %, uptime 1858 s, вільно диска ~65 GB
- mesh: queued 0 · claimed 0 · done 180 · failed 20 (историчное)
- піри fresh: #35 age 3s, #36 age 1s, #38 age 1s — усі ready

## Phase A — burst 30× matmul64 (52 MFLOP/task, 1.57 GFLOP)

- 30/30 done, **max_err = 0, failed = 0**
- Dispatch spread: route_peer 38×3 · 35×3 · 37×2 → фактично виконано #35×6 · #36 Pi×13 · #37×9 · #38×2
- Wall: enqueue 106 s → drain ~15 s → total 121 s
- MFLOPS/оп: cold ≈ 98–330, warm ≈ 640–3139 (браузерний лічильник; caveat: 0.17 ms прогони браузер-GPU, трактувати як upper bound)

## Phase B — mixed ops burst (12 layernorm + 12 softmax + 12 mlpblock, iters 32/32/8)

- 36/36 done, **failed = 0**; total 139 s
- mlpblock: Pi #36 ≈ **22 ms/op** (CPU, стабільно); телефони ≈ 148–306 ms/op; FF-PC #37 ≈ 171–292 ms/op
- max_err ≈ 2.7e-5 (browser f32-ordering vs Rust golden — у межах browser tolerance, всі shard-перевірки 0; це не golden-mismatch)
- softmax/layernorm: усі max_err = 0.0

## Phase C — heavyweight re-verification: `blk.5.ffn_up_shexp.weight` ×2 shards (group `g-403`)

- 2/2 done, max_err_model ≈ **2.4e-7 / 3.0e-7** — узгоджено з попереднім golden blk.5 (~2e-7), bench підтвердив регрес-стабільність після усіх сьогоднішніх редеплоїв
- wall: enqueue → drain ≈ 22 s (1 GFLOP total)

## Підсумок по стресу

| Метрика | Значення |
|---|---|
| Задач виконано | **68/68** (30 matmul64 + 12 layernorm + 12 softmax + 12 mlpblock + 2 weights_matvec) |
| Failed | **0** |
| Маршрутизація | 0 блокувань, EWMA-route працював під навантаженням, Pi отримував більшість (route перевага Pi для CPU-ops) |
| Hub після стресу | ram 90 %, cpu 68.8 % — **стабільний**, watchdog цілий, health 200 |
| Піри після стресу | усі alive/ready (age 1–5 s) |

## Що це доводить / не доводить

- ✅ Меш витримує burst-навантаження 3+ задач паралельно без втрат і без деградації якості (0 numerical failures).
- ✅ Routing справедливий під тиском (Pi стабільно навантажений — це нормально: він найшвидший на CPU-ops, phones щадить).
- ❌ **НЕ** доводить: production serving повної 35B (потрібен serve-стек + 24 GB вузол, якого нема), nor throughput хотспота під радіо-навантаженням, nor стабільність 2–4 годин поспіль (тест був ~4 хв).
- ⚠ Hub RAM ~90 % лишається вузьким місцем для будь-якого serve з тими ж умовами; stress не знижував якість, але і не використовував 35B forward.
