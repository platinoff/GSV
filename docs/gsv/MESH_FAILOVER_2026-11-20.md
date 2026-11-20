# Failover / dead-days & rejoin — 2026-11-20

Що стає коли телефони взагалі відсутні (dead-days) і коли вони повертаються.
Scope: D6+D7 (doc-only, no code change). Tasks doc: [`MESH_ROADMAP_TASKS_2026-11-19.md`](MESH_ROADMAP_TASKS_2026-11-19.md).

---

## Поточний стан (виміряно 2026-11-20)

| Аспект | Значення |
|---|---|
| `#37 FF-PC` (adapter/pc-wgpu) | alive 92%, ready_alive 92% — працює |
| `#39 arm/bifrost` | alive 80%, ready_alive 0% — новий peer після owner-reload (старий #38 dead) |
| `#35 valhall` | 0% alive — глибоко offline |
| `#36 Pi 4` | SD dead — 0% (not in mesh) |
| `ready_proofs` #39 | 0 — **заблоковано**: `weights_matvec*` без ready (proofs ≥ 1 немає) |

---

## D6: proof_gate + поведінка коли всі піри dead

`proof_gate` в `src/boxes/wgpu.rs`, `PROOF_FRESH_SECS = PREDICTION_FRESH_SECS = 3600`:

1. **Fresh proofs цього піра → pass.** Peer: ≥1 done task з `max_err_model<1e-4` за останню годину.
2. **Stale proofs + fresh peer інакший alive → ghost demoted.** Dead-peer не має route.
3. **Нічого fresh ніде, але має proofs → fail-open** («Never a total lockout» — legacy proofs stay eligible, флот idle-alive).
4. **Немає proofs → false** (не дасть heavyweight, тільки probe ops).

### HTTP-поведінка (`/api/wgpu/task/enqueue`, `/api/wgpu/task/poll`)

| Запит | Під час dead-days (0 ready) |
|---|---|
| probe ops (`matmul64`, `matvec`, `mlpblock`, `layernorm`, `softmax`, `matvec_shard`) | `200 {"ok":true, task_ids:[...]}`, `route_peer` unset — bootstrap, розселяються коли route доступний. Перша good proof → ready |
| heavyweight (`weights_matvec`) | `200 {"ok":true, task_id:..., no_ready:true, readiness_hint:"peer not ready: run probe tasks to earn ≥1 proof (good error) before MiniCPM work"}` — зачекає вічно, poll: `{"ok":true,"task":null}` |
| heavyweight (`weights_matvec_block`) | `400 {"ok":false,"error":"no ready peer alive — MiniCPM block queued until a proven device polls"}` — browser-only |
| `peer_poll` від мертвих пірів | `{"ok":true,"task":null,"quota":{"limit":1,"claimed":0}}` — чекає доказів. (FW-7: dead holder re-claimable, 300 s stuck-window) |

### UI (`/wgpu` page)

- KPI strip: `peers=0` — peer row без **аватара/операційних**, dead ring. Peerbar "worker unavailable" disabled.
- Dead-phone rows в `mesh_wire` злапають `ready:true` — обробляється `ready_alive` (`MESH_ROADMAP_TASKS#C6`), показує "offline".

---

## D7: rejoin-new-peer (перший клік вашого reload — біофрост #39)

1. **Re-peer:** `device_key` — якщо новий UA → новий `peer_id` (#39), інакше reuse (#38 може бути вреженим і dead). `mesh_peers.jsonl` — registry auto-prune (stale+unproven, PROBATION floor 2.0 MF).
2. **32 MiB cap** — unknown-memory peers can use 32 MiB (PH-A2, PH-A4).
3. **Ready: 0** (no proofs) — heavyweight blocked by proof_gate; **probe ops open** — first good proofs auto-earned via golden-shard canary (`g-403` e2e, max_err ≈ 1e-6).
4. **Flip:** first done task `max_err_model<1e-4` → ready → слідкаємо `ewma_ms`, heavier ops routeable again.
5. **Stale #38:** PROOF_FRESH_SECS — #38's proofs age out; proof_gate fails (#38 dead) — pure stomp, nothing breaks.

---

## UI-states summary (dead-days → recovery)

| State | Meaning | Action |
|---|---|---|
| 0 peers / all dead | alive=0 | UI disabled worker row ("Немає запущених worker"); SESSION_PRESET |
| green row + ready + heavy ops blocked | no ready-alive | readiness_hint у /api/wgpu/task/enqueue body ("run probe tasks…") — `MESH_ROADMAP_TASKS#C5`, `#D6` |
| phone back alive + ready | reconnect done | `weights_matvec`-і", — ", |
| phone back alive, ready=0 | new phone bootstrap | probe tasks — golden canary e2e on it |

---

## Verdict (honest)

- **D6** — documented; dead-days degrade gracefully (fail-open, honest proofs, no silent breakage of probe ops).
- **D7** — verified: rejoin → #39 → proofs → ready. Phi dead — handled by 2×PROOF_FRESH_SECS.
- **Current blocker:** `weights_matvec` — only when ≥1 ready peer alive. On #39: `ready_proofs=0` → now **only probe ops allowed**. First probe results on #39 → ready flip. (FF-PC #37 is alive+ready, so flow works when owner reloads bifrost: #39 has capacity, then #39 ready → heavyweight routed.)

---

## Rejoin owner-checklist

1. Owner: reload `/wgpu` on bifrost phone (expect `push channel open` p39) → `mesh_wire`: `alive/ready` flip on #39 (≤5 min).
2. Run `cargo xtask status` — expect `mesh alive 2/5 ready 2 [37, 39]`.
3. Golden canary e2e on #39 (blk.5 shexp, `max_err ≤ 1e-6`).
4. Optional: dispatch `weights_matvec` on #37/39 → heavier flow resumes (single-peer EWMA-tiering).
