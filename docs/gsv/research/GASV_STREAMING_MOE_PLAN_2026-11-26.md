# Streaming MoE plan — запуск gsv-versus-ai 4-квант на наявному залізі (2026-11-26)

## 1. Оцінка доступного заліза й стану

- Хаб: 7 560 МБ RAM (~1.7 ГБ вільно), 44 ГБ на S:, 21 ГБ артефакт у дзеркалі L:.
- Піри (live): `#43` PC RDNA2 (12 cores, `maxStorageBufferBindingSize=2048 MB`, ~100 MFLOP/s),
  `#39` bifrost phone (8 cores, 4 ГБ `deviceMemory`, 256 MB storage binding), `#44` valhall
  періодично dead. Отже **реально 2 піри**, не три.
- Артефакт: 733/733 тензори, `qwen35moe`, 40 шарів, embed 2048, 256 експертів / top-8,
  expert_ff 512; router `[2048,256]` ≈ 2 МБ/шар (80 МБ загалом), shared expert ≈ 2 МБ/шар,
  один routed expert gate/up/down ≈ 0.6–0.9 МБ; top-8 × 40 шарів ≈ **0.6 ГБ унікальних
  байтів експертів на токен**, + `output.weight` Q6_K ≈ 417 МБ/токен.

## 2. Що вже є (не треба писати з нуля)

- `weight_store::read_chunk_artifact` — range-read із 21 ГБ GGUF + sha256, fail-closed.
- `wgpu_dequant::{dequant_q4_k_range,dequant_q6_k_range}` — потокова дейквантизація.
- WGPU `weights_matvec` / `weights_matvec_block`, `x_spec=onehot:i:n`, result bus
  `GET /api/wgpu/task/result?group=`, proof_gate з `max_err_model`.
- `vision_forward.rs` — готовий патерн оркестрації: хаб тримає малі стани, великі matvec
  ідуть на піри, partial-и повертаються й перевіряються; checkpoint/resume.
