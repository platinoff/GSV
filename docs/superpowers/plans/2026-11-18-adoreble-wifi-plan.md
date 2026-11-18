# ADO — Adorable Wi-Fi: Wi-Fi матриця керування для GSV-екосистеми (план 2026-11-18)

**Статус:** research + design plan — **жодного коду поки нема**. Owner-гейти відзначені явно.
**Мета:** плагін-вузол керування Wi-Fi для GSV-екосистеми («adoreble_wifi»): веб-пейдж на loopback (127.0.0.1, не з інших хостів) + Rust-ядро (Rust 1.100), що вміє: (1) автоматично визначати/обирати устрій і режим Wi-Fi; (2) переводити адаптер між режимами `lan-client ↔ hotspot ↔ bridge(AP+STA) ↔ release(авто-повернення)`; (3) авто-запуск/авто-відпуск через налаштування; (4) взаємодія з mesh-пірами GSV hub (`/api/wgpu/ws`, `/api/wgpu/mesh`).

**Не цілі (свідомо):** не знімаємо Windows-контроль з поточної робочої картки; не робимо «захоплення» радіо; не експонуємо нічого за межі машини без owner-gate; не дублюємо GSV hub; k8s out-of-scope (у кластері нема жодного вузла під k8s).

---

## 1. Проблема / чому зараз

