//! Portable plugin auto-wire — `cargo xtask adopt <path>`.
//!
//! Owner ask (ECO-3, 2026-11-04): drop any portable project into the rust
//! environment (`S:/rust`) and it auto-wires into the ecosystem. Adopt performs
//! a light S0 digest of a **sibling git tree**, drafts the registration
//! artifacts, and — only with `--yes` — writes them:
//!
//! 1. thin `AGENTS.md` from the kit's [`PLUGIN_AGENTS.md`] template (if absent)
//! 2. `rust-toolchain.toml` stable-gnu pin (rust only)
//! 3. `docs/HANDOFF_NEW_SESSION.md` + `docs/NEXT_SESSION_PROMPT.md` skeletons
//! 4. registration row in `docs/gsv/PRODUCTS.md` (makes `registered` true)
//! 5. folder entry in `gsv.code-workspace` (GSV first)
//!
//! Routing is free: `cargo xtask products` / `gsv_products_scan` merges
//! workspace ∪ sibling git ∪ kit on every call — the scan is the list.
//! Dry-first discipline: no `--yes`, no mutation.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// Whom the orphan git trees are registered under — `docs/gsv/PRODUCTS.md`.
const PRODUCTS_MD: &str = "docs/gsv/PRODUCTS.md";
const WORKSPACE: &str = "gsv.code-workspace";
const PIN_TOML: &str = r#"# Ecosystem toolchain policy (owner): stay on the latest stable rust,
# gnu triple (MSYS2 builds). Floats with stable: 1.99.1 -> 1.100 -> ...
[toolchain]
channel = "stable-x86_64-pc-windows-gnu"
targets = ["x86_64-pc-windows-gnu"]
components = ["rustfmt", "clippy"]
"#;

/// Light S0 digest + drafted artifacts (dry — `plan` never mutates).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AdoptPlan {
    pub ok: bool,
    pub id: String,
    pub name: String,
    /// Display path `S:/rust/<dir>`.
    pub root: String,
    /// `rust` / `node` / `git` (mirror of products::discover kinds).
    pub kind: String,
    pub registered: bool,
    pub git_head: String,
    pub dirty_files: usize,
    pub pin_present: bool,
    pub cargo_name: Option<String>,
    pub handoff_present: bool,
    pub next_present: bool,
    pub agents_present: bool,
    /// Bind/listen ports found by a string survey over `src/**/*.rs`.
    pub ports: Vec<u16>,
    /// Every artifact with `content = None` is a **patch** (insertion); the
    /// rest are whole-file writes. Both only fire with `apply`.
    pub artifacts: Vec<AdoptArtifact>,
    /// Sibling warnings (dirty tree, already-adopted tree, missing dep chains).
    pub notes: Vec<String>,
}

/// One drafted registration artifact.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AdoptArtifact {
    /// Display-style path (`S:/rust/<tree>/<rel>`).
    pub path: String,
    /// `write` = new file (must be absent), `patch-products-row`,
    /// `patch-workspace-folder`.
    pub op: String,
    /// Whole-file content for `write` ops.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Human line about the insertion for patch ops.
    pub detail: String,
}

fn slug_of(path: &Path) -> String {
    path.file_name()
        .map(|s| {
            s.to_string_lossy()
                .to_lowercase()
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect()
        })
        .unwrap_or_default()
}

fn has_git(path: &Path) -> bool {
    let g = path.join(".git");
    g.is_dir() || g.is_file()
}

