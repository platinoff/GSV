//! GitHub origin lockstep — remote version / HEAD / issues.
//!
//! The live Update box used to look only at **this** tree (`src/` mtime and
//! on-disk `Cargo.toml`). A joiner who cloned an older tag never saw an update
//! even when `origin` on GitHub was ahead. This box probes GitHub (or a dry-run
//! stub) so Galaxy / MCP can say "pull then rebuild" without treating that as a
//! watchdog recopy.
//!
//! Cargo tests and `GSV_GITHUB_DRY_RUN=1` never open sockets. `gsv-server`
//! calls [`enable_live_api`] then [`spawn_refresh_loop`].

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::fingerprint;

/// Default origin when `git remote` / Cargo.toml repository is missing.
pub const DEFAULT_REPO: &str = "platinoff/GSV";
/// Process env that forces the stub (`1` / `true`).
pub const DRY_RUN_ENV: &str = "GSV_GITHUB_DRY_RUN";
/// Process env that makes the stub report origin ahead (unit tests).
pub const STUB_AHEAD_ENV: &str = "GSV_GITHUB_STUB_AHEAD";

const API_ROOT: &str = "https://api.github.com";
const RAW_ROOT: &str = "https://raw.githubusercontent.com";
const REFRESH: Duration = Duration::from_secs(300);
const HTTP_TIMEOUT: Duration = Duration::from_secs(3);

static LIVE_API: AtomicBool = AtomicBool::new(false);
static CACHE: Mutex<Option<(Instant, OriginProbe)>> = Mutex::new(None);
static ISSUES: Mutex<Vec<GhIssue>> = Mutex::new(Vec::new());

/// Allow outbound GitHub HTTPS (call from `gsv-server` / `gsv-mcp` mains only).
pub fn enable_live_api() {
    LIVE_API.store(true, Ordering::SeqCst);
}

/// True after [`enable_live_api`]. Cargo tests leave this false.
pub fn live_api_enabled() -> bool {
    LIVE_API.load(Ordering::SeqCst)
}

/// `GSV_GITHUB_DRY_RUN=1` (or `true`).
pub fn env_dry_run() -> bool {
    std::env::var(DRY_RUN_ENV)
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

fn stub_ahead_env() -> bool {
    std::env::var(STUB_AHEAD_ENV)
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

fn is_test_harness() -> bool {
    std::env::current_exe()
        .ok()
        .map(|p| p.to_string_lossy().replace('\\', "/").contains("/deps/"))
        .unwrap_or(false)
}

fn use_stub() -> bool {
    env_dry_run() || !live_api_enabled() || is_test_harness()
}

fn crate_ver(repo_root: &Path, running: &str) -> String {
    fingerprint::pkg_version(repo_root).unwrap_or_else(|| running.to_string())
}

/// Cached origin probe (never `bot_token`, never a GitHub PAT).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OriginProbe {
    pub ok: bool,
    pub dry_run: bool,
    pub repo: String,
    pub local_version: String,
    pub local_head: String,
    pub remote_version: String,
    pub remote_head: String,
    /// True when GitHub / origin is ahead of this install (pull needed).
    pub github_ahead: bool,
    /// Human hint (`git pull && cargo build` when ahead).
    pub hint: String,
}

impl Default for OriginProbe {
    fn default() -> Self {
        Self {
            ok: true,
            dry_run: true,
            repo: DEFAULT_REPO.into(),
            local_version: String::new(),
            local_head: String::new(),
            remote_version: String::new(),
            remote_head: String::new(),
            github_ahead: false,
            hint: String::new(),
        }
    }
}

/// One GitHub issue mapped onto the ticket board.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GhIssue {
    pub number: u64,
    pub title: String,
    pub body: String,
    pub html_url: String,
}