- Піри (телефони valhall/bifrost #35/#38, Pi4 #36) **флаплять**: у `/api/wgpu/console` постійно «hub unreachable (retrying…) → connection restored». Причини можливі дві: (A) хаб сам падав (RAM-смерті 2026-11-18: gsv-server тікає, watchdog респавнить); (B) радіо/AP канал перевантажений (Bell 5 ГГц ch44, RSSI -64, SNR ок, але можуть бути блекаути на AM/EDS).
- Якщо (B) — піри повинні бачити хаб не «через Bell 5 ГГц з глибоким NAT/чужим BSSID», а через **локальну 2.4 ГГц точку, якою керуємо ми** (або щонайменше знайти/перемкнутися на неї з меншою кількістю ретрансляцій).
- Плагін `adoreble_wifi` дає керовану топологію: ноут (або Pi) як точка доступу 2.4 ГГц + існуючий 5 ГГц уплінк → піри в тому ж L2, хаб досяжний без NAT-перетворень, з контролем потужності/каналу/безпеки.

---

## 2. Дослідження — що реально доступно (стан на 2026-11-18)

### 2.1 Windows (цей бокс, локальна станція, build 26200)

| Можливість | Стан | Вердикт |
|---|---|---|
| `netsh wlan start hostednetwork` | **Ні** — на Intel AC-8265: `Hosted network supported: No` (WDI картки + Windows 11: deprecated) | ❌ не використовувати |
| **WinRT `NetworkOperatorTetheringManager`** (`Windows.Networking.NetworkOperators`) | Є, є офіційний `NetworkOperatorTetheringManager.StartTetheringAsync/StopTetheringAsync`; саме цей API реалізує Settings → Mobile hotspot | ✅ **єдиний підтримуваний шлях для AP-режиму на одному адаптері** |
| Legacy `WlanSetInterface` / `WlanSetProfile` | працюють для профілів клієнта (SSID/key/керування підключенням) | ✅ для режиму `lan-client` (redirect SSID) |
| `netsh wlan show interfaces/drivers/profile` | працює (бачимо все: bands, ch, Rssi, auth) | ✅ для діагностики/capability matrix |
| `netsh wlan connect/show/set` + `WlanSetInterface` | перемикання профілів/SSID | ✅ для автоматичного перемикання мереж |
| Wi-Fi Direct (`Windows.Devices.WiFi`) | працює, але це **не** AP для звичайних клієнтів (потрібен Wi-Fi-Direct-підтримувані піри); не наш шлях | ⚠️ відкласти |
| Windows ICS (Internet Connection Sharing) + Mobile Hotspot | створює підмережу `192.168.137.0/24`, NAT у діючу мережу; піри виходять у LAN, вхідні від LAN до пірів закриті за замовчуванням (без port fwd) | ✅ для control-plane (піри → хаб outbound) |

**Локальна станція (перевірено 2026-11-18):** `Intel(R) Dual Band Wireless-AC 8265` (802.11a/b/g/n, 2×2, 2.4/5 ГГц, **не AC**), зараз у `BELL409` (5 ГГц, ch44, 240 Mbps, RSSI -64). `Hosted network supported: No`. ⇒ **На цьому адаптері режим AP = тільки через WinRT Tethering (Mobile Hotspot)**; перевірка підтримки конкретним драйвером — ADO-1.

**Як перевірити вручну (до коду):** Settings → Network & Internet → Mobile hotspot → увімкнути + діапазон 2.4 ГГц + підключити телефон. Якщо попрямувало — API режим підтверджений; якщо ні — дивимось на другий адаптер (USB-Dongle).

**Вікна конкурентних режимів (одна картка!):** AC 8265 — один радіо-блок на частоту, dual-band ≠ два радіо; **одночасно STA на 5 ГГц + AP на 2.4 ГГц можливо тільки якщо драйвер заявляє dual-band concurrent** (Intel каже «як правило — так, для 5300/7265/8265 серій», але це **треба перевірити емпірично** в ADO-1; якщо ні — потрібен другий адаптер або Pi як AP).

### 2.2 Linux / Raspberry Pi 4 (вузол ретрансляції)

| Можливість | Стан | Вердикт |
|---|---|---|
| `hostapd` 2.10+ / `wpa_supplicant` 2.10+ | стандарт у Debian/RPi OS; `driver=nl80211`; 4addr bridge: `ap_isolate=0` + `bridge=br0` + `wds_sta=1` | ✅ **повне WDS-реле** (L2, без NAT) |
| `nl80211` netlink control (`wpa_supplicant -i wlx... -Dnl80211 -c /etc/wpa_supplicant/wpa_supplicant.conf -B`) | працює на Pi 4/CM4 | ✅ керування STA |
| Rust crates: `netlink_wi` (високорівневий nl80211), `neli` (загальний netlink), `wl-nl80211` (низькорівневий), `hostapd`/`supplicant-rs` (команди до запущеного hostapd по control socket) | усе стабільно публічне | ✅ для `adoreble-linux` |
| `iw` / `ip` | інформація про режими адаптера (`iw list` → Supported interface modes: AP, mesh, managed, WDS, ...) | ✅ capability matrix |

**Pi 4 як ретранслятор — найнадійніший шлях** для телефонів (2.4 ГГц) докладно: `br0` = `eth0` + `wlan0` (AP) → L2-міст без NAT; піри отримують ті ж адреси що й LAN; хаб досяжний за тим самим IP. Альтернатива — NAT-хаб (маршрутизатор) за принципом «ноут як точка».

### 2.3 Rust-фреймворки / версії (цільовий стенд)

- **Rust 1.100.0** усе стабільне (уже 11 репо на stable-gnu). Rust 2024 edition (`edition = "2024"`).
- `tokio` 1.53, `axum` 0.8, `reqwest` 0.13, `tower` / `tower-http` (compression, cors), `serde`/`serde_json` 1.x, `thiserror` 2.x, `clap` 4.x (adoreble-cli), `tracing`/`tracing-appender` (логи), `mimalloc` (allocator), `regex-automata`, `futures`, `httparse`.
- **WebAssembly:** `wasm32-unknown-unknown`, `wasm-bindgen` 0.2 (для ui-класу WASM: сатуратор графіків + швидкі обчислення) — третій шлях (DOM-доступні дії лишаються на thin JS). В пайплайні `cargo build --target wasm32-unknown-unknown -p adoreble-ui`.
- **Windows crates:** `windows` 1.x (Win32/WiFi/NetworkManagement/WiFi + WinRT `Networking::NetworkOperators`), `windows-result`; WinRT-проекції пакету `windows` працюють напряму з `#[implement]` на WinRT-інтерфейсах.
- **Linux crates:** `netlink_wi`, `neli`, `hostapd` (supplicant-rs, якщо потрібен контроль запущеним hostapd).
- **GSV экосистема:** `gsv-wgpu` (mesh-піри робочі), `mesh-wasm` SIMD128 (`/assets/mesh-wasm.wasm`) як reference; GSV crate 0.248.0, live `:9999`.
- **Rust edition notes (1.100):** `if-let chains`, `std::alloc::Allocator` (не обов'язково), `let-else`, `OnceLock`, `LazyLock` — використовуємо за потреби.

### 2.4 Безпека (WPA/WPA3)

- **WPA3-SAE (CCMP-256/GCMP) де підтримується**, інакше **WPA2-PSK AES** (найменш спільний знаменник для телефонів).
- **WPA2-Enterprise / RADIUS** — out-of-scope для personal хотспота; для майбутнього mesh-ретрою — окремий банд.
- **802.11w (PMF) required** коли доступне; не форсувати на старих телефонах (зламаються).
- Канали 2.4: 1/6/11 (наши найменш зашумлені); 5 ГГц: 36-64 (DFS уникаємо: 52-140 — потрібен radar detect, не в кожному драйвері), 149-165 (UNII-3, ок у багатьох регіонах).
- Паролі **ніколи не зберігаємо**: читаємо профілі з `netsh wlan show profile name="..." user=...` у реальному часі; у конфігу не пишемо.

---

## 3. Архітектура плагіна (адаптер-агностик)

### 3.1 Prio-макет (workspace)

```
S:/rust/adoreble_wifi/                  (новий репо, owner gate)
├── Cargo.toml                          # workspace, edition 2024, resolver 2
├── AGENTS.md                           # local rules (GSV-kit global-first)
├── README.md
├── PRODUCTS.md-entry (в GSV PRODUCTS.md після owner go)
├── data/
│   └── adoreble.json                   # settings (gitignored)
├── crates/
│   ├── adoreble-core/                  # domain: WifiAdapter, WifiMode, State, events
│   ├── adoreble-windows/               # WinRT TetheringManager + netsh + WlanApi
│   ├── adoreble-linux/                 # nl80211/hostapd/wpa_supplicant (для Pi)
│   ├── adoreble-api/                   # loopback HTTP/WS control plane (axum)
│   ├── adoreble-cli/                   # adoreble-wifi binary
│   ├── adoreble-ui/                    # WASM/HTML thin glue (wasm32/html)
│   └── adoreble-xtask/                  # Rust xtask tasks (build/verify/release)
├── tests/
│   └── capability_matrix.rs            # обов'язкові probes
├── gsv/                                # docs (HANDOFF.md, NEXT_SESSION_PROMPT.md)
└── docs/
    ├── ADR-001-loopback-security.md
    ├── ADR-002-adapter-capability-detection.md
    ├── ADR-003-mode-state-machine.md
    └── WIFI_MATRIX.md                  # capability matrix по адаптерах
```

### 3.2 Доменна модель

```rust
enum WifiMode {
    LanClient,        // STA до відомої точки (зараз: BELL409)
    Hotspot,          // AP для пірів (2.4 GHz за замовчуванням)
    Bridge { upstream_ssid: String },  // STA+AP (або eth+AP) — L2-міст (Pi)
    ReleaseFallback,  // відпустити до першої робочої точки
}

enum Adoption {
    // Hot-swap: перемикатись між устроями, якщо SLA падає
    InternalWlan, UsbDongle, PiRelay, None,
}

struct Settings {
    enabled: bool,                  // false ⇒ нічого не робити
    preferred_mode: WifiMode,
    interface: AdapterUsbGuid,      // GUID обраного адаптера або "auto"
    hotspot: HotspotSettings { ssid, password, security: Wpa2Aes | Wpa3Sae, band: G2_4 | G5, channel: u8, max_clients: u8 },
    auto_start: bool,               // Windows Scheduled Task / systemd user unit
    auto_release: ReleasePolicy { unreachable_ms: u64, revert_to: WifiMode },
    gsv_hub: HubAddrResolution { explicit: Option<SocketAddr>, candidates: Vec<IpAddr>, mdns_timeout_ms: u64 },
    peer_probe: PeerProbe { url: String, interval_s: u64, timeout_ms: u64 }, // GSV /api/health
}
```

### 3.3 Стан-машина (mode switching)

```
                  ┌───────────────────────────────┐
                  │  Disabled (enabled=false)      │
                  └───────────────┬───────────────┘
                                  │ enable (owner gate)
                                  ▼
        ┌──────────┐   probe    ┌──────────┐   switch   ┌──────────┐
        │ LanClient ├────────────┤  Decide  ├────────────► Hotspot  │
        │ (default) │           │          │ mode      │ (2.4 GHz) │
        └─────┬─────┘           └────┬─────┘           └─────┬─────┘
              │ SLA>threshold        │                ▲      │ SLA drops/fail
              │ revert               ▼                │      ▼
              │              ┌───────────────┐   ┌────┴─────┐
              └─────────────► ReleaseFallback ◄──┤ Bridge   │
                              │ (safe revert)    │ (Pi 4    │
                              ▼                  │  eth+AP) │
                       (ping hub fail-open)      └──────────┘
```

Ключ: **fail-open** — якщо щось ламається, повертаємось до `LanClient` (або до попереднього робочого SSID); ніколи не залишаємось без мережи. Owner може вимкнути авто-перемикання (`enabled=false`).

### 3.4 Loopback HTTP API (Rust + axum, bind 127.0.0.1:9770 за замовчуванням)

| Маршрут | Метод | Опис |
|---|---|---|
| `GET /health` | GET | `{ok:true, version, enabled, mode, adoption}` |
| `GET /api/adapters` | GET | список адаптерів + capabilities (band, hotspot-capable?, concurrent?, WPA3?) |
| `GET /api/status` | GET | поточний mode, SSID, канал, RSSI, клієнти (якщо видно), gsv-hub reachability |
| `POST /api/mode` | POST `{mode}` | переключити режим (enabled=true) |
| `GET|POST /api/settings` | GET/POST | читання/запис `data/adoreble.json` (password redacted) |
| `GET /api/logs` | GET (SSE) | живі логи + WPA/DHCP/link-events |
| `POST /api/probe-hub` | POST | примусовий probe GSV hub за списком кандидатів |
| `GET /api/peers` | GET | `(proxy)` живі піри з GSV `/api/wgpu/mesh` (read-only) |

**Безпека:** bind тільки на 127.0.0.1; POST вимагає token у header `X-Adoreble-Token` (згенерований при першому запуску, збережений у `data/adoreble.token`, owner-readable only); IP-адресу не експонуємо; жодного зовнішнього доступу; усе офлайн-first.

### 3.5 Web dashboard (`ui/`)

- Thin HTML/CSS/JS + WASM (`wasm32-unknown-unknown`) для швидких обчислень (графіки RSSI/SNR/канал, gauge активності пірів, QR-генерація).
- Сторінки: Overview (mode, SSID, канал, RSSI, графік 60 хв), Adapters (картки: band/ch/RSSI/enable-hotspot button), Settings (форми: mode, auto_start, release policy), Peers (список + last-seen), Logs (SSE-в'ювер), QR Onboard (QR зі SSID+pass для телефону).
- Відкривається у браузері: `http://127.0.0.1:9770/ui/` — без авторизації поза loopback; token у localStorage (стандартний pattern, як у GSV Galaxy UI).

### 3.6 GSV hub integration (опційно)

- `GET /api/peers` проксує `/api/wgpu/mesh` хаба (read-only; не змінює хаб).
- Опційна реєстрація в GSV keep-live (`docs/gsv/GSV_SERVER.md`) як peer `adoreble_wifi` — після власника-гейту.
- GSV борд: квитки ADO-* на хаб-борді; FM row у `gsv/PRODUCTS.md`; HANDOFF у `docs/HANDOFF_NEW_SESSION.md`.

---

## 4. Режими роботи (деталі)

### 4.1 `LanClient` (default)
- Підключитись до відомого SSID профілю (читання з `netsh wlan show profiles` або WlanApi `WlanGetAvailableNetworkList`).
- Якщо поточна мережа не проходить probe GSV hub (3 спроби) — спробувати наступний кандидат з `gsv_hub.candidates`; якщо всі фейляться → `ReleaseFallback` (повернути до попереднього профілю + алерт у лог/GSV-тікет).
- Детекція адрес: кандидати = `GSV_LOCAL_ADDR` → збережені → `192.168.2.238`; mDNS `_gsv-http._tcp.local` як опція (не на першому етапі).

### 4.2 `Hotspot` (2.4 ГГц за замовчуванням)
- WinRT TetheringManager: `TetheringOperationalState` опитування → `StartTetheringAsync`/`StopTetheringAsync`.
- Параметри: SSID = `GSV-Peers-<suffix>` (авто-UUID), пароль — авто-генерований 12 симв (з `getrandom`), записується **тільки** в `data/adoreble.json` (gitignored) після owner-confirm; 2.4 ГГц ch1/6/11 (best by scan), WPA3-SAE якщо обидва піри підтримують, інакше WPA2-PSK AES; max_clients=8.
- Windows ICS створить `192.168.137.0/24` — піри отримують адреси з DHCP Windows; хаб досяжний `192.168.2.238` (через NAT), GSV `/api/wgpu/ws` працює нормально (outbound пірів).
- Якщо WinRT-Tethering недоступний на драйвері → fallback: USB-донгл або Pi.

### 4.3 `Bridge` (Pi 4: eth0↔wlan0, або ноут+USB: залежно від картки)
- `br0` з `eth0`+`wlan0` (AP), `hostapd` 4addr `wds_sta=1`, `ap_isolate=0`.
- Піри в L2-домені з хабом; жодного NAT; IP = LAN.
- Інтерфейси в `data/adoreble.json` явно вказуються (Pi: `eth0`,`wlan0`; ноут: `Wi-Fi`,`USB-WLAN`).

### 4.4 `ReleaseFallback` (безпечне повернення)
- Якщо активний режим не тримає SLA (N consecutive `/api/health` fail на хабі + 0 пірів протягом 5 хв) → `netsh wlan connect name="BELL409"` (або останній відомий good profile).
- Якщо Hotspot активний і піри зникли + хаб не відповідає → stop tethering → connect BELL409.
- Все фіксується в `data/adoreble-release.log` (append-only).

### 4.5 Авто-старт
- **Windows:** Scheduled Task `AdorebleWiFi` at logon (`schtasks /create /sc onlogon /tn AdorebleWiFi /tr ...adoreble-cli up --detach`) — `LogonType=interactive`, `RunLevel=highest` (потрібна elevation для netsh/tethering). Альтернатива: Windows Service через `adoreble-daemon` (Rust + `windows-service` crate) — більш надійно для headed; service може крутити UI так само через loopback.
- **Pi:** systemd user unit (`~/.config/systemd/user/adoreble-wifi.service`) з `Restart=on-failure`.

---

## 5. Милестони / банди (≈ 6 бандів, owner-gate ADO-1)

| Банд | Рядок | Owner-gate |
|---|---|---|
| **ADO-0** | Scaffold репо `S:/rust/adoreble_wifi`, workspace, AGENTS.md (GSV global-first), CI (GitHub Actions, windows+ubuntu, permissions: contents:read), PRODUCTS.md row в GSV | ✅ потрібен |
| **ADO-1** | `adoreble-cli scan` + capability matrix probe (WinRT Tethering? bands? concurrent? WPA3?) — задокументувати для AC-8265 + USB-донгл, якщо буде | ✅ потрібен |
| **ADO-2** | `LanClient` режим (netsh profile switching, probe-hub, ReleaseFallback, `data/adoreble.json`) + unit tests | ✅ потрібен |
| **ADO-3** | `Hotspot` режим (WinRT TetheringManager: create/start/stop, SSID auto-suffix, password gen, WPA2/WPA3) + ICS перевірка; `/api/adapters`, `/api/status` | ✅ потрібен |
| **ADO-4** | Web UI (WASM/HTML на 127.0.0.1:9770/ui): Overview+Adapters+Settings+QR; SSE логи; token auth | ✅ потрібен |
| **ADO-5** | `Bridge` на Pi 4 (eth0+wlan0 br0, hostapd 4addr), та tests проти mesh-пірів; GSV `/api/peers` proxy | ✅ потрібен |
| **ADO-6** | Auto-start (Scheduled Task/service), auto-release (SLA watchdog), release.log, HANDOFF/NEXT у gsv, wiring `cargo xtask products --state` бачить ADO | ✅ потрібен |

**DoD кожного банду:** `cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test --workspace` · capability matrix row у `WIFI_MATRIX.md` · один commit + push · GSV-тікет ADO-1..-6 у статусі done/evidence.

---

## 6. Ризики та невирішені питання

| Ризик | Захід |
|---|---|
| **AC-8265 не вміє STA+AP одночасно** (один радіо на частоту) | USB-донгл (2.4 GHz dedicated) для хотспота; Pi 4 — як повноцінний реле; AC-8265 лишається у `LanClient` |
| **Mobile Hotspot вимикає/не стартує без Інтернету** (поточний state) | Не вважаємо це блокером: ранній режим — тільки коли мережа є; перевірка цього факту в ADO-1 |
| **WPA3 не збігається з телефонами** | WPA2-PSK AES як default; WPA3 тільки коли обидва піри заявляють SAE |
| **DFS-канали (52-140) у 5 ГГц** роблять hotspot сліпим на хвилини і хаосять пірів | Використовувати тільки 36-64 або 149-165; для 2.4 ГГц — 1/6/11 |
| **Windows Firewall блокує наш `/api/peers` proxy / loopback для телефонів** | не експонуємо назовні нічого; тільки loopback для UI; піри ніколи не дотягуються до adoreble-api |
| **ICS 192.168.137.x перекидає пірів у NAT** — піри не можуть бути destination з боку LAN | приймаємо: піри тільки initiate WS до хабу, що вже працює саме так; для повного L2 — Pi-реле (ADO-5) |
| **Паролі в `data/adoreble.json`** можуть витекти | gitignored (як `data/` у плагінах); файл chmod 600 на Linux / ACL user-only на Windows; redact у будь-яких логах; у конфіг зберігаємо лише з owner-confirm через UI |
| **ADO може зламати мережу на машині** | fail-open fallback; нічого не змінюємо без owner-gate; все повертаємо при будь-якому SLA drop |
| **GSV hub не стабільний (RAM deaths)** | це окремий P0-тикет з HANDOFF (RAM/EDR) — ADO корисний тільки коли хаб живий |
| **ADO може стояти поруч із GSV-трафіком** (WebSocket :9999) | ніколи не змінює порти хабу; ADO лише надає L2-шлях |

**Відкриті питання до owner (без відповіді далі не йдемо):**
1. Створювати репо `adoreble_wifi` зараз (ADO-0) чи чекати?
2. Який перший канал: (A) Pi 4 як реле (потрібно збір hostapd-конфігу та br0); (B) ноут + USB-донгл хотспот (додати ~$10 адаптер); (C) тільки WinRT Tethering на AC-8265 як proof-of-concept?
3. Чи реєструємо ADO в GSV PRODUCTS.md одразу чи після ADO-2?
4. Чи ок використовувати `schtasks ONLOGON` без owner-gate після ADO-6 (потенційно агресивно) чи лишаємо ручний запуск через кнопку в UI?
5. GSV-тікет ADO-1 (capability matrix) — брати з хаб-борду коли будемо готові?

---

## 7. Що потрібно від owner (мінімум)

- **ADO-0:** yes/no на створення репо;
- **ADO-1:** yes/no на capability probe на поточному ноуті (не змінює нічого крім читання `netsh`/WinRT-опитування);
- **ADO-3:** вибір режиму (A/B/C з питання 2) — без цього не пишемо хотспот;
- **ADO-4:** house style UI (той самий GSV Galaxy / dark glass?) — потрібен шаблон;
- **ADO-5:** доступ до Pi 4 (SSH, нині він не в mesh як активний пір — `pi4-cpu #36 alive=false`);
- **ADO-6:** авто-старт (так/ні) та auto-release (так/ні) — можливо, лишити ручне керування;
- **PROD:** чи включати `/api/peers` proxy proxy чи тримати ADO повністю ізольованим.

---

## 8. Definition of done (для всього плагіна, не банду)

- `cargo xtask products --state` бачить `adoreble_wifi` (head/branch/dirty=0, wiring ok);
- capability matrix задокументовано у `WIFI_MATRIX.md` (адаптер · band · hotspot-capable · concurrent · WPA3 · test date);
- `adoreble-cli` вміє: `scan | status | up | down | release | logs | ui-qr`;
- UI на `127.0.0.1:9770/ui/`; token-auth; redact паролів;
- піри GSV mesh реально переключаються на adoreble-AP і стабільно тримають `/api/wgpu/ws`;
- fail-open повернення до `LanClient` при збої;
- один commit + push; GSV-тікет ADO-* done з evidence;
- відсутність будь-яких секретів у репо (перевірка перед першим push).

---

## 9. Джерела

- Microsoft Q&A / Docs: `hosted network supported: No` на WDI-картках; Windows 11 deprecated hosted network → Mobile Hotspot.
- WinRT API: `Windows.Networking.NetworkOperators.NetworkOperatorTetheringManager` (Start/StopTetheringAsync, TetheringOperationalState, TetheringOperationalStateChange).
- Rust: `windows` 1.x (`Win32::NetworkManagement::WiFi` + WinRT projections), `netlink_wi`, `neli`, `wl-nl80211`, `hostapd`/`supplicant-rs`.
- Linux: `hostapd.conf(5)`, `wpa_supplicant.conf(5)`, `iw(8)`, kernel nl80211 spec; RPi OS: `raspberrypi.com` docs (hostapd + br0 + 4addr `wds_sta=1`).
- GSV ecosystem: `S:\rust\GSV\{AGENTS.md, docs/HANDOFF_NEW_SESSION.md, PRODUCTS.md, gsv/GSV_ROUTES.md}`, `GSV crate 0.248.0`, WSP/JS-телеметрія сітки, `cargo xtask products --state` як реєстр.
- Локальна станція: `Intel Dual Band Wireless-AC 8265` (802.11n, 2.4/5 ГГц, `hostednetwork=No`, зараз 5 ГГц ch44 BELL409, 240 Mbps, RSSI -64).

---

## 10. Наступні кроки (після owner GO)

1. ADO-0 scaffold (окремий репо `S:/rust/adoreble_wifi`, wksp, CI, AGENTS.md);
2. ADO-1 capability matrix прогон на AC-8265 + (якщо є) USB-донглі + Pi 4;
3. ADO-2 LanClient + probe + fallback;
4. ADO-3 Hotspot (WinRT TetheringManager) + ICS перевірка;
5. ADO-4 WASM/HTML UI на loopback;
6. ADO-5 Bridge на Pi 4 (radio subset);
7. ADO-6 авто-старт/авто-відпуск + GSV wiring.

**Поки що жодних дій на живій мережі не виконуємо.** Чекаю відповіді owner (розділ 7, питання 1–5).