fn git_capture(cwd: &Path, args: &[&str]) -> String {
    crate::vision::command("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn display(p: &Path) -> String {
    crate::boxes::products::display_path(p)
}

/// Parse `:` + 3..=5 ASCII digits runs out of a bind/listen-ish source line.
fn numeric_ports(line: &str) -> Vec<u16> {
    let chars: Vec<char> = line.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == ':' {
            let mut j = i + 1;
            let mut digits = String::new();
            while j < chars.len() && chars[j].is_ascii_digit() {
                digits.push(chars[j]);
                j += 1;
            }
            if (3..=5).contains(&digits.len()) {
                if let Ok(p) = digits.parse::<u16>() {
                    out.push(p);
                }
                i = j;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// String survey of `src/**/*.rs` for bind/listen/port literals (cap 400 files).
fn port_survey(root: &Path) -> Vec<u16> {
    let mut files = Vec::new();
    collect_rs(&root.join("src"), 0, &mut files);
    let mut ports = BTreeSet::new();
    for f in files.iter().take(400) {
        let Ok(text) = fs::read_to_string(f) else {
            continue;
        };
        for line in text.lines() {
            let low = line.to_ascii_lowercase();
            if !(low.contains("bind") || low.contains("listen") || low.contains("port")) {
                continue;
            }
            if low.starts_with("//") || low.starts_with('#') {
                continue;
            }
            ports.extend(numeric_ports(line));
        }
    }
    ports.into_iter().collect()
}

fn collect_rs(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > 4 || out.len() >= 400 {
        return;
    }
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_rs(&p, depth + 1, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// Parse a rust `Cargo.toml` [package] name (mirror of products::scan).
fn package_name(root: &Path) -> Option<String> {
    let text = fs::read_to_string(root.join("Cargo.toml")).ok()?;
    let mut in_package = false;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_package = t == "[package]";
            continue;
        }
        if in_package {
            if let Some(rest) = t.strip_prefix("name") {
                if !rest.starts_with([' ', '\t', '=']) {
                    continue;
                }
                let name = rest
                    .trim()
                    .trim_start_matches('=')
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .trim();
                if !name.is_empty() {
                    return Some(name.to_string());
                }
            }
        }
    }
    None
}

fn read(p: &Path) -> String {
    fs::read_to_string(p).unwrap_or_default()
}

/// Pull the template body (skip the kit-side header lines before the first
/// `This tree is a **portable plugin**` paragraph).
fn plugin_agents_body(kit_root: &Path) -> String {
    let text = read(&kit_root.join("docs/gsv/PLUGIN_AGENTS.md"));
    if let Some(at) = text.find("This tree is a ") {
        text[at..].to_string()
    } else if text.is_empty() {
        "This tree is a **portable plugin** of the rust-folder environment.\n\
         Global rules first: `S:/rust/GSV/AGENTS.md`.\n"
            .into()
    } else {
        text
    }
}

fn handoff_skeleton(id: &str, root: &str, date: &str) -> String {
    format!(
        "# HANDOFF — {id}\n\n\
         > Skeleton generated by `cargo xtask adopt {root}` on {date}.\n\
         > Fill in before the first drain (canon: `S:/rust/GSV/docs/gsv/PRODUCTS.md`).\n\n\
         ## Router\n\n\
         - Global first: `S:/rust/GSV/AGENTS.md` + `S:/rust/GSV/docs/gsv/GSV_AGI_PATH.md`.\n\
         - Registered row: `S:/rust/GSV/docs/gsv/PRODUCTS.md`.\n\n\
         ## State (fill)\n\n\
         - Test command / gates.\n\
         - Ports, scheduler, failing bands.\n\n\
         ## Next (fill)\n\n\
         - Next band for the drain.\n\n\
         ## Gates\n\n\
         - `cargo fmt --all --check` -> `cargo clippy --all-targets -- -D warnings` -> `cargo test`\n",
        id = id,
        root = root,
        date = date,
    )
}

fn next_skeleton(id: &str, date: &str) -> String {
    format!(
        "# NEXT session prompt — {id}\n\n\
         > Skeleton generated by `cargo xtask adopt` on {date}. Replace with the\n\
         > concrete next band before the drain.\n\n\
         - Open `S:/rust/GSV/gsv.code-workspace` (GSV first).\n\
         - Read HANDOFF_NEW_SESSION.md, then drain the next band.\n",
        id = id,
        date = date,
    )
}

fn agents_md(dir_name: &str, kit_root: &Path) -> String {
    format!(
        "# AGENTS.md — {dir_name} (plugin)\n\nThis tree is a **portable plugin** of the rust-folder environment.\n\
         {body}\n",
        dir_name = dir_name,
        body = plugin_agents_body(kit_root),
    )
}

fn products_row(id: &str, root: &str, kind: &str) -> String {
    if kind == "node" {
        format!(
            "| **{id}** | `{root}` | `{root}/AGENTS.md` | `{root}/docs/ROADMAP.md` | `npm test` | n/a (node; GSV loc-audit does not apply) |"
        )
    } else {
        format!(
            "| **{id}** | `{root}` | `{root}/docs/HANDOFF_NEW_SESSION.md` | `{root}/docs/NEXT_SESSION_PROMPT.md` | `cargo fmt --all` -> `cargo clippy --all-targets -- -D warnings` -> `cargo test` (adopted via `cargo xtask adopt`; adjust commands in this row) | Rust **95-100%** |"
        )
    }
}

/// Insert the row after the last `| **` line of the first markdown table.
pub fn append_products_row(text: &str, row: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let sep = lines.iter().position(|l| l.starts_with("|--"))?;
    let mut insert_at = sep;
    for (i, l) in lines.iter().enumerate().skip(sep + 1) {
        if !l.starts_with('|') {
            break;
        }
        if l.starts_with("| **") {
            insert_at = i;
        }
    }
    let mut out: Vec<&str> = Vec::with_capacity(lines.len() + 1);
    for (i, l) in lines.iter().enumerate() {
        out.push(l);
        if i == insert_at {
            out.push(row);
        }
    }
    Some(out.join("\n"))
}

/// Insert the folder block right after the GSV-first `.` entry. Closes either
/// `},` or a trailing `}` (single-entry workspace) so JSON stays valid.
pub fn insert_workspace_folder(text: &str, name: &str, rel: &str) -> Option<String> {
    let any = format!("\"path\": \"{rel}\"");
    if text.contains(&any) {
        return Some(text.to_string());
    }
    let lines: Vec<&str> = text.lines().collect();
    let anchor = lines.iter().position(|l| l.contains("\"path\": \".\""))?;
    let close = lines[anchor..]
        .iter()
        .position(|l| {
            let t = l.trim();
            t == "}," || t == "}"
        })
        .map(|o| anchor + o)?;
    let mut out: Vec<String> = Vec::with_capacity(lines.len() + 4);
    for (i, l) in lines.iter().enumerate() {
        if i == close {
            out.push("    },".into());
            out.push("    {".into());
            out.push(format!("      \"name\": \"{name}\","));
            out.push(format!("      \"path\": \"{rel}\""));
            out.push("    },".into());
            continue;
        }
        out.push((*l).to_string());
    }
    Some(out.join("\n"))
}

/// Dry digest + artifact draft for a sibling git tree under the rust folder.
/// Never mutates; pair with [`apply`].
pub fn plan(kit_root: &Path, path_str: &str) -> Result<AdoptPlan, String> {
    let resolved: PathBuf = Path::new(path_str)
        .canonicalize()
        .or_else(|_| {
            let guess = kit_root.parent().unwrap_or(kit_root).join(path_str);
            guess.canonicalize()
        })
        .map_err(|_| format!("adopt: path not found: {path_str}"))?;
    if !resolved.is_dir() {
        return Err(format!("adopt: not a directory: {}", display(&resolved)));
    }
    let expected_parent = kit_root
        .parent()
        .map(|p| display(p).to_lowercase())
        .unwrap_or_default();
    let parent = resolved
        .parent()
        .map(|p| display(p).to_lowercase())
        .unwrap_or_default();
    if expected_parent.is_empty() || parent != expected_parent {
        return Err(format!(
            "adopt: expects a sibling git tree under {} (got {})",
            crate::boxes::products::display_path(kit_root.parent().unwrap_or(kit_root)),
            display(&resolved)
        ));
    }
    if !has_git(&resolved) {
        return Err(format!(
            "adopt: {} is not a git tree — S0 it first (`git init` there, commit, then re-adopt)",
            display(&resolved)
        ));
    }
    let id = slug_of(&resolved);
    if id == "gsv" {
        return Err("adopt: the host kit cannot adopt itself".into());
    }
    let dir_name = resolved
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| id.clone());
    let cargo = resolved.join("Cargo.toml").is_file();
    let kind = if cargo {
        "rust"
    } else if resolved.join("package.json").is_file() {
        "node"
    } else {
        "git"
    };
    let pin = resolved.join("rust-toolchain.toml");
    let agents = resolved.join("AGENTS.md");
    let handoff = resolved.join("docs/HANDOFF_NEW_SESSION.md");
    let next = resolved.join("docs/NEXT_SESSION_PROMPT.md");
    let registered = {
        let text = read(&kit_root.join(PRODUCTS_MD));
        let needle = format!("| **{id}**");
        text.to_ascii_lowercase()
            .contains(&needle.to_ascii_lowercase())
    };
    let root_disp = display(&resolved);
    let mut notes: Vec<String> = Vec::new();
    let dirty = git_capture(&resolved, &["status", "--porcelain"])
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count();
    if dirty > 0 {
        notes.push(format!(
            "dirty git tree ({dirty} path(s)) — commit before the first drained gate"
        ));
    }
    if registered {
        notes.push("already registered in PRODUCTS.md — row patch skipped".into());
    }
    if read(&pin).contains("channel") {
        notes.push("rust-toolchain.toml already present — pin artifact skipped".into());
    }
    if !cargo {
        notes.push("no Cargo.toml — toolchain pin skipped (not a rust crate)".into());
    }

    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let mut artifacts: Vec<AdoptArtifact> = Vec::new();
    if !agents.is_file() {
        artifacts.push(AdoptArtifact {
            path: format!("{root_disp}/AGENTS.md"),
            op: "write".into(),
            content: Some(agents_md(&dir_name, kit_root)),
            detail: "thin AGENTS.md from docs/gsv/PLUGIN_AGENTS.md".into(),
        });
    } else {
        notes.push("AGENTS.md already present — kept".into());
    }
    if cargo && !pin.is_file() {
        artifacts.push(AdoptArtifact {
            path: format!("{root_disp}/rust-toolchain.toml"),
            op: "write".into(),
            content: Some(PIN_TOML.into()),
            detail: "stable-gnu ecosystem pin (floats 1.99.1 -> 1.100)".into(),
        });
    }
    if !handoff.is_file() {
        artifacts.push(AdoptArtifact {
            path: format!("{root_disp}/docs/HANDOFF_NEW_SESSION.md"),
            op: "write".into(),
            content: Some(handoff_skeleton(&id, &root_disp, &date)),
            detail: "HANDOFF skeleton (fill before first drain)".into(),
        });
    } else {
        notes.push("HANDOFF_NEW_SESSION.md already present — kept".into());
    }
    if !next.is_file() {
        artifacts.push(AdoptArtifact {
            path: format!("{root_disp}/docs/NEXT_SESSION_PROMPT.md"),
            op: "write".into(),
            content: Some(next_skeleton(&id, &date)),
            detail: "NEXT skeleton".into(),
        });
    } else {
        notes.push("NEXT_SESSION_PROMPT.md already present — kept".into());
    }
    let products_text = read(&kit_root.join(PRODUCTS_MD));
    let row = products_row(&id, &root_disp, kind);
    if !registered {
        match append_products_row(&products_text, &row) {
            Some(updated) => artifacts.push(AdoptArtifact {
                path: format!("{}/{}", display(kit_root), PRODUCTS_MD),
                op: "patch-products-row".into(),
                content: Some(updated),
                detail: format!("row: {row}"),
            }),
            None => notes.push("PRODUCTS.md table not found — row appended by hand".into()),
        }
    }
    let ws_text = read(&kit_root.join(WORKSPACE));
    match insert_workspace_folder(&ws_text, &dir_name, &format!("../{dir_name}")) {
        Some(updated) if updated != ws_text => artifacts.push(AdoptArtifact {
            path: format!("{}/{}", display(kit_root), WORKSPACE),
            op: "patch-workspace-folder".into(),
            content: Some(updated),
            detail: format!("folders += {{ name: {dir_name}, path: ../{dir_name} }} (GSV first)"),
        }),
        _ => notes.push("already wired into gsv.code-workspace — folder skipped".into()),
    }

    Ok(AdoptPlan {
        ok: true,
        id,
        name: dir_name.clone(),
        root: root_disp,
        kind: kind.into(),
        registered,
        git_head: git_capture(&resolved, &["rev-parse", "--short", "HEAD"]),
        dirty_files: dirty,
        pin_present: read(&pin).contains("channel"),
        cargo_name: if cargo { package_name(&resolved) } else { None },
        handoff_present: handoff.is_file(),
        next_present: next.is_file(),
        agents_present: agents.is_file(),
        ports: port_survey(&resolved),
        artifacts,
        notes,
    })
}

/// Render the dry preview (deterministic; no trailing mutation words).
pub fn render_preview(plan: &AdoptPlan) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "adopt: {} kind={} id={} root={}\n",
        plan.name, plan.kind, plan.id, plan.root
    ));
    out.push_str(&format!(
        "  git_head={} dirty_files={} registered={} pin={} handoff={} next={}\n",
        if plan.git_head.is_empty() {
            "-"
        } else {
            &plan.git_head
        },
        plan.dirty_files,
        plan.registered,
        plan.pin_present,
        plan.handoff_present,
        plan.next_present,
    ));
    if let Some(n) = &plan.cargo_name {
        out.push_str(&format!("  cargo={n}\n"));
    }
    if !plan.ports.is_empty() {
        let list: Vec<String> = plan.ports.iter().map(|p| p.to_string()).collect();
        out.push_str(&format!("  ports={}\n", list.join(",")));
    }
    for a in &plan.artifacts {
        out.push_str(&format!("  [{}] {} — {}\n", a.op, a.path, a.detail));
    }
    for n in &plan.notes {
        out.push_str(&format!("  note: {n}\n"));
    }
    out.push_str(
        "  dry-run preview only — repeat with `cargo xtask adopt <path> --yes` to write\n",
    );
    out
}

