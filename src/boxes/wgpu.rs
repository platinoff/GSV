//! WGPU phone-test page (owner steer 2026-11-12: «відкрию браузер на телефоні»).
//!
//! Hub serves a small WebGPU runner at `GET /wgpu` (LAN same-origin passes the
//! CSRF gate in `--allow-lan` mode). The browser runs a WGSL 64×64 matmul on the
//! phone GPU, checks it against a JS CPU reference, and posts the result envelope
//! back to `POST /api/wgpu/report`; reports persist to `data/wgpu_reports.jsonl`
//! and read back through `GET /api/wgpu` + `GET /api/wgpu/reports`. No LLM
//! compute — matmul-level smoke of the GSV-WGPU browser lane (G4) + mesh (G5) +
//! provider row (G6) envelope path.

use serde_json::{json, Value};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Canonical report keys (everything else is dropped at the gate).
const REPORT_KEYS: [&str; 8] = [
    "ok",
    "task_id",
    "peer",
    "adapter",
    "dims",
    "elapsed_ms",
    "max_err",
    "ua",
];

/// Validate a browser report envelope. Required: non-empty `task_id`/`peer`,
/// finite non-negative `elapsed_ms`/`max_err` numbers.
pub fn validate_report(v: &Value) -> Result<(), String> {
    if !v.is_object() {
        return Err("report must be a JSON object".into());
    }
    for key in ["task_id", "peer"] {
        let s = v
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        if s.is_empty() {
            return Err(format!("report field `{key}` must be a non-empty string"));
        }
        if s.len() > 120 {
            return Err(format!("report field `{key}` too long"));
        }
    }
    for key in ["elapsed_ms", "max_err"] {
        match v.get(key) {
            Some(Value::Number(n)) => {
                let f = n.as_f64().unwrap_or(-1.0);
                if !f.is_finite() || f < 0.0 {
                    return Err(format!("report field `{key}` must be finite >= 0"));
                }
            }
            _ => return Err(format!("report field `{key}` must be a number")),
        }
    }
    Ok(())
}

/// Keep only canonical keys (bounded strings/numbers) so phone POSTs cannot
/// smuggle arbitrary JSON onto disk.
fn sanitize_report(v: &Value) -> Value {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut out = json!({ "ts": ts });
    if let Some(obj) = v.as_object() {
        for key in REPORT_KEYS {
            if let Some(val) = obj.get(key) {
                let bounded = match key {
                    "elapsed_ms" | "max_err" | "ok" => val.clone(),
                    _ => Value::String(
                        val.as_str()
                            .unwrap_or("")
                            .chars()
                            .take(160)
                            .collect::<String>(),
                    ),
                };
                out[key] = bounded;
            }
        }
    }
    out
}

/// Append one report to `data/wgpu_reports.jsonl` (capped ring, last 200 rows).
pub fn record_report(data_dir: &Path, v: &Value) -> Result<(), String> {
    validate_report(v)?;
    let sanitized = sanitize_report(v);
    let path = data_dir.join("wgpu_reports.jsonl");
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let mut lines: Vec<&str> = existing.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() >= 200 {
        let keep = lines.len() - 199;
        lines.drain(..keep);
    }
    let mut body = lines.join("\n");
    if !body.is_empty() {
        body.push('\n');
    }
    body.push_str(
        &serde_json::to_string(&sanitized).map_err(|_| "report encode failed".to_string())?,
    );
    std::fs::create_dir_all(data_dir).map_err(|e| format!("data dir: {e}"))?;
    std::fs::write(&path, body).map_err(|e| format!("write report: {e}"))
}

/// Parse every JSONL line (tolerates a torn tail — skips unparsable rows).
fn parse_reports(data_dir: &Path) -> Vec<Value> {
    let text = std::fs::read_to_string(data_dir.join("wgpu_reports.jsonl")).unwrap_or_default();
    text.lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .collect()
}

/// Latest report + count (`GET /api/wgpu`).
pub fn wire(data_dir: &Path) -> Value {
    let reports = parse_reports(data_dir);
    json!({
        "ok": true,
        "count": reports.len(),
        "latest": reports.last().cloned().unwrap_or(Value::Null),
    })
}

/// Last 20 reports (`GET /api/wgpu/reports`).
pub fn reports_wire(data_dir: &Path) -> Value {
    let all = parse_reports(data_dir);
    let tail: Vec<Value> = all.iter().rev().take(20).rev().cloned().collect();
    json!({ "ok": true, "count": all.len(), "reports": tail })
}

/// `GET /wgpu` — the phone browser runner (thin HTML/JS glue, Rust-served).
pub fn page() -> &'static str {
    PAGE
}

