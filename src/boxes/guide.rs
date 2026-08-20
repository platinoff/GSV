//! Galaxy About + hover blurbs + distinct card icons (English).
//!
//! Keeps human-facing copy in one table so nav chips, card titles, the About
//! box, and `/api/ui/icon/:name` stay in lockstep.

use serde_json::{json, Value};

use super::ui::esc;

/// One Galaxy card (or chrome control) for About + hover tips.
#[derive(Clone, Copy)]
pub struct GuideEntry {
    pub id: &'static str,
    pub title: &'static str,
    pub group: &'static str,
    pub r#use: &'static str,
    pub blurb: &'static str,
}

/// Sidebar group blurbs (English).
pub const GROUP_GUIDE: [(&str, &str, &str); 4] = [
    (
        "ops",
        "Ops",
        "Live process, products, Telegram, tickets, and MCP.",
    ),
    (
        "vision",
        "Vision",
        "Documentation galaxy: map, node preview, snapshot sync.",
    ),
    (
        "sprint",
        "Sprint",
        "PH-S* queue, board, progress, and the focus map.",
    ),
    (
        "studio",
        "Studio",
        "Models, token usage, Rust ratio, speed, and clippy history.",
    ),
];

/// Dashboard cards first, then chrome. Ids match [`super::ui::CARD_NAMES`].
pub const CARD_GUIDE: &[GuideEntry] = &[
    GuideEntry {
        id: "about",
        title: "About",
        group: "ops",
        r#use: "how to use Galaxy",
        blurb: "English guide: groups, cards, hover tips, and header controls.",
    },
    GuideEntry {
        id: "health",
        title: "Health",
        group: "ops",
        r#use: "process pulse",
        blurb: "Server version, uptime, disk, watchdog, and last fingerprint.",
    },
    GuideEntry {
        id: "products",
        title: "Products",
        group: "ops",
        r#use: "pick a repo",
        blurb: "Choose which VDT product this server is working on, then open its folder.",
    },
    GuideEntry {
        id: "fingerprints",
        title: "Fingerprints",
        group: "ops",
        r#use: "who drained",
        blurb: "Append-only log of actor, IDE, model, and time for each drain.",
    },
    GuideEntry {
        id: "ranks",
        title: "Ranks",
        group: "ops",
        r#use: "merit ladder",
        blurb: "IT + army mix. Jun-nub is level 0. Channel host displays marshal-orchestrator. Bad tests after a commit drop one rank.",
    },
    GuideEntry {
        id: "sw",
        title: "Service Worker",
        group: "ops",
        r#use: "offline shell",
        blurb: "Caches the Galaxy shell so the page still opens if the process is down.",
    },
    GuideEntry {
        id: "watchdog",
        title: "Watchdog",
        group: "ops",
        r#use: "keep :9999 up",
        blurb: "Probes health and recopies debug to live when the server goes missing.",
    },
    GuideEntry {
        id: "mcp",
        title: "MCP",
        group: "ops",
        r#use: "Cursor tools",
        blurb: "Tools and resources for Cursor, OpenCode, and Grok talking to this server.",
    },
    GuideEntry {
        id: "settings",
        title: "Settings",
        group: "ops",
        r#use: "Godfather store",
        blurb: "Telegram channel, bot token (redacted), jail id, squad cap, and channel role (host/mate/guest).",
    },
    GuideEntry {
        id: "telegram",
        title: "Telegram",
        group: "ops",
        r#use: "channel bind",
        blurb: "Bind and poll the Godfather channel. Tickets and bus land here. Role is host, mate, or guest.",
    },
    GuideEntry {
        id: "tickets",
        title: "Tickets",
        group: "ops",
        r#use: "work board",
        blurb: "Claim, walk, hook (roadmap / plan / GitHub issues), and next-action inbox for solo or squad MCP bots.",
    },
    GuideEntry {
        id: "update",
        title: "Update",
        group: "ops",
        r#use: "swap live bin",
        blurb: "Apply a rebuilt binary, or pull from GitHub when origin is ahead of this install.",
    },
    GuideEntry {
        id: "tracker",
        title: "Tracker",
        group: "ops",
        r#use: "what ran",
        blurb: "Technical log of the last workflow: sprints, commands, status, timestamps.",
    },
    GuideEntry {
        id: "sli",
        title: "SLI console",
        group: "ops",
        r#use: "command catalog",
        blurb: "Allowed server commands from src/bin and cargo xtask. Unused = new SLI ideas.",
    },
    GuideEntry {
        id: "toolchain",
        title: "Toolchain",
        group: "ops",
        r#use: "tool versions",
        blurb: "rustc, cargo, clippy, rustfmt, MSYS2, git, Cursor — what this machine runs.",
    },
    GuideEntry {
        id: "hooks-tests",
        title: "Tests hooks",
        group: "ops",
        r#use: "last test bins",
        blurb: "Reads target/ test artifacts without rebuilding.",
    },
    GuideEntry {
        id: "hooks-bench",
        title: "Bench hooks",
        group: "ops",
        r#use: "Criterion medians",
        blurb: "Reads target/criterion without rebuilding.",
    },
    GuideEntry {
        id: "preview",
        title: "Box preview",
        group: "ops",
        r#use: "syntax file",
        blurb: "Render a repo-relative file with Rust-aware colors. Absolute paths are rejected.",
    },
    GuideEntry {
        id: "terminal",
        title: "SLI terminal",
        group: "ops",
        r#use: "run allowlist",
        blurb: "Execute one whitelisted SLI command. No bash, no cat, no shell metacharacters.",
    },
    GuideEntry {
        id: "vision",
        title: "Vision",
        group: "vision",
        r#use: "snapshot head",
        blurb: "Manifest revision, next sprint, and git HEAD of the vision snapshot.",
    },
    GuideEntry {
        id: "vision-map",
        title: "Vision Map",
        group: "vision",
        r#use: "galaxy graph",
        blurb: "Docs and code as nodes. Click a layer chip to filter; search by id or path.",
    },
    GuideEntry {
        id: "vision-sync",
        title: "Vision Sync",
        group: "vision",
        r#use: "remirror docs",
        blurb: "Copy docs/vision into data snapshots and report drift.",
    },
    GuideEntry {
        id: "doc-preview",
        title: "Doc Preview",
        group: "vision",
        r#use: "read a node",
        blurb: "Open one vision node plus its 1-hop neighbors.",
    },
    GuideEntry {
        id: "sprint-queue",
        title: "Sprint Queue",
        group: "sprint",
        r#use: "planned PH-S*",
        blurb: "Upcoming sprints from the manifest queue plus the active sprint.",
    },
    GuideEntry {
        id: "sprint-board",
        title: "Sprint Board",
        group: "sprint",
        r#use: "open / closed",
        blurb: "Columns for open, closed, and planned PH-S* items.",
    },
    GuideEntry {
        id: "sprint-progress",
        title: "Sprint Progress",
        group: "sprint",
        r#use: "percent closed",
        blurb: "Closed share plus per-layer node counts.",
    },
    GuideEntry {
        id: "sprint-map",
        title: "Sprint Map",
        group: "sprint",
        r#use: "modules + edges",
        blurb: "Modules and link kinds for the active sprint.",
    },
    GuideEntry {
        id: "sprint-focus",
        title: "Sprint Focus",
        group: "sprint",
        r#use: "in-scope nodes",
        blurb: "Bright nodes belong to this sprint. Dim nodes are everything else.",
    },
    GuideEntry {
        id: "ide",
        title: "IDE",
        group: "studio",
        r#use: "Cursor chats",
        blurb: "Read-only Cursor and OpenCode sessions. Select one to preview the last messages.",
    },
    GuideEntry {
        id: "omni",
        title: "OmniRouter",
        group: "studio",
        r#use: "AI providers",
        blurb: "Provider catalog, quotas, and which model to pick next. Test a provider id.",
    },
    GuideEntry {
        id: "usage",
        title: "Usage",
        group: "studio",
        r#use: "token spend",
        blurb: "Prompt and completion tokens for this process (OmniRouter + MCP + OmniRoute).",
    },
    GuideEntry {
        id: "ratio",
        title: "Ratio",
        group: "studio",
        r#use: "Rust 95-100%",
        blurb: "Rust vs non-Rust product LOC. GSV must stay in the 95-100% band.",
    },
    GuideEntry {
        id: "speed-index",
        title: "Speed Index",
        group: "studio",
        r#use: "test duration",
        blurb: "How long cargo test took. Green marker = ok, red = failed. Height is seconds.",
    },
    GuideEntry {
        id: "rust-diagnostics",
        title: "Rust Diagnostics",
        group: "studio",
        r#use: "clippy history",
        blurb:
            "Orange bars are warnings, red bars are errors. Latest clippy command in the footer.",
    },
    GuideEntry {
        id: "gpu-mode",
        title: "GPU",
        group: "chrome",
        r#use: "Eco / FX / Ms",
        blurb: "Starfield cost. Eco = few stars, FX = glow, Ms = medium. Click to cycle.",
    },
    GuideEntry {
        id: "rss-ticker",
        title: "Feed",
        group: "chrome",
        r#use: "sprint ticker",
        blurb: "Scrolling sprint feed from feed.json. Click an item for its title.",
    },
    GuideEntry {
        id: "power-menu",
        title: "Power",
        group: "chrome",
        r#use: "sync / reload",
        blurb:
            "Soft sync remirrors Vision. Reload refreshes cards. Force offline is a test switch.",
    },
    GuideEntry {
        id: "panel-dock",
        title: "Dock",
        group: "chrome",
        r#use: "restore cards",
        blurb: "Minimized cards land here. Click a chip to restore.",
    },
    GuideEntry {
        id: "fullscreen",
        title: "Fullscreen",
        group: "chrome",
        r#use: "one card",
        blurb: "Only one card can be fullscreen. Esc restores it.",
    },
    GuideEntry {
        id: "galaxy-backdrop",
        title: "Galaxy",
        group: "chrome",
        r#use: "nebula bg",
        blurb: "Decorative nebula behind the dashboard. GPU mode does not change this layer.",
    },
    GuideEntry {
        id: "starfield",
        title: "Starfield",
        group: "chrome",
        r#use: "stars overlay",
        blurb: "Twinkling stars. Density follows the GPU Eco / FX / Ms button.",
    },
    GuideEntry {
        id: "node-search",
        title: "Node search",
        group: "chrome",
        r#use: "find a node",
        blurb: "Search vision nodes by id, label, or path. Lives inside Vision Map.",
    },
];

