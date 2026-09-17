//! Phone emulator core (T8): a Rust double of the Telegram Mini App client.
//!
//! The owner taps real phones all day with no visibility into what actually
//! persisted — this module lets the agent drive the same flows headlessly:
//! an emulated phone with a lifecycle (foreground / background / killed), a
//! two-tier store (disk dir = real phone storage, in-memory cache = the
//! evictable IDB equivalent), and an `initData` signer that mirrors
//! [`crate::security::initdata`] so scenarios use fresh handshakes instead of
//! the pinned 2025 fixture.
//!
//! Hermetic by construction: scenarios run against an in-process router
//! (never live `:9800`/`:9999`); the disk dir lives under the test's temp
//! area and is removed by the caller.

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

type HmacSha256 = Hmac<Sha256>;

/// Emulated WebView lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lifecycle {
    /// App open: timers run, sockets live.
    Foreground,
    /// App minimized inside Telegram: timers frozen, sockets dropped.
    Background,
    /// Telegram killed: RAM gone, disk kept.
    Killed,
}

/// Device storage double: `disk_dir` is the phone's real storage (survives
/// everything), `cache` is the IDB equivalent (fast, evictable under OS
/// pressure), `persist_granted` mirrors `navigator.storage.persist()`.
#[derive(Debug)]
pub struct DeviceStorage {
    disk_dir: PathBuf,
    cache: HashMap<String, Vec<u8>>,
    persist_granted: bool,
}

impl DeviceStorage {
    pub fn new(disk_dir: PathBuf) -> io::Result<Self> {
        std::fs::create_dir_all(&disk_dir)?;
        Ok(Self {
            disk_dir,
            cache: HashMap::new(),
            persist_granted: false,
        })
    }

    pub fn disk_dir(&self) -> &Path {
        &self.disk_dir
    }

    /// Write bytes to real storage (Download-folder equivalent).
    pub fn save_to_disk(&self, name: &str, bytes: &[u8]) -> io::Result<PathBuf> {
        let path = self.disk_dir.join(sanitize_name(name));
        std::fs::write(&path, bytes)?;
        Ok(path)
    }

    pub fn read_disk(&self, name: &str) -> Option<Vec<u8>> {
        std::fs::read(self.disk_dir.join(sanitize_name(name))).ok()
    }

    pub fn disk_has(&self, name: &str) -> bool {
        self.disk_dir.join(sanitize_name(name)).is_file()
    }

    pub fn cache_put(&mut self, name: &str, bytes: Vec<u8>) {
        self.cache.insert(name.to_string(), bytes);
    }

    pub fn cache_get(&self, name: &str) -> Option<&[u8]> {
        self.cache.get(name).map(Vec::as_slice)
    }

    pub fn cache_len(&self) -> usize {
        self.cache.len()
    }

    /// OS storage-pressure simulation: the whole IDB equivalent is gone.
    pub fn evict_cache(&mut self) {
        self.cache.clear();
    }

    /// Double of `navigator.storage.persist()`: in the emulator it is
    /// granted on request (the real API may deny silently in a WebView —
    /// that denial is what the client surfaces, not what we fake here).
    pub fn request_persist(&mut self) {
        self.persist_granted = true;
    }

    pub fn persist_granted(&self) -> bool {
        self.persist_granted
    }
}

fn sanitize_name(name: &str) -> String {
    let clean: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if clean.is_empty() {
        "model.gguf".to_string()
    } else {
        clean
    }
}

/// One emulated phone: identity (Telegram user JSON), lifecycle, storage.
#[derive(Debug)]
pub struct EmuPhone {
    pub id: String,
    pub lifecycle: Lifecycle,
    pub storage: DeviceStorage,
    pub user_json: String,
}

impl EmuPhone {
    pub fn new(id: &str, disk_dir: PathBuf, user_json: &str) -> io::Result<Self> {
        Ok(Self {
            id: id.to_string(),
            lifecycle: Lifecycle::Foreground,
            storage: DeviceStorage::new(disk_dir)?,
            user_json: user_json.to_string(),
        })
    }

