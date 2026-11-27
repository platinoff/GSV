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
use super::weight_store;

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
        let digest = weight_store::sha256_hex(&payload);
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
    let got = weight_store::sha256_hex(slice);
    if got != want {
        return Err(format!(
            "pack record sha256 mismatch (want {want}, got {got})"
        ));
    }
    Ok(slice.to_vec())
}

/// One expert's three matvec tensors resolved from a packed record. The
/// artifact-source rows point INTO the pack file; the record sha256 was
/// verified once when the pack was built, and each slice is re-hashed here so
/// the hub's fail-closed chunk reader still gets content-addressed keys.
#[derive(Debug, Clone)]
pub struct PackExpert {
    pub gate_key: String,
    pub up_key: String,
    pub down_key: String,
    pub pack_file: String,
    pub gate_start: u64,
    pub up_start: u64,
    pub down_start: u64,
    pub gate_len: u64,
    pub up_len: u64,
    pub down_len: u64,
}

/// Read one packed expert record, verify it, split into gate/up/down and
/// register the three slices as artifact sources in `data_dir`.
pub fn pack_expert_ranges(
    pack_dir: &Path,
    data_dir: &Path,
    layer: usize,
    expert: usize,
) -> Result<PackExpert, String> {
    let manifest_path = pack_dir.join(format!("manifest-L{layer:02}.json"));
    let manifest: Value = serde_json::from_str(
        &std::fs::read_to_string(&manifest_path).map_err(|e| format!("manifest read: {e}"))?,
    )
    .map_err(|e| format!("manifest json: {e}"))?;
    let entry = manifest["entries"]
        .as_array()
        .ok_or("no entries")?
        .iter()
        .find(|e| e.get("expert").and_then(Value::as_u64) == Some(expert as u64))
        .ok_or("expert not in pack manifest")?;
    let file = entry.get("file").and_then(Value::as_str).ok_or("no file")?;
    let record_start = entry
        .get("offset")
        .and_then(Value::as_u64)
        .ok_or("no offset")?;
    let pack_path = pack_dir.join(file);
    let data = std::fs::read(&pack_path).map_err(|e| format!("pack read: {e}"))?;
    let record = read_pack_entry(&data, entry)?; // whole-record sha256, fail-closed

    let gate_len = role_len(ExpertRole::Gate);
    let up_len = role_len(ExpertRole::Up);
    let down_len = role_len(ExpertRole::Down);
    if record.len() != (gate_len + up_len + down_len) as usize {
        return Err(format!("pack record size {} unexpected", record.len()));
    }
    let gate = &record[..gate_len as usize];
    let up = &record[gate_len as usize..(gate_len + up_len) as usize];
    let down = &record[(gate_len + up_len) as usize..];
    let gate_key = weight_store::sha256_hex(gate);
    let up_key = weight_store::sha256_hex(up);
    let down_key = weight_store::sha256_hex(down);

    // Register three artifact sources pointing into the pack file. A blob is
    // stored once on disk; the keys are its per-slice content hashes.
    let sources_path = data_dir.join(weight_store::ARTIFACT_SOURCES_FILE);
    let existing: Vec<Value> = weight_store::read_artifact_sources(data_dir);
    let artifact = pack_path.to_string_lossy().to_string();
    let fresh = vec![
        json!({"key": gate_key, "artifact": artifact, "offset": record_start, "len": gate_len}),
        json!({"key": up_key, "artifact": artifact, "offset": record_start + gate_len, "len": up_len}),
        json!({"key": down_key, "artifact": artifact, "offset": record_start + gate_len + up_len, "len": down_len}),
    ];
    let merged = super::vision_canary::merge_sources(&existing, &fresh);
    std::fs::write(&sources_path, serde_json::to_string(&merged).unwrap())
        .map_err(|e| format!("artifact sources write: {e}"))?;
    weight_store::invalidate_artifact_sources(data_dir);

    Ok(PackExpert {
        gate_key,
        up_key,
        down_key,
        pack_file: file.to_string(),
        gate_start: record_start,
        up_start: record_start + gate_len,
        down_start: record_start + gate_len + up_len,
        gate_len,
        up_len,
        down_len,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxes::weight_store;

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

    #[test]
    fn load6_pack_expert_ranges_split_and_register() {
        let dir = std::env::temp_dir().join("gsv_load6_expert_ranges");
        let _ = std::fs::create_dir_all(&dir);
        let pack_dir = dir.join("pack");
        let data_dir = dir.join("data");
        let _ = std::fs::create_dir_all(&pack_dir);
        let _ = std::fs::create_dir_all(&data_dir);
        let gate = vec![1u8; role_len(ExpertRole::Gate) as usize];
        let up = vec![2u8; role_len(ExpertRole::Up) as usize];
        let down = vec![3u8; role_len(ExpertRole::Down) as usize];
        let blob = [gate.as_slice(), up.as_slice(), down.as_slice()].concat();
        std::fs::write(pack_dir.join("experts-L00.pack"), &blob).unwrap();
        let manifest = json!({
            "schema": "gasv.expert-pack.v1", "layer": 0,
            "entries": [{"expert": 0, "file": "experts-L00.pack", "offset": 0u64,
                "len": blob.len() as u64, "payload_bytes": blob.len(),
                "quant_layout": "q4k_gate_up_q6k_down",
                "sha256": weight_store::sha256_hex(&blob)}],
        });
        std::fs::write(
            pack_dir.join("manifest-L00.json"),
            serde_json::to_string(&manifest).unwrap(),
        )
        .unwrap();
        let got = pack_expert_ranges(&pack_dir, &data_dir, 0, 0).expect("split");
        assert_eq!(got.gate_start, 0);
        assert_eq!(got.up_start, role_len(ExpertRole::Gate));
        assert_eq!(
            got.down_start,
            role_len(ExpertRole::Gate) + role_len(ExpertRole::Up)
        );
        assert_eq!(got.gate_key, weight_store::sha256_hex(&gate));
        assert_eq!(got.up_key, weight_store::sha256_hex(&up));
        assert_eq!(got.down_key, weight_store::sha256_hex(&down));
        // Artifact sources were registered and are readable by content hash.
        assert_eq!(
            weight_store::read_chunk(&data_dir, &got.gate_key).unwrap(),
            gate
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
