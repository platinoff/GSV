# Ð¡Ð¿Ð¸ÑÐ¾Ðº Ð·Ð°Ð²Ð´Ð°Ð½Ñ Ð¼Ð¾Ð¶Ð»Ð¸Ð²Ð¸Ñ Ñ Ð¿Ð»Ð°Ð½Ð°Ñ â mesh-ecosystem (2026-11-19, scope: ÑÐ°Ð± + bifrost #38)

**Owner ask 2026-11-19:** Â«Ð·ÑÐ¾Ð±Ð¸ÑÐ¸ ÑÐ¿Ð¸ÑÐ¾Ðº Ð·Ð°Ð²Ð´Ð°Ð½Ñ Ð¼Ð¾Ð¶Ð»Ð¸Ð²Ð¸Ñ Ñ Ð¿Ð»Ð°Ð½Ð°Ñ â¦ Ð±ÑÐ¾ÑÑÐ¾ÑÑ Ð±ÑÐ´Ðµ ÑÑÐ»ÑÐºÐ¸ Ð¿ÑÐ´ÐºÐ»ÑÑÐµÐ½Â» +
Â«Ð¿Ð¾ÑÑÐ¼ Ð³ÑÑ Ð¿ÑÑÑ Ð· ÑÐ°Ð¼Ð°ÑÑ Ð²ÑÐ´Ð¿Ð¾Ð²ÑÐ´Ð½Ð¾Â».

**ÐÐ¾Ð½ÑÐµÐºÑÑ Ð²Ð¸Ð¼ÑÑÑÐ½Ð¾ 2026-11-19:**

| Ð¡ÐºÐ»Ð°Ð´Ð¾Ð²Ð° | Ð¡ÑÐ°Ð½ |
|---|---|
| Hub `gsv-server` | Ð¶Ð¸Ð²Ð¸Ð¹, `:9999`, crate 0.248.0, RAM ~88â90 %, TTFB 2â3 s |
| `#38 bifrost` (M2004J19C) | online, charging, proofs_fresh â¥ 8, battery ~76 %, **ÑÐ´Ð¸Ð½Ð¸Ð¹ Ð¿ÑÐ´ÐºÐ»ÑÑÐµÐ½Ð¸Ð¹ peer** |
| `#35 valhall` (Galaxy A54) | ~9 Ð³Ð¾Ð´ offline; Ð¿Ð¾Ð²ÐµÑÐ½ÐµÑÑÑÑ â re-peer ÑÐµÑÐµÐ· `device_key` SHA-256 |
| `#37 FF-PC` | Ð½Ðµ Ð¿ÑÐ¸ÑÑÑÐ½Ñ |
| `#36 Pi 4` | **Ð¼ÐµÑÑÐ²Ð°** (SD), Ð¿Ð¾Ð·Ð° Ð³ÑÐ¾Ñ |
| 35B Ð²Ð°Ð³Ð¸ | blk.0â¦39 ÑÐ¾Ð·ÑÐµÐ»ÐµÐ½Ñ Ð½Ð° ÑÐ»Ð¾Ñ ÑÐ°Ð½ÑÑÐµ (40/40 block-verified, max_err â¤ 3.5e-6); mirror Ð½Ð° `L:/gsv_mirror/wgpu_weights` (~520 MB, 127 chunks) |

---

## ÐÐ±Ð¼ÐµÐ¶ÐµÐ½Ð½Ñ (Ð¶Ð¾ÑÑÑÐºÑ, Ð· owner-ÑÑÑÐµÐ½Ñ)

1. **ÐÐ¸ÑÐµ ÑÐ¾Ð·Ð²'ÑÐ·ÑÐ²Ð°Ð½Ñ Ð·Ð°Ð´Ð°ÑÑ** â Ð½ÑÑÐ¾Ð³Ð¾, ÑÐ¾ Ð²Ð¸Ð¼Ð°Ð³Ð°Ñ Pi, Ð´ÑÑÐ³Ð¾Ð³Ð¾ ÑÐµÐ»ÐµÑÐ¾Ð½Ð°, Ð½Ð¾Ð²Ð¾Ð³Ð¾ Ð·Ð°Ð»ÑÐ·Ð°.
2. **ÐÐ¾Ð´Ð½Ð¸Ñ Ð·Ð°Ð¿Ð¸ÑÑÐ² Ð½Ð° Ð´Ð¸ÑÐºÐ¸ Ð¿ÑÑÑÐ²** (Ð½ÑÑÐ¾Ð³Ð¾ Ð² ÑÐµÐ»ÐµÑÐ¾Ð½, Ð½ÑÑÐ¾Ð³Ð¾ Ð² SD) â policy.
3. **RAM-ÑÐ°Ð± ~90 %** â Ð¶Ð¾Ð´Ð½Ð° Ð·Ð°Ð´Ð°ÑÐ° Ð½Ðµ ÑÐ¼ÑÑ Ð´Ð¾Ð´Ð°Ð²Ð°ÑÐ¸ ÑÐ¾Ð½Ð¾Ð²Ð¾Ð³Ð¾ Ð½Ð°Ð²Ð°Ð½ÑÐ°Ð¶ÐµÐ½Ð½Ñ Ð±ÐµÐ· Ð²Ð¸Ð³ÑÐ°ÑÑ.
4. **Wi-Fi 2.4 GHz â ÑÑÐµÐ»ÐµÑ** â dispatch-ÐºÐ°Ð½Ð´Ð¸Ð´Ð°ÑÐ¸ ÑÑÐ»ÑÐºÐ¸ ÑÐ°ÐºÑ, ÑÐ¾ Ð¿ÐµÑÐµÐ¶Ð¸Ð²Ð°ÑÑÑ ÐºÐ»ÑÑÐ½ÑÑÑÐºÐ¸Ð¹ ÑÐ°Ð¹Ð¼Ð°ÑÑ.
5. ÐÑÐ´Ñ-ÑÐºÐµ ÑÐ¾ÑÐºÐ°Ð½Ð½Ñ ÐºÐ¾Ð´Ñ â `cargo fmt` â `clippy` â `cargo test` â **Ð¾Ð´Ð¸Ð½ commit** â push.
6. Owner-gated Ð¿ÑÐ½ÐºÑÐ¸ Ð½Ðµ Ð²Ð¸ÐºÐ¾Ð½ÑÑÑÑÑÑ Ð±ÐµÐ· ÑÐ²Ð½Ð¾Ð³Ð¾ Â«Ð³Ð¾Â» Ñ ÑÐµÑÑÑ.

