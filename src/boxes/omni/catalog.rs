//! OmniRouter catalog — providers, models, clients, quota timers.
//!
//! Researched **2026-08-18** for Rust + web development across OmniRouter,
//! Cursor, OpenCode, and Grok (xAI). Token windows from vendor docs / the
//! OpenCode sheet; free-tier RPM/RPD/reset windows from official limit pages
//! (OpenRouter, Groq, Cerebras, NVIDIA NIM, Hugging Face, Copilot, Cursor).
//! `None` means the sheet/docs do not publish a number — do not invent one.

/// Shared research stamp (MCP + Galaxy card).
pub const RESEARCHED_AT: &str = "2026-08-18";

/// Rate-limit window used for MCP auto-switch after 429 / RPM exhaustion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuotaSpec {
    pub rpm: Option<u32>,
    pub rpd: Option<u32>,
    pub tpm: Option<u32>,
    /// Rolling window (seconds) the router waits after a 429.
    pub reset_secs: u32,
    /// Daily/monthly window (seconds); 0 = none.
    pub daily_reset_secs: u32,
    pub notes: &'static str,
}

pub const QUOTA_PAID: QuotaSpec = QuotaSpec {
    rpm: None,
    rpd: None,
    tpm: None,
    reset_secs: 60,
    daily_reset_secs: 0,
    notes: "paid / no published free cap — 429 still honors Retry-After (default 60s)",
};

const fn q_rpm(rpm: u32, rpd: u32, notes: &'static str) -> QuotaSpec {
    QuotaSpec {
        rpm: Some(rpm),
        rpd: Some(rpd),
        tpm: None,
        reset_secs: 60,
        daily_reset_secs: 86_400,
        notes,
    }
}

const fn q_month(notes: &'static str) -> QuotaSpec {
    QuotaSpec {
        rpm: None,
        rpd: None,
        tpm: None,
        reset_secs: 60,
        daily_reset_secs: 2_592_000,
        notes,
    }
}

/// A provider (API vendor / aggregator).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub region: &'static str,
    pub free: bool,
    pub default_base_url: &'static str,
    pub notes: &'static str,
    pub quota: QuotaSpec,
}

/// A single model entry. Same `id` may appear on several hosts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub provider: &'static str,
    pub context_window: Option<u32>,
    pub max_output: Option<u32>,
    pub free: bool,
    pub recommended: bool,
    pub tier: &'static str,
    /// Strong on Rust (borrow checker, cargo, unsafe, macros).
    pub rust: bool,
    /// Strong on HTML/CSS/JS, REST, UI glue.
    pub web: bool,
    /// Native clients: `omni` · `cursor` · `opencode` · `grok`.
    pub clients: &'static [&'static str],
}

/// IDE / CLI that consumes the shared catalog (not always an Omni upstream).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: &'static str,
    pub quota: QuotaSpec,
    pub notes: &'static str,
    pub rust_models: &'static [&'static str],
    pub web_models: &'static [&'static str],
    pub free_models: &'static [&'static str],
}

const C_ALL: &[&str] = &["omni", "cursor", "opencode", "grok"];
const C_NO_GROK: &[&str] = &["omni", "cursor", "opencode"];
const C_OMNI_OC: &[&str] = &["omni", "opencode"];
const C_CURSOR: &[&str] = &["cursor"];
const C_GROK: &[&str] = &["omni", "cursor", "opencode", "grok"];
const C_OMNI: &[&str] = &["omni"];

#[allow(clippy::too_many_arguments)]
const fn m(
    id: &'static str,
    name: &'static str,
    provider: &'static str,
    context_window: Option<u32>,
    max_output: Option<u32>,
    free: bool,
    recommended: bool,
    tier: &'static str,
    rust: bool,
    web: bool,
    clients: &'static [&'static str],
) -> ModelSpec {
    ModelSpec {
        id,
        name,
        provider,
        context_window,
        max_output,
        free,
        recommended,
        tier,
        rust,
        web,
        clients,
    }
}

