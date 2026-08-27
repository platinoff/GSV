//! Local tunnel manager (ngrok). Stands up a public HTTPS endpoint that maps
//! to the local HTTP listener so Telegram WebApp buttons (`/app`) and webhooks
//! are reachable from phones / remote clients.
//!
//! The manager is non-authoritative at startup: `resolved_public_url()` checks
//! the explicit `TELENETIS_PUBLIC_URL` first, then auto-derives the live ngrok
//! URL and stores it on `AppState` for the webhook/button codepaths to reuse.
//!
//! ## Key security note
//! The ngrok authtoken is a secret (like the bot token). It is read from the
//! local environment / `.env` (`NGROK_AUTHTOKEN`) and never logged or written
//! into the tunnel config file we produce.

use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;
use tokio::time::sleep;

use crate::config::Config;
use crate::error::TelenetisError;

/// Local HTTP endpoint of the ngrok agent that exposes tunnel metadata.
pub const NGROK_API: &str = "http://127.0.0.1:4040";

/// Where the ngrok agent is expected to be on Windows when not on PATH.
pub const NGROK_FALLBACKS: &[&str] = &[
    r"C:\Users\plati\AppData\Local\Temp\opencode\ngrok\ngrok.exe",
    r"C:\Users\plati\AppData\Local\Microsoft\WinGet\Packages\Ngrok.Ngrok_Microsoft.Winget.Source_8wekyb3d8bbwe\ngrok.exe",
];

/// True when the local ngrok agent is already serving a tunnel (API reachable
/// and at least one tunnel exists). Used to avoid spawning a duplicate agent.
async fn agent_already_up() -> bool {
    let client = reqwest::Client::new();
    match client
        .get(format!("{NGROK_API}/api/tunnels"))
        .timeout(Duration::from_secs(3))
        .send()
        .await
    {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

/// True if `name` resolves on the PATH (exact Win32 logic is unnecessary; a
/// simple per-`;` scan across `PATH` is enough for our use).
fn on_path(name: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    let mut name_bin = name.to_string();
    if !name_bin.ends_with(".exe") {
        name_bin.push_str(".exe");
    }
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(&name_bin);
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().to_string());
        }
        let bare = dir.join(name);
        if bare.is_file() {
            return Some(bare.to_string_lossy().to_string());
        }
    }
    None
}

/// Resolve the ngrok executable path, falling back across known locations.
fn resolve_bin(config: &Config) -> Option<String> {
    if let Some(bin) = &config.ngrok_bin {
        if !bin.is_empty() {
            return Some(bin.clone());
        }
    }
    if let Ok(bin) = std::env::var("NGROK_BIN") {
        if !bin.is_empty() {
            return Some(bin);
        }
    }
    if let Some(bin) = on_path("ngrok") {
        return Some(bin);
    }
    NGROK_FALLBACKS
        .iter()
        .find(|p| std::path::Path::new(p).exists())
        .map(|p| p.to_string())
}

/// Spawn the ngrok agent (detached) targeting the local listener.
async fn spawn(config: &Config) -> Result<(), TelenetisError> {
    let bin = resolve_bin(config)
        .ok_or_else(|| TelenetisError::Tunnel("ngrok binary not found".to_string()))?;
    let addr = format!("http://127.0.0.1:{}", config.port);
    let mut cmd = Command::new(&bin);
    cmd.arg("http").arg(&addr).arg("--log=stdout");
    // Keep the agent after this function returns.
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let child = cmd
        .spawn()
        .map_err(|e| TelenetisError::Tunnel(format!("failed to spawn ngrok ({bin}): {e}")))?;
    // Detach: we don't manage the agent lifecycle beyond this call.
    drop(child);
    Ok(())
}

/// Poll the local ngrok API until an HTTPS public URL appears (or timeout).
async fn fetch_url(timeout_secs: u64) -> Result<String, TelenetisError> {
    let client = reqwest::Client::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        if let Ok(resp) = client
            .get(format!("{NGROK_API}/api/tunnels"))
            .timeout(Duration::from_secs(3))
            .send()
            .await
        {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(tunnels) = json.get("tunnels").and_then(|v| v.as_array()) {
                    for t in tunnels {
                        let url = t.get("public_url").and_then(|v| v.as_str()).unwrap_or("");
                        let proto = t.get("proto").and_then(|v| v.as_str()).unwrap_or("");
                        if proto == "https" && url.starts_with("https://") {
                            return Ok(url.trim_end_matches('/').to_string());
                        }
                    }
                }
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(TelenetisError::Tunnel(
                "timed out waiting for an HTTPS public URL from ngrok".to_string(),
            ));
        }
        sleep(Duration::from_millis(500)).await;
    }
}

/// Ensure a tunnel is up and return the public HTTPS base URL.
pub async fn ensure_public_url(config: &Config) -> Result<String, TelenetisError> {
    // Explicit configuration wins — the operator may provide a fixed host.
    if let Some(url) = &config.public_url {
        if !url.is_empty() {
            return Ok(url.trim_end_matches('/').to_string());
        }
    }
    if !config.tunnel_enabled {
        // Without a configured public URL and with tunnels disabled, fall back
        // to the local listener (only useful on the same machine's client).
        return Ok(format!("http://127.0.0.1:{}", config.port));
    }
    if !agent_already_up().await {
        spawn(config).await?;
    }
    fetch_url(20).await
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn test_config() -> Config {
        Config {
            bot_token: "test".to_string(),
            gsv_url: "http://127.0.0.1:9999".to_string(),
            port: 9800,
            jail_id: "test-jail".to_string(),
            godfather_channel_id: 0,
            webhook_url: None,
            public_url: None,
            tunnel_enabled: true,
            ngrok_bin: None,
        }
    }

    #[tokio::test]
    async fn explicit_public_url_wins() {
        let mut cfg = test_config();
        cfg.public_url = Some("https://example.ngrok.app".to_string());
        let url = ensure_public_url(&cfg).await.unwrap();
        assert_eq!(url, "https://example.ngrok.app");
    }

    #[tokio::test]
    async fn disabled_tunnel_falls_back_to_local() {
        let mut cfg = test_config();
        cfg.tunnel_enabled = false;
        cfg.public_url = None;
        let url = ensure_public_url(&cfg).await.unwrap();
        assert_eq!(url, "http://127.0.0.1:9800");
    }
}