/// `owner/repo` from a git remote URL.
pub fn parse_github_repo(url: &str) -> Option<String> {
    let t = url.trim();
    if t.is_empty() {
        return None;
    }
    let t = t.trim_end_matches(".git").trim_end_matches('/');
    if let Some(rest) = t.strip_prefix("git@github.com:") {
        let rest = rest.trim_start_matches('/');
        if rest.contains('/') {
            return Some(rest.to_string());
        }
    }
    for prefix in [
        "https://github.com/",
        "http://github.com/",
        "ssh://git@github.com/",
        "git://github.com/",
    ] {
        if let Some(rest) = t.strip_prefix(prefix) {
            let rest = rest.trim_start_matches('/');
            if rest.contains('/') {
                return Some(rest.to_string());
            }
        }
    }
    None
}

/// `[package].version` from a Cargo.toml snippet.
pub fn parse_pkg_version(toml_src: &str) -> Option<String> {
    let v: toml::Value = toml::from_str(toml_src).ok()?;
    v.get("package")?
        .get("version")?
        .as_str()
        .map(str::to_string)
}

fn parse_triple(ver: &str) -> (u64, u64, u64) {
    let s = ver.trim().trim_start_matches('v');
    // Numeric semver core only: drop any -pre / +build suffix so
    // "0.205.0-rc.1" parses as (0, 205, 0) instead of failing on "0-rc".
    let core = s.split(['-', '+']).next().unwrap_or(s);
    let mut it = core.split('.');
    let a = it.next().and_then(|x| x.parse().ok()).unwrap_or(0);
    let b = it.next().and_then(|x| x.parse().ok()).unwrap_or(0);
    let c = it.next().and_then(|x| x.parse().ok()).unwrap_or(0);
    (a, b, c)
}

/// True when the version carries a semver pre-release tag (`-rc.1`, `-beta`).
/// Build metadata (`+meta`) is precedence-neutral and does not count.
fn is_prerelease(ver: &str) -> bool {
    let s = ver.trim().trim_start_matches('v');
    s.contains('-')
}

/// True when `remote` is a greater semver than `local`. On an identical
/// numeric triple a plain release outranks its own pre-release
/// ("0.205.0" > "0.205.0-rc.1"); anything else ties.
pub fn version_gt(remote: &str, local: &str) -> bool {
    if remote.trim().is_empty() || local.trim().is_empty() {
        return false;
    }
    let r = parse_triple(remote);
    let l = parse_triple(local);
    if r != l {
        return r > l;
    }
    !is_prerelease(remote) && is_prerelease(local)
}

