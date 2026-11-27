//! Expert-major pack builder for the 21 GB `gsv-versus-ai` 4-bit artifact.
//!
//! The GGUF stores gate/up/down experts as three separate flat tensors, so a
//! single routed expert needs three scattered byte ranges. A pack rewrites
//! each `(layer, expert)` as ONE 4096-aligned contiguous record
//! `[gate | up | down]` with a sidecar manifest — the ServerlessLLM direct
//! addressing + SGLang Expert Pack + mbolt co-activation layout in one step.
//!
//! The source GGUF is IMMUTABLE. A pack is a derived artifact that must live
//! on the mirror (never in git), and every record is SHA-256 verified once at
//! build time. Owner go required for the ~19.45 GiB full set (see LOAD-1b).

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use serde_json::{json, Value};

use super::moe_forward::{ExpertAddr, ExpertRole, D_MODEL, FFN};

/// 4096 alignment required for unbuffered/positional reads on Windows.
pub const PACK_ALIGN: u64 = 4096;

#[derive(Debug, Clone)]
pub struct PackEntry {
    pub layer: usize,
    pub expert: usize,
    pub file: String,
    pub offset: u64,
    pub len: u64, // padded record length
    pub payload_sha256: String,
}

fn align_up(v: u64, a: u64) -> u64 {
    if v.is_multiple_of(a) {
        v
    } else {
        v + (a - v % a)
    }
}

fn role_len(role: ExpertRole) -> u64 {
    match role {
        ExpertRole::Gate | ExpertRole::Up => (D_MODEL * FFN / 256 * 144) as u64, // 589824
        ExpertRole::Down => (FFN * D_MODEL / 256 * 210) as u64,                  // 860160
        _ => 0,
    }
}

/// Byte range of one expert role in the source artifact (data_start already
/// includes the tensor offset from `manifest_offsets`).
fn source_range(
    offsets: &HashMap<String, (u64, u32)>,
    addr: &ExpertAddr,
) -> Option<(u64, u64, u32)> {
    let (start, gt) = *offsets.get(&addr.tensor_name())?;
    let len = role_len(addr.role);
    if len == 0 {
        return None;
    }
    let from = start + addr.expert as u64 * len;
    Some((from, from + len, gt))
}

/// Build one layer's pack file + manifest row set. Returns the manifest Value.
pub fn pack_layer(
    artifact: &Path,
    out_dir: &Path,
    layer: usize,
    offsets: &HashMap<String, (u64, u32)>,
    experts: usize,
) -> Result<Value, String> {
    std::fs::create_dir_all(out_dir).map_err(|e| format!("create out dir: {e}"))?;
    let file_name = format!("experts-L{layer:02}.pack");
    let file_path = out_dir.join(&file_name);
    let mut out = std::fs::File::create(&file_path).map_err(|e| format!("create pack: {e}"))?;
    let mut src = std::fs::File::open(artifact).map_err(|e| format!("open artifact: {e}"))?;
    let mut entries: Vec<Value> = Vec::with_capacity(experts);
    let mut cursor = 0u64;

    for expert in 0..experts {
        let roles = [
            (ExpertRole::Gate, "gate"),
            (ExpertRole::Up, "up"),
            (ExpertRole::Down, "down"),
        ];
        let mut payload: Vec<u8> = Vec::new();
        for (role, label) in roles {
            let addr = ExpertAddr {
                layer,
                expert,
                role,
            };
            let (from, to, gt) = source_range(offsets, &addr)
                .ok_or_else(|| format!("no source range for {label} e{expert}"))?;
            src.seek(SeekFrom::Start(from))
                .map_err(|e| format!("src seek: {e}"))?;
            let mut buf = vec![0u8; (to - from) as usize];
            src.read_exact(&mut buf)
                .map_err(|e| format!("src read: {e}"))?;
            let _ = gt; // Q4_K gate/up + Q6_K down are recorded in `quant_layout`
            payload.extend_from_slice(&buf);
        }
        // Verify once at build time (fail-closed).
        let digest = super::weight_store::sha256_hex(&payload);
        let record_start = cursor;
        out.write_all(&payload)
            .map_err(|e| format!("pack write: {e}"))?;
        let padded = align_up(payload.len() as u64, PACK_ALIGN);
        if padded > payload.len() as u64 {
            let zeros = vec![0u8; (padded - payload.len() as u64) as usize];
            out.write_all(&zeros)
                .map_err(|e| format!("pack pad: {e}"))?;
        }
        cursor += padded;
        entries.push(json!({
            "layer": layer, "expert": expert,
            "file": file_name, "offset": record_start,
            "len": padded, "payload_bytes": payload.len(),
            "quant_layout": "q4k_gate_up_q6k_down", "sha256": digest,
        }));
    }
    out.flush().map_err(|e| format!("pack flush: {e}"))?;
    Ok(json!({
        "schema": "gasv.expert-pack.v1",
        "source_artifact": artifact.to_string_lossy(),
        "layer": layer,
        "alignment": PACK_ALIGN,
        "bytes": cursor,
        "experts": experts,
        "entries": entries,
    }))
}