/// Execute the plan (`--yes`). Skips anything that already materialized since
/// the plan (idempotent), keeps untouched files as-is, and returns log lines.
pub fn apply(_kit_root: &Path, plan: &AdoptPlan) -> Result<Vec<String>, String> {
    let mut log = Vec::new();
    for a in &plan.artifacts {
        match a.op.as_str() {
            "write" => {
                let p = de_display(&a.path);
                if p.is_file() {
                    log.push(format!("skip (exists): {}", a.path));
                    continue;
                }
                if let Some(dir) = p.parent() {
                    fs::create_dir_all(dir).map_err(|e| format!("adopt: {}: {e}", display(dir)))?;
                }
                let content = a
                    .content
                    .as_deref()
                    .ok_or_else(|| format!("adopt: write without content: {}", a.path))?;
                fs::write(&p, content).map_err(|e| format!("adopt: {}: {e}", a.path))?;
                log.push(format!("wrote: {}", a.path));
            }
            "patch-products-row" | "patch-workspace-folder" => {
                let p = de_display(&a.path);
                let content = a
                    .content
                    .as_deref()
                    .ok_or_else(|| format!("adopt: patch without content: {}", a.path))?;
                let before = read(&p);
                if before == content {
                    log.push(format!("skip (already patched): {}", a.path));
                    continue;
                }
                fs::write(&p, content).map_err(|e| format!("adopt: {}: {e}", a.path))?;
                log.push(format!("patched: {} ({})", a.path, a.detail));
            }
            other => return Err(format!("adopt: unknown artifact op: {other}")),
        }
    }
    if log.is_empty() {
        log.push("adopt: nothing to write (already wired)".into());
    }
    log.push(
        "routing: `cargo xtask products` picks the tree up automatically (scan is the list)".into(),
    );
    Ok(log)
}