    /// Minimize inside Telegram: timers frozen, sockets dropped.
    pub fn minimize(&mut self) {
        self.lifecycle = Lifecycle::Background;
    }

    /// Return to the app: timers and sockets may resume.
    pub fn restore(&mut self) {
        if self.lifecycle == Lifecycle::Background {
            self.lifecycle = Lifecycle::Foreground;
        }
    }

    /// Telegram itself killed: RAM (cache) gone, disk kept.
    pub fn kill(&mut self) {
        self.lifecycle = Lifecycle::Killed;
        self.storage.evict_cache();
    }

    /// Whether JS timers / poll loops run in this state.
    pub fn timers_live(&self) -> bool {
        self.lifecycle == Lifecycle::Foreground
    }

    /// Signed `initData` handshake for `bot_token` at `auth_date`
    /// (transport-encoded, ready for `?initData=`).
    pub fn init_data(&self, bot_token: &str, auth_date: i64) -> String {
        sign_init_data(bot_token, &self.user_json, auth_date)
    }
}

/// Sign an `initData` handshake exactly the way
/// [`crate::security::initdata::verify_init_data`] checks it: decoded pairs
/// are sorted and joined with `\n`, keyed by `HMAC(WebAppData, bot_token)`.
/// Returns the RAW form (raw `&` separators, user value percent-encoded) —
/// pass it straight to `verify_init_data`, or through [`transport_encode`]
/// for `?initData=` URLs (axum decodes once before verify sees it).
pub fn sign_init_data(bot_token: &str, user_json: &str, auth_date: i64) -> String {
    let query_id = "EMU000001queryid0001";
    let check = format!("auth_date={auth_date}\nquery_id={query_id}\nuser={user_json}");
    let mut mac = HmacSha256::new_from_slice(b"WebAppData").expect("hmac key");
    mac.update(bot_token.as_bytes());
    let secret = mac.finalize().into_bytes();
    let mut mac = HmacSha256::new_from_slice(&secret).expect("hmac key");
    mac.update(check.as_bytes());
    let hash: String = mac
        .finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    format!(
        "auth_date={auth_date}&query_id={query_id}&user={}&hash={hash}",
        percent_encode(user_json)
    )
}

/// Transport-encode a raw `initData` string for `?initData=` URLs (the whole
/// string, separators included — axum decodes it once before verify parses).
pub fn transport_encode(init_data_raw: &str) -> String {
    percent_encode(init_data_raw)
}