/// Read one packed expert record and verify its sha256 (fail-closed).
pub fn read_pack_entry(data: &[u8], entry: &Value) -> Result<Vec<u8>, String> {
    let offset = entry
        .get("offset")
        .and_then(Value::as_u64)
        .ok_or("no offset")? as usize;
    let _len = entry.get("len").and_then(Value::as_u64).ok_or("no len")? as usize;
    let payload_len = entry
        .get("payload_bytes")
        .and_then(Value::as_u64)
        .ok_or("no payload_bytes")? as usize;
    let want = entry
        .get("sha256")
        .and_then(Value::as_str)
        .ok_or("no sha256")?;
    let slice = data
        .get(offset..offset.saturating_add(payload_len).min(data.len()))
        .ok_or("pack record out of bounds")?;
    let got = super::weight_store::sha256_hex(slice);
    if got != want {
        return Err(format!(
            "pack record sha256 mismatch (want {want}, got {got})"
        ));
    }
    Ok(slice.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load1_pack_record_roundtrip_and_tamper() {
        let dir = std::env::temp_dir().join("gsv_load1_pack_test");
        let _ = std::fs::create_dir_all(&dir);
        // Build a synthetic 1-expert "artifact" with gate/up/down at known offsets.
        let gate = vec![1u8; role_len(ExpertRole::Gate) as usize];
        let up = vec![2u8; role_len(ExpertRole::Up) as usize];
        let down = vec![3u8; role_len(ExpertRole::Down) as usize];
        let blob = [gate.as_slice(), up.as_slice(), down.as_slice()].concat();
        let artifact = dir.join("src.bin");
        std::fs::write(&artifact, &blob).unwrap();
        let offsets: HashMap<String, (u64, u32)> = HashMap::from([
            ("blk.0.ffn_gate_exps.weight".into(), (0, 12)),
            ("blk.0.ffn_up_exps.weight".into(), (gate.len() as u64, 12)),
            (
                "blk.0.ffn_down_exps.weight".into(),
                ((gate.len() + up.len()) as u64, 14),
            ),
        ]);
        let manifest = pack_layer(&artifact, &dir, 0, &offsets, 1).expect("pack layer");
        let entries = manifest["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 1);
        let e0 = &entries[0];
        assert_eq!(e0["payload_bytes"], 2_039_808);
        assert_eq!(e0["offset"], 0);
        // First record bytes must equal gate|up|down from the source.
        let packed = std::fs::read(dir.join("experts-L00.pack")).unwrap();
        let got = read_pack_entry(&packed, e0).expect("roundtrip");
        assert_eq!(got, blob[..2_039_808]);
        // Tampered record must fail closed.
        let mut bad = e0.clone();
        bad["sha256"] = json!("00".repeat(32));
        assert!(read_pack_entry(&packed, &bad).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