fn origin_url(repo_root: &Path) -> Option<String> {
    let out = crate::vision::command("git")
        .args([
            "-C",
            &repo_root.to_string_lossy(),
            "remote",
            "get-url",
            "origin",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn cargo_repository(repo_root: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(repo_root.join("Cargo.toml")).ok()?;
    let v: toml::Value = toml::from_str(&raw).ok()?;
    v.get("package")?
        .get("repository")?
        .as_str()
        .and_then(parse_github_repo)
}

/// Resolve `owner/repo` from origin, then Cargo.toml, then the GSV default.
pub fn origin_repo(repo_root: &Path) -> String {
    origin_url(repo_root)
        .and_then(|u| parse_github_repo(&u))
        .or_else(|| cargo_repository(repo_root))
        .unwrap_or_else(|| DEFAULT_REPO.to_string())
}

fn local_head(repo_root: &Path) -> String {
    crate::vision::git_head(repo_root).unwrap_or_default()
}

fn local_has_object(repo_root: &Path, sha: &str) -> bool {
    let sha = sha.trim();
    if sha.len() < 7 {
        return false;
    }
    crate::vision::command("git")
        .args(["-C", &repo_root.to_string_lossy(), "cat-file", "-e", sha])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Origin is ahead when the remote crate is newer, or HEAD is a commit we lack.
pub fn compute_ahead(
    local_ver: &str,
    remote_ver: &str,
    local_head: &str,
    remote_head: &str,
    have_remote: bool,
) -> bool {
    if version_gt(remote_ver, local_ver) {
        return true;
    }
    let local = local_head.trim();
    let remote = remote_head.trim();
    if remote.is_empty() || local.is_empty() || remote == local {
        return false;
    }
    !have_remote
}

fn ahead_hint(ahead: bool) -> String {
    if ahead {
        "git pull && cargo build, then Apply (local tree is not newer yet)".into()
    } else {
        String::new()
    }
}

/// Dry-run probe: same version as this crate unless [`STUB_AHEAD_ENV`].
pub fn stub_probe(local_version: &str, local_head: &str) -> OriginProbe {
    let ahead = stub_ahead_env();
    let remote_version = if ahead {
        bump_patch(local_version)
    } else {
        local_version.to_string()
    };
    let remote_head = if ahead {
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into()
    } else if local_head.is_empty() {
        "stubhead".into()
    } else {
        local_head.to_string()
    };
    OriginProbe {
        ok: true,
        dry_run: true,
        repo: DEFAULT_REPO.into(),
        local_version: local_version.to_string(),
        local_head: local_head.to_string(),
        remote_version,
        remote_head,
        github_ahead: ahead,
        hint: ahead_hint(ahead),
    }
}

fn bump_patch(ver: &str) -> String {
    let (a, b, c) = parse_triple(ver);
    format!("{a}.{b}.{}", c.saturating_add(1))
}

/// Two stub issues so cargo tests can hook GitHub without sockets.
pub fn stub_issues() -> Vec<GhIssue> {
    vec![
        GhIssue {
            number: 1,
            title: "GitHub origin lockstep".into(),
            body: "Installer must see an update when origin is ahead of the local crate.".into(),
            html_url: "https://github.com/platinoff/GSV/issues/1".into(),
        },
        GhIssue {
            number: 2,
            title: "Channel guest walk".into(),
            body: "Mate/guest pick from the board; host can hook GitHub issues.".into(),
            html_url: "https://github.com/platinoff/GSV/issues/2".into(),
        },
    ]
}

/// Board steps from GitHub issues (`GH#N title`, scenario `github-issues`).
pub fn issue_steps(issues: &[GhIssue]) -> Vec<(String, String)> {
    issues
        .iter()
        .map(|i| {
            let title = format!("GH#{} {}", i.number, i.title);
            let body = if i.body.is_empty() {
                i.html_url.clone()
            } else {
                format!("{}\n{}", i.body, i.html_url)
            };
            (title, body)
        })
        .collect()
}

fn store_cache(p: OriginProbe) {
    if let Ok(mut g) = CACHE.lock() {
        *g = Some((Instant::now(), p));
    }
}

/// Last probe if fresh; otherwise a stub so HTTP/MCP never blocks on GitHub.
pub fn cached_probe(repo_root: &Path, running: &str) -> OriginProbe {
    let local_ver = crate_ver(repo_root, running);
    let head = local_head(repo_root);
    if let Ok(g) = CACHE.lock() {
        if let Some((t, p)) = g.as_ref() {
            if t.elapsed() < REFRESH {
                return p.clone();
            }
        }
    }
    stub_probe(&local_ver, &head)
}

/// `github_ahead` from cache (false until the first refresh).
pub fn cached_ahead() -> bool {
    CACHE
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|(_, p)| p.github_ahead))
        .unwrap_or(false)
}

fn gh_headers() -> reqwest::header::HeaderMap {
    let mut h = reqwest::header::HeaderMap::new();
    if let Ok(v) = reqwest::header::HeaderValue::from_str("GSV (https://github.com/platinoff/GSV)")
    {
        h.insert(reqwest::header::USER_AGENT, v);
    }
    if let Ok(v) = reqwest::header::HeaderValue::from_str("application/vnd.github+json") {
        h.insert(reqwest::header::ACCEPT, v);
    }
    if let Ok(tok) = std::env::var("GITHUB_TOKEN").or_else(|_| std::env::var("GH_TOKEN")) {
        let t = tok.trim();
        if !t.is_empty() {
            if let Ok(v) = reqwest::header::HeaderValue::from_str(&format!("Bearer {t}")) {
                h.insert(reqwest::header::AUTHORIZATION, v);
            }
        }
    }
    h
}

async fn http_get_text(client: &reqwest::Client, url: &str) -> Option<String> {
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    resp.text().await.ok()
}

async fn http_get_json(client: &reqwest::Client, url: &str) -> Option<Value> {
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    resp.json().await.ok()
}

async fn fetch_remote(repo: &str) -> (String, String) {
    let client = match reqwest::Client::builder()
        .timeout(HTTP_TIMEOUT)
        .default_headers(gh_headers())
        .build()
    {
        Ok(c) => c,
        Err(_) => return (String::new(), String::new()),
    };
    let mut sha = String::new();
    for git_ref in ["main", "master"] {
        let url = format!("{API_ROOT}/repos/{repo}/commits/{git_ref}");
        if let Some(v) = http_get_json(&client, &url).await {
            if let Some(s) = v.get("sha").and_then(Value::as_str) {
                sha = s.to_string();
                break;
            }
        }
    }
    let mut ver = String::new();
    for git_ref in ["main", "master"] {
        let url = format!("{RAW_ROOT}/{repo}/{git_ref}/Cargo.toml");
        if let Some(txt) = http_get_text(&client, &url).await {
            if let Some(v) = parse_pkg_version(&txt) {
                ver = v;
                break;
            }
        }
    }
    (ver, sha)
}

async fn fetch_issues_live(repo: &str) -> Vec<GhIssue> {
    let client = match reqwest::Client::builder()
        .timeout(HTTP_TIMEOUT)
        .default_headers(gh_headers())
        .build()
    {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let url = format!("{API_ROOT}/repos/{repo}/issues?state=open&per_page=10");
    let Some(v) = http_get_json(&client, &url).await else {
        return Vec::new();
    };
    let Some(arr) = v.as_array() else {
        return Vec::new();
    };
    arr.iter()
        .filter(|i| i.get("pull_request").is_none())
        .filter_map(|i| {
            let number = i.get("number").and_then(Value::as_u64)?;
            let title = i.get("title").and_then(Value::as_str).unwrap_or("").trim();
            if title.is_empty() {
                return None;
            }
            Some(GhIssue {
                number,
                title: title.to_string(),
                body: i
                    .get("body")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim()
                    .to_string(),
                html_url: i
                    .get("html_url")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            })
        })
        .collect()
}

fn store_issues(issues: Vec<GhIssue>) {
    if let Ok(mut g) = ISSUES.lock() {
        *g = issues;
    }
}

/// Refresh the origin cache (stub in tests; HTTPS from live `gsv-server`).
pub async fn refresh(repo_root: &Path, running: &str) -> OriginProbe {
    let local_ver = crate_ver(repo_root, running);
    let head = local_head(repo_root);
    if use_stub() {
        let p = stub_probe(&local_ver, &head);
        store_cache(p.clone());
        store_issues(stub_issues());
        return p;
    }
    let repo = origin_repo(repo_root);
    let (remote_ver, remote_head) = fetch_remote(&repo).await;
    let have = local_has_object(repo_root, &remote_head);
    let ahead = compute_ahead(&local_ver, &remote_ver, &head, &remote_head, have);
    let p = OriginProbe {
        ok: true,
        dry_run: false,
        repo,
        local_version: local_ver,
        local_head: head,
        remote_version: remote_ver,
        remote_head,
        github_ahead: ahead,
        hint: ahead_hint(ahead),
    };
    store_cache(p.clone());
    store_issues(fetch_issues_live(&p.repo).await);
    p
}

/// Open GitHub issues (stub in tests; last live fetch otherwise).
pub fn issues_now() -> Vec<GhIssue> {
    if use_stub() {
        return stub_issues();
    }
    ISSUES.lock().map(|g| g.clone()).unwrap_or_default()
}

/// Background origin probe every 5 minutes (live `gsv-server` only).
pub fn spawn_refresh_loop(repo_root: PathBuf, running: String) {
    if !live_api_enabled() {
        return;
    }
    tokio::spawn(async move {
        loop {
            let _ = refresh(&repo_root, &running).await;
            tokio::time::sleep(REFRESH).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_github_repo_https_ssh_and_git() {
        assert_eq!(
            parse_github_repo("https://github.com/platinoff/GSV.git").as_deref(),
            Some("platinoff/GSV")
        );
        assert_eq!(
            parse_github_repo("git@github.com:platinoff/GSV.git").as_deref(),
            Some("platinoff/GSV")
        );
        assert_eq!(
            parse_github_repo("ssh://git@github.com/platinoff/GSV").as_deref(),
            Some("platinoff/GSV")
        );
        assert!(parse_github_repo("https://example.com/x").is_none());
    }

    #[test]
    fn parse_pkg_version_reads_package() {
        let toml = "[package]\nname=\"gsv\"\nversion=\"0.190.0\"\n";
        assert_eq!(parse_pkg_version(toml).as_deref(), Some("0.190.0"));
    }

    #[test]
    fn version_gt_semver() {
        assert!(version_gt("0.191.0", "0.190.0"));
        assert!(!version_gt("0.190.0", "0.190.0"));
        assert!(!version_gt("0.189.0", "0.190.0"));
        assert!(version_gt("1.0.0", "0.190.0"));
    }

    #[test]
    fn version_gt_prerelease_ranks_below_release() {
        // Pre-release suffix parses numerically (old code: "0-rc" → 0).
        assert!(version_gt("v0.205.0", "0.205.0-rc.1"));
        assert!(version_gt("0.205.0", "0.205.0-beta"));
        assert!(!version_gt("0.205.0-rc.1", "0.205.0"));
        // Same-triple pre-releases never outrank each other by suffix.
        assert!(!version_gt("0.205.0-rc.2", "0.205.0-rc.1"));
        // Build metadata is precedence-neutral.
        assert!(!version_gt("0.205.0", "0.205.0+build.7"));
        // A newer triple still wins regardless of suffixes.
        assert!(version_gt("0.206.0-rc.1", "0.205.9"));
    }

    #[test]
    fn compute_ahead_remote_version_or_missing_sha() {
        assert!(compute_ahead("0.180.0", "0.190.0", "aaa", "bbb", false));
        assert!(!compute_ahead("0.190.0", "0.190.0", "aaa", "aaa", true));
        assert!(compute_ahead("0.190.0", "0.190.0", "aaa", "bbb", false));
        assert!(!compute_ahead("0.190.0", "0.190.0", "aaa", "bbb", true));
    }

    #[test]
    fn stub_issues_are_board_ready() {
        let steps = issue_steps(&stub_issues());
        assert_eq!(steps.len(), 2);
        assert!(steps[0].0.starts_with("GH#1 "), "{}", steps[0].0);
        assert!(steps[1].0.starts_with("GH#2 "), "{}", steps[1].0);
    }

    #[test]
    fn stub_probe_matches_local_unless_ahead_env() {
        let p = stub_probe("0.190.0", "abc");
        assert!(p.ok);
        assert!(p.dry_run);
        assert_eq!(p.repo, DEFAULT_REPO);
        if stub_ahead_env() {
            assert!(p.github_ahead);
            assert!(version_gt(&p.remote_version, "0.190.0"));
        } else {
            assert!(!p.github_ahead);
            assert_eq!(p.remote_version, "0.190.0");
        }
    }
}
