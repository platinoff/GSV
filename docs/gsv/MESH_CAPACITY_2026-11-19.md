# Mesh capacity & tuning — 2026-11-19 (live measurements)

**Context:** Adoreble Wi-Fi хотспот `GSV-Peers-6442` (2.4 GHz, AC-8265, BELL409-ICS) піднятий 2026-11-18 (ADO-3); піри `#35 valhall` (Galaxy A54) і `#38 bifrost` (Android) на AP; `#36 Pi4` — на BELL409 LAN (`192.168.2.168`); `#37 FF-PC` — на цьому ноуті (Firefox, Firefox PC). Хаб `gsv-server 0.248.0` слухає і на `192.168.137.1` (AP) і на `192.168.2.238` (BELL409) — обидва інтерфейси доступні пірам одночасно.

## 1. Канали (виміряно 2026-11-19 ~01:15)

| Канал | Стан | Показники |
|---|---|---|
| Hub → AP gateway `192.168.137.1` (2.4 GHz) | ✅ ping 0 ms, 0% loss (8/8) | HTTP TTFB 2.1–2.2 s (хаб CPU/RAM-bound, не лінк) |
| Hub → BELL409 LAN `192.168.2.238` (5 GHz ch44) | ✅ ping 0 ms, 0% loss | HTTP TTFB 2.0–3.0 s |
| `192.168.2.1` (BELL409 gateway) | 2–134 ms avg 49 ms | зовнішній лінк непередбачуваний — причини флапів пірів |
| Hub disk read probe | ≈ **387 MB/s** (1 MiB random read) | 61.6 % зайнято (169 GiB) |
| Хаб Wi-Fi uplink (netsh) | BELL409 5 GHz ch44 802.11n, RSSI −65 dBm, signal 99 % | ↓/↑ **270 Mbps** PHY |
| Телефон #35 / #38 (NetworkInformation) | 4g, rtt 100 / 50 ms, downlink 1.45–1.75 Mbps | браузерний canary — реальна TCP-віддача вища |
| Pi 4 (mesh-node) | wlan0 → **GSV-Peers-6442** (перенесено 2026-11-19), IP 192.168.137.222, `MESH_HUB=http://192.168.137.1:9999`, host ED25519 збігається з 2.168, NM profile GSV-Peers-6442 (priority 20) з фолбеком на BELL409 (priority 0) | autoconnect-priority 20 · hub age=1s |

**Висновок:** усі локальні шляхи не звужують меш — вузьке місце №1 = **хаб** (RAM ~90 %, TTFB 2–3 s), не радіоканал. 2.4 GHz AP — найнадійніший шлях для телефонів (0% loss, власник підтвердив стабільність). BELL409 далі хороша як fallback.

## 2. Mesh бенчмарк (probe `matmul64 ×100`, quota-gated, 2026-11-19)

| task | op | iters | route→peer | peer виконав | MFLOPS | elapsed |
|---|---|---|---|---|---|---|
| mt-329 | matmul64 | 100 | 38 | **#35** | 217.2 | 2.41 ms |
| mt-330 | matmul64 | 100 | 35 | **#37** | 327.7 | 1.60 ms |
| mt-331 | matmul64 | 100 | 35 | **#38** | 687.1 | 0.76 ms |
| mt-332 | matmul64 | 100 | 38 | **#35** | ~690 | 0.76 ms |
| mt-333 | matmul64 | 100 | 35 | **#37** | 672.2 | 0.78 ms |
| mt-334 | matmul64 | 100 | 37 | **#35** | 3139.5* | 0.17 ms |
| mt-335 | matmul64 | 100 | 37 | **#37** | ~672* | ~0.8 ms |
| mt-336 | matmul64 | 100 | 38 | **#38** | 879.7 | 0.60 ms |

*mt-334 — підозріло високий MFLOPS; імовірно перший warmed-GPU прогін або некоректний лічильник браузера. Реалістична смуга: **~600–880 MFLOPS** (full dispatch), cold-start ~200–330.

Маршрутизація розподілилась рівномірно (route_peer 38×3 · 35×3 · 37×2), усі `max_err = 0`. **#37 (FF-PC) виконав першу роботу** після довгого «ghost»-періоду.

Stand-smoke на живому хабі після усіх сьогоднішніх редеплоїв: **87 passed / 0 failed**.

## 3. Доналаштування, зроблені на основі цих даних

| Зміна | Де | Ефект |
|---|---|---|
| Телеметрія mesh-сторінки 60 → **5 s** | `src/boxes/wgpu.rs` | піри фактично live в /sys |
| /api/wgpu/mesh **fresh-row bugfix** | `src/boxes/wgpu.rs` (`aa5f25d`) | capability `alive:false` для всіх → freshest-row вибір; live: усі alive/ready |
| /sys: disk usage bar + read-speed probe | `src/boxes/sysmon.rs` (`43af0c8`) | видно capacity; 61.6 % / 387 MB/s |
| /sys: WAN Wi-Fi row (netsh) | те саме | uplink чесно видно: 270 Mbps, RSSI, signal |
| /sys: LAN interfaces rows | те саме | усі 3 інтерфейси з масками |
| /sys: per-peer net bars | те саме | downlink/bat/link_speed міні-шкали |
| CREATE_NO_WINDOW для netsh/ipconfig проб | `cd136a9` | cmd-flash bugfix |
| pa11y a11y фікси (poolAI) | `a4f975825` | WCAG 2.2 AA в CI |

## 4. Що це означає для тюнінгу екосистеми (рекомендації)

1. **AP — primary для телефонів, BELL409 — fallback.** Тепло відносно F зі smoke-тестами: телефони на 137.1 отримують 0% loss, і хаб один доступний з обох підмереж.
2. **Хаб гальмує диспетчерність**, не піри: RAM ~90 % постійно, TTFB 2–3 s. Перед масовими dispatch-ами — або зменшити супутні процеси, або винести роль serve на Pi. Не підвищувати паралельність понад 1 heavyweight/peer без звільнення RAM.
3. **Pi 4** лишається на BELL409 (інакше ризик втрати його робочої мережі); його пропускна спроможність — CPU-ops (`layernorm`, `softmax`, `minilm`), не matmul-critical-path.
4. **#37 FF-PC**: після перших живих result-ів (mt-330/333) proofs є — можна ставити в тяжкі shards з обережністю (він smoke-host, не резерв).
5. **Mesh dispatch**: маршрутизація predicted-latency (R1/R2) уже розносить навантаження; з поточними числами оптимально тримати `max_concurrent_per_peer = 1` для weights_matvec_block.
6. **Телефонна батарея**: якість мережі > 5с телеметрії практично не впливає на заряд (крихітні POST-и), лишаємо 5 s.

## 5. Чесно не виміряно

- Throughput хотспота під навантаженням (потрібен iperf-подібний клієнт на піра).
- 5 GHz 80/160 MHz link rates на BELL409 (netsh показує тільки поточну оцінку 802.11n 270 Mbps).
- End-to-end токен/с serve-стека (serve зупинено; потребує окремої сесії).
- Між більшістю наших запитів ≈ 2 s TTFB — це host saturation, не мережа.
