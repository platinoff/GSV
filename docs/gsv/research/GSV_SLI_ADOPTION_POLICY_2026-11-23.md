# GSV SLI adoption policy + plugin automation matrix (2026-11-23)

**Ticket:** `t-1795406229356654300` · **Scope:** docs-only. No plugin code, no secrets.
**Fact source:** `gsv_sli` catalog (2026-11-23: `used_count=7`, `unused_count=40`), `cargo xtask products --wiring`, `cargo xtask rules-check`, research spec `ECOSYSTEM_1_100_STEADY_STATE_2026-11-23.md`.

## 1. Що насправді означає `used` / `unused`

`used` у SLI catalog — це **лічильник використання в поточній сесії/каталозі**, а not ознака застарілості чи зламаності. Приклади: `gsv_live`, `gsv_watchdog`, `disk`, `bump`, `push`, `gsv_mcp` мають `used:false`, хоча це активна інфраструктура (хаб `:9999` тримається watchdog-ом, MCP — 60 tools живий). Тому **не можна** робити висновок «40 зайвих SLI» — це хибна інтерпретація метрики.

Правильне застосування `unused`: сигнал до **огляду**, чи варто задіяти-surface у поточному drain-пайплайні, а не do видалення.

## 2. Категоризація 47 surface-ів

| Категорія | Surface-и | Політика |
|---|---|---|
| **A. Live infra (unused counter ≠ dead)** | `live`, `watchdog`, `watchdog-install`, `disk`, `bump`, `push`, `git`, `gsv_live`, `gsv_watchdog`, `gsv_server`, `gsv_mcp`, `gsv_cursor_rules`, `gsv_keep_live_boot_verify` | Не чіпати. Працюють як сервіси/CLI; в haul не викликались — норма. |
| **B. Periodic / scheduler** | `alive-watch`, `cycle-watch`, `cycle-watch-install`, `status`, `report`, `sys-history`, `vault-note` | Задіяти за розкладом (HKCU Run/XTask вже частина), не в кожному drain. |
| **C. Owner-gated** | `tunnel` (cloudflared), `gsv_apk` (physical device), `watchdog-install` (persist), `git push`, `cycle-watch-install` | Тільки явний owner go. Ніколи автоматично з MCP/агента. |
| **D. Verify/read-only gate** | `eco-check` ✅used, `rules-check` ✅used, `fm-xlink`, `mirrors --check`, `products --state/--wiring`, `sync --check`, `inventory`, `intel`, `fingerprint-recheck/dedup`, `gasv-publish-check`, `mirror-inventory`, `gsv_loc_audit`, `gsv_rust_diagnostics`, `gsv_speed_index`, `gsv_http_stand_smoke`, `gsv_ui_probe` | Компонувати в єдиний безпечний драйн-гейт (§3). |
| **E. Adoption candidates (safe, unused)** | `catalog`, `record-scenario-bench`, `gsv_mds`, `gsv_vision_sync` | Додати в NEXT/runbook як рекомендовані при相应 роботі; без авто-запуску. |

Used у цій сесії (7): `eco-check`, `fingerprint`, `products`, `record-rust`, `record-speed`, `rules-check`, `sync`.

## 3. Єдиний безпечний drain-гейт (composition, без мутацій)

Вже доведений ланцюг (2026-11-23):

```bash
cargo xtask eco-check                    # pin 1.100 + products + FM + vision (read-only)
cargo xtask rules-check                  # git/fingerprint/vision/registry/ratio drift (read-only)
cargo xtask fm-xlink                     # FM↔tickets advisory, --strict для CI
cargo xtask mirrors --check              # skills drift, non-zero on drift
cargo xtask products --state --wiring    # git HEAD/dirty + host/plugin wiring
cargo run --bin gsv-loc-audit -- --stretch-96   # Rust ratio ≥95%
```

Політика: **read-only за замовчуванням**; mutating (`bump`, `fingerprint`, `push`, `live`, `watchdog-install`, `tunnel`) — окремі кроки з owner/drain-правилами, нечастина compose-гейту.

## 4. Plugin automation matrix (див. PRODUCTS.md)

| Product | Toolchain pin class | Wiring | Remote | Gate authority |
|---|---|---|---|---|
| gsv (host) | `stable-x86_64-pc-windows-gnu` | na (host) | origin | cargo test + clippy + eco-check |
| poolai | GNU stable | ok | origin | CI (GitHub Actions), local fmt/clippy/check |
| linfs, telenetis, llama-rs, rebook, orr_desktop, omniroute, rusty-engine, gsv-wgpu, sport-forcast, coreboard_labs_shop, adoreble_wifi | GNU stable (adoreble host-agnostic `stable`) | ok | origin (крім gsv-wgpu — **no remote, owner gate**) | product CI + local gate |
| mesh-node | host-agnostic `stable` | **ok** (2026-11-23, old exception closed) | **no remote, owner gate** | Pi/LAN agent |
| freebeat.ai, platinoff-profile | n/a (git-only) | na | unregistered | out of scope |

omniroute = policy-down (`:20128`), не піднімати без owner.

## 5. Decision

SLI adoption = **документувати семантику + категоризувати + скомпонувати read-only гейт**, без масового «adopt» і без Changes у плагінах. 40 unused не є проблемою misma собою. G13 залишається blocked; жодного full-LLM/quality claim.
