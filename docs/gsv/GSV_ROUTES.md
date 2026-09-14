# GSV_ROUTES.md — маршрути сервісів (band 233, LAN-first)

Канон: хто кого кличе і на якій адресі. **Замість `127.0.0.1` усюди, де
маршрут бачить пір (телефон / VM / edge-PC), — локальна (LAN) адреса
машини.** Loopback лишається валідним на самій машині: live-сервери біндяться
`0.0.0.0` (усі інтерфейси), тож `127.0.0.1` і LAN-IP — той самий listener.

Резолвер адреси (усі продукти, дзеркала): `GSV_LOCAL_ADDR` env →
cargo-test-harness ⇒ `127.0.0.1` (контракти детерміновані) → default-route
IPv4 (UDP-trick) → `127.0.0.1`.

| Порт | Сервіс | Bind після band 233 | Хто кличе → маршрут |
|------|--------|---------------------|---------------------|
| **9999** | `gsv-server` (live UI, `/mcp`, `/api/omni`, `/api/ingest`) | `0.0.0.0` (`--allow-lan` авто у live/watchdog-спавна) | Cursor `.cursor/mcp.json` → `http://127.0.0.1:9999/mcp` (той самий listener); телефон/VM → `http://<lan>:9999/`; llama_rs `GSV_LIVE=1` → `POST /api/ingest` (`GSV_LIVE_URL`, дефолт локальна); `gsv-watchdog` → `/api/health` + `/api/update/apply` (probe-хост = локальна, не `0.0.0.0`) |
| **9800** | `telenetis` (Telegram webhook + Mini App) | `0.0.0.0` (був) | GSV keep_live → `/health`; Telegram cloud → `/webhook` (тунель або LAN); сам кличе GSV `TELENETIS_GSV_URL` і poolAI `TELENETIS_POOLAI_URL` — дефолти тепер локальна адреса |
| **8091** | `poolAI` coordinator (edge-воркери) | `0.0.0.0` | `llama_edge --coordinator` (дефолт `http://<lan>:8091`, remote edge — явний URL); telenetis `/vm`-view |
| **8080** | `llama_serve` deep tier (`bunke-rock`) | `0.0.0.0` (дефолт змінено) | GSV omni `catalog::resolved_default_base_url` → `http://<lan>:8080/v1`; llama_edge `serve_url()` (`LLAMA_SERVE_URL` override) |
| **8082** | `llama_serve` fast tier (`bunke-rock-fast`) | `0.0.0.0` | GSV omni `lama-1.5`; `LLAMA_SERVE_FAST_URL` override |
| **50052** | `ggml-rpc-server` (Linux-хости) | per-host | `llama_edge --rpc-endpoint` advertised локальною адресою; ALLBGP `--rpc` план у `grid.rs` |
| **20128** | `OmniRoute` (зовнішній проксі) | — (зараз **DOWN**) | keep_live-проба + `usage` pull — локальна адреса; `OMNIROUTE_URL` / `GSV_OMNIROUTE_URL` override. Поки down — сесійний chat іде **через GSV-omni напряму на llama-rs тієри**, минаючи OmniRoute |

## Сесійний флоу llama-rs (що було зламано)

1. `llama_rs` / `llama_edge` / `llama_serve` і GSV-сторона жили на
   `127.0.0.1` — пір у VM/телефоні бачив лише **свій** loopback → connect
   refused / heartbeat never lands.
2. Реєстрація edge у poolAI advertising-ала `"address": "127.0.0.1"` —
   координатор клив не туди.
3. Тепер: бінд `0.0.0.0`, advertised/URL-и = локальна адреса, GSV-гейт
   (`gate_post`) приймає власну LAN-адресу і RFC1918 origin'и в LAN-режимі;
   інтернет-origin'и досі відхиляються.

## Перевірка (жива)

```bash
# слухачі
netstat -ano | grep LISTENING | grep -E ':(9999|9800|8091|8080|8082|20128)'
# здоров'я 4 пірів (gsv/telenetis/llama_rs/omniroute) + hints
MCP: gsv_keep_live · GET http://<lan>:9999/api/health
# маршрутизація omni без мережі (dry-run)
curl -s -X POST "http://<lan>:9999/api/omni/v1/chat/completions" \
  -H 'Content-Type: application/json' -H 'X-Omni-Dry-Run: 1' \
  -H 'X-Omni-Tier: fast' -d '{"messages":[{"role":"user","content":"hi"}]}'
```

## Env-контракт

| Змінна | Ефект |
|--------|-------|
| `GSV_LOCAL_ADDR` | примусова локальна адреса для всіх резолверів (GSV, telenetis, llama-rs, xtask); **не експортувати глобально** — ті тоді «зїдають» loopback-дефолти тестів |
| `GSV_LIVE_URL` | куди `GSV_LIVE=1` шле `/api/ingest` (дефолт `http://<local>:9999`) |
| `GSV_HOST` | bind хост live-супервізора (`cargo xtask live`, `gsv-live`, `gsv-watchdog`); дефолт `0.0.0.0` |
| `GSV_ALLOW_LAN=1` | env-форма `--allow-lan` для ручного запуску `gsv-server` |
| `OMNIROUTE_URL` / `GSV_OMNIROUTE_URL` | base OmniRoute (keep_live + usage) |
| `TELENETIS_GSV_URL` / `TELENETIS_POOLAI_URL` | перекривають LAN-дефолти telenetis |
| `LLAMA_SERVE_URL` / `LLAMA_SERVE_FAST_URL` | перекривають llama-rs тієри |
| `POOLAI_HTTP_PORT` | порт координатора (this box: **8091**; default 8080 конфліктує з llama_serve) |
| `POOLAI_GALAXY_CAPABILITY_VERIFY_PK_HEX` | **pub-ключ** оператора (не сид! `221c2252…8235` = pub від сида `c254…0e3e` з edge-вbs); без нього `register-remote` → 403 `capability_signature_invalid` |
| `POOLAI_SERVICE_USER` / `POOLAI_SERVICE_PASS` | env-сіяний сервісний акаунт Telenetis (Viewer; poolAI band 233) |
| `LLAMA_EDGE_SIGNING_KEY` | сид підпису capability-doc у edge (збігається з vbs) |

Keep-live omniroute-проба: LAN-base → **loopback-twin fallback** (зовнішні
проксі часто біндять лише `127.0.0.1`; entry.url = той, що відповів).
