//! Torrent distribution of tensor bytes (swarm path b, owner order).
//!
//! Phones fetch model bytes from the host; plain HTTP already works, but
//! the owner wants torrent mechanics (piece integrity, resume, future
//! phone-to-phone P2P). This module builds `.torrent` metainfo in pure
//! Rust: bencode writer, piece SHA1 over the file, `url-list` webseeds
//! pointing at this server's `/models/*` (trackerless — no `announce`
//! key, browsers cannot do UDP DHT anyway; a local WS tracker for real
//! P2P is a follow-up). Built metainfo is cached in memory keyed by
//! (path, size, mtime); hashing runs in `spawn_blocking`.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Piece size for model torrents: 1 MiB slices a 0.4–10 GiB GGUF into
/// hundreds–thousands of verifiable pieces (whole-file re-fetch on
/// corruption is the failure mode this exists to prevent).
pub const PIECE_LEN: u64 = 1024 * 1024;

/// Minimal bencode value. Dict keys sort via `BTreeMap` (canonical form —
/// the info-hash is a SHA1 over these exact bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Benc {
    Int(i64),
    Bytes(Vec<u8>),
    List(Vec<Benc>),
    Dict(BTreeMap<Vec<u8>, Benc>),
}

impl Benc {
    pub fn encode(&self, out: &mut Vec<u8>) {
        match self {
            Benc::Int(v) => {
                out.extend_from_slice(format!("i{v}e").as_bytes());
            }
            Benc::Bytes(b) => {
                out.extend_from_slice(format!("{}:", b.len()).as_bytes());
                out.extend_from_slice(b);
            }
            Benc::List(items) => {
                out.push(b'l');
                for item in items {
                    item.encode(out);
                }
                out.push(b'e');
            }
            Benc::Dict(map) => {
                out.push(b'd');
                for (k, v) in map {
                    out.extend_from_slice(format!("{}:", k.len()).as_bytes());
                    out.extend_from_slice(k);
                    v.encode(out);
                }
                out.push(b'e');
            }
        }
    }

    pub fn encoded(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.encode(&mut out);
        out
    }
}

fn bstr(s: &str) -> Benc {
    Benc::Bytes(s.as_bytes().to_vec())
}

/// SHA1 hex of raw bytes (info-hash display, tests).
pub fn sha1_hex(data: &[u8]) -> String {
    use sha1::Digest;
    format!("{:x}", sha1::Sha1::digest(data))
}

/// Hash every `piece_len` slice of the file at `path` (blocking IO —
/// callers run this under `spawn_blocking`). Returns the concatenated
/// 20-byte digests plus the file length.
pub fn hash_pieces(path: &Path, piece_len: u64) -> Result<(Vec<u8>, u64), String> {
    use sha1::Digest;
    use std::io::Read;

    let piece_len = piece_len.max(1) as usize;
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let total = file.metadata().map_err(|e| e.to_string())?.len();
    let mut pieces = Vec::new();
    let mut buf = vec![0u8; piece_len];
    loop {
        let mut filled = 0usize;
        while filled < piece_len {
            match file.read(&mut buf[filled..]) {
                Ok(0) => break,
                Ok(n) => filled += n,
                Err(e) => return Err(e.to_string()),
            }
        }
        if filled == 0 {
            break;
        }
        pieces.extend_from_slice(&sha1::Sha1::digest(&buf[..filled]));
    }
    Ok((pieces, total))
}

/// Build single-file metainfo for `name`/`total_len` with `pieces` digests
/// and `webseeds` (BEP19 `url-list`). No `announce` — trackerless.
pub fn build_metainfo(
    name: &str,
    total_len: u64,
    piece_len: u64,
    pieces: Vec<u8>,
    webseeds: Vec<String>,
) -> Benc {
    let mut info = BTreeMap::new();
    info.insert(b"length".to_vec(), Benc::Int(total_len as i64));
    info.insert(b"name".to_vec(), bstr(name));
    info.insert(b"piece length".to_vec(), Benc::Int(piece_len as i64));
    info.insert(b"pieces".to_vec(), Benc::Bytes(pieces));
    let mut top = BTreeMap::new();
    top.insert(b"info".to_vec(), Benc::Dict(info));
    top.insert(
        b"url-list".to_vec(),
        Benc::List(webseeds.into_iter().map(|u| bstr(&u)).collect()),
    );
    top.insert(
        b"comment".to_vec(),
        bstr("telenetis tensor bytes (swarm path b)"),
    );
    Benc::Dict(top)
}

