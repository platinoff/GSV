//! Telenetis Mini App feature freeze: identity + chrome + LAN.
//!
//! Tensor / WebGPU stay a **probe**. KVM / swarm-as-OS are not growth.
//! Phone worker is the hub APK (`apk_edge`). Canon: GSV_AGI_PATH.md.

/// Frozen surface name.
pub const SURFACE: &str = "shell";

fn norm(raw: &str) -> String {
    raw.trim().to_ascii_lowercase().replace(['_', ' '], "-")
}

/// Features Telenetis may still grow.
pub fn may_grow(kind: &str) -> bool {
    matches!(
        norm(kind).as_str(),
        "identity" | "chrome" | "shell" | "dashboard" | "board" | "lan" | "webhook"
    )
}

/// Path (b) WebGPU / Mini App GGUF — probe only, not the phone worker.
pub fn is_probe(kind: &str) -> bool {
    matches!(norm(kind).as_str(), "tensor" | "webgpu" | "wllama" | "gguf")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freeze_shell_only() {
        assert_eq!(SURFACE, "shell");
        assert!(may_grow("identity"));
        assert!(may_grow("chrome"));
        assert!(may_grow("lan"));
        assert!(!may_grow("tensor"));
        assert!(!may_grow("kvm"));
        assert!(!may_grow("swarm"));
        assert!(!may_grow("start-worker"));
        assert!(is_probe("tensor"));
        assert!(is_probe("WebGPU"));
        assert!(!is_probe("board"));
    }
}