/// Lookup a guide row (dashboard or chrome).
pub fn entry(id: &str) -> Option<&'static GuideEntry> {
    CARD_GUIDE.iter().find(|e| e.id == id)
}

/// HTML attribute pair for native title + CSS/JS `data-tip`.
pub fn tip_attrs(text: &str) -> String {
    let t = esc_attr(text);
    format!(" title='{t}' data-tip='{t}'")
}

fn esc_attr(s: &str) -> String {
    esc(s).replace('\'', "&#39;").replace('"', "&quot;")
}

/// Inline 16×16 icon (currentColor stroke) for nav / headings.
pub fn icon_markup(id: &str) -> String {
    format!(
        "<svg class='ico' viewBox='0 0 16 16' width='16' height='16' aria-hidden='true' fill='none' stroke='currentColor' stroke-width='1.35' stroke-linecap='round' stroke-linejoin='round'>{}</svg>",
        icon_inner(id)
    )
}

/// Standalone SVG document for `GET /api/ui/icon/:name`.
pub fn icon_document(id: &str) -> Option<String> {
    let e = entry(id)?;
    let inner = icon_inner(id).replace("currentColor", "#7eb8ff");
    Some(format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16' width='32' height='32' role='img'><title>{}</title><desc>{}</desc><g fill='none' stroke='#7eb8ff' stroke-width='1.35' stroke-linecap='round' stroke-linejoin='round'>{}</g></svg>",
        esc_attr(e.title),
        esc_attr(e.blurb),
        inner
    ))
}

