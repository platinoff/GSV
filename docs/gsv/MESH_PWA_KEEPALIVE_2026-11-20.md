# PWA keep-alive — bifrost #39, 2.4 GHz (2026-11-20)

**Why:** телефон-PWA `/wgpu` — якщо Chrome не відпустить конекти процессора (Doze / Battery Saver / "optimized"), bifrost бачить `worker: hub unreachable` retries й все стирає. (10 unreachable/16 min observed in the phone console, 2026-11-19.)

B3-верифікація live (2026-11-20): #39 = `push channel open` + свіжі telemetry → OK, ready_alive 0% (proofs 0 — on grounds of design).

## 1. Android → Battery → Apps → Chrome → "Unrestricted"
(Sets it for ALL chrome tabs, not just this one; do not rely on per-tab adaptive battery — it will throttle you.)

## 2. Keep Screen on for the PWA
Settings → Display → Screen timeout: set "Never" (or long) for the Chrome PWA; during soak (24h), keep phone charged + AC.

## 3. Wi-Fi-HAC check (2.4 GHz + congestion on this AP)
After 30 min soak: `curl -s http://127.0.0.1:9999/api/wgpu/console` — expect `push channel open` for #39 only (stale retries should not grow).

## 4. Client-side (for the account)
In `/wgpu` page JS (`connectPush`, `ws.onclose`): add a debounced retry with 20s base instead of instant reconnect (currently 5 s), backoff ×2..4. — If owner wants this client-side fix (one-liner: `setTimeout(connectPush, 20000 + attempts*5000)` in ws.onclose + retry counter), it's on a separate task.

---

**B3 final:** 
1. Steps 1-2 (30 min), then verify via curl: expect `push channel open p39` (no new `worker: hub unreachable` for the day).
2. If still throttling — the `--verbose` push config (page): not needed; hub push is 2s, fine.

**Status at 2026-11-20 04:2x:** #39 bifrost: `push channel open` (fresh, ~03:40). ready_alive 0%, proofs 0. (see MESH_FAILOVER_2026-11-20, MESH_ROADMAP_TASKS#B7-live)