const PAGE: &str = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>GSV-WGPU phone test</title>
<style>
body{font-family:ui-monospace,monospace;background:#0b0f1a;color:#d8e2f0;margin:0;padding:16px;line-height:1.5}
h1{font-size:16px;margin:0 0 12px}
pre{background:#111827;border:1px solid #24304d;border-radius:8px;padding:10px;white-space:pre-wrap;word-break:break-word;font-size:12px}
button{background:#6d5ae0;color:#fff;border:0;border-radius:8px;padding:10px 16px;font-size:14px;margin:8px 0}
button:disabled{opacity:.5}
.badge{display:inline-block;border-radius:999px;padding:2px 10px;font-size:11px;border:1px solid #24304d}
</style>
</head>
<body>
<h1>GSV-WGPU · phone browser test (matmul 64×64)</h1>
<div><span id="cap" class="badge">checking WebGPU…</span></div>
<button id="go" onclick="run()">Run matmul on this device</button>
<pre id="out">idle</pre>
<pre id="feed">hub feed: (tap Run to refresh)</pre>
<script>
const out=document.getElementById('out'),feed=document.getElementById('feed'),cap=document.getElementById('cap'),go=document.getElementById('go');
function log(m){out.textContent+=m+'\n';}
function setCap(t,ok){cap.textContent=t;cap.style.borderColor=ok?'#3aa66a':'#c0392b';}
async function post(report){try{const r=await fetch('api/wgpu/report',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(report)});const d=await r.json();log('hub ack: '+JSON.stringify(d));}catch(e){log('hub post failed: '+e);}}
async function refreshFeed(){try{const d=await(await fetch('api/wgpu')).json();feed.textContent='hub feed: count='+d.count+(d.latest?'\nlatest: '+JSON.stringify(d.latest):'');}catch(e){feed.textContent='hub feed error: '+e;}}
const WGSL=`
@group(0) @binding(0) var<storage,read> a:array<f32>;
@group(0) @binding(1) var<storage,read> b:array<f32>;
@group(0) @binding(2) var<storage,read_write> c:array<f32>;
@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) g:vec3<u32>) {
  let n=64u; let x=g.x; let y=g.y;
  if (x>=n || y>=n) { return; }
  var acc=0.0;
  for (var k=0u; k<n; k++) { acc = acc + a[y*n+k]*b[k*n+x]; }
  c[y*n+x]=acc;
}`;
async function run(){
  go.disabled=true; out.textContent='';
  if(!navigator.gpu){setCap('no navigator.gpu',false);const ua=navigator.userAgent||'';const mv=ua.match(/Chrome\/(\d+)/);const chromeV=mv?mv[1]:'?';log('WebGPU unavailable in this browser');log('userAgent: '+ua.slice(0,160));log('Chrome major: '+chromeV+' (WebGPU needs Chrome 121+ on Android 12+ with Vulkan)');log('fix: Play Store -> update Chrome; or chrome://flags -> enable "Unsafe WebGPU" -> Relaunch');await post({ok:false,task_id:'phone-e2e-matmul64',peer:'browser-webgpu',adapter:'no-navigator-gpu',dims:'64x64',elapsed_ms:0,max_err:0,ua:ua.slice(0,160)});go.disabled=false;return;}
  setCap('navigator.gpu present',true);
  try{
    const adapter=await navigator.gpu.requestAdapter();
    if(!adapter){throw new Error('requestAdapter returned null');}
    const device=await adapter.requestDevice();
    let adapterName='adapter'; try{const i=adapter.info||{};adapterName=[i.vendor,i.architecture,i.device].filter(Boolean).join('/')||'adapter';}catch(e){}
    log('adapter: '+adapterName);
    const N=64,A=new Float32Array(N*N),B=new Float32Array(N*N),REF=new Float32Array(N*N);
    for(let i=0;i<N*N;i++){A[i]=(i%7)+1;B[i]=((i*3)%5)+1;}
    for(let y=0;y<N;y++)for(let x=0;x<N;x++){let acc=0;for(let k=0;k<N;k++)acc+=A[y*N+k]*B[k*N+x];REF[y*N+x]=acc;}
    const aBuf=device.createBuffer({size:A.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
    const bBuf=device.createBuffer({size:B.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
    const cBuf=device.createBuffer({size:A.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC});
    device.queue.writeBuffer(aBuf,0,A);device.queue.writeBuffer(bBuf,0,B);
    const pipeline=device.createComputePipeline({layout:'auto',compute:{module:device.createShaderModule({code:WGSL}),entryPoint:'main'}});
    const bg=device.createBindGroup({layout:pipeline.getBindGroupLayout(0),entries:[{binding:0,resource:{buffer:aBuf}},{binding:1,resource:{buffer:bBuf}},{binding:2,resource:{buffer:cBuf}}]});
    const cRead=device.createBuffer({size:A.byteLength,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});
    const enc=device.createCommandEncoder();const pass=enc.beginComputePass();
    pass.setPipeline(pipeline);pass.setBindGroup(0,bg);pass.dispatchWorkgroups(8,8);pass.end();
    enc.copyBufferToBuffer(cBuf,0,cRead,0,A.byteLength);
    const t0=performance.now();device.queue.submit([enc.finish()]);
    await device.queue.onSubmittedWorkDone();
    const ms=performance.now()-t0;
    await cRead.mapAsync(GPUMapMode.READ);const C=new Float32Array(cRead.getMappedRange().slice(0));cRead.unmap();
    let maxErr=0;for(let i=0;i<N*N;i++){const e=Math.abs(C[i]-REF[i]);if(e>maxErr)maxErr=e;}
    log('device C[0..3]: '+Array.from(C.slice(0,4)).join(', '));
    log('elapsed_ms='+ms.toFixed(2)+' max_err='+maxErr.toExponential(2));
    setCap(maxErr<1e-3?'PASS':'FAIL',maxErr<1e-3);
    await post({ok:maxErr<1e-3,task_id:'phone-e2e-matmul64',peer:'browser-webgpu',adapter:adapterName,dims:'64x64',elapsed_ms:Math.round(ms*100)/100,max_err:maxErr,ua:navigator.userAgent.slice(0,120)});
    aBuf.destroy();bBuf.destroy();cBuf.destroy();cRead.destroy();
  }catch(e){setCap('error: '+e,false);log('error: '+e);await post({ok:false,task_id:'phone-e2e-matmul64',peer:'browser-webgpu',adapter:'error: '+String(e).slice(0,80),dims:'64x64',elapsed_ms:0,max_err:0,ua:String(e).slice(0,120)});}
  await refreshFeed();go.disabled=false;
}
refreshFeed();
</script>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("gsv_wgpu_test_{}_{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn validate_report_accepts_minimal_and_rejects_missing() {
        let ok = json!({"task_id":"t1","peer":"browser-webgpu","elapsed_ms":12.5,"max_err":0.0});
        assert!(validate_report(&ok).is_ok());
        let no_task = json!({"peer":"browser-webgpu","elapsed_ms":1,"max_err":0});
        assert!(validate_report(&no_task).is_err());
        let neg = json!({"task_id":"t1","peer":"p","elapsed_ms":-1,"max_err":0});
        assert!(validate_report(&neg).is_err());
        let nan_shape = json!({"task_id":"t1","peer":"p","elapsed_ms":"fast","max_err":0});
        assert!(validate_report(&nan_shape).is_err());
        let not_obj = json!("hello");
        assert!(validate_report(&not_obj).is_err());
    }

    #[test]
    fn record_report_appends_and_reports_wire_reads_back() {
        let dir = temp_dir("ring");
        let r1 = json!({"task_id":"t1","peer":"browser-webgpu","elapsed_ms":10.25,"max_err":0.5,"bogus":"dropped"});
        record_report(&dir, &r1).expect("record 1");
        let r2 = json!({"task_id":"t2","peer":"browser-webgpu","elapsed_ms":20.0,"max_err":0.0});
        record_report(&dir, &r2).expect("record 2");
        let w = wire(&dir);
        assert_eq!(w["ok"], true);
        assert_eq!(w["count"], 2);
        assert_eq!(w["latest"]["task_id"], "t2");
        assert!(w["latest"].get("bogus").is_none());
        assert!(w["latest"]["ts"].is_u64());
        let rs = reports_wire(&dir);
        assert_eq!(rs["count"], 2);
        assert_eq!(rs["reports"].as_array().map(|a| a.len()), Some(2));
        assert_eq!(rs["reports"][0]["task_id"], "t1");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn record_report_rejects_invalid_envelope() {
        let dir = temp_dir("reject");
        let bad = json!({"task_id":"  ","peer":"browser-webgpu","elapsed_ms":1,"max_err":0});
        assert!(record_report(&dir, &bad).is_err());
        let w = wire(&dir);
        assert_eq!(w["count"], 0);
        assert!(w["latest"].is_null());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn page_carries_webgpu_runner_markers() {
        let p = page();
        assert!(p.contains("navigator.gpu"));
        assert!(p.contains("api/wgpu/report"));
        assert!(p.contains("@compute @workgroup_size(8,8)"));
        assert!(p.contains("matmul 64×64"));
    }
}