---

## A. ÐÐ¾ÐºÑÐ¼ÐµÐ½ÑÐ¸ Ð¹ ÐºÐ¾Ð½ÑÐ¾Ð»ÑÐ´Ð°ÑÑÑ (Ð±ÐµÐ· ÐºÐ¾Ð´Ñ, Ð±ÐµÐ· owner)

| # | ÐÐ°Ð²Ð´Ð°Ð½Ð½Ñ | ÐÑÑÐ¸Ð»Ð»Ñ | Gate | ÐÐ°Ð»ÐµÐ¶Ð½Ð¾ÑÑÑ |
|---|---|---|---|---|
| A1 | **SESSION_PRESET.md** â ÑÑÐ¸ÑÐ»Ð¸Ð¹ Ð¾Ð¿ÐµÑÐ°ÑÐ¾ÑÑÑÐºÐ¸Ð¹ Ð¿ÑÐµÑÐµÑ (100â200 ÑÑÐ´ÐºÑÐ²): ÑÐº Ð¿ÑÐ´Ð½ÑÑÐ¸ ÑÐ°Ð±, ÑÐº Ð·âÑÐ´Ð½Ð°ÑÐ¸ÑÑ Ð· bifrost, ÐºÐ¾Ð¼Ð°Ð½Ð´Ð¸ health/mesh/console/telemetry, ÑÐµÐºÐ»ÑÑÑ Ð²ÑÐ´Ð½Ð¾Ð²Ð»ÐµÐ½Ð½Ñ Ð¿ÑÑÐ»Ñ Ð¿Ð°Ð´ÑÐ½Ð½Ñ â **â done 2026-11-19** | S | no | `docs/gsv/INDEX.md` |
| A2 | **INDEX.md refresh** â `SESSION_PRESET.md` Ñ Ð¶Ð¸Ð²Ñ Ð´Ð¾ÐºÑÐ¼ÐµÐ½ÑÐ¸ â **â done 2026-11-19** | S | no | A1 |
| A3 | **k8s-Ð¿Ð»Ð°Ð½ â OBSOLETE banner** â `2026-11-19-mesh-k8s-maxload-plan.md` Ð¿Ð¾Ð·Ð½Ð°ÑÐµÐ½Ð¾ Ð·Ð°ÑÑÐ°ÑÑÐ»Ð¸Ð¼ (Pi dead) â **â done 2026-11-19** | S | no | â |
| A4 | **HANDOFF Ð´ÐµÐºÐ¾Ð¼Ð¿ÑÐµÑÑÑ** â ÑÑÑÐ¾ÑÐ¸ÑÐ½Ñ Ð±Ð»Ð¾ÐºÐ¸ (2026-10-24 ÑÑÐ°Ð½ Â· band 239â244 Â· ÑÑÑÐ¾ÑÑÑ Ð±Ð¾ÐºÑÑÐ² 109â126) Ð²Ð¸Ð½ÐµÑÐµÐ½Ð¾ Ð² `docs/archive/2026-11/HANDOFF_ARCHIVE_2026-11-19.md`; HANDOFF 450 â **117 ÑÑÐ´ÐºÑÐ²**, Ð»Ð¸ÑÐ°ÑÑÑÑÑ Â«Ð¡ÑÐ°Ð»Ð¸Ð¹ ÑÑÐ°Ð½ + ÑÐµÑÐ³Ð°Â» â **â done 2026-11-19** | M | no | A2 |
| A5 | **NEXT_SESSION_PROMPT** â ÑÐ¸Ð½ÑÑÐ¾Ð½ÑÐ·ÑÐ²Ð°ÑÐ¸ Ð· ÑÐµÐ°Ð»ÑÐ½ÑÑÑÑ (Pi dead, ÑÑÐ»ÑÐºÐ¸ bifrost, hot path = AP 2.4 GHz) | S | no | A1 |