/// Sprite sheet of dashboard icons (About reference).
pub fn icons_sheet() -> String {
    let dash: Vec<&GuideEntry> = CARD_GUIDE.iter().filter(|e| e.group != "chrome").collect();
    let cols = 6usize;
    let cell = 88.0_f64;
    let rows = dash.len().div_ceil(cols);
    let w = cols as f64 * cell;
    let h = 28.0 + rows as f64 * cell;
    let mut body = String::new();
    for (i, e) in dash.iter().enumerate() {
        let x = (i % cols) as f64 * cell + 10.0;
        let y = 24.0 + (i / cols) as f64 * cell;
        let inner = icon_inner(e.id).replace("currentColor", "#7eb8ff");
        body.push_str(&format!(
            "<g transform='translate({x:.0} {y:.0})'><rect width='68' height='72' rx='8' fill='#121a2a' stroke='rgba(120,160,220,0.18)'/><g transform='translate(26 10) scale(1.4)' fill='none' stroke='#7eb8ff' stroke-width='1.35' stroke-linecap='round' stroke-linejoin='round'>{inner}</g><text x='34' y='62' text-anchor='middle' font-family='ui-monospace, Cascadia Code, Consolas, monospace' font-size='8' fill='#8b9cb8'>{}</text><title>{}</title></g>",
            esc_attr(e.title),
            esc_attr(e.blurb),
        ));
    }
    format!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='{w:.0}' height='{h:.0}' viewBox='0 0 {w:.0} {h:.0}' role='img'><title>GSV card icons</title><desc>Distinct glyph per Galaxy card. Hover a tile for the one-line tip.</desc><rect width='{w:.0}' height='{h:.0}' fill='#0a0e18'/><text x='12' y='16' font-family='ui-monospace, Cascadia Code, Consolas, monospace' font-size='11' fill='#e8843c'>Galaxy card icons — each glyph is a different object</text>{body}</svg>"
    )
}