/// Percent-encode a query value (Telegram client style, UTF-8 bytes).
pub fn percent_encode(input: &str) -> String {
    const UNRESERVED: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.~";
    let mut out = String::new();
    for b in input.as_bytes() {
        if UNRESERVED.contains(b) {
            out.push(*b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Build a `Range: bytes=<loaded>-` resume header (client-side resume
/// bookkeeping double). `None` when starting from zero or already complete.
pub fn resume_range_header(loaded: u64, total: u64) -> Option<String> {
    if loaded == 0 || (total > 0 && loaded >= total) {
        return None;
    }
    Some(format!("bytes={loaded}-"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::initdata::{verify_init_data, InitDataError, DEFAULT_MAX_AGE_SECS};

    const BOT_TOKEN: &str = "test";
    const USER: &str = "{\"id\":279058397,\"first_name\":\"Vlad\",\"language_code\":\"en\"}";

    #[test]
    fn signer_passes_server_verification_fresh() {
        let auth = 1_800_000_000;
        let init = sign_init_data(BOT_TOKEN, USER, auth);
        assert!(verify_init_data(&init, BOT_TOKEN, auth, DEFAULT_MAX_AGE_SECS).is_ok());
    }

    #[test]
    fn signer_stale_fails_as_stale() {
        let auth = 1_800_000_000;
        let init = sign_init_data(BOT_TOKEN, USER, auth);
        let err = verify_init_data(
            &init,
            BOT_TOKEN,
            auth + DEFAULT_MAX_AGE_SECS as i64 + 5,
            DEFAULT_MAX_AGE_SECS,
        )
        .expect_err("stale must fail");
        assert_eq!(err, InitDataError::Stale);
    }

    #[test]
    fn signer_tampered_user_fails_signature() {
        let auth = 1_800_000_000;
        let init = sign_init_data(BOT_TOKEN, USER, auth);
        let tampered = init.replace("279058397", "999999999");
        let err = verify_init_data(&tampered, BOT_TOKEN, auth, DEFAULT_MAX_AGE_SECS)
            .expect_err("tampered must fail");
        assert_eq!(err, InitDataError::SignatureMismatch);
    }

    #[test]
    fn signer_wrong_token_fails() {
        let auth = 1_800_000_000;
        let init = sign_init_data(BOT_TOKEN, USER, auth);
        assert!(verify_init_data(&init, "other", auth, DEFAULT_MAX_AGE_SECS).is_err());
    }

    #[test]
    fn lifecycle_freezes_timers_in_background() {
        let dir = std::env::temp_dir().join("emu_lifecycle_unit");
        let _ = std::fs::remove_dir_all(&dir);
        let mut phone = EmuPhone::new("p1", dir, USER).unwrap();
        assert!(phone.timers_live());
        phone.minimize();
        assert_eq!(phone.lifecycle, Lifecycle::Background);
        assert!(!phone.timers_live());
        phone.restore();
        assert!(phone.timers_live());
    }

    #[test]
    fn kill_drops_cache_keeps_disk() {
        let dir = std::env::temp_dir().join("emu_kill_unit");
        let _ = std::fs::remove_dir_all(&dir);
        let mut phone = EmuPhone::new("p1", dir.clone(), USER).unwrap();
        phone.storage.save_to_disk("m.gguf", b"bytes").unwrap();
        phone.storage.cache_put("m.gguf", b"bytes".to_vec());
        phone.kill();
        assert_eq!(phone.lifecycle, Lifecycle::Killed);
        assert_eq!(phone.storage.cache_len(), 0);
        assert!(phone.storage.disk_has("m.gguf"));
        assert!(!phone.timers_live());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn evict_is_os_pressure_cache_only() {
        let dir = std::env::temp_dir().join("emu_evict_unit");
        let _ = std::fs::remove_dir_all(&dir);
        let mut phone = EmuPhone::new("p1", dir.clone(), USER).unwrap();
        phone.storage.save_to_disk("m.gguf", b"bytes").unwrap();
        phone.storage.cache_put("m.gguf", b"bytes".to_vec());
        phone.storage.evict_cache();
        assert!(phone.storage.cache_get("m.gguf").is_none());
        assert_eq!(phone.storage.read_disk("m.gguf").unwrap(), b"bytes");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn persist_flag_defaults_off_grants_on_request() {
        let dir = std::env::temp_dir().join("emu_persist_unit");
        let _ = std::fs::remove_dir_all(&dir);
        let mut phone = EmuPhone::new("p1", dir.clone(), USER).unwrap();
        assert!(!phone.storage.persist_granted());
        phone.storage.request_persist();
        assert!(phone.storage.persist_granted());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resume_range_header_shapes() {
        assert_eq!(resume_range_header(0, 100), None);
        assert_eq!(resume_range_header(10, 100), Some("bytes=10-".to_string()));
        assert_eq!(resume_range_header(100, 100), None);
        assert_eq!(resume_range_header(150, 100), None);
    }

    #[test]
    fn sanitize_keeps_disk_dir_safe() {
        let dir = std::env::temp_dir().join("emu_sanitize_unit");
        let _ = std::fs::remove_dir_all(&dir);
        let phone = EmuPhone::new("p1", dir.clone(), USER).unwrap();
        let path = phone.storage.save_to_disk("../../evil", b"x").unwrap();
        // Traversal collapsed to a single segment: the file lands directly
        // inside the disk dir (a ".." substring in the name is harmless).
        assert_eq!(path.parent(), Some(dir.as_path()));
        assert!(path.is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
