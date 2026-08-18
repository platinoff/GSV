# OmniRouter shared catalog (Rust + web)

**Status:** Band **157** · **Date:** 2026-08-18  
**Wire:** `GET /api/omni` · `GET /api/omni/route?task=rust|web&prefer_free=true`  
**MCP:** `gsv_omni_route` · `gsv_omni_chat` (empty `model` auto-picks) · resource `gsv://docs/omni-catalog`  
**Code:** `src/boxes/omni/catalog.rs` + `quota.rs` (durable `data/omni_quota.json`, never git)

Shared notes for **OmniRouter**, **Cursor**, **OpenCode**, and **Grok** so a coding agent can pick a Rust/web model and **switch host when a free-tier timer is cooling**.

## How MCP auto-switch works

1. Catalog publishes `quota.reset_secs` (rolling window) and optional `rpm` / `rpd` / `daily_reset_secs`.
2. A live `429` (or RPM burst) writes `cooldown_until` into `data/omni_quota.json`.
3. `gsv_omni_route` / empty-model `gsv_omni_chat` skip cooling providers until `cooldown_secs == 0`.
4. Free chain (default): `groq` → `openrouter` → `nvidia` → `cerebras` → `huggingface` → `google` (Flash) → `opencode-zen`.
5. Honor upstream `Retry-After` when present.

Owner explicit `X-Omni-Provider` still wins (bypass cooldown).

## Recommended for this repo (Rust + web)

| Model | Provider | Clients | Why |
|-------|----------|---------|-----|
| **Grok 4.6** | xAI / Cursor pool | omni, cursor, opencode, grok | Agent + Rust; 500K API ctx (Cursor UI 256K) |
| **GPT-5.2 Codex** | OpenAI | omni, cursor, opencode | Rust/code |
| **Claude Sonnet 4.6** | Anthropic | omni, cursor, opencode | Agent + web UI |
| **Gemini 3 Pro** | Google | omni, cursor, opencode | 1M ctx docs/web |
| **Kimi K2.7 Code** | Moonshot | omni, cursor, opencode | Cheap Rust + frontend |
| **GPT-5.3 Codex** | OpenAI / Zen | omni, cursor, opencode | Newer Codex on OpenCode Zen |

Cursor-only speed lane: **Composer 2.5** (Cursor Models monthly pool). Not an Omni upstream unless you configure a base URL.

## Clients

| Client | Kind | Timer | Rust picks | Web picks | Free |
|--------|------|-------|------------|-----------|------|
| **omni** | proxy | per-provider `reset_secs` | Grok 4.6, Codex, Kimi K2.7, Qwen Coder | Grok 4.6, Sonnet 4.6, Gemini 3 Pro | Groq / OpenRouter `:free` / NIM / Zen pickle |
| **cursor** | IDE | **monthly billing cycle** (two pools) | Grok 4.6, Composer 2.5, Codex, Sonnet 4.6 | same + Gemini 3 Pro / Opus 4.6 | none (Pro Other Models ≥$20) |
| **opencode** | IDE | Zen free anti-abuse 429; Go monthly caps | GPT-5.3 Codex, Grok 4.6, Kimi K2.7 | Sonnet 4.6, Gemini 3.1 Pro, MiniMax M2.7 | Big Pickle, DeepSeek V4 Flash Free, Nemotron 3 Ultra Free |
| **grok** | CLI/bot | paid xAI RPS/TPM (T0 Grok 4.6 ~150 RPS / 50M TPM) | Grok 4.6, Grok Build 0.1 | Grok 4.6 / 4.5 | use Omni free chain via MCP |

Cursor **Cursor Models** pool: Grok 4.6, Grok 4.5, Composer 2.5. **Other Models** pool is token-priced (Pro includes ≥$20/mo).

## Free-tier timers (Omni upstreams)

Do not treat these as SLAs — they move. Numbers below are what the catalog encodes for switching.

| Provider | Free? | RPM | RPD / other | `reset_secs` | Notes |
|----------|-------|-----|-------------|--------------|-------|
| OpenRouter `:free` | yes | 20 | 50/day (1000 after $10 lifetime credits) | 60 | daily window 86400s |
| Groq | yes | ~30 | ~1000/day; TPM 6–30K | 60 | org-level; `x-ratelimit-reset-tokens` often seconds |
| NVIDIA NIM | yes | ~40/model | no published daily cap | 60 | traffic-dependent |
| Cerebras | yes | ~5 | ~1M tokens/day | 60 | 8K ctx on some free rows; catalog no longer assumes Qwen3 Coder 480B |
| Hugging Face | yes | — | $0.10/mo credits | 60 (429) | monthly 2_592_000s |
| Google Flash | yes | ~10 | ~1500/day | 60 | Pro is paid; project quotas vary |
| OpenCode Zen free | yes | ~20 | anti-abuse | 60 | `User-Agent: opencode/`; 429 = switch |
| GitHub Copilot Free | yes | — | 2000 completions/mo | monthly UTC day-1 | not an Omni default base_url |
| Cursor | no (included pool) | — | monthly billing | monthly | not an Omni upstream |

Paid APIs (OpenAI, Anthropic, xAI, DeepSeek, Moonshot, MiniMax, Qwen, Z.AI): `reset_secs=60` only to honor `Retry-After`.

## Sources (2026-08-18)

- [Cursor models & pricing](https://cursor.com/docs/models-and-pricing) — Grok 4.6 / Composer 2.5 pools
- [OpenCode Zen](https://opencode.ai/docs/zen/) — Codex / Claude 4.6 / Grok 4.6 / free pickle & Nemotron
- [xAI Grok 4.6](https://docs.x.ai/developers/models/grok-4.6) — 500K ctx, coding+agent
- [OpenRouter limits](https://openrouter.ai/docs/api/reference/limits) — `:free` 20 RPM / 50–1000 RPD
- [Groq rate limits](https://console.groq.com/docs/rate-limits)
- [Cerebras rate limits](https://inference-docs.cerebras.ai/support/rate-limits)
- [Hugging Face Inference Providers pricing](https://huggingface.co/docs/inference-providers/en/pricing)
- [GitHub Copilot usage-based billing](https://docs.github.com/en/copilot/concepts/billing/usage-based-billing-for-individuals)

Canon code stays in Rust (`catalog.rs`). This file is the human/MCP snapshot of the same data.