/// Catalog of known providers.
pub fn providers() -> &'static [ProviderSpec] {
    static PROVIDERS: &[ProviderSpec] = &[
        ProviderSpec {
            id: "openai",
            name: "OpenAI",
            region: "Global",
            free: false,
            default_base_url: "https://api.openai.com/v1",
            notes: "GPT-5.2 / 5.2-Codex / 5.3-Codex · 400K ctx / 128K out",
            quota: QUOTA_PAID,
        },
        ProviderSpec {
            id: "xai",
            name: "xAI (Grok)",
            region: "Global",
            free: false,
            default_base_url: "https://api.x.ai/v1",
            notes: "Grok 4.6 (500K ctx, coding+agent) · Grok Build 0.1 (256K) · API paid, T0 ~150 RPS",
            quota: QuotaSpec {
                rpm: None,
                rpd: None,
                tpm: Some(50_000_000),
                reset_secs: 1,
                daily_reset_secs: 0,
                notes: "paid xAI API; Cursor Grok 4.6 uses the Cursor Models monthly pool instead",
            },
        },
        ProviderSpec {
            id: "anthropic",
            name: "Anthropic",
            region: "Global",
            free: false,
            default_base_url: "https://api.anthropic.com/v1",
            notes: "Claude Opus/Sonnet 4.5–4.6 · 200K ctx (1M extended) / 64K out",
            quota: QUOTA_PAID,
        },
        ProviderSpec {
            id: "google",
            name: "Google",
            region: "Global",
            free: true,
            default_base_url: "https://generativelanguage.googleapis.com/v1beta/openai",
            notes: "Gemini 3 Pro paid; Gemini 3 Flash free-tier ~10 RPM / 1500 RPD (project-specific)",
            quota: q_rpm(10, 1_500, "Flash free-tier ~10 RPM / 1500 RPD, reset midnight Pacific; Pro is paid"),
        },
        ProviderSpec {
            id: "minimax",
            name: "MiniMax",
            region: "China",
            free: false,
            default_base_url: "https://api.minimax.io/v1",
            notes: "M2.7 current; M2.1 deprecated on OpenCode Zen (2026-03-15)",
            quota: QUOTA_PAID,
        },
        ProviderSpec {
            id: "deepseek",
            name: "DeepSeek",
            region: "China",
            free: false,
            default_base_url: "https://api.deepseek.com/v1",
            notes: "V4-Pro / V4-Flash · 1M ctx / 384K out (trial credits; peak hours extra)",
            quota: QUOTA_PAID,
        },
        ProviderSpec {
            id: "moonshot",
            name: "Moonshot AI (Kimi)",
            region: "China",
            free: false,
            default_base_url: "https://api.moonshot.cn/v1",
            notes: "Kimi K3 / K2.7 Code · 1M / 256K ctx — strong Rust+frontend",
            quota: QUOTA_PAID,
        },
        ProviderSpec {
            id: "zai",
            name: "Z.AI (Zhipu)",
            region: "China",
            free: false,
            default_base_url: "https://open.bigmodel.cn/api/paas/v4",
            notes: "GLM-5.1 / 5.2 current; GLM-4.6 deprecated on Zen (2026-03-15)",
            quota: QUOTA_PAID,
        },
        ProviderSpec {
            id: "qwen",
            name: "Alibaba (Qwen)",
            region: "China",
            free: false,
            default_base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
            notes: "Qwen3 Coder 480B (Zen deprecated 2026-02-06) · Qwen3.5–3.7 Plus/Max on OpenCode",
            quota: QUOTA_PAID,
        },
        ProviderSpec {
            id: "openrouter",
            name: "OpenRouter",
            region: "Global",
            free: true,
            default_base_url: "https://openrouter.ai/api/v1",
            notes: ":free variants — 20 RPM; 50 RPD (<$10 credits) or 1000 RPD after $10 lifetime",
            quota: q_rpm(20, 50, "free :free 20 RPM + 50 RPD (1000 RPD after $10 lifetime credits); Retry-After on 429"),
        },
        ProviderSpec {
            id: "groq",
            name: "Groq",
            region: "Global",
            free: true,
            default_base_url: "https://api.groq.com/openai/v1",
            notes: "Llama 3.3 70B / Qwen3 / GPT-OSS · typical free 30 RPM / 1K RPD / 6–30K TPM",
            quota: QuotaSpec {
                rpm: Some(30),
                rpd: Some(1_000),
                tpm: Some(6_000),
                reset_secs: 60,
                daily_reset_secs: 86_400,
                notes: "free org-level ~30 RPM / 1K RPD; x-ratelimit-reset-tokens often <10s",
            },
        },
        ProviderSpec {
            id: "cerebras",
            name: "Cerebras",
            region: "Global",
            free: true,
            default_base_url: "https://api.cerebras.ai/v1",
            notes: "gpt-oss-120b / GLM-4.7; free trial ~5 RPM / 30K TPM / 1M TPD (not Qwen3 Coder 480B)",
            quota: QuotaSpec {
                rpm: Some(5),
                rpd: None,
                tpm: Some(30_000),
                reset_secs: 60,
                daily_reset_secs: 86_400,
                notes: "free trial 5 RPM + 1M tokens/day; 8K ctx cap on some free rows",
            },
        },
        ProviderSpec {
            id: "nvidia",
            name: "NVIDIA (build.nvidia.com)",
            region: "Global",
            free: true,
            default_base_url: "https://integrate.api.nvidia.com/v1",
            notes: "Nemotron 3 / GLM / Kimi · ~40 RPM per model, no published daily cap",
            quota: QuotaSpec {
                rpm: Some(40),
                rpd: None,
                tpm: None,
                reset_secs: 60,
                daily_reset_secs: 0,
                notes: "community baseline ~40 RPM/model, traffic-dependent, not an SLA",
            },
        },
        ProviderSpec {
            id: "huggingface",
            name: "Hugging Face",
            region: "Global",
            free: true,
            default_base_url: "https://router.huggingface.co/v1",
            notes: "Inference Providers · $0.10/mo free credits (PRO $2)",
            quota: q_month("$0.10 monthly credits on free accounts; no RPM SLA — 429 → wait reset_secs"),
        },
        ProviderSpec {
            id: "copilot",
            name: "GitHub Copilot",
            region: "Global",
            free: true,
            default_base_url: "",
            notes: "Free: 2000 completions/mo + limited chat; AI Credits reset 00:00 UTC day 1",
            quota: q_month("Copilot Free 2000 completions/mo; credits reset 1st of month 00:00 UTC"),
        },
        ProviderSpec {
            id: "cursor",
            name: "Cursor (IDE pool)",
            region: "Global",
            free: false,
            default_base_url: "",
            notes: "Cursor Models pool: Grok 4.6 / 4.5 / Composer 2.5 — monthly billing reset; Other Models ≥$20/mo on Pro",
            quota: q_month("two pools reset with the Cursor billing cycle (not UTC month)"),
        },
        ProviderSpec {
            id: "opencode-zen",
            name: "OpenCode Zen",
            region: "Global",
            free: true,
            default_base_url: "https://opencode.ai/zen/v1",
            notes: "PAYG + free: Big Pickle, DeepSeek V4 Flash Free, Nemotron 3 Ultra Free (anti-abuse 429)",
            quota: q_rpm(
                20,
                200,
                "free Zen models: User-Agent opencode/; 429 FreeUsageLimitError is a timer gate — switch host",
            ),
        },
        ProviderSpec {
            id: "opencode-go",
            name: "OpenCode Go",
            region: "Global",
            free: false,
            default_base_url: "",
            notes: "$10/mo CN coding models (Qwen/Kimi/MiniMax/GLM); monthly request caps",
            quota: q_month("$10 Go sub — monthly request caps per model (e.g. Qwen3.5 Plus ~50k req/mo)"),
        },
        ProviderSpec {
            id: "302ai",
            name: "302.AI",
            region: "China",
            free: false,
            default_base_url: "https://api.302.ai/v1",
            notes: "Aggregate of Chinese + global models",
            quota: QUOTA_PAID,
        },
    ];
    PROVIDERS
}