## B. Ð¢ÐµÐ»ÐµÑÐ¾Ð½ #38 bifrost â ÑÑÐ°Ð±ÑÐ»ÑÐ½ÑÑÑÑ ÐºÐ¾Ð½ÐµÐºÑÑ (ÐºÐ»ÑÑÐ¾Ð²Ð° Ð±Ð¾Ð»ÑÐ»Ñ)

Ð¡Ð¸Ð¼Ð¿ÑÐ¾Ð¼: `worker: hub unreachable (retryingâ¦)` / `hub: connection restored` ÐºÐ¾Ð¶Ð½Ñ ÐºÑÐ»ÑÐºÐ° ÑÐµÐºÑÐ½Ð´ Ñ
`/api/wgpu/console`. ÐÐ¾Ð¶Ð»Ð¸Ð²Ñ Ð¿ÑÐ¸ÑÐ¸Ð½Ð¸: Android Doze/App standby Ð²Ð±Ð¸Ð²Ð°Ñ Ð²ÐºÐ»Ð°Ð´ÐºÑ Chrome; 2.4 GHz congestion;
push-ÐºÐ°Ð½Ð°Ð» (RFC 6455) ÑÐ¾Ð·ÑÐ°ÑÐ¾Ð²Ð°Ð½Ð¸Ð¹ Ð½Ð° LAN, ÑÐµÐ»ÐµÑÐ¾Ð½ â Ð½Ð° Ð¼ÐµÐ¶Ñ Ð´Ð°Ð»ÑÐ½Ð¾ÑÑÑ AP.

| # | ÐÐ°Ð²Ð´Ð°Ð½Ð½Ñ | ÐÑÑÐ¸Ð»Ð»Ñ | Gate | ÐÐ°Ð»ÐµÐ¶Ð½Ð¾ÑÑÑ |
|---|---|---|---|---|
| B1 | **30-ÑÐ² Ð¿Ð°ÑÐ¸Ð²Ð½Ð¸Ð¹ Ð·Ð°Ð¿Ð¸Ñ** `/api/wgpu/console` + `/api/sys/metrics` (age/proofs/battery) â ÑÐ°Ð±Ð»Ð¸ÑÑ Â«ÑÑÑÐ³Ð³ÐµÑ / ÑÐ¾Ð·ÑÐ¸Ð² / Ð²ÑÐ´Ð½Ð¾Ð²Ð»ÐµÐ½Ð½ÑÂ» | S | no | live ÑÐ°Ð± |
| B2 | **Push vs poll A/B** â ÐºÐ»ÑÑÐ½Ñ `/wgpu` PWA: Ð¿ÐµÑÐµÐ²ÑÑÐ¸ÑÐ¸ ÑÐ¾ WS-ÑÐµÐºÐ¾Ð½ÐµÐºÑ Ð· backoff Ð¿ÑÐ°ÑÑÑ, Ñ ÑÐ¸ Ð´Ð¾Ð¿Ð¾Ð¼Ð°Ð³Ð°Ñ fallback Ð½Ð° SSE/poll ÐºÐ¾Ð¶Ð½Ñ 5 s Ð¿ÑÐ¸ ÑÐ¾Ð·ÑÐ¸Ð²Ñ push | M | no | B1 |
| B3 | **PWA keep-alive — [MESH_PWA_KEEPALIVE_2026-11-20.md](MESH_PWA_KEEPALIVE_2026-11-20.md)** — Android → Battery → Apps → Chrome = "Unrestricted" (NOT per-tab adaptive); Screen-timeout "Never" д PWA. Client-fix (2-й фланг, owner-gated): ws.onclose реконект 5 s → debounce 20 s ×2..4. Live-верифікація 04:2x: p39 `push channel open` — **✅ done 2026-11-20** | M | no (2-й фланг, клієнт-фікс) | MESH_PWA_KEEPALIVE_2026-11-20.md |

| B4 | **Heartbeat ÑÑÐ½ÑÐ½Ð³** â push mesh-metrics cadence 2 s â **5 s** Ð·Ð° Ð·Ð°Ð¼Ð¾Ð²ÑÑÐ²Ð°Ð½Ð½ÑÐ¼ (env `GSV_WGPU_WS_PUSH_SECS` 1..=30) â **â done 2026-11-19** (Ð¼ÐµÐ½ÑÐµ ÐºÐ¾Ð½ÑÐµÐ½ÑÐ½ Ð½Ð° 2.4 GHz + Ð¼ÐµÐ½ÑÐµ Ð½Ð°Ð²Ð°Ð½ÑÐ°Ð¶ÐµÐ½Ð½Ñ Ð½Ð° ÑÐ°Ð±; reversible ÑÐ»Ð°Ð³Ð¾Ð¼) | S | no | B1 |
| B5 | **Auto re-register Ð¿ÑÑÐ»Ñ ÑÐµÑÑÐ°ÑÑÑ** â Ð¿ÐµÑÐµÐ²ÑÑÐ¸ÑÐ¸: `device_key` Ð²ÑÐ´Ð½Ð¾Ð²Ð»ÑÑÑÑÑÑ, `peer_id` #38 Ð½Ðµ Ð¼ÑÐ³ÑÑÑ Ð¿ÑÑÐ»Ñ reload ÑÑÐ¾ÑÑÐ½ÐºÐ¸ | S | no | B1 |
| B6 | **ÐÐ·ÐµÑÐºÐ°Ð»Ð¾ 35B-ÑÐ°Ð½ÐºÑÐ² Ñ phone cache** â IndexedDB LRU 128 MiB Ð²Ð¶Ðµ Ñ (R3); Ð¿ÐµÑÐµÐ²ÑÑÐ¸ÑÐ¸ hit-rate Ð¿ÑÑÐ»Ñ 24 h soak; Ð·Ð° Ð¿Ð¾ÑÑÐµÐ±Ð¸ Ð¿ÑÐ´Ð½ÑÑÐ¸ Ð»ÑÐ¼ÑÑ | M | no | B1 |

