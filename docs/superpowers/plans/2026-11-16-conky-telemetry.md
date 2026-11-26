# Conky `/sys` — розширення телеметрії з пірів і мережі (2026-11-16)

**Мета:** зробити `/sys` справжнім «кокпітом екосистеми» — кожен пір показує реальні можливості (ядра, RAM, GPU, радіо/мережа, температура, заряд, ряльова якість каналу), а не просто `adapter/class/alive`. Чим точніші дані, тим краще ми калібруємо dispatch, placement, fleet sizing і діагностуємо деградацію до збою.

**Стан зараз:** піри репортують `adapter`, `ua`, `device_class`, `cores` (Chrome), `device_memory` (Chrome, округлення степеня 2), `last_seen_ts/lease_until`. `/sys` додає: `ready`, `ready_proofs`, `est_shard_gb`, `weight_max_err`, `best_task_mflops`, `last_seen_age_secs`. Хаб вимірює: CPU%, RAM, disk, commit, Top RSS, uptime.

---

## 1. Живий recon (зроблено 2026-11-16 на Pi4 + пробних пірах)

### Pi4 (нативний агент, Linux) — реально доступні **без root/без tool'ів**:
| Джерело | Приклад (live) | Цінність |
|---|---|---|
| `/proc/net/wireless` | `wlan0: 50. -60. -256` → link=50/70, signal=-60 dBm | **реальний WiFi RSSI/якість** для класифікації каналу |
| `/sys/class/net/wlan0/statistics/rx_bytes` / `tx_bytes` | 235,602,182 / 2,835,940 | пропускна здатність (delta/poll) |
| `/sys/class/net/wlan0/speed` | `Invalid argument` (Broadcom обмеження) | link speed для wlan0 недоступний; **eth0 працює** (`speed: 1000`) |
| `carrier` | 1 | канал угору/вниз |
| `/proc/meminfo` | MemTotal 3,879,688 kB; MemAvailable 3,608,900 | точна RAM (не Chrome-округлена 4/8) |
| `/proc/loadavg` | `0.13 0.14 0.57 1/165 9676` | нававантаження |
| `/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq` | `1800000` (1.8 GHz) | частота |
| `/sys/class/thermal/thermal_zone0/temp` | `29693` (29.7 °C) | температура |
| `/sys/class/hwmon/hwmon0/temp1_input` | `29206` (29.2 °C) | альтернативний термодатчик |
| `/proc/uptime` | `3803.25` | uptime агента |
| `nproc` (у Rust через available_parallelism) | 4 | ядра CPU (системний) |

### Браузерні піри (Android / Chrome WebGPU) — що є:
| API | Chrome | Firefox | Safari | Вміст |
|---|---|---|---|---|
| `navigator.hardwareConcurrency` | ✅ | ✅ | ✅ | логічних ядер |
| `navigator.deviceMemory` | ✅ | ❌ | ❌ | GB, округлено до степенів 2 (0.25…8) |
| `navigator.connection` (NetworkInformation) | ✅ | ❌ | ❌ | `effectiveType` (slow-2g/2g/3g/4g), `downlink` Mbps, `rtt` ms, `saveData`, `type` (wifi/cellular/ethernet) — **оцінка, не виміряно** (MDN: Limited availability) |
| `navigator.getBattery()` | ✅ | ❌ (прибрано) | ❌ (macOS only) | `level`, `charging`, `chargingTime`, `dischargingTime` |
| `navigator.userAgentData` (UA-CH `getHighEntropyValues`) | ✅ | ❌ | ❌ | `architecture`, `bitness`, `model`, `platformVersion` |
| WebGPU `GPUAdapter.requestAdapterInfo()` | ✅ | ✅ | ✅ | `vendor`, `architecture`, `device`, `description` |
| WebGPU `GPUAdapter.limits` | ✅ | ✅ | ✅ | `maxStorageBufferBindingSize`, `maxBufferSize`, `maxComputeInvocationsPerWorkgroup`, `maxBindGroups` тощо — **реальний стельник розміру шардів** |
| `performance.memory` | ✅ | ❌ | ❌ | `usedJSHeapSize` (приблизно) |
| Точний WiFi dBm | ❌ | ❌ | ❌ | **ніде недоступно** |
| CPU модель / частота / температура | ❌ | ❌ | ❌ | **ніде** |
| Per-core util / GPU util / live VRAM | ❌ | ❌ | ❌ | ніде |

### Хаб (уже є або легко додати):
- `wgpu_tasks.elapsed_ms` по піру → **RTT-проксі** (derived; чесно label як derived)
- `capability_rows` → `mflops_max` (виміряно, не вигадка)
- `weight_max_err` (модельова точність піра на реальних тензорах)
- Підсумкові `alive/ready/shards` + Top RSS хаба

---

## 2. Принципи (як робимо)

1. **Provenance labels на КОЖНЕ поле**: `kernel` (з /proc,sysfs) > `browser-est` (Chrome-оцінка) > `derived` (обчислено з RTT) > `measured` (MFLOPS з тестів). Наказ від «нічого не вигадати» — фальшиве число гірше за `null`.
2. **Null — чесно.** Якщо джерело недоступне (Firefox, Broadcom wlan0, браузер без батареї) — `null`, не `0`.
3. **Мepзеризація cheap**: пір репортить раз на poll (2 с); хаб рахує delta байтів сам.
4. **Zero новых привілеїв**: агент працює як `pi` user; тільки читання world-readable `/proc`,`/sys`.
5. **Один JSONL його розширюється, нічого не ламається** (backward-compatible rows).

---

## 3. План реалізації (фази)