/// Shared client notes (Cursor / OpenCode / Grok / OmniRouter).
pub fn clients() -> &'static [ClientSpec] {
    static CLIENTS: &[ClientSpec] = &[
        ClientSpec {
            id: "omni",
            name: "OmniRouter (GSV)",
            kind: "proxy",
            quota: QUOTA_PAID,
            notes: "OpenAI-compatible proxy. MCP gsv_omni_route skips cooling hosts until reset_secs.",
            rust_models: &[
                "grok-4.6",
                "gpt-5.2-codex",
                "gpt-5.3-codex",
                "kimi-k2.7-code",
                "qwen3-coder-480b",
            ],
            web_models: &["grok-4.6", "claude-sonnet-4.6", "gemini-3-pro", "composer-2.5"],
            free_models: &[
                "groq-llama",
                "groq-qwen",
                "openrouter:auto",
                "nemotron",
                "gemini-3-flash",
                "zen-big-pickle",
            ],
        },
        ClientSpec {
            id: "cursor",
            name: "Cursor",
            kind: "ide",
            quota: q_month("Cursor Models pool (Grok 4.6/4.5, Composer 2.5) + Other Models $20 Pro; monthly billing reset"),
            notes: "First-party pool for long Rust/web agent runs. Composer 2.5 for cheap speed. Third-party Codex/Claude from Other Models.",
            rust_models: &[
                "grok-4.6",
                "composer-2.5",
                "gpt-5.2-codex",
                "gpt-5.3-codex",
                "claude-sonnet-4.6",
                "kimi-k2.7-code",
            ],
            web_models: &[
                "grok-4.6",
                "composer-2.5",
                "claude-sonnet-4.6",
                "gemini-3-pro",
                "claude-opus-4.6",
            ],
            free_models: &[],
        },
        ClientSpec {
            id: "opencode",
            name: "OpenCode",
            kind: "ide",
            quota: q_rpm(20, 200, "Zen free models anti-abuse; Go monthly caps; Zen PAYG otherwise"),
            notes: "Zen gateway (gpt-5.3-codex, grok-4.6, Claude 4.6, free pickle/nemotron). Any OpenAI-compatible provider also works, including OmniRouter.",
            rust_models: &[
                "gpt-5.3-codex",
                "gpt-5.2-codex",
                "grok-4.6",
                "kimi-k2.7-code",
                "qwen3.7-plus",
            ],
            web_models: &["claude-sonnet-4.6", "grok-4.6", "gemini-3.1-pro", "minimax-m2.7"],
            free_models: &[
                "zen-big-pickle",
                "zen-deepseek-v4-flash-free",
                "zen-nemotron-3-ultra-free",
            ],
        },
        ClientSpec {
            id: "grok",
            name: "Grok CLI / Grok Bot",
            kind: "cli",
            quota: QUOTA_PAID,
            notes: "xAI API grok-4.6 (500K) or Cursor Grok 4.6 pool. Grok Bot follows Cursor MCP policy — same gsv_mcp_openbot.",
            rust_models: &["grok-4.6", "grok-build-0.1"],
            web_models: &["grok-4.6", "grok-4.5"],
            free_models: &[],
        },
    ];
    CLIENTS
}