/// `S:/rust/x/y` -> Windows path (display format is our own patch/write key).
fn de_display(s: &str) -> PathBuf {
    let raw = s.replace('/', "\\");
    PathBuf::from(raw.trim_start_matches("\\\\?\\"))
}

/// CLI shim: plan, print (or apply after `--yes`), never itself gates tests.
pub fn run(kit_root: &Path, path: &str, yes: bool) -> Result<String, String> {
    let plan = plan(kit_root, path)?;
    if !yes {
        return Ok(render_preview(&plan));
    }
    let mut out = render_preview(&plan);
    for line in apply(kit_root, &plan)? {
        out.push_str(&line);
        out.push('\n');
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "gsv-adopt-{}-{}-{}",
            tag,
            std::process::id(),
            chrono::Utc::now().timestamp_subsec_millis()
        ));
        fs::create_dir_all(&dir).expect("temp kit");
        dir
    }

    fn kit_shell(tag: &str) -> PathBuf {
        let kit = temp_root(tag);
        fs::create_dir_all(kit.join("docs/gsv")).unwrap();
        fs::write(
            kit.join("docs/gsv/PRODUCTS.md"),
            "# Products\n\n| id | Root |\n|----|------|\n| **gsv** | `S:/rust/GSV` |\n\n periods |\n",
        )
        .unwrap();
        fs::write(
            kit.join(WORKSPACE),
            "{\n  \"folders\": [\n    {\n      \"name\": \"GSV\",\n      \"path\": \".\"\n    }\n  ]\n}\n",
        )
        .unwrap();
        fs::write(
            kit.join("docs/gsv/PLUGIN_AGENTS.md"),
            "# PLUGIN_AGENTS.md — template\n\n\
             This tree is a **portable plugin** of the rust-folder environment (`S:/rust`).\n",
        )
        .unwrap();
        kit
    }

    fn sibling_gtree(kit: &Path, name: &str, src: &str) -> PathBuf {
        let p = kit.parent().unwrap().join(name);
        fs::create_dir_all(p.join("src")).unwrap();
        fs::create_dir_all(p.join(".git")).unwrap();
        fs::write(p.join("Cargo.toml"), "[package]\nname = \"demo\"\n").unwrap();
        fs::write(
            p.join("src/main.rs"),
            "fn main() {\n    let l = tokio::net::TcpListener::bind(\"0.0.0.0:9741\");\n}\n",
        )
        .unwrap();
        if !src.is_empty() {
            fs::write(p.join("src/seed.rs"), src).unwrap();
        }
        p
    }

    #[test]
    fn plan_requires_git_tree() {
        let kit = kit_shell("nogit");
        let p = kit.parent().unwrap().join("bare");
        fs::create_dir_all(&p).unwrap();
        let err = plan(&kit, &display(&p)).unwrap_err();
        assert!(err.contains("not a git tree"), "{err}");
        let _ = fs::remove_dir_all(std::env::temp_dir().join("gsv-adopt-nogit"));
        let _ = fs::remove_dir_all(&p);
    }

    #[test]
    fn plan_requires_sibling() {
        let kit = kit_shell("deep");
        let deep = kit.join("nested/hello");
        fs::create_dir_all(deep.join(".git")).unwrap();
        let err = plan(&kit, &display(&deep)).unwrap_err();
        assert!(err.contains("sibling git tree"), "{err}");
        let _ = fs::remove_dir_all(&kit);
    }

    #[test]
    fn plan_digests_and_apply_wires() {
        let outer = temp_root("kit0");
        let kit = outer.join("GSV");
        fs::create_dir_all(kit.join("docs/gsv")).unwrap();
        fs::write(
            kit.join("docs/gsv/PRODUCTS.md"),
            "| id | Root |\n|----|------|\n| **gsv** | `S:/rust/GSV` |\n",
        )
        .unwrap();
        fs::write(
            kit.join(WORKSPACE),
            "{\n  \"folders\": [\n    {\n      \"name\": \"GSV\",\n      \"path\": \".\"\n    }\n  ]\n}\n",
        )
        .unwrap();
        fs::write(
            kit.join("docs/gsv/PLUGIN_AGENTS.md"),
            "This tree is a **portable plugin** of the rust-folder environment (`S:/rust`).\n",
        )
        .unwrap();
        let tree = sibling_gtree(&kit, "demo-tree", "");
        tree.canonicalize().unwrap();
        let p = plan(&kit, &display(&tree)).expect("plan");
        assert_eq!(p.id, "demo-tree");
        assert_eq!(p.kind, "rust");
        assert!(p.ports.contains(&9741), "ports={:?}", p.ports);
        assert!(!p.registered);
        let ops: HashSet<&str> = p.artifacts.iter().map(|a| a.op.as_str()).collect();
        assert!(ops.contains("write"));
        assert!(ops.contains("patch-products-row"));
        assert!(ops.contains("patch-workspace-folder"));
        // dry: nothing materializes
        let agents = tree.join("AGENTS.md");
        assert!(!agents.is_file());
        // apply
        let log = apply(&kit, &p).expect("apply");
        assert!(log.iter().any(|l| l.contains("wrote:")), "{log:?}");
        assert!(agents.is_file(), "AGENTS.md written");
        assert!(tree.join("rust-toolchain.toml").is_file());
        assert!(tree.join("docs/HANDOFF_NEW_SESSION.md").is_file());
        assert!(tree.join("docs/NEXT_SESSION_PROMPT.md").is_file());
        let products = read(&kit.join("docs/gsv/PRODUCTS.md"));
        assert!(products.contains("| **demo-tree** |"), "{products}");
        let ws = read(&kit.join(WORKSPACE));
        assert!(ws.contains("\"../demo-tree\""), "{ws}");
        assert!(ws.find("\"path\": \".\"").unwrap() < ws.find("\"../demo-tree\"").unwrap());
        // idempotent second round
        let plan_two = plan(&kit, &display(&tree)).unwrap();
        assert!(plan_two.registered);
        assert!(plan_two.artifacts.is_empty(), "{:?}", plan_two.artifacts);
        let log2 = apply(&kit, &plan_two).unwrap();
        assert!(
            log2.iter().any(|l| l.contains("nothing to write")),
            "{log2:?}"
        );
        let _ = fs::remove_dir_all(&outer);
    }

    #[test]
    fn products_row_goes_after_last_row() {
        let text = "| id | R |\n|----|---|\n| **a** | x |\n| **b** | y |\n\ntail\n";
        let out = append_products_row(text, "| **c** | z |").unwrap();
        assert!(out.contains("| **c** | z |"));
        let lines: Vec<&str> = out.lines().collect();
        let b = lines.iter().position(|l| l.starts_with("| **b**")).unwrap();
        let c = lines.iter().position(|l| l.starts_with("| **c**")).unwrap();
        assert_eq!(c, b + 1);
    }

    #[test]
    fn workspace_keeps_gsv_first() {
        let ws = "{\n  \"folders\": [\n    {\n      \"name\": \"GSV\",\n      \"path\": \".\"\n    }\n  ]\n}\n";
        let out = insert_workspace_folder(ws, "Foo", "../Foo").unwrap();
        let dot = out.find("\".\"").unwrap();
        let foo = out.find("../Foo").unwrap();
        assert!(dot < foo);
        // idempotent
        assert_eq!(
            out.clone(),
            insert_workspace_folder(&out, "Foo", "../Foo").unwrap()
        );
    }

    #[test]
    fn ports_skip_comments_and_fluff() {
        let src_dir = temp_root("ports");
        fs::create_dir_all(src_dir.join("src")).unwrap();
        fs::write(
            src_dir.join("src/on.rs"),
            "// note /* :12345 comment */\nlet _ = TcpListener::bind(\"0.0.0.0:8091\");\n",
        )
        .unwrap();
        fs::write(src_dir.join("src/lib.rs"), "fn f() {}\n").unwrap();
        let ports = port_survey(&src_dir);
        assert_eq!(ports, vec![8091]);
        let _ = fs::remove_dir_all(&src_dir);
    }
}