/// Info-hash (SHA1 over the bencoded `info` dict) for logging/display.
pub fn info_hash_hex(metainfo: &Benc) -> Option<String> {
    match metainfo {
        Benc::Dict(top) => {
            let info = top.get(b"info".as_slice())?;
            Some(sha1_hex(&info.encoded()))
        }
        _ => None,
    }
}

struct CachedTorrent {
    size: u64,
    mtime_secs: u64,
    bytes: Vec<u8>,
}

static TORRENT_CACHE: std::sync::LazyLock<Mutex<HashMap<PathBuf, CachedTorrent>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

fn mtime_secs(path: &Path) -> u64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Metainfo bytes for a model file, cached by (path, size, mtime).
/// Blocking (hashes the whole file on miss) — callers use spawn_blocking.
pub fn metainfo_for_file(
    path: &Path,
    name: &str,
    webseeds: Vec<String>,
) -> Result<Vec<u8>, String> {
    let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if !meta.is_file() {
        return Err("not a file".into());
    }
    let size = meta.len();
    let mtime = mtime_secs(path);
    if let Ok(cache) = TORRENT_CACHE.lock() {
        if let Some(hit) = cache.get(path) {
            if hit.size == size && hit.mtime_secs == mtime {
                return Ok(hit.bytes.clone());
            }
        }
    }
    let (pieces, total) = hash_pieces(path, PIECE_LEN)?;
    let meta = build_metainfo(name, total, PIECE_LEN, pieces, webseeds);
    let bytes = meta.encoded();
    if let Ok(mut cache) = TORRENT_CACHE.lock() {
        cache.insert(
            path.to_path_buf(),
            CachedTorrent {
                size,
                mtime_secs: mtime,
                bytes: bytes.clone(),
            },
        );
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bencode_reference_vectors() {
        assert_eq!(Benc::Int(42).encoded(), b"i42e");
        assert_eq!(Benc::Int(-3).encoded(), b"i-3e");
        assert_eq!(Benc::Bytes(b"spam".to_vec()).encoded(), b"4:spam");
        assert_eq!(
            Benc::List(vec![bstr("a"), Benc::Int(1)]).encoded(),
            b"l1:ai1ee"
        );
        // Dict keys sort: a < b regardless of insertion order.
        let mut map = BTreeMap::new();
        map.insert(b"b".to_vec(), Benc::Int(2));
        map.insert(b"a".to_vec(), Benc::Int(1));
        assert_eq!(Benc::Dict(map).encoded(), b"d1:ai1e1:bi2ee");
    }

    #[test]
    fn sha1_hex_known_vector() {
        assert_eq!(sha1_hex(b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
    }

    #[test]
    fn metainfo_roundtrip_shape_and_stable_hash() {
        let dir = std::env::temp_dir().join(format!("tns-tor-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("tiny.gguf");
        let data = vec![7u8; 3000];
        std::fs::write(&file, &data).unwrap();
        let seeds = vec!["http://x/models/tiny.gguf".to_string()];
        let a = metainfo_for_file(&file, "tiny.gguf", seeds.clone()).expect("build");
        let b = metainfo_for_file(&file, "tiny.gguf", seeds).expect("cached");
        assert_eq!(a, b, "cache returns identical bytes");
        // 3000 bytes at 1 MiB pieces = exactly one piece.
        assert_eq!(a.windows(6).filter(|w| *w == b"pieces").count(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn metainfo_missing_file_errors() {
        let missing = std::env::temp_dir().join("tns-tor-nope.gguf");
        let _ = std::fs::remove_file(&missing);
        assert!(metainfo_for_file(&missing, "nope.gguf", vec![]).is_err());
    }
}