/// Full model registry.
pub fn models() -> &'static [ModelSpec] {
    static MODELS: &[ModelSpec] = &[
        // ── Recommended for GSV Rust + web (2026-08-18) ─────────────
        m(
            "grok-4.6",
            "Grok 4.6",
            "xai",
            Some(500_000),
            None,
            false,
            true,
            "flagship",
            true,
            true,
            C_GROK,
        ),
        m(
            "gpt-5.2-codex",
            "GPT-5.2 Codex",
            "openai",
            Some(400_000),
            Some(128_000),
            false,
            true,
            "code",
            true,
            true,
            C_NO_GROK,
        ),
        m(
            "claude-sonnet-4.6",
            "Claude Sonnet 4.6",
            "anthropic",
            Some(200_000),
            Some(64_000),
            false,
            true,
            "agent",
            true,
            true,
            C_NO_GROK,
        ),
        m(
            "gemini-3-pro",
            "Gemini 3 Pro",
            "google",
            Some(1_000_000),
            Some(65_000),
            false,
            true,
            "flagship",
            true,
            true,
            C_NO_GROK,
        ),
        m(
            "kimi-k2.7-code",
            "Kimi K2.7 Code",
            "moonshot",
            Some(256_000),
            Some(32_000),
            false,
            true,
            "code",
            true,
            true,
            C_NO_GROK,
        ),
        m(
            "gpt-5.3-codex",
            "GPT-5.3 Codex",
            "openai",
            Some(400_000),
            Some(128_000),
            false,
            true,
            "code",
            true,
            true,
            C_NO_GROK,
        ),
        // ── Flagships kept from the sheet (not the rust+web top 6) ──
        m(
            "gpt-5.2",
            "GPT-5.2",
            "openai",
            Some(400_000),
            Some(128_000),
            false,
            false,
            "flagship",
            true,
            true,
            C_NO_GROK,
        ),
        m(
            "claude-opus-4.5",
            "Claude Opus 4.5",
            "anthropic",
            Some(200_000),
            Some(64_000),
            false,
            false,
            "flagship",
            true,
            true,
            C_NO_GROK,
        ),
        m(
            "claude-sonnet-4.5",
            "Claude Sonnet 4.5",
            "anthropic",
            Some(200_000),
            Some(64_000),
            false,
            false,
            "agent",
            true,
            true,
            C_NO_GROK,
        ),
        m(
            "claude-opus-4.6",
            "Claude Opus 4.6",
            "anthropic",
            Some(200_000),
            Some(64_000),
            false,
            false,
            "flagship",
            true,
            true,
            C_NO_GROK,
        ),
        m(
            "minimax-m2.1",
            "MiniMax M2.1",
            "minimax",
            Some(1_000_000),
            Some(131_000),
            false,
            false,
            "flagship",
            true,
            true,
            C_OMNI_OC,
        ),
        m(
            "minimax-m2.7",
            "MiniMax M2.7",
            "minimax",
            Some(1_000_000),
            Some(131_000),
            false,
            false,
            "flagship",
            true,
            true,
            C_OMNI_OC,
        ),
        m(
            "composer-2.5",
            "Composer 2.5",
            "cursor",
            None,
            None,
            false,
            false,
            "fast",
            true,
            true,
            C_CURSOR,
        ),
        m(
            "grok-4.5",
            "Grok 4.5",
            "xai",
            Some(500_000),
            None,
            false,
            false,
            "flagship",
            true,
            true,
            C_GROK,
        ),
        m(
            "grok-build-0.1",
            "Grok Build 0.1",
            "xai",
            Some(256_000),
            None,
            false,
            false,
            "code",
            true,
            false,
            C_ALL,
        ),
        m(
            "gemini-3-flash",
            "Gemini 3 Flash",
            "google",
            Some(1_000_000),
            Some(65_000),
            true,
            false,
            "fast",
            true,
            true,
            C_NO_GROK,
        ),
        m(
            "gemini-3.1-pro",
            "Gemini 3.1 Pro",
            "google",
            Some(1_000_000),
            Some(65_000),
            false,
            false,
            "flagship",
            true,
            true,
            C_NO_GROK,
        ),
        // ── Chinese ─────────────────────────────────────────────────
        m(
            "deepseek-v4-pro",
            "DeepSeek V4-Pro",
            "deepseek",
            Some(1_000_000),
            Some(384_000),
            false,
            false,
            "flagship",
            true,
            true,
            C_OMNI_OC,
        ),
        m(
            "deepseek-v4-flash",
            "DeepSeek V4-Flash",
            "deepseek",
            Some(1_000_000),
            Some(384_000),
            false,
            false,
            "fast",
            true,
            true,
            C_OMNI_OC,
        ),
        m(
            "kimi-k3",
            "Kimi K3",
            "moonshot",
            Some(1_000_000),
            Some(128_000),
            false,
            false,
            "flagship",
            true,
            true,
            C_NO_GROK,
        ),
        m(
            "glm-4.6",
            "GLM-4.6",
            "zai",
            Some(200_000),
            Some(128_000),
            false,
            false,
            "flagship",
            true,
            true,
            C_OMNI,
        ),
        m(
            "glm-5.1",
            "GLM-5.1",
            "zai",
            Some(200_000),
            Some(128_000),
            false,
            false,
            "flagship",
            true,
            true,
            C_OMNI_OC,
        ),
        m(
            "qwen3-coder-480b",
            "Qwen3 Coder 480B",
            "qwen",
            Some(256_000),
            Some(65_000),
            false,
            false,
            "code",
            true,
            false,
            C_OMNI_OC,
        ),
        m(
            "qwen3.7-plus",
            "Qwen3.7 Plus",
            "qwen",
            Some(256_000),
            Some(65_000),
            false,
            false,
            "code",
            true,
            true,
            C_OMNI_OC,
        ),
        // ── Free / fast hosts ───────────────────────────────────────
        m(
            "qwen3-coder-480b",
            "Qwen3 Coder 480B",
            "cerebras",
            Some(256_000),
            Some(65_000),
            true,
            false,
            "code",
            true,
            false,
            C_OMNI,
        ),
        m(
            "groq-llama",
            "Llama 3.3 70B (Groq)",
            "groq",
            Some(128_000),
            None,
            true,
            false,
            "fast",
            true,
            true,
            C_OMNI,
        ),
        m(
            "groq-qwen",
            "Qwen3 32B (Groq)",
            "groq",
            Some(128_000),
            None,
            true,
            false,
            "fast",
            true,
            true,
            C_OMNI,
        ),
        m(
            "groq-deepseek",
            "DeepSeek (Groq fast)",
            "groq",
            None,
            None,
            true,
            false,
            "fast",
            true,
            true,
            C_OMNI,
        ),
        m(
            "gpt-oss-120b",
            "GPT-OSS 120B (Groq)",
            "groq",
            None,
            None,
            true,
            false,
            "open",
            true,
            true,
            C_OMNI,
        ),
        m(
            "nemotron",
            "Nemotron 3",
            "nvidia",
            None,
            None,
            true,
            false,
            "open",
            true,
            true,
            C_OMNI,
        ),
        m(
            "kimi-k2-hf",
            "Kimi-K2 (HF)",
            "huggingface",
            None,
            None,
            true,
            false,
            "open",
            true,
            true,
            C_OMNI,
        ),
        m(
            "glm-4.6-hf",
            "GLM-4.6 (HF)",
            "huggingface",
            None,
            None,
            true,
            false,
            "open",
            true,
            true,
            C_OMNI,
        ),
        m(
            "openrouter:auto",
            "OpenRouter (aggregator, any model incl. `:free`)",
            "openrouter",
            None,
            None,
            true,
            false,
            "aggregator",
            true,
            true,
            C_OMNI,
        ),
        m(
            "copilot-gpt-5x",
            "GPT-5.x (Copilot)",
            "copilot",
            None,
            None,
            true,
            false,
            "aggregator",
            true,
            true,
            C_CURSOR,
        ),
        m(
            "zen-gpt-5.1-codex",
            "GPT-5.1 Codex (Zen)",
            "opencode-zen",
            None,
            None,
            false,
            false,
            "aggregator",
            true,
            true,
            C_OMNI_OC,
        ),
        m(
            "zen-qwen3-coder",
            "Qwen3 Coder (Zen)",
            "opencode-zen",
            None,
            None,
            false,
            false,
            "aggregator",
            true,
            false,
            C_OMNI_OC,
        ),
        m(
            "zen-big-pickle",
            "Big Pickle (Zen free)",
            "opencode-zen",
            None,
            None,
            true,
            false,
            "fast",
            true,
            true,
            C_OMNI_OC,
        ),
        m(
            "zen-deepseek-v4-flash-free",
            "DeepSeek V4 Flash Free (Zen)",
            "opencode-zen",
            None,
            None,
            true,
            false,
            "fast",
            true,
            true,
            C_OMNI_OC,
        ),
        m(
            "zen-nemotron-3-ultra-free",
            "Nemotron 3 Ultra Free (Zen)",
            "opencode-zen",
            None,
            None,
            true,
            false,
            "open",
            true,
            true,
            C_OMNI_OC,
        ),
        m(
            "go-coding",
            "Open coding models (Go)",
            "opencode-go",
            None,
            None,
            false,
            false,
            "aggregator",
            true,
            true,
            C_OMNI_OC,
        ),
        m(
            "302ai:auto",
            "302.AI (aggregate, CN + global)",
            "302ai",
            None,
            None,
            false,
            false,
            "aggregator",
            false,
            false,
            C_OMNI,
        ),
    ];
    MODELS
}

