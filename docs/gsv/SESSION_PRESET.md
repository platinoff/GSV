# SESSION_PRESET — GSV mesh operator (2026-11-19)

Стислий операторський пресет для сесії GSV mesh. Канон — HANDOFF + INDEX; тут тільки
те, що потрібно щоб **розгорнути, перевірити й відновити** хаб і піри.

## Поточний стан (19.11.2026)

| Складова | Значення |
|---|---|
| Hub | `gsv-server` v0.248.0, `:9999` (LAN `http://192.168.2.238:9999`), crate 0.248.0 |
| Peers alive | `#37 FF-PC` (pc-wgpu) + `#38 bifrost` (arm phone) · `#35 valhall` — ~offline · `#36 Pi 4` — **мертва** |
| Mesh transport | Wi-Fi (Adoreble AP `GSV-Peers-6442` 2.4 GHz `192.168.137.1`; BELL409 fallback) |
| 35B ваги | `L:/gsv_mirror/wgpu_weights` (~520 MB, 127 chunks) — 40/40 шарів розселені раніше |
| Live UI | `http://192.168.2.238:9999/` (Galaxy) · `/sys` монітор · `/wgpu` phone PWA |
| One-glance | `cargo xtask status` → alive/ready/console/RAM/disk |

## Команди (MSYS2 bash, НЕ PowerShell)

```bash
export PATH="/c/Users/plati/.cargo/bin:$HOME/.cargo/bin:/ucrt64/bin:/usr/bin:$PATH"
unset RUSTUP_TOOLCHAIN
cd /s/rust/GSV

cargo xtask status            # one-glance: hub/mesh/console/RAM/disk
cargo xtask alive-watch --count 3 --every 10   # health probe, rc 0/1
cargo xtask cycle-watch       # phone reconnect-storm watchdog (dry; --alert → Telegram Godfather)
cargo xtask cycle-watch --count 3 --every 60   # обмежене запуски
cargo xtask cycle-watch-install # периодичний: HKCU Run GSV-cycle-watch (schtasks — тільки з admin)
cargo xtask disk              # S0 disk guard
cargo xtask rules-check       # drift gates (live lockstep)
```

## HTTP перевірки (живцем)

| Endpoint | Що показує |
|---|---|
| `curl -s 192.168.2.238:9999/api/health` | ok, version, uptime, disk, keep_live, crate_version |
| `curl -s 192.168.2.238:9999/api/wgpu/mesh` | peers[] (alive/ready/age_secs/lease), done/failed/claimed, best_peer |
| `curl -s 192.168.2.238:9999/api/wgpu/console` | ring ≤200 рядків (`worker: hub unreachable…` / `hub: connection restored`) |
| `curl -s 192.168.2.238:9999/api/wgpu/telemetry` | latest 20 телеметрія телефонів |
| `curl -s 192.168.2.238:9999/api/sys/metrics` | /sys wire (телеметрія 5 s) |

## Телефон-пір (#38 bifrost) — операційні правила

- Реконект-мікроспики норма (10 unreachable / 11 restored за 6.6 год; ≤3 хв blackout) — лог у `/api/wgpu/console`.
- Дії для стійкості **на телефоні** (owner-гейт, по одному): екран увімкнений під час soak; Chrome → Battery optimization → Not optimized; PWA shortcut на домашній екран.
- **Ніколи** не редагувати/переboot/писати в телефон чи SD — owner go per action.
- `device_key` SHA-256 фіксує peer_id — після reload сторінки #38 лишається #38.

## Hub-падіння / відновлення

1. Перевірити живість: `cargo xtask alive-watch` (rc≠1 → добре).
2. RAM: `cargo xtask status` → `ram 8x%`; при >92% — завершити зайве перед будь-яким `cargo build --release` (типові тихі смерті хаба).
3. `gsv-watchdog` (setsid) реплікає `target/live/gsv-server.exe` автоматично; **не** вбивати `target/live/gsv-server.exe` під час `cargo test`/`build`.
4. Після оновлення бінарника: `cargo xtask live` (copy debug→live + respawn) — інакше хаб лишається на старій збірці.
5. Якщо hub не піднявся → `cargo xtask disk --clean` якщо вільно <5 GB → `cargo xtask live` → re-check `alive-watch`.

## Push-канал (RFC 6455)

- `/api/wgpu/ws` — hub → телефон `{"type":"mesh-metrics",…}` кожні **5 s** (env `GSV_WGPU_WS_PUSH_SECS`, 1..=30).
- Розрив push ≠ крах: клієнт перепідключає за 5 s; SSE/poll fallback не вимагає дій оператора.
- Тест після деплою: `curl -i -N -H "Connection: Upgrade" -H "Upgrade: websocket" -H "Sec-WebSocket-Version: 13" -H "Sec-WebSocket-Key: x" http://192.168.2.238:9999/api/wgpu/ws` → `101`.

## Дисципліна комітів

- Один осмислений commit (docs або код), не `git add -A` — тільки файли задачі.
- Код-зміна → `cargo fmt --all` → `cargo clippy --all-targets` → `cargo test` (exit 0) → stretch-96.
- Docs-only → commit без bump; bands (`cargo xtask bump --band N`) тільки на band-closer.
- Push одразу після commit (owner 2026-11-19).
- `data/*` (крім .gitkeep) і секрети — ніколи не комітити.

## Куди дивитись далі

- [`INDEX.md`](INDEX.md) — карта `docs/`
- [`../HANDOFF_NEW_SESSION.md`](../HANDOFF_NEW_SESSION.md) — повний стан
- [`MESH_ROADMAP_TASKS_2026-11-19.md`](MESH_ROADMAP_TASKS_2026-11-19.md) — список можливих завдань (A–H)
- [`MESH_PI_BOOT_INCIDENT_2026-11-19.md`](MESH_PI_BOOT_INCIDENT_2026-11-19.md) — інцидент Pi (SD dead; правила)
