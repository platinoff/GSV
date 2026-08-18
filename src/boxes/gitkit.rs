//! VDT git automation — `cargo xtask git <status|log|fetch|commit|push>`.
//!
//! Replaces local `comitmsg/*.sh` wrappers. Commit messages are `comitmsg/*.md`.
//! Logs may be `comitmsg/*.log`. This tool never runs `git add -A` and never
//! stages secrets or message files (except `comitmsg/README.md`).

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::vision;

/// Allowlisted `cargo xtask git` verbs.
pub const GIT_CMDS: &[&str] = &["status", "log", "fetch", "commit", "push"];

/// Help for `cargo xtask git` with no args / `--help`.
pub fn help_text() -> String {
    "Usage: cargo xtask git <status|log|fetch|commit|push>\n\n\
     status   git status -sb\n\
     log      git log -1 --oneline\n\
     fetch    git fetch\n\
     commit   git commit --file comitmsg/<name>.md  (no add -A)\n\
     push     git push origin main\n\n\
     Messages: comitmsg/*.md   Logs: comitmsg/*.log   Never stage comitmsg/* except README.md.\n"
        .into()
}

/// True when a path must not be staged (secrets, data, commit-message files).
pub fn forbidden_stage(path: &str) -> bool {
    let n = path.replace('\\', "/");
    let base = n.rsplit('/').next().unwrap_or(&n);
    if base.starts_with(".env") || n.contains("/.env") {
        return true;
    }
    if n.ends_with(".pem") || n.ends_with(".key") {
        return true;
    }
    if n.contains("/certs/") && n.ends_with(".pem") {
        return true;
    }
    if (n.contains("/data/") || n.starts_with("data/"))
        && !n.ends_with("/.gitkeep")
        && base != ".gitkeep"
    {
        return true;
    }
    if n.contains("/comitmsg/") || n.starts_with("comitmsg/") {
        return !n.ends_with("comitmsg/README.md");
    }
    false
}

/// Resolve `--file` to an existing `comitmsg/*.md` under `repo`.
pub fn resolve_commit_file(repo: &Path, file: &Path) -> Result<PathBuf, String> {
    if file.as_os_str().is_empty() {
        return Err("commit requires --file comitmsg/<name>.md".into());
    }
    let raw = file.to_string_lossy().replace('\\', "/");
    if raw.contains("..") {
        return Err("commit --file must not contain ..".into());
    }
    if raw.ends_with(".txt") {
        return Err("comitmsg messages are .md (not .txt)".into());
    }
    let joined = if file.is_absolute() {
        file.to_path_buf()
    } else {
        repo.join(file)
    };
    let canon = joined
        .canonicalize()
        .map_err(|_| format!("commit file not found: {}", file.display()))?;
    let comit = repo
        .join("comitmsg")
        .canonicalize()
        .map_err(|_| "comitmsg/ missing".to_string())?;
    if !canon.starts_with(&comit) {
        return Err("commit --file must be under comitmsg/".into());
    }
    match canon.extension().and_then(|e| e.to_str()) {
        Some("md") => Ok(canon),
        other => Err(format!(
            "commit --file must be a .md message (got {other:?})"
        )),
    }
}

fn apply_head_identity(cmd: &mut Command, repo: &Path) {
    let configured = vision::command("git")
        .current_dir(repo)
        .args(["config", "--get", "user.email"])
        .output()
        .ok()
        .filter(|o| o.status.success() && !String::from_utf8_lossy(&o.stdout).trim().is_empty());
    if configured.is_some() {
        return;
    }
    let Ok((ok, text)) = capture(repo, &["log", "-1", "--format=%an%n%ae"]) else {
        return;
    };
    if !ok {
        return;
    }
    let mut lines = text.lines().map(str::trim).filter(|s| !s.is_empty());
    let Some(name) = lines.next() else {
        return;
    };
    let Some(email) = lines.next() else {
        return;
    };
    cmd.env("GIT_AUTHOR_NAME", name)
        .env("GIT_AUTHOR_EMAIL", email)
        .env("GIT_COMMITTER_NAME", name)
        .env("GIT_COMMITTER_EMAIL", email);
}