pub fn provider(id: &str) -> Option<&'static ProviderSpec> {
    providers().iter().find(|p| p.id == id)
}

pub fn client(id: &str) -> Option<&'static ClientSpec> {
    clients().iter().find(|c| c.id == id)
}

pub fn find_models(id: &str) -> Vec<&'static ModelSpec> {
    models().iter().filter(|m| m.id == id).collect()
}

pub fn models_for_provider(provider_id: &str) -> Vec<&'static ModelSpec> {
    models()
        .iter()
        .filter(|m| m.provider == provider_id)
        .collect()
}

pub fn recommended_models() -> Vec<&'static ModelSpec> {
    models().iter().filter(|m| m.recommended).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn providers_are_unique_and_cover_models() {
        let ids: Vec<&str> = providers().iter().map(|p| p.id).collect();
        for p in providers() {
            assert_eq!(
                ids.iter().filter(|id| **id == p.id).count(),
                1,
                "dup provider {}",
                p.id
            );
            assert!(p.quota.reset_secs >= 1, "{} reset_secs", p.id);
        }
        for m in models() {
            assert!(
                provider(m.provider).is_some(),
                "model {} references unknown provider {}",
                m.id,
                m.provider
            );
            assert!(!m.clients.is_empty(), "model {} has no clients", m.id);
        }
        assert!(provider("xai").is_some());
        assert!(provider("cursor").is_some());
    }

    #[test]
    fn recommended_list_is_rust_web_research() {
        let rec: Vec<&str> = recommended_models().iter().map(|m| m.id).collect();
        assert_eq!(
            rec,
            vec![
                "grok-4.6",
                "gpt-5.2-codex",
                "claude-sonnet-4.6",
                "gemini-3-pro",
                "kimi-k2.7-code",
                "gpt-5.3-codex",
            ]
        );
        for m in recommended_models() {
            assert!(m.rust && m.web, "{} should fit rust+web", m.id);
        }
    }

    #[test]
    fn token_windows_present_on_flagships() {
        for id in ["gpt-5.2", "claude-opus-4.5", "gemini-3-pro", "grok-4.6"] {
            let m = find_models(id);
            assert!(!m.is_empty());
            for spec in m {
                assert!(spec.context_window.is_some(), "{id} ctx missing");
            }
        }
        for id in ["openrouter:auto", "composer-2.5"] {
            for spec in find_models(id) {
                assert_eq!(spec.context_window, None, "{id} ctx must be varies");
            }
        }
    }

    #[test]
    fn qwen3_coder_is_hosted_by_two_providers() {
        let hosts = find_models("qwen3-coder-480b");
        assert_eq!(hosts.len(), 2);
        assert!(hosts.iter().any(|m| m.provider == "qwen"));
        assert!(hosts.iter().any(|m| m.provider == "cerebras"));
    }

    #[test]
    fn free_hosts_publish_timer_windows() {
        for id in ["openrouter", "groq", "nvidia", "cerebras", "huggingface"] {
            let p = provider(id).expect(id);
            assert!(p.free, "{id} should be marked free");
            assert!(
                p.quota.rpm.is_some() || p.quota.daily_reset_secs > 0,
                "{id} needs a timer"
            );
        }
    }

    #[test]
    fn clients_cover_four_hosts() {
        let ids: Vec<&str> = clients().iter().map(|c| c.id).collect();
        assert_eq!(ids, vec!["omni", "cursor", "opencode", "grok"]);
        for c in clients() {
            assert!(!c.rust_models.is_empty(), "{} rust_models", c.id);
            assert!(!c.web_models.is_empty(), "{} web_models", c.id);
        }
    }
}
