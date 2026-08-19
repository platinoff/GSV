//! Light memory / disk / speed probe (`gsv-mds`).
//!
//! Owner pick band 175: a tiny Rust app a solo bot can walk as a ticket band.
//! Disk reuses [`super::xtask::disk_report`]. Memory is an alloc sample plus
//! Windows `GlobalMemoryStatusEx` when available. Speed is an Instant xor-fold.

use std::path::Path;
use std::time::Instant;

use serde::Serialize;
use serde_json::{json, Value};

use super::xtask::{self, DiskReport};

/// Bytes allocated for the memory sample.
pub const SAMPLE_BYTES: usize = 1024 * 1024;
/// Iterations of the speed xor-fold.
pub const SPEED_ITERS: u64 = 100_000;

/// Process / OS memory sample.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MemoryProbe {
    pub sample_bytes: u64,
    pub alloc_ns: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avail_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_pct: Option<u32>,
}

/// Tiny Instant microbench.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SpeedProbe {
    pub label: String,
    pub iters: u64,
    pub ns_per_iter: u64,
}

/// Combined MDS report (`GET /api/mds` / `gsv_mds` / `gsv-mds` bin).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MdsReport {
    pub ok: bool,
    pub name: String,
    pub memory: MemoryProbe,
    pub disk: DiskReport,
    pub speed: SpeedProbe,
}

fn alloc_sample() -> (u64, u64) {
    let start = Instant::now();
    let mut buf = vec![0u8; SAMPLE_BYTES];
    for (i, b) in buf.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(31);
    }
    let checksum = buf.iter().fold(0u8, |a, b| a ^ b);
    std::hint::black_box(checksum);
    let ns = start.elapsed().as_nanos() as u64;
    (SAMPLE_BYTES as u64, ns.max(1))
}

#[cfg(windows)]
fn os_phys() -> Option<(u64, u64, u32)> {
    #[repr(C)]
    #[allow(dead_code)]
    struct MemoryStatusEx {
        dw_length: u32,
        dw_memory_load: u32,
        ull_total_phys: u64,
        ull_avail_phys: u64,
        ull_total_page_file: u64,
        ull_avail_page_file: u64,
        ull_total_virtual: u64,
        ull_avail_virtual: u64,
        ull_avail_extended_virtual: u64,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GlobalMemoryStatusEx(status: *mut MemoryStatusEx) -> i32;
    }
    let mut s = MemoryStatusEx {
        dw_length: std::mem::size_of::<MemoryStatusEx>() as u32,
        dw_memory_load: 0,
        ull_total_phys: 0,
        ull_avail_phys: 0,
        ull_total_page_file: 0,
        ull_avail_page_file: 0,
        ull_total_virtual: 0,
        ull_avail_virtual: 0,
        ull_avail_extended_virtual: 0,
    };
    let ok = unsafe { GlobalMemoryStatusEx(&mut s) };
    if ok == 0 {
        None
    } else {
        Some((s.ull_total_phys, s.ull_avail_phys, s.dw_memory_load))
    }
}

#[cfg(not(windows))]
fn os_phys() -> Option<(u64, u64, u32)> {
    None
}

/// Fill a 1 MiB buffer and time it; attach OS phys if the probe works.
pub fn memory_probe() -> MemoryProbe {
    let (sample_bytes, alloc_ns) = alloc_sample();
    let os = os_phys();
    MemoryProbe {
        sample_bytes,
        alloc_ns,
        total_bytes: os.map(|o| o.0),
        avail_bytes: os.map(|o| o.1),
        load_pct: os.map(|o| o.2),
    }
}

/// Xor-fold `SPEED_ITERS` times; ns/iter is at least 1 so benches stay nonzero.
pub fn speed_probe() -> SpeedProbe {
    let start = Instant::now();
    let mut acc = 0u64;
    for i in 0..SPEED_ITERS {
        acc ^= i.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
    std::hint::black_box(acc);
    let ns = start.elapsed().as_nanos() as u64;
    SpeedProbe {
        label: "xor-fold".into(),
        iters: SPEED_ITERS,
        ns_per_iter: (ns / SPEED_ITERS).max(1),
    }
}

/// Probe memory, disk, and speed for `repo_root`.
pub fn report(repo_root: &Path) -> MdsReport {
    let disk = xtask::disk_report(repo_root, false);
    let memory = memory_probe();
    let speed = speed_probe();
    let ok = disk.ok && memory.sample_bytes > 0 && speed.ns_per_iter > 0;
    MdsReport {
        ok,
        name: "gsv-mds".into(),
        memory,
        disk,
        speed,
    }
}

/// JSON wire for HTTP / MCP.
pub fn wire(repo_root: &Path) -> Value {
    serde_json::to_value(report(repo_root))
        .unwrap_or_else(|_| json!({ "ok": false, "error": "mds encode failed" }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn memory_sample_is_one_mib() {
        let m = memory_probe();
        assert_eq!(m.sample_bytes, SAMPLE_BYTES as u64);
        assert!(m.alloc_ns > 0);
    }

    #[test]
    fn speed_ns_nonzero() {
        let s = speed_probe();
        assert_eq!(s.iters, SPEED_ITERS);
        assert!(s.ns_per_iter >= 1);
        assert_eq!(s.label, "xor-fold");
    }

    #[test]
    fn report_sees_this_crate() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let r = report(&root);
        assert!(r.ok, "{r:?}");
        assert_eq!(r.name, "gsv-mds");
        assert!(!r.disk.repo.is_empty());
        let v = wire(&root);
        assert_eq!(v["ok"], true);
        assert_eq!(v["speed"]["label"], "xor-fold");
    }
}