fn icon_inner(id: &str) -> &'static str {
    match id {
        "about" => {
            "<circle cx='8' cy='8' r='6'/><path d='M8 7.2v4'/><circle cx='8' cy='5.2' r='0.7' fill='currentColor' stroke='none'/>"
        }
        "health" => "<path d='M1 9h3l1.5-4 2 8 2-6 1.5 2H15'/>",
        "products" => {
            "<rect x='2' y='4' width='7' height='9' rx='1'/><rect x='7' y='2' width='7' height='9' rx='1'/>"
        }
        "fingerprints" => {
            "<path d='M8 14c0-5 4-5 4-9A4 4 0 0 0 4 5'/><path d='M6 14c0-3.5 2.4-3.2 2.4-7'/><path d='M10 14c.2-2.4-1-2.6-1-5'/>"
        }
        "ranks" => {
            "<path d='M8 2.2 10 6l4 .4-3 2.7.8 4L8 11.2 4.2 13.1 5 9.1 2 6.4 6 6z'/>"
        }
        "sw" => "<path d='M3 10a5 5 0 0 1 10 0'/><rect x='2' y='10' width='12' height='4' rx='1'/>",
        "watchdog" => {
            "<path d='M8 2 3 4v4c0 3.2 2.2 5.5 5 6 2.8-.5 5-2.8 5-6V4z'/><circle cx='8' cy='8' r='1.6'/>"
        }
        "mcp" => {
            "<rect x='1.5' y='5' width='5' height='6' rx='1'/><rect x='9.5' y='5' width='5' height='6' rx='1'/><path d='M6.5 8h3'/>"
        }
        "settings" => {
            "<circle cx='8' cy='8' r='2.2'/><path d='M8 2.2v1.6M8 12.2v1.6M2.2 8h1.6M12.2 8h1.6M4 4l1.1 1.1M10.9 10.9 12 12M12 4l-1.1 1.1M5.1 10.9 4 12'/>"
        }
        "telegram" => "<path d='M2 8 14 3 11 13 8 9 4 11z'/><path d='M8 9 14 3'/>",
        "tickets" => {
            "<rect x='2' y='4' width='12' height='8' rx='1.4'/><path d='M2 8h12'/><circle cx='5' cy='8' r='0.7' fill='currentColor' stroke='none'/>"
        }
        "update" => "<path d='M12.5 6A5 5 0 1 0 13 9'/><path d='M12.5 3.5v3h-3'/>",
        "tracker" => {
            "<circle cx='4' cy='4' r='1.3'/><circle cx='4' cy='8' r='1.3'/><circle cx='4' cy='12' r='1.3'/><path d='M7 4h7M7 8h7M7 12h5'/>"
        }
        "sli" => "<rect x='2' y='3' width='12' height='10' rx='1.4'/><path d='M5 8h2l1.4 2 2.2-4'/>",
        "toolchain" => {
            "<path d='M4 12 10 6l2 2-6 6H4z'/><path d='M11 5c1-1 2.6-1 3.2.2'/>"
        }
        "hooks-tests" => {
            "<path d='M5 2h6v3l-2 3v4H7V8L5 5z'/><path d='M6.5 12.5 8 14l2.2-2.4'/>"
        }
        "hooks-bench" => {
            "<circle cx='8' cy='9' r='5'/><path d='M8 9V5.5M8 9l3 1.5'/>"
        }
        "preview" => "<path d='M4 4h8v10H4z'/><path d='M6 7h4M6 9.5h3'/>",
        "terminal" => "<rect x='2' y='3' width='12' height='10' rx='1.4'/><path d='M5 7l2 1.5L5 10M8.5 10.5H11'/>",
        "vision" => {
            "<circle cx='8' cy='8' r='2.4'/><path d='M1.8 8c2-3.4 4.4-5 6.2-5s4.2 1.6 6.2 5c-2 3.4-4.4 5-6.2 5s-4.2-1.6-6.2-5z'/>"
        }
        "vision-map" => {
            "<circle cx='4' cy='4' r='1.4'/><circle cx='12' cy='5' r='1.4'/><circle cx='7' cy='12' r='1.4'/><path d='M5.2 4.6 10.8 5.2M4.6 5.4 6.2 10.8M11.2 6.2 8.2 11'/>"
        }
        "vision-sync" => "<path d='M4 6a4.5 4.5 0 0 1 8 1'/><path d='M12 4.2v3h-3'/><path d='M12 10a4.5 4.5 0 0 1-8-1'/><path d='M4 11.8v-3h3'/>",
        "doc-preview" => "<path d='M4 2.5h6l3 3V14H4z'/><path d='M10 2.5V6h3'/><path d='M6 9h5M6 11.2h3.5'/>",
        "sprint-queue" => "<path d='M3 4h10M3 8h10M3 12h7'/><circle cx='13' cy='12' r='1.3'/>",
        "sprint-board" => {
            "<rect x='2' y='3' width='3.5' height='10' rx='0.6'/><rect x='6.2' y='5' width='3.5' height='8' rx='0.6'/><rect x='10.5' y='7' width='3.5' height='6' rx='0.6'/>"
        }
        "sprint-progress" => {
            "<circle cx='8' cy='8' r='5.2'/><path d='M8 2.8A5.2 5.2 0 0 1 13.2 8H8z' fill='currentColor' stroke='none' opacity='0.85'/>"
        }
        "sprint-map" => {
            "<rect x='2' y='3' width='5' height='4' rx='0.8'/><rect x='9' y='9' width='5' height='4' rx='0.8'/><path d='M7 5h2.5L11 9'/>"
        }
        "sprint-focus" => {
            "<circle cx='8' cy='8' r='5.5'/><circle cx='8' cy='8' r='1.2'/><path d='M8 2.2v2.2M8 11.6v2.2M2.2 8h2.2M11.6 8h2.2'/>"
        }
        "ide" => {
            "<rect x='2' y='3' width='9' height='7' rx='1'/><rect x='5' y='7' width='9' height='6' rx='1'/>"
        }
        "omni" => {
            "<path d='M8 2 13 8 8 14 3 8z'/><circle cx='8' cy='8' r='1.4'/>"
        }
        "usage" => "<path d='M3 12V8h2v4zM7 12V5h2v7zM11 12V3h2v9z'/>",
        "ratio" => {
            "<circle cx='8' cy='8' r='5.5'/><path d='M8 2.5A5.5 5.5 0 1 1 2.8 10' />"
        }
        "speed-index" => {
            "<path d='M2 12 5 8l3 2 5-7'/><circle cx='13' cy='3' r='1.1' fill='currentColor' stroke='none'/>"
        }
        "rust-diagnostics" => {
            "<path d='M8 2.5 14 13H2z'/><path d='M8 6.5v3.2'/><circle cx='8' cy='11.4' r='0.6' fill='currentColor' stroke='none'/>"
        }
        "gpu-mode" => "<rect x='2' y='5' width='12' height='7' rx='1.2'/><path d='M5 12v1.5h6V12M6.5 7.5h3'/>",
        "rss-ticker" => "<path d='M3 12a1 1 0 1 0 0.01 0z'/><path d='M3 8.2a4 4 0 0 1 4.8 4.8'/><path d='M3 5a7.2 7.2 0 0 1 8 8'/>",
        "power-menu" => "<path d='M8 3v5'/><path d='M5.2 4.4a5 5 0 1 0 5.6 0'/>",
        "panel-dock" => "<rect x='2' y='3' width='12' height='10' rx='1.4'/><path d='M2 6.2h12'/>",
        "fullscreen" => "<path d='M3 6V3h3M13 6V3h-3M3 10v3h3M13 10v3h-3'/>",
        "galaxy-backdrop" => {
            "<ellipse cx='8' cy='8' rx='6' ry='2.2' transform='rotate(-20 8 8)'/><circle cx='8' cy='8' r='1.3' fill='currentColor' stroke='none'/>"
        }
        "starfield" => {
            "<path d='M8 2.5 9 6h3.5L10 8.3 11.2 12 8 9.8 4.8 12 6 8.3 2.5 6H6z'/>"
        }
        "node-search" => {
            "<circle cx='7' cy='7' r='4'/><path d='M10.2 10.2 14 14'/>"
        }
        _ => "<rect x='3' y='3' width='10' height='10' rx='2'/>",
    }
}

