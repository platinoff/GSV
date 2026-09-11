//! GSV UI probe — Firefox click-through audit of the live Galaxy page.
//!
//! Drives geckodriver (`target/live/geckodriver.exe`) against a headless
//! Firefox: opens the Galaxy UI, injects error/fetch/console observers,
//! clicks every *safe* `data-action` per group, and writes a JSON report
//! (`target/live/ui_probe_report.json`) + screenshots. Destructive actions
//! (settings save, product open, ticket mutations, ranks review, update
//! apply) are recorded as `skipped-destructive`, never clicked.
//!
//! `cargo run --bin gsv-ui-probe` (server must be up on :9999).

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

use serde_json::{json, Value};

const DEFAULT_URL: &str = "http://127.0.0.1:9999/";
const DEFAULT_GD: &str = "http://127.0.0.1:4444";
const DEFAULT_FIREFOX: &str = r"C:\Program Files\Mozilla Firefox\firefox.exe";
const GROUPS: [&str; 4] = ["sprint", "vision", "ops", "studio"];

/// Actions that mutate server state or launch external apps — never clicked.
pub fn is_destructive(action: &str) -> bool {
    matches!(
        action,
        "settings-save"
            | "product-open"
            | "tickets-create"
            | "tickets-claim"
            | "tickets-done"
            | "tickets-error"
            | "tickets-walk"
            | "tickets-from-scenario"
            | "tickets-hook"
            | "tickets-bench"
            | "ranks-review"
            | "apply-update"
            | "update-apply"
    )
}

/// Actions that toggle a mode and must be clicked twice to restore.
pub fn is_toggle(action: &str) -> bool {
    matches!(
        action,
        "auto-toggle" | "power-toggle" | "card-min" | "card-fs"
    )
}

struct Args {
    url: String,
    gd: String,
    geckodriver: PathBuf,
    firefox: String,
    kit_root: PathBuf,
}

fn parse_args() -> Args {
    let argv: Vec<String> = std::env::args().collect();
    let mut kit_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut url = DEFAULT_URL.to_string();
    let mut gd = DEFAULT_GD.to_string();
    let mut firefox = DEFAULT_FIREFOX.to_string();
    let mut i = 1;
    while i + 1 < argv.len() {
        match argv[i].as_str() {
            "--url" => url = argv[i + 1].clone(),
            "--gd" => gd = argv[i + 1].clone(),
            "--firefox" => firefox = argv[i + 1].clone(),
            "--kit-root" => kit_root = PathBuf::from(&argv[i + 1]),
            _ => {}
        }
        i += 2;
    }
    let geckodriver = kit_root.join("target/live/geckodriver.exe");
    Args {
        url,
        gd,
        geckodriver,
        firefox,
        kit_root,
    }
}

async fn gd_call(
    client: &reqwest::Client,
    base: &str,
    method: reqwest::Method,
    path: &str,
    body: Option<Value>,
) -> Result<Value, String> {
    let mut req = client.request(method, format!("{base}{path}"));
    if let Some(b) = body {
        req = req.json(&b);
    }
    let res = req.send().await.map_err(|e| format!("{path}: {e}"))?;
    let status = res.status();
    let v: Value = res.json().await.map_err(|e| format!("{path}: body: {e}"))?;
    if !status.is_success() {
        return Err(format!("{path} -> {status}: {v}"));
    }
    Ok(v)
}

async fn execute(
    client: &reqwest::Client,
    args: &Args,
    session: &str,
    script: &str,
    argv: Vec<Value>,
) -> Result<Value, String> {
    let v = gd_call(
        client,
        &args.gd,
        reqwest::Method::POST,
        &format!("/session/{session}/execute/sync"),
        Some(json!({ "script": script, "args": argv })),
    )
    .await?;
    // W3C envelopes script results as {"value": ...} — unwrap for callers.
    Ok(v.get("value").cloned().unwrap_or(v))
}

const INJECT: &str = r#"
window.__probe={errs:[],fetches:[],console:[]};
window.addEventListener('error',e=>__probe.errs.push(String(e.message||e)));
window.addEventListener('unhandledrejection',e=>__probe.errs.push('promise:'+String(e.reason)));
for (const k of ['error','warn']){const f=console[k];console[k]=function(){__probe.console.push(k+': '+Array.from(arguments).map(String).join(' '));return f.apply(console,arguments)};}
const of_=window.fetch.bind(window);
window.fetch=function(...a){const u=String(a[0]);const t=Date.now();
 return of_(...a).then(r=>{__probe.fetches.push(u+' '+r.status+' '+(Date.now()-t)+'ms');
   if(r.status>=400)__probe.errs.push('http '+r.status+' '+u);return r;},
  e=>{__probe.errs.push('fetch-fail '+u+' '+e);throw e;});};
