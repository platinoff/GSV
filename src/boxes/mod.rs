//! GSV boxes — panels/capabilities of the Galaxy StarWalker Vision server.
//!
//! | Box | Rust module | Endpoint | Data source |
//! |-----|-------------|----------|-------------|
//! | Tracker | `tracker` | `/api/tracker` | FM §5.12, history, loc-audit |
//! | SLI console | `sli` | `/api/sli` | `src/bin/` + `cargo xtask` |
//! | Toolchain | `toolchain` | `/api/toolchain` | toolchain, env |
//! | IDE | `ide` | `/api/ide/…` | opencode/cursor sessions |
//! | Update | `update` | `/api/update` · `/api/update/apply` · `/events` | live copy + version |
//! | Box preview | `preview` | `/api/preview` | files |
//! | SLI terminal | `terminal` | `/api/terminal` | SLI catalog |
//! | Tests/bench hooks | `hooks` | `/api/hooks/…` | `target/` artifacts |
//! | OmniRouter | `omni` | `/api/omni/…` | provider/model catalog + config + proxy |
//! | Ratio | `ratio` | `/api/ratio` | `GSV/data/rust_ratio.json` (Rust 95–100%) |
//! | Vision | `vision` | `/api/vision*` | `docs/vision/` manifest + feed mirror |
//! | UI fragments | `ui` | `/api/ui/card/:name` | server-rendered card HTML |
//! | Products | `products` | `/api/products` | workspace ∪ sibling git ∪ kit |
//! | Fingerprints | `fingerprint` | `/api/fingerprints` | `docs/gsv/fingerprints.jsonl` |
//! | Service Worker | `sw` | `/sw.js` · `/api/sw` | Rust-rendered shell cache |
//! | Watchdog | `watchdog` | `/api/watchdog` | `target/live/watchdog.json` heartbeat + respawn live copy |
//! | Usage | `usage` | `/api/usage` | per-session token counts (OmniRouter + MCP + OmniRoute) |
//! | Settings | `settings` | `/api/settings` | Godfather channel + redacted token + co-workflows |
//! | Telegram | `telegram` | `/api/telegram` · `/api/telegram/bus` · `/api/telegram/ticket` | Godfather bind + MCP bus + ticket ingest (dry-run queue in tests) |
//! | Tickets | `tickets` | `/api/tickets` · `/api/tickets/claim` · `/api/tickets/done` · `/api/tickets/error` · `/api/tickets/presence` · `/api/tickets/walk` · `/api/tickets/hook` · `/api/tickets/bench` | git JSONL board + MCP claim/solo-squad + scenario band walk + roadmap/plan hook + scenario bench |
//! | MDS | `mds` | `/api/mds` | light memory/disk/speed probe (`gsv-mds`) |
//! | Xtask | `xtask` | `/api/xtask` · `/api/disk` | `cargo xtask` product automation (no `.sh`) |

pub mod fingerprint;
pub mod gitkit;
pub mod hooks;
pub mod ide;
pub mod mds;
pub mod omni;
pub mod preview;
pub mod products;
pub mod ratio;
pub mod settings;
pub mod sli;
pub mod sw;
pub mod telegram;
pub mod terminal;
pub mod tickets;
pub mod toolchain;
pub mod ui;
pub mod update;
pub mod usage;
pub mod vision;
pub mod watchdog;
pub mod xtask;

pub use fingerprint::Fingerprint;
pub use ide::{IdeSelection, IdeSession, IdeWire};
pub use omni::{OmniConfig, OmniRouter, OmniWire, ProviderConfig, ProviderWire, RoutingConfig};
pub use preview::PreviewWire;
pub use products::{ProductRow, ProductScan};
pub use ratio::{AuditConfig, CategoryLoc, ProductCategory, RustRatioReport};
pub use sli::{SliCatalog, SliEntry, SliWire};
pub use terminal::{TerminalRequest, TerminalResponse};
pub use toolchain::{ToolchainEntry, ToolchainWire};
pub use update::{UpdateCheckParams, UpdateWire};