| B7 | **RSSI/netinfo → telemetry/hub** — **✅ done 2026-11-19 (перевірено живцем 2026-11-20):** phone-JS probeTelemetry у `/wgpu` (band 855ab65): ua, lang, screen, dpr, online, `navigator.connection` → downlink/effectiveType/rtt/saveData, `deviceMemory`→mem, battery+charging, storage estimate; POST → `/api/wgpu/telemetry` → санітазія → `data/wgpu_telemetry.jsonl` ring 100. **Live (2026-11-20):** `GET /api/wgpu/telemetry` → latest rows: peer#39 `battery 0.78 charging true cores 8 dpr 2.75 effectiveType 4g rtt 100 saveData false screen 393x851 storageQuota 10.7GB` (плюс page/host rows). Тобто RSSI/netinfo у /sys — **уже живий шлях всю добу** (5 s cadence; fail-loud post ≠ log `telemetry post rejected` при non-200). | M | no | `/api/wgpu/telemetry` (live-verified) |


## C. ÐÐ²ÑÐ¾Ð¼Ð°ÑÐ¸Ð·Ð°ÑÑÑ Ð¾Ð¿ÐµÑÐ°ÑÑÐ¹Ð½Ð¾Ñ Ð´ÑÑÐ»ÑÐ½Ð¾ÑÑÑ (Rust-first, Ð±ÐµÐ· Ð½Ð¾Ð²Ð¸Ñ ÑÐµÑÐ²ÑÑÑÐ²)

| # | ÐÐ°Ð²Ð´Ð°Ð½Ð½Ñ | ÐÑÑÐ¸Ð»Ð»Ñ | Gate | ÐÐ°Ð»ÐµÐ¶Ð½Ð¾ÑÑÑ |
|---|---|---|---|---|
| C1 | **`cargo xtask status`** â Ð¾Ð´Ð¸Ð½ ÑÑÐ´Ð¾Ðº: hub health Â· version Â· mesh alive/ready(alive) ids Â· tasks done Â· console last line Â· RAM/disk (`--json`, `--url`) â **â done 2026-11-19** | S | no | `src/xtask.rs` |
| C2 | **`cargo xtask alive-watch`** â probe `/api/health` N ÑÐ°Ð·ÑÐ² (`--every N`, `--count N`, default 1 probe); rc 0/1; stdout only (scheduler-friendly) â **â done 2026-11-19** | S | no | C1 |
| C3 | **`cargo xtask cycle-watch`** — консольний watchdog reconnect-storm-ів телефона: ring `/api/wgpu/console` + rolling 24 h стан (`data/cycle_watch_state.json`), пороги `--hourly 10` / `--ten-min 3` / `--blackout-secs 180` / `--quiet-secs 900`, rate-limit алертів 30 хв; за замовчуванням **dry**, `--alert` → Telegram у **Godfather-канал** через `sendMessage` (без Telenetis) — **✅ done 2026-11-19**. `cargo xtask cycle-watch-install` — **HKCU Run `GSV-cycle-watch` → wscript + `gsv_cycle_watch_start.vbs`** (window-0 запуск, патерн = watchdog; dedup через `data/cycle_watch.lock` 2 інстанси/1200 s) — **✅ done 2026-11-19**; schtasks → access denied, без admin. lib 541 | M | done | console ring |
| C4 | **`cargo xtask report`** â markdown-Ð·Ð²ÑÑ ÑÐ· `sys_metrics_history.jsonl`: CPU/RAM avg/min/max, disk trend, tasks_done/failed Î (counter-reset-safe), per-peer alive/ready share, done Î, proofs fresh, EWMA avg; `--days N`, stdout Ð°Ð±Ð¾ `--out` â **â done 2026-11-19**; Ð¿ÐµÑÑÐ¸Ð¹ Ð·ÑÐ°Ð·Ð¾Ðº Ñ `docs/gsv/reports/MESH_REPORT_2026-11-19.md` | M | no | `sys-history` |
| C6 | **EWMA-Ð¿Ð°ÑÑÑÐµÑ** â `ewma_elapsed_ms` + `proofs_fresh` Ð²Ð¶Ðµ Ð² `/api/sys/metrics` + /sys UI ewma-ÐºÐ¾Ð»Ð¾Ð½ÐºÐ° (d83c35e); ÑÑÐ°ÑÑÑ-ÑÐµÑÐ½ÑÑÑÑ Ð¿ÐµÑÐµÐ²ÑÑÐµÐ½Ð¾ Ð¶Ð¸Ð²ÑÐµÐ¼ (report Ð²ÑÐ´Ð¾Ð±ÑÐ°Ð¶Ð°Ñ `ready_alive`-share) | S | no | d83c35e |
| C5 | **Dispatch gate Ð°ÑÐ´Ð¸Ñ** â Ð¿ÑÐ¾Ð¹Ð´ÐµÐ½Ð¾ `wgpu.rs`: enqueue (`restrict_ready = is_heavyweight`, ranked aliveâ§proof_gate; single-shard `best_peer_for_task().filter(ready_of)`) â `peer_poll` (heavyweight Ð²ÑÐ´ÐºÐ¸Ð´Ð°ÑÑÑÑÑ Ð´Ð»Ñ Ð½Ðµ-ready Ð¿ÑÑÑÐ² + `readiness_hint`; `weights_matvec_block` â Ð»Ð¸ÑÐµ browser-ÐºÐ»Ð°ÑÐ¸; sticky route) â `proof_gate` (freshâtrue; stale+fresh-aliveâghost demoted; Ð½ÑÑÐ¾Ð³Ð¾ fresh Ð½ÑÐ´Ðµ â any-proof pass, no lockout). **ÐÑÑ Ð½Ðµ Ð·Ð½Ð°Ð¹Ð´ÐµÐ½Ð¾; fail Ð½Ð°Ð¿ÑÐ°Ð²Ð»ÐµÐ½Ð¸Ð¹ Ñ `readiness_hint`, Ð½Ðµ Ñ Ð¼Ð¾Ð²ÑÐ°Ð·Ð½Ð¸Ð¹ "Ð·Ð°ÑÑÑÑÐ³" â â verified 2026-11-19** | S | no | wgpu.rs |
| C7 | **Stand-smoke Ñ CI** â GitHub workflow `gsv`: Ð·Ð°Ð¿ÑÑÐº `gsv-http-stand-smoke` Ð¿ÑÐ¾ÑÐ¸ ÑÐ¸Ð¼ÑÐ°ÑÐ¾Ð²Ð¾Ð³Ð¾ ÑÐ°Ð±Ð° (skip ÑÐºÑÐ¾ ÑÐµÐ¿Ð¾Ð·Ð¸ÑÐ¾ÑÑÐ¹ Ð½Ðµ Ð¼Ð¾Ð¶Ðµ Ð¿ÑÐ´Ð½ÑÑÐ¸ `:9999`) | M | no | stand-smoke bin |