fn capture(cwd: &Path, args: &[&str]) -> Result<(bool, String), String> {
    let mut cmd = vision::command("git");
    cmd.current_dir(cwd).args(args);
    if args.first() == Some(&"commit") {
        apply_head_identity(&mut cmd, cwd);
    }
    let o = cmd.output().map_err(|e| format!("git: {e}"))?;
    let mut text = String::from_utf8_lossy(&o.stdout).into_owned();
    let err = String::from_utf8_lossy(&o.stderr);
    if !err.is_empty() {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&err);
    }
    Ok((o.status.success(), text))
}

fn parse_commit_file(args: &[String]) -> Result<PathBuf, String> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--file" || args[i] == "-F" {
            let path = args
                .get(i + 1)
                .ok_or_else(|| "commit --file needs a path".to_string())?;
            return Ok(PathBuf::from(path));
        }
        if let Some(rest) = args[i].strip_prefix("--file=") {
            return Ok(PathBuf::from(rest));
        }
        if args[i] == "-m"
            || args[i] == "--message"
            || args[i] == "--amend"
            || args[i] == "--no-verify"
        {
            return Err("cargo xtask git commit only accepts --file comitmsg/*.md".into());
        }
        i += 1;
    }
    Err("commit requires --file comitmsg/<name>.md".into())
}

/// Dispatch allowlisted git verbs. Unknown names error (no raw git passthrough).
pub fn run(repo: &Path, args: &[String]) -> Result<String, String> {
    if args.is_empty()
        || args
            .iter()
            .any(|a| a == "--help" || a == "-h" || a == "help")
    {
        return Ok(help_text());
    }
    match args[0].as_str() {
        "status" => {
            let (ok, text) = capture(repo, &["status", "-sb"])?;
            if ok {
                Ok(text)
            } else {
                Err(text)
            }
        }
        "log" => {
            let (ok, text) = capture(repo, &["log", "-1", "--oneline"])?;
            if ok {
                Ok(text)
            } else {
                Err(text)
            }
        }
        "fetch" => {
            let (ok, text) = capture(repo, &["fetch"])?;
            if ok {
                Ok(text)
            } else {
                Err(text)
            }
        }
        "commit" => {
            let file = parse_commit_file(&args[1..])?;
            let path = resolve_commit_file(repo, &file)?;
            let name = path
                .file_name()
                .ok_or_else(|| "commit file has no name".to_string())?
                .to_string_lossy();
            let rel = format!("comitmsg/{name}");
            let (ok, text) = capture(repo, &["commit", "--file", &rel])?;
            let mut out = text;
            let (_, st) = capture(repo, &["status", "-sb"])?;
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("=== git status ===\n");
            out.push_str(&st);
            if ok {
                Ok(out)
            } else {
                Err(out)
            }
        }
        "push" => {
            let (ok_st, status) = capture(repo, &["status", "-sb"])?;
            let _ = ok_st;
            let o = vision::command("git")
                .current_dir(repo)
                .args(["push", "origin", "main"])
                .output()
                .map_err(|e| format!("git push: {e}"))?;
            let mut out = format!("=== git status ===\n{status}\n=== git push origin main ===\n");
            out.push_str(&String::from_utf8_lossy(&o.stdout));
            out.push_str(&String::from_utf8_lossy(&o.stderr));
            if !o.status.success() {
                return Err(out);
            }
            out.push_str("=== done ===\n");
            Ok(out)
        }
        other => Err(format!(
            "unknown git verb '{other}' — use status|log|fetch|commit|push"
        )),
    }
}

/// `cloudflared tunnel --url http://{host}:{port}` argv (does not spawn).
pub fn tunnel_argv(host: &str, port: u16) -> Vec<String> {
    vec![
        "cloudflared".into(),
        "tunnel".into(),
        "--url".into(),
        format!("http://{host}:{port}"),
    ]
}

