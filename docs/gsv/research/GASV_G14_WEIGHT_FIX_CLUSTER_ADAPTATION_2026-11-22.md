# GASV-G14 — Weight-fix / cluster / plan adaptation (2026-11-22)

**Status:** research + plan adaptation done (docs-only). **No weights changed. No full-LLM claim.**
**Product:** gsv (host) + rusty-engine (plugin). **Owner ask:** переглянути задачі/стани/плани; адаптувати плани під виправлення ваг кластером за правилами GSV/WGPU/NanoJev/poolAI/плагінів; 3 моди кластера; ваги «як Астра»; невеликий vision-додаток теж зі зміною ваг; прибрати Perl/Python з процесу; після адаптації —git push + самарі, далі тікетами.

---

## 1. Steady state (fact-checked 2026-11-22)

| Item | State | Evidence |
|---|---|---|
| Cluster | **Valhall (#40) забрав owner** → активний **Min**: hub RDNA2 WGPU + bifrost #39. Med (#40) і Max (#36 Pi4) — spec/gated. | owner msg 2026-11-22; `/api/wgpu/mesh` |
| Hub | `gsv-server` 0.248.0, `:9999`, watchdog alive, version_lag=false, disk 43G free | `gsv_health`/`gsv_disk`/`gsv_watchdog` |
| G9 artifact | `gsv-versus-ai-full-q4k.gguf` 733/733 tensors, 21,166,255,488 B, sha256 `2f37d609…`, direct-copy Q4_K/Q6_K source | `GASV_G9_FULL_Q4K_ASSEMBLY_2026-11-21.md` |
| Canonical publish | **G2b F32 502 tensors** `25fadfbb…` (partial pack). Full-q4k **не** в publish-set (owner-gated) | `gsv-versus-ai.publish.json` |
| G11 fitness | `g-711` blk.3.attn_output (peers 37,40) + `g-712` blk.11.attn_output (peers 39,37) — **PASS, max_err_model=6.94e-18** (<1e-4). Але G11 тікет **reclaimed** (lease expired 2026-11-22 11:59) — scorecard/закриття не доведені. | `wgpu_tasks`/mesh groups; `gsv_tickets` events |
| rusty-engine | **3bb3397 pushed, clean**. NanoJev eval-oracle port тепер у git (був uncommitted). | git log/status |
| Ticket board | G8/G9 done; **G10 todo**, G11 open, G12 todo, G13 blocked | `docs/gsv/tickets.jsonl` |

### 1.1 Чому «iceman не запускав фітнес» — коректна відповідь

Канарейки **реально виконані й PASS** (`g-711`/`g-712`, 6.94e-18). Проблема не в відсутності прогону, а в **незавершеному циклі**: G11-тікет не було закрито (lease сплив → reclaimed), scorecard/report не записано, NanoJev-oracle рядок не зафіксовано, G10 лишився `todo` хоча artifact-report існує. Тобто треба не «перезапускати все», а **закрити fitness-цикл по правилах** (ticket claim → evidence → done) і виправити розсинхрон станів.

---

## 2. Cluster modes (3, адаптовано під повернення/підключення пірів)

| Mode | Peers | Статус 2026-11-22 | Що дозволено |
|---|---|---|---|
| **Min** | hub RDNA2 WGPU (pc-wgpu #37) + bifrost phone #39 | **активний** (Valhall знято) | probe ops, weights_matvec на shexp/attn_output, canaries <1e-4, proof gate, fail-open |
| **Med** | Min + valhall #40 | spec, owner поверне телефон | +паралельні expert shards, throughput-порівняння; не вимагати для gates |
| **Max** | Med + Pi4 #36 | spec/gated (Pi dead/remote owner-gated) | документувати; не dispatch без alive+proof |

Правило: **gate proruns у Min**; Med/Max — лише прискорення/масштабування, не зміна критеріїв якості. Якщо через WGPU в іншій мережі підключиться більше пірів — вони потрапляють у Med/Max за тим самим proof/readiness контрактом, без послаблення gate.

---

## 3. Weight-fix toolchain — що реально є (ресерч)

### 3.1 Astra-подібний механізм уже в Rust (`rusty-engine`)

| Компонент | Що робить | Файл |
|---|---|---|
| `rusty audit -m <model> -p "text"` | activation audit: per-layer `entry_rms_mean`, `delta_rms_mean`, `delta_rms_max` — де «застрягає» сигнал | `src/bin/rusty.rs` `audit_cmd` |
| `rusty pack-mint -m <model> -t <tensor> (-b <idx>\|--all) [--zero]` | WSP-pack: identity receipt або zero-block ablation; **`base_hash` fail-closed** | `src/bin/rusty.rs` `pack_mint_cmd`, `src/patch.rs` |
| `rusty serve [--patch <wsp>]` | staged `patched_<pid>.gguf`, оригінал не чіпається | `src/bin/rusty.rs` `serve_cmd` |
| `patch.rs` | block-bitmap patch-at-dequant: **Q8_0 32/34B, Q4_K 256/144B, Q5_K 256/176B, Q6_K 256/210B**; `parse_wsp`, `stage_patch`, `validate_pack_against_file`, `patch_tensor_blocks`, `dequant_with_patches` | `src/patch.rs` |
| Tests | SHA vectors, q8_0/q4_k/q6_k patch roundtrip, wrong-base-hash rejected, out-of-range/len-mismatch/overlap rejected, real-model identity smokes | `src/patch.rs` tests L827–1233 |

WSP pack format: `<tensor> <block_idx> <hex-block-bits> <base_hash_sha256>\n`. `base_hash` = SHA-256 **оригінального** блоку; розбіжність → fail-closed. Router/gate ваги — **freeze never patch**.

### 3.2 NanoJev — eval/oracle, не генератор ваг

`rusty nanojev {predict,replay,debug-leaf,debug-tensor,inspect}` на `best.safetensors` (2.3GB, Qwen3-0.6B DecisionModel, 322 tensors). Дає `top1_agreement_rate`, `max/mean_abs_logit_delta`,_tensor-дані debug_. **Не вміє** GGUF writer, transplant, patch arbitrary GGUF. Роль: score/gate/prioritize кандидатів. (Commit `3bb3397`, тепер у git.)

### 3.3 Вердикти Astra/WSP (чому «просто взяти найкращі ваги» не працює)

- **Astra Exp. 8:** Qwen3.5-0.8B, 64 output columns у MLP L15/L19 = **131,072 ваги (0.016%)** → MATH-500 7.9→48.1, GSM8K 13.8→67.1, IFEval 21→62.3. Але **Gemma 3 270M не покращився** → закон: *«редагування ваг може прибрати перешкоду, що ховала здатність, але не додає здатності, якої модель не має»*.
- **Наш MiniCPM-1B:** zero 1 блоку — нема ефекту; zero **всіх 8192 блоків** `blk.23.attn_output.weight` — декодування змінилось, жоден факт не розблокувався. 2/2 trials ROLLBACK. Напрям «single-direction/few-column» **вичерпано**.
- **Кандидати далі:** Granite-A800M (MoE, audit-only), 35B shared-expert (audit-gated), **1B dense qwen35 = кандидат №1** на obstacle-unlock. Router/gate — freeze.
- **Висновок:** «вбрати найкращі ваги з моделей у rusty-engine» можна лише як **аудитований, сумісний transplant** (manifest + shape/dtype/semantics + checksum), а не довільне злиття. Astra-подібний patch — це **прибрати перешкоду**, не «додати знання».

---

## 4. Vision addon — окремий трек (не тихо ігнорувати, не тихо патчити)

Owner хоче невеликий vision-додаток теж зі зміною ваг. Факти:

- `mmproj-*.gguf` (610M/1.1G/1.1G) = arch `clip`, 548M, **all BF16 (ggml=30), 460 tensors**: `v.blk.0..26`, `v.patch_embd`, `v.position_embd [1152×4900]`, `v.vit_merger.*`, `mm.{input_norm,up,down}`.
- `patch.rs` **не підтримує BF16/F16/F32** для block-patch (лише Q8_0/Q4_K/Q5_K/Q6_K). Отже прямий Astra-патч mmproj **неможливий без нового flexible-type patch lane**.
- Попередній research (`WSP_JEV_ASTRA_RESEARCH` placement L173–184) мав `mmproj (vision) = ❌ not patched`. Owner-запит **суперечить** цьому вердикту → не перезаписуємо мовчки, а відкриваємо **новий feasibility-тікет** (G18) з явними gates.
- B12/B13 (`PH-1202..1206`) — ViT/merger/image-prefix/multimodal serve на qwen35; voxel-патч має йти **після** B13 text core, інакше нема оракула для перевірки.

---

## 5. Адаптація планів (що змінюється)

1. `2026-11-21-gsv-versus-ai-wgpu-cluster-skeleton.md`: статус → «G9/G10 artifact built, G11 canary PASS but ticket reclaimed»; cluster set — активний **Min**, Med/Max gated; ** жодного Perl/Python**; G13 лишається blocked.
2. `2026-11-21-gsv-full-4bit-moe-cluster-plan.md`: прибрати рядок «simple perl file» (був джерелом порушення); G9/G10 статуси звірити; додати Phase G14 (weight-fix) і G15 (vision) як **research-first, owner-gated**.
3. `GASV_G11_...` draft → записати факти g-711/g-712 PASS + reclaim root cause; scorecard шаблон.
4. `GASV_G12_...` draft → Min-only активний, Med/Max spec.
5. Усі нові інструменти/скрипти — **Rust (`cargo xtask`/`rusty`) + MSYS2 bash + MCP GSV tools**. Perl/Python у project work заборонені (timeline tools — теж: для JSONЛ правок використовувати MCP `gsv_tickets_*` або Rust-утиліти).

---

## 6. Ticket roadmap (create via `gsv_tickets_create`, не Perl-patches)

| Ticket | Title | Depends | Owner gate |
|---|---|---|---|
| G14 (this) | Research/plan adaptation + rusty-engine NanoJev commit | — | no (docs) |
| G15 | G11 fitness scorecard finalize + ticket close (evidence g-711/g-712, NanoJev oracle line) | G14 | no |
| G16 | G10/G12 status reconcile + Min placement manifest (hub #37 + bifrost #39) | G15 | canary dispatch ok |
| G17 | Astra-style candidate selection: `rusty audit` на 1B dense qwen35 + Granite A800M; pick obstacle-vs-ability class | G16 | no (audit-only) |
| G18 | Vision/mmproj weight-change feasibility (BF16→patchable lane or reject with evidence); B13 gate | G17 | **owner** (суперечить попередньому ❌ вердикту) |
| G19 | Staged Q4_K patch candidate: `pack-mint → serve --patch` → NanoJev replay + Min canary; accept/reject scorecard | G17 | publish owner-gated |
| G13 | Runtime infer/quality gate — залишається **blocked** | G19 | owner runtime/RAM node |

---

## 7. Definition of done (G14)

- [x] Research: weight-fix toolchain + Astra verdicts + vision feasibility + cluster modes — з citations.
- [x] Plans адаптовано (status/stale/perf, без Perl/Python).
- [x] rusty-engine NanoJev port committed+pushed (`3bb3397`), gate green.
- [x] GSV docs commit + push + самарі HEADs.
- [ ] Tickets G15–G19 створені через MCP (далі — iceman підхоплює).
- [ ] G13 лишається blocked (чесний full-LLM boundary).