/// Donut for the Ratio card (distinct from bar charts).
pub fn ratio_ring_svg(pct: f64, band_min: f64) -> String {
    let pct = pct.clamp(0.0, 100.0);
    let ok = pct + 0.001 >= band_min;
    let color = if ok { "#3fb96e" } else { "#e05b5b" };
    let r = 26.0_f64;
    let c = 2.0 * std::f64::consts::PI * r;
    let dash = (pct / 100.0) * c;
    format!(
        "<svg class='ratio-ring' viewBox='0 0 72 72' width='72' height='72' role='img'{tip}><title>Rust LOC {pct:.1}% (band {band_min:.0}%+)</title><circle cx='36' cy='36' r='{r}' fill='none' stroke='#1e2a3d' stroke-width='7'/><circle cx='36' cy='36' r='{r}' fill='none' stroke='{color}' stroke-width='7' stroke-linecap='round' stroke-dasharray='{dash:.1} {c:.1}' transform='rotate(-90 36 36)'/><text x='36' y='40' text-anchor='middle' font-family='ui-monospace, Cascadia Code, Consolas, monospace' font-size='11' fill='{color}'>{pct:.0}%</text></svg>",
        tip = tip_attrs(&format!(
            "Rust product LOC is {pct:.1}%. Gate is {band_min:.0}% or higher."
        )),
    )
}