## D. Mesh-hardening Ð· Ð¾Ð´Ð½Ð¸Ð¼ ÑÐµÐ»ÐµÑÐ¾Ð½Ð¾Ð¼ (Ð±ÑÑÑÐ¾ÑÑ ÑÐº ÑÐ´Ð¸Ð½Ð¸Ð¹ worker)

| # | ÐÐ°Ð²Ð´Ð°Ð½Ð½Ñ | ÐÑÑÐ¸Ð»Ð»Ñ | Gate | ÐÐ°Ð»ÐµÐ¶Ð½Ð¾ÑÑÑ |
|---|---|---|---|---|
| D1 | **Golden-shard canary** â periodic (owner-triggered) `weights_matvec` blk.5 shexp Ð½Ð° #38, Ð·Ð²ÑÑÐºÐ° `max_err_model â¤ 1e-6`; Ð´ÐµÐ»ÑÑÐ° â Ð»Ð¾Ð³, Ð½Ðµ Ð±ÑÐ»ÑÑÐµ | S | owner Ð´Ð»Ñ Ð·Ð¼ÑÐ½ dispatch | L: mirror |
| D2 | **40/40 re-verify Ð¾Ð´Ð½Ð¸Ð¼ peer** â Ð¿ÑÐ¾Ð³ÑÐ½ ÑÑÑÑ blk-shardÑÐ² ÑÑÐ»ÑÐºÐ¸ ÑÐµÑÐµÐ· #38 (Ð´Ð¾ÐºÐ°Ð· single-phone 35B ÑÑÐ°Ð½ÑÐ¿Ð¾ÑÑÑ); ~35â40 ÑÐ², Ð±Ð°ÑÐ°ÑÐµÑ | M | owner (power) | D1 |
| D3 | **EWMA single-peer sanity** â Ð´Ð¾Ð´Ð°Ð½Ð¾ ÑÐµÑÑ `single_peer_router_degenerates_cleanly` (tier-0 single peer chosen; tier-3 probation routable; saturated â None; dead â None; no panics) â **â done 2026-11-19** (Ð´ÑÐ»ÐµÐ½Ð½Ñ Ð½Ð° Ð½ÑÐ»Ñ/NaN-key Ð½Ðµ ÑÑÐ½ÑÑ Ð² ÐºÐ¾Ð´Ñ â sort Ð²Ð¸ÐºÐ¾ÑÐ¸ÑÑÐ¾Ð²ÑÑ `unwrap_or(Equal)`; Ð·Ð°ÑÑÐºÑÐ¾Ð²Ð°Ð½Ð¾ ÑÐµÑÑ-ÐºÐ¾Ð½ÑÑÐ°ÐºÑÐ¾Ð¼) | S | no | wgpu.rs |
| D4 | **IndexedDB eviction audit** â 128 MiB LRU Ð¿ÐµÑÐµÐ¿Ð¾Ð²Ð½ÐµÐ½Ð½Ñ â ÑÐ°Ð½ÐºÐ¸ Ð²Ð¸ÑÑÑÐ½ÑÑÑÑÑÑ; Ð»Ð¾Ð³ÑÐ²Ð°ÑÐ¸ eviction count; ÑÐºÑÐ¾ > N/Ð´Ð¾Ð±Ñ â docs-Ð½Ð¾ÑÐ°ÑÐºÐ° | S | no | B6 |
| D5 | **L: mirror integrity sweep** â sha256 ÑÑÑÑ ÑÐ°Ð¹Ð»ÑÐ² Ñ `L:/gsv_mirror/wgpu_weights/chunks/` Ð¿ÑÐ¾ÑÐ¸ `index.json`: **31/31 chunks OK** (size + sha256 == filename, 116 433 397 B), `MIRROR-OK`; loose `blk*_f32` (Ð¿Ð¾Ð·Ð° wgpu_weights) â Ð±ÐµÐ· manifest, Ð½Ðµ Ð¿ÐµÑÐµÐ²ÑÑÑÐ»Ð¸ÑÑ; **â done 2026-11-19** (Ð½ÑÐ°Ð½Ñ: Ð·Ð°ÑÐ°Ð· 31 chunk, Ð½Ðµ 127 â Ð´Ð·ÐµÑÐºÐ°Ð»o ÑÑÐ¸Ð¼ÑÑÐ¾Ð²Ð°Ð½Ð¾/LRU Ð¿ÑÑÐ»Ñ 35B-ÑÐ¾Ð·ÑÐµÐ»ÐµÐ½Ð½Ñ) | M | no | L: drive |
| D6 | **Proof TTL review + failover doc** — `MESH_FAILOVER_2026-11-20.md` (proof_gate semantics, HTTP behavior, UI-states, rejoin checklist) — **✅ done 2026-11-20** (read-only audit; fail-open, honest proofs, no full lockout) | S | no | MESH_FAILOVER_2026-11-20.md, wgpu.rs |