/// Locate `cloudflared` on PATH.
pub fn find_cloudflared() -> Option<PathBuf> {
    find_on_path("cloudflared")
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let exe = if cfg!(windows) && !name.ends_with(".exe") {
        format!("{name}.exe")
    } else {
        name.to_string()
    };
    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        let p = dir.join(&exe);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

/// Owner-opt-in public tunnel to loopback GSV. Never auto-started.
pub fn run_tunnel(host: &str, port: u16) -> Result<String, String> {
    if crate::boxes::update::is_cargo_test_harness() {
        return Err("tunnel skipped in cargo-test harness".into());
    }
    let bin = find_cloudflared().ok_or_else(|| {
        "cloudflared not on PATH — install it, then: cargo xtask tunnel".to_string()
    })?;
    let argv = tunnel_argv(host, port);
    let url = argv.last().cloned().unwrap_or_default();
    eprintln!("gsv-tunnel: owner opt-in. {url} will be public (including /mcp). Ctrl+C stops it.");
    let st = Command::new(&bin)
        .args(&argv[1..])
        .status()
        .map_err(|e| format!("cloudflared: {e}"))?;
    if st.success() {
        Ok("tunnel ended".into())
    } else {
        Err(format!("cloudflared exited {st}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn forbidden_stage_blocks_secrets_and_comitmsg() {
        assert!(forbidden_stage(".env"));
        assert!(forbidden_stage("data/gsv_usage.json"));
        assert!(forbidden_stage("comitmsg/.band156-commit-msg.md"));
        assert!(forbidden_stage("certs/server.pem"));
        assert!(!forbidden_stage("comitmsg/README.md"));
        assert!(!forbidden_stage("src/boxes/gitkit.rs"));
        assert!(!forbidden_stage("data/.gitkeep"));
    }

    #[test]
    fn resolve_commit_file_requires_md_under_comitmsg() {
        let root = std::env::temp_dir().join(format!("gsv-gitkit-{}", std::process::id()));
        let comit = root.join("comitmsg");
        fs::create_dir_all(&comit).expect("dir");
        let md = comit.join("band156.md");
        fs::write(&md, "feat: test\n").expect("write");
        fs::write(comit.join("old.txt"), "nope\n").expect("txt");
        fs::write(root.join("outside.md"), "x\n").expect("out");

        let ok = resolve_commit_file(&root, Path::new("comitmsg/band156.md")).expect("md");
        assert!(ok.ends_with("band156.md"), "{}", ok.display());

        let txt = resolve_commit_file(&root, Path::new("comitmsg/old.txt")).unwrap_err();
        assert!(txt.contains(".md"), "{txt}");

        let trav = resolve_commit_file(&root, Path::new("comitmsg/../outside.md")).unwrap_err();
        assert!(trav.contains("..") || trav.contains("comitmsg"), "{trav}");

        let missing = resolve_commit_file(&root, Path::new("comitmsg/nope.md")).unwrap_err();
        assert!(missing.contains("not found"), "{missing}");
    }

    #[test]
    fn unknown_verb_rejected() {
        let err = run(Path::new("."), &["rebase".into()]).unwrap_err();
        assert!(err.contains("unknown"), "{err}");
    }

    #[test]
    fn tunnel_argv_points_at_loopback() {
        let a = tunnel_argv("127.0.0.1", 9999);
        assert_eq!(a[0], "cloudflared");
        assert_eq!(a[1], "tunnel");
        assert_eq!(a[3], "http://127.0.0.1:9999");
    }

    #[test]
    fn help_names_md_not_txt() {
        let h = help_text();
        assert!(h.contains("comitmsg/*.md"), "{h}");
        assert!(h.contains(".log"), "{h}");
        assert!(!h.contains("*.txt"), "{h}");
    }
}
