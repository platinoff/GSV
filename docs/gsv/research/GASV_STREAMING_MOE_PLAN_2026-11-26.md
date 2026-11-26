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

1. **Huge-expert числова коректність.** Живий канар `g-7360` на `blk.0.ffn_gate_exps.weight`
   дав `max_err 795.8` на 8/8 шарів — peer-обчислення для одногоhot-вектора хибне.
   Знайдена причина: WASM Q4_K kernel мав субнормальний f16 шлях з е=−10 замість bias-1
   (GSV `f16_to_f32` починає e=113), через що `d`/`min` scaling роздувався ~2^14.
   Виправлено в `crates/mesh-wasm`, додано golden-тест проти GSV `dequant_q4_k`;
   **потрібен reload `/wgpu` на пірах**, щоб підхопили новий `.wasm`.
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
- **BENCH-2** — після reload пірів: sweep shards 2/4/8 × gate/up/down expert, записати
  `max_err`, MFLOPS, роутинг, `/proc` RSS у `data/moe_bench.jsonl`; вибрати конфіг.
- **BENCH-3** — чесний факт: partial pack `gsv-versus-ai-g2b-q4_0.gguf` не запускний
  (`missing required key 'gsv.attention.head_count'`); повний 4-квант треба stream-ити.
- **STREAM-1** — цей документ + перший可行ний slice після стабілізації numbers.

**Висновок:** головний блокер — **не RAM і не bandwidth, а відсутній expert-aware forward
оркестратор** + тимчасова числова помилка на huge-expert шарді. Range-read із дзеркала вже
вирішує «21 ГБ > 7.7 ГБ», як і передбачав owner.