/// Layout payload extras: human titles + blurbs for glue.
pub fn layout_cards_json() -> Value {
    json!(CARD_GUIDE
        .iter()
        .filter(|e| e.group != "chrome")
        .map(|e| json!({
            "id": e.id,
            "title": e.title,
            "group": e.group,
            "use": e.r#use,
            "blurb": e.blurb,
            "icon": icon_markup(e.id),
        }))
        .collect::<Vec<_>>())
}

/// About card body — English how-to.
pub fn render_about(d: &Value) -> String {
    if d.get("ok").and_then(Value::as_bool) == Some(false) {
        return format!(
            "<span class='err'>{}</span>",
            esc(d
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("unavailable"))
        );
    }
    let mut out = String::from("<div class='about-hero'><div class='about-mark'>");
    out.push_str(&icon_markup("about"));
    out.push_str(
        "</div><div><strong>Galaxy StarWalker Vision</strong><div class='dim'>Live dashboard for the GSV vision server at <kbd>127.0.0.1:9999</kbd>. Hover any control, chip, or card title for a one-line tip.</div></div></div>",
    );
    out.push_str(
        "<h3 class='about-h'>How to move around</h3><ol class='about-ol'>\
<li>Left tabs switch <strong>Ops</strong>, <strong>Vision</strong>, <strong>Sprint</strong>, and <strong>Studio</strong>.</li>\
<li>This About card stays on screen in every tab until you minimize it (then restore from the dock).</li>\
<li><kbd>_</kbd> minimizes a card. <kbd>□</kbd> opens it below the header so the buttons stay clickable. <kbd>Esc</kbd> exits.</li>\
<li>Header: <strong>A− / A+</strong> changes text size (default 14), <strong>GPU</strong> changes star density, <strong>Auto</strong> refreshes cards every 60s, <strong>Resync</strong> refreshes now, <strong>Power</strong> remirrors Vision.</li>\
<li>Yellow <strong>Update</strong> badge = a newer binary is ready. Click it to swap the live copy.</li>\
</ol>",
    );
    out.push_str(
        "<h3 class='about-h'>What the pictures mean</h3><ul class='about-ol'>\
<li><strong>Speed Index</strong> is a line: height = cargo test seconds, green dot = pass, red = fail.</li>\
<li><strong>Rust Diagnostics</strong> is stacked bars: orange = clippy warnings, red = errors.</li>\
<li><strong>Sprint Focus</strong> is a map: bright nodes are in the sprint, dim nodes are not.</li>\
<li><strong>Ratio</strong> is a ring: how much of the product is Rust (gate 95%+).</li>\
<li><strong>Vision Map</strong> poster is a static L0–L5 legend. The live graph is the colored chips under it.</li>\
<li>Each card has its own glyph in the sidebar — they are not decorative clones.</li>\
</ul>",
    );
    out.push_str(
        "<div class='dim' style='margin:8px 0 6px'>Icon legend (same glyphs as the sidebar)</div>",
    );
    out.push_str(
        "<img class='about-sheet' src='/api/ui/icons.svg' alt='Galaxy card icons — hover a tile for the tip' width='528' height='auto'>",
    );
    for (gid, glabel, gblurb) in GROUP_GUIDE {
        out.push_str(&format!(
            "<h3 class='about-h'>{} <span class='dim'>{}</span></h3><div class='about-grid'>",
            esc(glabel),
            esc(gblurb)
        ));
        for e in CARD_GUIDE.iter().filter(|e| e.group == gid) {
            out.push_str(&format!(
                "<a class='about-item' href='#b-{id}' data-group='{gid}' data-card-jump='{id}'{tip}>{icon}<span><strong>{}</strong><span class='dim'>{}</span></span></a>",
                esc(e.title),
                esc(e.blurb),
                id = e.id,
                tip = tip_attrs(e.blurb),
                icon = icon_markup(e.id),
            ));
        }
        out.push_str("</div>");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guide_ids_are_unique_and_titled() {
        let mut seen = std::collections::BTreeSet::new();
        for e in CARD_GUIDE {
            assert!(seen.insert(e.id), "duplicate guide id {}", e.id);
            assert!(!e.title.is_empty());
            assert!(e.blurb.len() > 12, "{}", e.id);
            assert!(!icon_inner(e.id).is_empty());
        }
        assert!(entry("about").is_some());
        assert!(entry("sprint-focus").is_some());
    }

    #[test]
    fn icons_are_not_clones() {
        let mut bodies = std::collections::BTreeSet::new();
        for e in CARD_GUIDE {
            assert!(bodies.insert(icon_inner(e.id)), "icon clone for {}", e.id);
        }
        let about = icon_document("about").expect("about icon");
        assert!(about.contains("<svg"));
        assert!(about.contains("About"));
        assert!(icon_document("nope").is_none());
        let sheet = icons_sheet();
        assert!(sheet.contains("Galaxy card icons"));
        assert!(sheet.contains("Health"));
    }

    #[test]
    fn about_english_howto() {
        let html = render_about(&json!({ "ok": true }));
        assert!(html.contains("How to move around"), "{html}");
        assert!(html.contains("127.0.0.1:9999"), "{html}");
        assert!(html.contains("data-card-jump='tickets'"), "{html}");
        assert!(html.contains("/api/ui/icons.svg"), "{html}");
        let err = render_about(&json!({ "ok": false, "error": "stand-error" }));
        assert!(
            err.contains("<span class='err'>stand-error</span>"),
            "{err}"
        );
    }

    #[test]
    fn ratio_ring_marks_band() {
        let ok = ratio_ring_svg(96.4, 95.0);
        assert!(ok.contains("96%"), "{ok}");
        assert!(ok.contains("#3fb96e"), "{ok}");
        let bad = ratio_ring_svg(80.0, 95.0);
        assert!(bad.contains("#e05b5b"), "{bad}");
    }
}