return 1;"#;

const LIST_ACTIONS: &str = r#"
return Array.from(document.querySelectorAll('[data-action]'))
 .map((e,i)=>({i,a:e.getAttribute('data-action'),
   t:(e.getAttribute('data-product-id')||e.getAttribute('data-ticket-id')||e.getAttribute('data-scenario-id')||e.getAttribute('data-source')||''),
   v:!!e.offsetParent,
   card:(e.closest('.card')&&e.closest('.card').getAttribute('data-card'))||''}))
 .filter(o=>o.v);"#;

const CLICK_BY_INDEX: &str = r#"
const el=document.querySelectorAll('[data-action]')[arguments[0]];
if(!el) return 'missing';
el.click(); return 'clicked';"#;

const PROBE_STATE: &str = r#"
return {errs:window.__probe.errs.length,
  newErrs:window.__probe.errs.slice(window.__seen||0),
  newConsole:window.__probe.console.slice(window.__seenC||0),
  newFetches:window.__probe.fetches.slice(window.__seenF||0)};"#;

const MARK_SEEN: &str = r#"
window.__seen=window.__probe.errs.length;
window.__seenC=window.__probe.console.length;
window.__seenF=window.__probe.fetches.length; return 1;"#;

fn spawn_geckodriver(args: &Args) -> Result<Child, String> {
    if !args.geckodriver.is_file() {
        return Err(format!(
            "missing {} — download geckodriver into target/live/",
            args.geckodriver.display()
        ));
    }
    let log_path = args.kit_root.join("target/live/geckodriver.log");
    let log = std::fs::File::create(&log_path).map_err(|e| e.to_string())?;
    let stderr = log.try_clone().map_err(|e| e.to_string())?;
    let mut cmd = Command::new(&args.geckodriver);
    cmd.args(["--port", "4444"])
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(stderr));
    gsv::vision::hide_console(&mut cmd);
    cmd.spawn().map_err(|e| e.to_string())
}