- `rusty-engine/src/moe.rs` — `route_noaux_tc`, `expert_ffn`, `combine`, `ExpertCache`
  (поки unit-tested на synthetic F32, не зв'язаний із GGUF).

## 3. Що бракує (за пріоритетом)

1. **Huge-expert числова коректність — ВИПРАВЛЕНО Й ПІДТВЕРДЖЕНО LIVE.** Канар `g-7360`
   на `blk.0.ffn_gate_exps.weight` дав `max_err 795.8` на 8/8 — не зсув блоку (як у §3),
   а субнормальний f16 у WASM `mesh_dequant_q4_k`: `e = exp = 0` замість канонічного
   `e = 113` (bias-1), через що `d`/`min` scaling роздувався ~2^14. Виправлено з
   golden-тестом проти GSV `dequant_q4_k`. Повторний канар **`g-7362`: 8/8 shard,
   `ok:true`, `max_err_model 1.73e-18 … 3.47e-18`**; compute-time throughput
   **131.75 MFLOPS**, усі шарди на `#43`, wall ~282 c через послідовний
   `concurrent_limit=1` і очікування claim. `.wasm` 27 462 B задеплоєно з `no-store`
   (md5 `c6bb36f1…`), піри підхопили новий код після reload `/wgpu`.
2. **Per-expert адресація.** `rusty artifact-chunks` вміє лише цілий тензор; треба
   `--expert N` / byte-range, інакше range-read читає 4 МБ chunkи замість 0.6–0.9 МБ експерта.
3. **Compact x у `weights_matvec_block`** (зараз повні x-масиви, 256 КБ body cap).
4. **Streaming qwen35moe forward**: router → top-8 → expert matvec → combine → attention/GDN/norm
   на хабі → logits. Орієнтовно 1 500–2 200 LOC; 40 шарів, context cap 4K–8K (KV ~100 МБ).
5. **Hot-expert LRU** (256–512 МБ, shared expert pinned) і KV/state budget guard.

## 4. Що реально по продуктивності

- llama-rs вимірював Qwen3-30B-A3B IQ2_XXS (9.65 ГБ) на 7.4 ГБ RAM: **0.497 tok/s**,
  TTFT 298.5 c — expert slices перечитувались з SSD.
- Триярусний expert cache (16 ГБ VRAM + 12 ГБ RAM) дає 50→80 t/s, але це
  зовсім інше залізо; наші телефони мають 256 МБ storage binding.
- Наш conjecture: ~1.5–3 c/token best case по bandwidth + ~10–15 c/token по compute
  на 2 пірах = **<1 tok/s**. Це повільно, але не неможливо; без G13 жодного quality-claim.

## 5. План за тікетами

- **BENCH-1** (harness `cargo xtask moe-bench`) — зроблено, unit/contract green.
- **BENCH-2** — sweep **gate/up/down при `shards=8`** (не 2/4: хабний квотний ліміт
  67 108 864 FLOP/шард відхиляє 2/4 для тензора 2048×131072; перевірено unit-тестом
  `huge_expert_needs_eight_shards_under_hub_quota`). Харнес має `--group <id>` для
  resume завершених груп без повторного enqueue; `mflops` рахується з compute-time
  (`elapsed_ms` шардів), а не з wall/queue часу. Результати → `data/moe_bench.jsonl`.
- **BENCH-3** — чесний факт: partial pack `gsv-versus-ai-g2b-q4_0.gguf` не запускний
  (`missing required key 'gsv.attention.head_count'`); повний 4-квант треба stream-ити.
- **STREAM-1** — цей документ + перший可行ний slice після стабілізації numbers.

### Результати BENCH-2 (2026-11-26, `data/moe_bench.jsonl`)

| Конфіг | Група | Результат | compute MFLOPS | wall (claim+queue) | max_err_model |
|---|---|---|---|---|---|
| `gate` `blk.0.ffn_gate_exps.weight` 2048×131072, 8 shard | `g-7363` | PASS 8/8 | **130.98** (4.099 c) | 279 c | 3.47e-18 |
| `up` `blk.0.ffn_up_exps.weight` 2048×131072, 8 shard | `g-7364` | PASS 8/8 | **187.06** (2.87 c) | 270 c | 3.47e-18 |
| `down` `blk.0.ffn_down_exps.weight` 512×524288 (Q6_K), 8 shard | `g-7365`/`g-7366` | FAIL → root cause знайдено | — | — | пір `max_err=0`, але `task result post FAILED after 3 attempts`; задача лишалась `claimed` і повторювалась рівно через **301 c** |

**Root cause down-петлі:** `TASK_CLAIM_REQUEUE_SECS = 300` у `src/boxes/wgpu.rs`. Шард Q6_K
down — ~220 МБ; пір стрімить його з дзеркала довше, ніж 300 c, хаб перечерговує
задачу (`stuck-claim`), і пізній POST результату приходить уже не до власника
→ HTTP 400 → 3 ретраї → вічний цикл. Виправлено: для широких тензорів
(`cols > 200_000`) TTL = **1200 c**; TDD-тест `huge_expert_shard_gets_a_longer_claim_lease`
(600-c claim лишається активним для wide і requeue-иться для narrow). Додано логування
HTTP-статусу й тіла відмови в `postResultWithRetry` (`task result POST rejected: HTTP …`).

Висновок по продуктивності: чистий compute на `#43` — **131–187 MFLOPS** (тобто ~0.5–0.7 с на
один expert gate/up matvec), але **wall ~270–280 c** через послідовний `concurrent_limit=1`
і повільний claim/queue цикл. Отже головний резерв тюнінгу — не SIMD-ядро, а **оркестрація
черги та конфігурація concurrent limit**, а також надійність POST результату (для `down`).
Жодного quality-claim.

**Висновок:** головний блокер — **не RAM і не bandwidth, а відсутній expert-aware forward
оркестратор** + тимчасова числова помилка на huge-expert шарді. Range-read із дзеркала вже
вирішує «21 ГБ > 7.7 ГБ», як і передбачав owner.