| D7 | **Failover документовано** — `MESH_FAILOVER_2026-11-20.md` (rejoin checklist, timeline bifrost #38→#39, ready_flip, UI-states) — **✅ done 2026-11-20** (docs-only; peer_poll dead → `task: null` ✅ verified, block err 400 ✅, hint ✅) | S | no | MESH_FAILOVER_2026-11-20.md |


## E. poolAI CI / Ð°11y (Ð·Ð¾Ð²Ð½ÑÑÐ½ÑÐ¹ ÑÑÐµÐº, Ð½Ðµ GSV-ÐºÐ¾Ð´)

| # | ÐÐ°Ð²Ð´Ð°Ð½Ð½Ñ | ÐÑÑÐ¸Ð»Ð»Ñ | Gate | ÐÐ°Ð»ÐµÐ¶Ð½Ð¾ÑÑÑ |
|---|---|---|---|---|
| E1 | **Run 1711 Ð¼Ð¾Ð½ÑÑÐ¾ÑÐ¸Ð½Ð³** â Ð²Ð¸ÐºÐ¾Ð½Ð°Ð½Ð¾ 2026-11-19: Pa11y WCAG â green (a11y-ÑÑÐºÑ Ð²ÐµÑÐ¸ÑÑÐºÐ¾Ð²Ð°Ð½Ð¾), Playwright admin E2E â failure (Â«Build poolai and run smoke testsÂ», Ð»Ð¾Ð³Ð¸ 403 anonymous); ubuntu â, windows â (cloud-sdk green), ÐÐ¾Ðº. 1418 â. ÐÐµÑÐ°Ð»Ñ: `ECOSYSTEM_CI_REGISTRY` Â§6.1 â **â done 2026-11-19** | S | no | GitHub |
| E2 | **a11y.yml Ð²ÑÑÑÐ½Ñ / poolAI CI re-run** â ÑÐºÑÐ¾ Playwright E2E Ð·Ð½Ð¾Ð²Ñ ÑÐµÑÐ²Ð¾Ð½Ð¸Ð¹: authorized re-run Ð°Ð±Ð¾ `workflow_dispatch` a11y.yml (owner-ÐºÐ»Ð°Ñ) | S | owner | E1 |
| E3 | **Ubuntu leg** â non-blocking ÑÐ¶Ðµ (`4ebb1e410`); Ð»Ð¸ÑÐ¸ÑÐ¸ ÑÐº Ñ | â | done | â |
| E4 | **ECO-8.3 dep-sweep Ð¿ÑÐ¾Ð´Ð¾Ð²Ð¶ÐµÐ½Ð½Ñ** â ÑÐµÑÑÐ° Ð¿Ð»Ð°Ð³ÑÐ½ÑÐ² Ð¿Ð¾ Ð¾Ð´Ð½Ð¾Ð¼Ñ (LinFS, ORR, rusty-engine, sport-forcast, CoreBoard) | L | no, Ð°Ð»Ðµ Ð¿Ð¾ Ð¾Ð´Ð½Ð¾Ð¼Ñ Ð·Ð° ÑÐµÑÑÑ | ECO board |

## F. adoreble_wifi / ÑÐ½ÑÑÐ° Ð²Ð»Ð°ÑÐ½Ð¸ÐºÐ° (Ð¿Ð»Ð°Ð³ÑÐ½, Ð½Ðµ GSV)

| # | ÐÐ°Ð²Ð´Ð°Ð½Ð½Ñ | ÐÑÑÐ¸Ð»Ð»Ñ | Gate | ÐÐ°Ð»ÐµÐ¶Ð½Ð¾ÑÑÑ |
|---|---|---|---|---|
| F1 | **ADO-7 ÐºÐ°Ð½Ð´Ð¸Ð´Ð°ÑÐ¸ (docs-only Ð¿Ð¾ÐºÐ¸)** â battery-friendly hotspot auto-off timer; QR-ÑÑÐ¾ÑÑÐ½ÐºÐ° status page; SLA probe â Telegram (owner-key) | â | owner | ADO-6 |
| F2 | **PRODUCTS.md row** â `adoreble_wifi` Ð·Ð°ÑÐµÑÑÑÑÐ¾Ð²Ð°Ð½Ð¾; Ð¿ÑÐ´ÑÑÐ¸Ð¼ÑÐ²Ð°ÑÐ¸ Ð°ÐºÑÑÐ°Ð»ÑÐ½Ð¸Ð¼ (HEAD, ÑÐµÑÑÑ count) | S | no | PRODUCTS.md |
| F3 | **`ado` CLI parity with UI** â `hotspot status` Ð²Ð¶Ðµ Ñ; Ð¿ÐµÑÐµÐ²ÑÑÐ¸ÑÐ¸ `--json` output Ð´Ð»Ñ ÑÐºÑÐ¸Ð¿ÑÑÐ² | S | no | adoreble_wifi |
| F4 | **HKCU Run entry audit** â `AdorebleWiFi` Ð°Ð²ÑÐ¾Ð·Ð°Ð¿ÑÑÐº: Ð·Ð°Ð´Ð¾ÐºÑÐ¼ÐµÐ½ÑÑÐ²Ð°ÑÐ¸ ÑÐº Ð²Ð¸Ð´Ð°Ð»Ð¸ÑÐ¸ (owner ÑÐ½ÑÑÑÑÐºÑÑÑ) | S | owner | AdorebleWiFi |

## G. WSP / research (owner-gated, Ð½Ðµ Ð·Ð°ÑÐ°Ð·)

| # | ÐÐ°Ð²Ð´Ð°Ð½Ð½Ñ | ÐÑÑÐ¸Ð»Ð»Ñ | Gate |
|---|---|---|---|
| G1 | WSP Phase 2 â **Ð½Ðµ Ð²ÑÐ´ÐºÑÐ¸Ð²Ð°ÑÐ¸** Ð±ÐµÐ· ÑÐ²Ð½Ð¾Ð³Ð¾ owner go (Astra-ÑÑÐ¸Ð»Ñ patch Ð½Ðµ Ð¼Ð°Ñ ÑÐ¾Ð±Ð¾ÑÐ¾Ñ Ð³ÑÐ¿Ð¾ÑÐµÐ·Ð¸ Ð½Ð° MiniCPM-1B; Granite-A800M serve-ÑÐ¾Ð·ÑÐ»ÑÐ´ÑÐ²Ð°Ð½Ð½Ñ Ð½Ðµ Ð·Ð°Ð²ÐµÑÑÐµÐ½Ðµ) | â | owner |
| G2 | Granite-A800M serve retry Ð½Ð° `:8085` (UCRT-dll Ð²Ð¶Ðµ ÑÐºÐ¾Ð¿ÑÐ¹Ð¾Ð²Ð°Ð½Ñ Ð² `target/release/`) â ÑÑÐ»ÑÐºÐ¸ Ð¿ÑÑÐ»Ñ owner go | M | owner |
| G3 | NanoJev port feasibility â Ð»Ð¸ÑÐ°ÑÑÑÑÑ doc-only; Pi dead â Ð½ÑÐºÑÐ´Ð¸ Ð´ÐµÐ¿Ð»Ð¾ÑÑÐ¸; skip | â | owner |

## H. Ð¯Ð²Ð½Ð¾ Ð²Ð¸ÐºÐ»ÑÑÐµÐ½Ð¾ (Ð½Ðµ Ð¿ÑÐ¾Ð¿Ð¾Ð½ÑÐ²Ð°ÑÐ¸)

- k3s/k8s/Docker Ð½Ð° Pi â **Ð¼ÐµÑÑÐ²Ð° SD**; policy Â«Ð´Ð¸ÑÐºÐ¸ Ð¿ÑÑÑÐ² Ð½Ðµ ÑÑÐ¿Ð°ÑÐ¼Ð¾Â».
- ÐÑÐ´Ñ-ÑÐºÑ Ð·Ð¼ÑÐ½Ð¸ Ð½Ð° ÑÐµÐ»ÐµÑÐ¾Ð½Ñ (ADB, flags, system settings) Ð±ÐµÐ· owner go per action.
- 35B Ð¿Ð¾Ð²Ð½Ð¸Ð¹ forward Ð½Ð° ÑÐ°Ð±Ñ â RAM Ð½Ðµ ÑÑÐ³Ð½Ðµ (7.7 GB total).
- ÐÑÐ´Ð½ÑÑÑÑ `:8091` (OmniRoute), tunnels, User-scope MCP â owner-gated Ð·Ð°Ð²Ð¶Ð´Ð¸.
- Ð ÐµÐ°Ð½ÑÐ¼Ð°ÑÑÑ Pi (reflash, edit `cmdline.txt`, Ð½Ð¾Ð²Ñ Ð¾Ð±ÑÐ°Ð·Ð¸) â owner-only, Ð¿ÑÑÐ»Ñ ÑÑÐ·Ð¸ÑÐ½Ð¾Ð³Ð¾ Ð´Ð¾ÑÑÑÐ¿Ñ.

---

## Ð ÐµÐºÐ¾Ð¼ÐµÐ½Ð´Ð¾Ð²Ð°Ð½Ð¸Ð¹ Ð¿Ð¾ÑÑÐ´Ð¾Ðº

1. **A1âA3** (ÑÑÐ¾Ð³Ð¾Ð´Ð½Ñ, docs) â commit `docs: ...` â push â **â Ð·ÑÐ¾Ð±Ð»ÐµÐ½Ð¾**
2. **B1** (Ð·Ð°Ð¿Ð¸Ñ ÑÑÐ°ÑÑÑÐ½Ð³Ñ) â Ð²Ð¸ÑÐ½Ð¾Ð²Ð¾Ðº B2/B4 â Ð´ÑÑÐ±Ð½Ð¸Ð¹ ÐºÐ¾Ð´-ÑÑÐºÑ ÑÐºÑÐ¾ Ð¿ÑÐ´ÑÐ²ÐµÑÐ´Ð¶ÐµÐ½Ð¾ â **â Ð·ÑÐ¾Ð±Ð»ÐµÐ½Ð¾ (B4)**
3. **C1 + C2** (xtask status/alive-watch) â commit â **â Ð·ÑÐ¾Ð±Ð»ÐµÐ½Ð¾**
4. **D3** (single-peer EWMA unit-ÑÐµÑÑ) â commit â **â Ð·ÑÐ¾Ð±Ð»ÐµÐ½Ð¾**
5. **D1** (golden canary, owner) Ð°Ð±Ð¾ ~~A4~~ â **A4 â Ð·ÑÐ¾Ð±Ð»ÐµÐ½Ð¾ 2026-11-19**.
6. **E1** â Ð¿ÐµÑÐµÐ²ÑÑÐµÐ½Ð¾ 2026-11-19 (Pa11y â; Playwright â â owner, Ð´Ð¸Ð². Â§6.1 ÑÐµÑÑÑÑÑ).

## ÐÑÐ¸ÑÐµÑÑÑ Ð³Ð¾ÑÐ¾Ð²Ð½Ð¾ÑÑÑ ÐºÐ¾Ð¶Ð½Ð¾Ð³Ð¾ Ð¿ÑÐ½ÐºÑÑ

- ÐÐ¾Ð´ â `cargo fmt` + `cargo clippy --all-targets` + `cargo test` (exit 0) + `--stretch-96` Ð´Ð»Ñ GSV-ÑÐµÐ¿Ð¾.
- Docs â Ð¿Ð¾ÑÐ¸Ð»Ð°Ð½Ð½Ñ Ð¶Ð¸Ð²Ñ, `cargo xtask sync` (vision lockstep) Ð½Ðµ Ð»Ð°Ð¼Ð°ÑÑÑÑÑ.
- ÐÐ´Ð¸Ð½ commit Ð½Ð° Ð¿ÑÐ½ÐºÑ; push ÑÑÐ»ÑÐºÐ¸ Ð¿ÑÑÐ»Ñ Ð·ÐµÐ»ÐµÐ½Ð¾Ð³Ð¾ gate.

| Зауваження 2026-11-19 (C6 жива верифікація) | Парітет `/api/wgpu/mesh` vs `/api/sys/metrics` перевірений живцем (03:xx): обидві повертають **id=39 arm/bifrost alive** (FF-PC #37 повернувся, dead ~02:31); `cargo xtask status` — `alive 1/5 ready 0 [39]`. Денний звіт: `MESH_REPORT_2026-11-20.md` — `ready` виправлено на `ready_alive` (FF-PC #37: alive 92%, ready_alive 92%; біофрост #39: alive 80%, ready 0) + `ready_stale` колонка (1468 dead rows з ready:true). 

| Зауваження 2026-11-20 (FF-PC #37 знову dead) | FF-PC #37 вийшов під час дня (age_secs≈12330 = ~3.4 h, dead ~04:0x). Зараз єдиний підключений peer — **#39 bifrost** (alive 80%, ready_alive 0%, proofs 0). `cargo xtask status` — `alive 1/5 ready 0 [39]` (на 2026-11-20). Чекаємо на нову `mesh worker online` або рестарт під час воно на телефоні. |