async fn wait_gd(client: &reqwest::Client, args: &Args) -> Result<(), String> {
    for _ in 0..40 {
        if let Ok(v) = gd_call(client, &args.gd, reqwest::Method::GET, "/status", None).await {
            if v.pointer("/value/ready").and_then(Value::as_bool) == Some(true) {
                return Ok(());
            }
            if v.get("ready").and_then(Value::as_bool) == Some(true) {
                return Ok(());
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    Err("geckodriver /status not ready".into())
}

async fn settle(ms: u64) {
    tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
}

async fn snapshot(client: &reqwest::Client, args: &Args, session: &str) -> Result<Value, String> {
    execute(client, args, session, PROBE_STATE, vec![]).await
}

async fn screenshot(client: &reqwest::Client, args: &Args, base: &str, name: &str) {
    if let Ok(v) = gd_call(
        client,
        &args.gd,
        reqwest::Method::GET,
        &format!("{base}/screenshot"),
        None,
    )
    .await
    {
        if let Some(b64) = v.pointer("/value").and_then(Value::as_str) {
            use std::io::Write;
            let path = args
                .kit_root
                .join(format!("target/live/ui_probe_{name}.png"));
            let png = base64_decode(b64);
            if let Ok(mut f) = std::fs::File::create(&path) {
                let _ = f.write_all(&png);
            }
        }
    }
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let args = parse_args();
    let client = reqwest::Client::new();
    let mut driver: Option<Child> = None;

    let started = gd_call(&client, &args.gd, reqwest::Method::GET, "/status", None)
        .await
        .is_ok();
    if !started {
        match spawn_geckodriver(&args) {
            Ok(child) => driver = Some(child),
            Err(e) => {
                eprintln!("ui-probe: {e}");
                return std::process::ExitCode::FAILURE;
            }
        }
    }
    if let Err(e) = wait_gd(&client, &args).await {
        eprintln!("ui-probe: {e}");
        if let Some(mut d) = driver {
            let _ = d.kill();
        }
        return std::process::ExitCode::FAILURE;
    }

    let profile = args.kit_root.join("target/live/ff-probe-profile");
    let _ = std::fs::create_dir_all(&profile);
    let caps = json!({
        "capabilities": {
            "alwaysMatch": {
                "browserName": "firefox",
                "pageLoadStrategy": "normal",
                "moz:firefoxOptions": {
                    "binary": args.firefox,
                    "args": ["--headless", "--width", "1440", "--height", "900",
                             "--profile", profile.to_string_lossy()],
                    "prefs": {"remote.active-protocols": 1}
                }
            }
        }
    });
    let session_res = match gd_call(
        &client,
        &args.gd,
        reqwest::Method::POST,
        "/session",
        Some(caps),
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            eprintln!("ui-probe: session: {e}");
            if let Some(mut d) = driver {
                let _ = d.kill();
            }
            return std::process::ExitCode::FAILURE;
        }
    };
    let session = session_res
        .pointer("/value/sessionId")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if session.is_empty() {
        eprintln!("ui-probe: no sessionId");
        return std::process::ExitCode::FAILURE;
    }

    let mut report = json!({"url": args.url, "groups": {}});
    let base = format!("/session/{session}");

    let outcome = probe_run(&client, &args, &session, &base, &mut report).await;
    screenshot(&client, &args, &base, "99-final").await;

    let _ = gd_call(
        &client,
        &args.gd,
        reqwest::Method::DELETE,
        &format!("/session/{session}"),
        None,
    )
    .await;
    if let Some(mut d) = driver {
        let _ = d.kill();
    }

    let out = args.kit_root.join("target/live/ui_probe_report.json");
    report["errors_run"] = json!(outcome.err());
    match serde_json::to_string_pretty(&report) {
        Ok(s) => {
            if let Err(e) = std::fs::write(&out, s) {
                eprintln!("ui-probe: write: {e}");
                return std::process::ExitCode::FAILURE;
            }
        }
        Err(e) => {
            eprintln!("ui-probe: serde: {e}");
            return std::process::ExitCode::FAILURE;
        }
    }
    println!(
        "ui-probe: report={} groups={} js_errs={} skipped={}",
        out.display(),
        report["groups"].as_object().map_or(0, |m| m.len()),
        report["js_errors_total"].as_u64().unwrap_or(0),
        report["skipped_total"].as_u64().unwrap_or(0),
    );
    std::process::ExitCode::SUCCESS
}

struct RunErr(Vec<String>);

impl RunErr {
    fn err(&self) -> Value {
        Value::from(self.0.clone())
    }
}

async fn probe_run(
    client: &reqwest::Client,
    args: &Args,
    session: &str,
    base: &str,
    report: &mut Value,
) -> RunErr {
    let mut errs = Vec::new();
    if let Err(e) = gd_call(
        client,
        &args.gd,
        reqwest::Method::POST,
        &format!("{base}/url"),
        Some(json!({"url": args.url})),
    )
    .await
    {
        errs.push(format!("goto: {e}"));
        return RunErr(errs);
    }
    settle(3500).await;
    if let Err(e) = execute(client, args, session, INJECT, vec![]).await {
        errs.push(format!("inject: {e}"));
        return RunErr(errs);
    }
    screenshot(client, args, base, "00-initial").await;

    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut js_total = 0u64;
    let mut skipped_total = 0u64;

    for group in GROUPS {
        let _ = execute(
            client,
            args,
            session,
            "const t=document.querySelector('.nav-tab[data-group=\"'+arguments[0]+'\"]');if(t)t.click();return 1;",
            vec![json!(group)],
        )
        .await;
        settle(1500).await;
        let _ = execute(client, args, session, MARK_SEEN, vec![]).await;

        let list = match execute(client, args, session, LIST_ACTIONS, vec![]).await {
            Ok(v) => v,
            Err(e) => {
                errs.push(format!("{group} list: {e}"));
                continue;
            }
        };
        let items = list.as_array().cloned().unwrap_or_default();
        let mut actions = Vec::new();
        for item in items {
            let name = item.get("a").and_then(Value::as_str).unwrap_or("");
            let card = item.get("card").and_then(Value::as_str).unwrap_or("");
            let pid = item.get("t").and_then(Value::as_str).unwrap_or("");
            let key = format!("{group}|{card}|{name}|{pid}");
            let idx = match item.get("i").and_then(Value::as_u64) {
                Some(i) => i,
                None => continue,
            };
            let mut entry = json!({
                "group": group, "card": card, "action": name,
                "context": pid, "deduped": seen.contains(&key),
            });
            if is_destructive(name) {
                entry["result"] = json!("skipped-destructive");
                skipped_total += 1;
                actions.push(entry);
                continue;
            }
            if seen.contains(&key) {
                entry["result"] = json!("seen-in-earlier-group");
                continue;
            }
            seen.insert(key);
            let clicks = if is_toggle(name) { 2 } else { 1 };
            let mut res = json!([]);
            let mut clicked = true;
            for _ in 0..clicks {
                let before = snapshot(client, args, session).await.ok();
                let r = execute(client, args, session, CLICK_BY_INDEX, vec![json!(idx)])
                    .await
                    .unwrap_or(Value::Null);
                if r.as_str() != Some("clicked") {
                    clicked = false;
                    break;
                }
                settle(900).await;
                let after = snapshot(client, args, session).await.unwrap_or(Value::Null);
                let (nb, na) = (
                    before.as_ref().and_then(|b| b.get("newErrs")),
                    after.get("newErrs"),
                );
                let diff: Vec<Value> = match (nb, na) {
                    (Some(Value::Array(b)), Some(Value::Array(a))) => a[b.len()..].to_vec(),
                    (None, Some(Value::Array(a))) => a.clone(),
                    _ => vec![],
                };
                let fetches: Vec<Value> = after
                    .get("newFetches")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                res = json!({"errs": diff, "fetches": fetches.len()});
                let _ = execute(client, args, session, MARK_SEEN, vec![]).await;
            }
            if clicked {
                entry["result"] = json!("clicked");
                entry["outcome"] = res;
            } else {
                entry["result"] = json!("missing");
                errs.push(format!("{group}/{card}/{name} missing"));
            }
            actions.push(entry);
        }
        report["groups"][group] = json!(actions);
        js_total += actions
            .iter()
            .filter_map(|a| a.pointer("/outcome/errs").and_then(Value::as_array))
            .map(|e| e.len() as u64)
            .sum::<u64>();
    }

    let final_state = snapshot(client, args, session).await.unwrap_or(Value::Null);
    report["final"] = json!({
        "title": execute(client, args, session, "return document.title;", vec![]).await.ok(),
        "cards": execute(client, args, session, "return document.querySelectorAll('.card').length;", vec![]).await.ok(),
        "all_js_errors": final_state.get("newErrs").cloned().unwrap_or(json!([])),
        "console": final_state.get("newConsole").cloned().unwrap_or(json!([])),
        "fetch_log_tail": final_state.get("newFetches").cloned().unwrap_or(json!([])),
    });
    report["js_errors_total"] = json!(js_total);
    report["skipped_total"] = json!(skipped_total);
    RunErr(errs)
}

fn base64_decode(input: &str) -> Vec<u8> {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity(input.len() / 4 * 3);
    let mut buf: u32 = 0;
    let mut bits = 0;
    for c in input.bytes() {
        if c == b'=' || c == b'\n' || c == b'\r' {
            continue;
        }
        let pos = T.iter().position(|&x| x == c);
        let Some(v) = pos else { continue };
        buf = (buf << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1u32 << bits) - 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{is_destructive, is_toggle};

    #[test]
    fn destructive_actions_are_never_clicked() {
        for a in [
            "settings-save",
            "product-open",
            "tickets-create",
            "tickets-claim",
            "tickets-done",
            "tickets-error",
            "apply-update",
        ] {
            assert!(is_destructive(a), "{a}");
        }
        assert!(!is_destructive("resync"));
        assert!(!is_destructive("vision-sync"));
    }

    #[test]
    fn toggles_restore_after_double_click() {
        for a in ["auto-toggle", "power-toggle", "card-min", "card-fs"] {
            assert!(is_toggle(a), "{a}");
        }
        assert!(!is_toggle("resync"));
    }

    #[test]
    fn base64_roundtrip_matches_std() {
        // [0,1,2,3] → "AAECAw==" ; [0,1,2] → "AAEC" (in-bin decoder, no extra deps).
        assert_eq!(super::base64_decode("AAECAw=="), vec![0, 1, 2, 3]);
        assert_eq!(super::base64_decode("AAEC"), vec![0, 1, 2]);
        assert_eq!(super::base64_decode(""), Vec::<u8>::new());
    }
}
