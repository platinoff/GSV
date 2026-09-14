//! GSV (Galaxy StarWalker Vision) — Rust-first vision server library.
//!
//! `gsv` is a standalone Rust-first vision server (`S:/rust/GSV`) with boxes:
//! Tracker, SLI console, Toolchain, IDE, Update, Box preview, SLI terminal,
//! Tests/bench hooks, OmniRouter (Rust AI proxy/router), and MCP
//! (`gsv_mcp_openbot` stdio + `POST`/`DELETE /mcp`). Runtime/API/tools are
//! Rust-only; UI is a thin JS/DOM glue (0% WebAssembly for now, 0-5% horizon).
//!
//! Workspace layout:
//! ```text
//! S:/rust/GSV/
//!   Cargo.toml            standalone package `gsv`
//!   src/lib.rs            this library (server + boxes)
//!   src/bin/gsv_server.rs `gsv-server` binary entry point
//!   ui/index.html         embedded single-page UI (include_str!)
//!   docs/vision/          vision canon (manifest/feed/extensions)
//!   data/                 durable box stores (gsv_tracker.json, ...)
//!   tests/                Rust integration tests (contracts + update flow)
//! ```

pub mod app_error;
pub mod boxes;
pub mod mcp;
pub mod net;
pub mod security;
pub mod server;
pub mod state;
pub mod tracker;
pub mod vision;

pub use app_error::AppError;
pub use state::AppState;

/// Canonical GSV server name (UI header / health payload).
pub const GSV_SERVER_NAME: &str = "Galaxy StarWalker Vision";

/// Default listen host.
pub const DEFAULT_HOST: &str = "127.0.0.1";

/// Default listen port.
pub const DEFAULT_PORT: u16 = 9999;

/// Canon live UI URL for feed items, pointer pages, and sample links.
/// Band 233: built on the machine's local (LAN) address so phone / VM /
/// edge peers open the same link the desktop sees; loopback under the
/// cargo-test harness keeps contract assertions deterministic.
pub fn live_ui_url(fragment: &str) -> String {
    let host = net::local_addr();
    if fragment.is_empty() {
        format!("http://{host}:{DEFAULT_PORT}/")
    } else {
        format!("http://{host}:{DEFAULT_PORT}/{fragment}")
    }
}

/// Canon Streamable HTTP MCP URL (Cursor `.cursor/mcp.json` `url`).
pub fn mcp_http_url() -> String {
    format!("http://{}:{DEFAULT_PORT}/mcp", net::local_addr())
}

/// Build-time version of the GSV server binary.
pub fn gsv_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