### Фаза 0 — фундамент даних (Pi-side, низький ризик)
- Агент шле в `poll` (разом із peer heartbeat): `meminfo_avail_kb`, `loadavg1`, `uptime_secs`, `cpu_mhz` (cpu0), `temp_c` (thermal_zone0), `net_rx_bytes`, `net_tx_bytes`, `wifi_link` (0..70), `wifi_signal_dbm` (-60), `wifi_noise_dbm`, `carrier`, `net_type` (wifi|eth), `link_speed_mbps` (eth0 only).
- `peer_join` лишається незмінним — розширення join'у передбачити окремим кроком.
- Досвід на живій Pi: **всі ці поля доступні без встановлення пакетів**, `iw` не потрібен (quality/level читаються з `/proc/net/wireless` у форматі WE 22: `wlan0: 50. -60.`).

### Фаза 1 — браузерні піри (Chrome-first; Firefox/Safari деградовано)
- `connections`: `downlink`, `effectiveType`, `rtt`, `type`, `saveData` (оцінка, label `browser-est`).
- `battery`: `level`, `charging` (лише Chrome; де немає — `null`).
- UA-CH high entropy: `architecture`, `bitness`, `model` (Chrome; дозволити https/контекста пір).
- WebGPU: `adapterInfo` + `limits.maxStorageBufferBindingSize` (критично для розміру shard) — це **може змінити план розселення** (точні стелі замість евристики ½).
- `performance.memory.usedJSHeapSize` (Chrome; label `browser-est`).

### Фаза 2 — хаб: агрегація та похідні
- `mesh_net` у `/api/sys/metrics`: per-peer `net_rx_mbps`/`net_tx_mbps` (delta між poll'ами), fleet sums.
- `tasks_jsonl` компакція: тримати останні ~2k рядків або 7 днів; інакше `read_jsonl` з часом стане пляшковим горлечком.
- mtime-кеш snapshot для `peers/tasks` (як `gsv-vision-sync` робить з manifest) — менше CPU під час 500 ms refresh.
- RTT proxy: `p50/p95 elapsed_ms` на пір з останніх N задач (виведено, label `derived`).

### Фаза 3 — `/sys` панель «Radio & net» + компакт
- Новий блок у `/sys`: peer | class | up | cores | mem free | GPU | link | signal | dBm | RTT | battery | temp | mflops | ready.
- Сигнал: бар 0–70 на базі `/proc/net/wireless`; dBm підпис.
- `/api/sys/metrics` розширюється полями; чужим клієнтам нічого не ламається.

### Фаза 4 — флот-інтелект (для Pi-кластера та пірів)
- Capacity forecast: сума `min(maxStorageBufferBindingSize, policy_cap)` по пірах — підказка placement’у.
- Деградація: RTT p95 > X ms або signal < 40 → підсвітити пір (`warn`), автоматично знижувати пріоритет у routing.
- Power profile (Pi): `cpu_mhz` & temperature — троттлінг помітний через `scaling_cur_freq`.

### Захоплення out-of-scope (чесно)
- Браузерний WiFi dBm — неможливо.
- GPU temp/load — неможливо ні з браузера, ні з WebGPU.
- Real RSS памʼяті вкладки без Chrome `performance.memory` — лише приблизно.
- pi4 `speed` для wlan0 — недоступний на Broadcom; вимірюємо RTT і байти замість.

---

## 4. Оп-план (по задачах, без даних owner-гейтів)

- [ ] PH-A6.0 doc-ред рецензія/затвердження формату розширених рядків піра (цей документ).
- [ ] PH-A6.1 mesh-node агент: розширити `peer_join`/`poll` телеметрією (Фаза 0) — малі пatchи, `MSRV 1.100.0` (екосистемний стандарт, факточний stable).
- [ ] PH-A6.2 GSV `/api/sys/metrics`: рахунок delta rx/tx, похідні, JSONL compact (Фаза 2).
- [ ] PH-A6.3 `/wgpu` сторінка: підібрати `connection`/`battery`/UA-CH/WebGPU limits (Фаза 1).
- [ ] PH-A6.4 `/sys` панель: блок «Radio & net» + кольорове маркування `ready/warn/crit` (Фаза 3).
- [ ] PH-A6.5 флот-діагностика: RTT p50/p95 по піру, деградаційні алерти (Фаза 4).
- [ ] ECO-10 roadmapрядок PH-A6.

## 5. Метрики успіху

- ≥ 80 % пірів показують `meminfo_avail_kb`/`wifi_signal_dbm` (нативні) без `null`.
- `/sys` refresh коштує < 5 ms (було ~10 ms при двох JSONL-сканах після compact).
- 0 викривлених "0"-значень: відсутнє джерело → `null` (перевіряється unit-тестами).
- Pi fleet: розміщення placement'у враховує реальні `maxStorageBufferBindingSize` замість евристики ½.

## 6. Ризики та межі

- **Guest isolation**: нічого з `/proc`, що порушує `pi`-користувача; PSK WiFi **не читається і не логаться**.
- **PII**: `ua` Chrome-версія анонімізується як і раніше (перші 80 символів).
- **False confidence**: Chrome `downlink/rtt` — оцінка, не факт; маркуємо `browser-est`.
- **Firefox/Safari**: лише `hardwareConcurrency`, `GPUAdapter.info`; решта `null` — не ховати.
- **SD/RAM**: розширення рядків +кілька KB на пір — не впливає на надійність.

## 7. Звʼязки

- Реалізація: `docs/gsv/GSV_TECH_ROADMAP.md` → ECO-10 (PH-A6 …).
- Розширення полів: `src/boxes/wgpu.rs` (hub rows) + `crates/mesh-node/src/main.rs` (Pi agent) + `src/boxes/sysmon.rs` (панель).
- Протокол: `POST /api/wgpu/peer/join` body — нові optional keys; `wgpu_peers.jsonl` — новый optional keys (ignore-unknown на читанні).
