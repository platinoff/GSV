//! Raw server→client WebSocket push channel for mesh metrics.
//!
//! The hub's HTTP/1 accept loop (`server::serve`) bypasses hyper by design —
//! the wire filter demands unframed chunk handling for `/mcp`, and hyper never
//! sees the upgrade extension that axum's `WebSocketUpgrade` requires. This
//! module therefore owns the RFC 6455 handshake and the frame codec itself.
//!
//! Scope: LAN-only (same bind as every `/api/wgpu/*` route), read-only
//! (the hub only sends; client frames are consumed and ignored except
//! ping→pong and close), tiny payloads on a 2 s cadence.

use sha1::{Digest, Sha1};

/// RFC 6455 `Sec-WebSocket-Accept` for the client-supplied key.
pub(crate) fn accept_key(key: &str) -> String {
    const GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let mut h = Sha1::new();
    h.update(key.trim().as_bytes());
    h.update(GUID);
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, h.finalize())
}

/// Encode a server→client frame for `opcode` (0x1 text, 0x8 close, 0xA pong).
fn frame(opcode: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() + 10);
    out.push(0x80 | opcode);
    let len = payload.len();
    if len < 126 {
        out.push(len as u8);
    } else if len <= u16::MAX as usize {
        out.push(126);
        out.extend_from_slice(&(len as u16).to_be_bytes());
    } else {
        out.push(127);
        out.extend_from_slice(&(len as u64).to_be_bytes());
    }
    out.extend_from_slice(payload);
    out
}

/// Server→client text frame.
pub(crate) fn frame_text(payload: &str) -> Vec<u8> {
    frame(0x1, payload.as_bytes())
}

/// Server→client close frame.
pub(crate) fn frame_close(reason: &str) -> Vec<u8> {
    frame(0x8, reason.as_bytes())
}

/// Server→client pong frame (echoes a client ping).
pub(crate) fn frame_pong(payload: &[u8]) -> Vec<u8> {
    frame(0xA, payload)
}

/// Largest client frame we will buffer (16 MiB); phones send tiny frames only.
pub(crate) const MAX_FRAME: usize = 16 * 1024 * 1024;

/// A decoded client→server frame.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ClientFrame {
    /// Opcodes 0x0/0x1/0x2 — payload discarded (metrics flow is hub→peer).
    Data,
    Ping(Vec<u8>),
    Pong,
    Close,
}

/// Decode the frame at the front of `buf`.
///
/// Returns `(frame, consumed)` when the buffer holds one complete frame,
/// `None` while more bytes are needed. Poisoned (oversized) frames report
/// `Close` so the caller tears the connection down instead of growing `buf`
/// unbounded.
pub(crate) fn decode_frame(buf: &[u8]) -> Option<(ClientFrame, usize)> {
    if buf.len() < 2 {
        return None;
    }
    let opcode = buf[0] & 0x0F;
    let masked = buf[1] & 0x80 != 0;
    let mut len = u64::from(buf[1] & 0x7F);
    let mut off = 2usize;
    if len == 126 {
        if buf.len() < off + 2 {
            return None;
        }
        len = u64::from(u16::from_be_bytes([buf[off], buf[off + 1]]));
        off += 2;
    } else if len == 127 {
        if buf.len() < off + 8 {
            return None;
        }
        len = u64::from_be_bytes(buf[off..off + 8].try_into().expect("8-byte len"));
        off += 8;
    }
    if len as usize > MAX_FRAME {
        return Some((ClientFrame::Close, buf.len()));
    }
    let mask_off = off;
    if masked {
        off += 4;
    }
    let payload_len = len as usize;
    if buf.len() < off + payload_len {
        return None;
    }
    let payload = &buf[off..off + payload_len];
    let decoded = |keep: bool| -> Vec<u8> {
        if masked {
            let key = &buf[mask_off..mask_off + 4];
            payload
                .iter()
                .enumerate()
                .map(|(i, b)| b ^ key[i & 3])
                .collect()
        } else if keep {
            payload.to_vec()
        } else {
            Vec::new()
        }
    };
    let frame = match opcode {
        0x0..=0x2 => ClientFrame::Data, // we never read client payloads
        0x8 => ClientFrame::Close,
        0x9 => ClientFrame::Ping(decoded(true)),
        0xA => ClientFrame::Pong,
        _ => ClientFrame::Data, // unknown opcode: ignore, keep the connection
    };
    Some((frame, off + payload_len))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accept_key_matches_rfc6455_vector() {
        assert_eq!(
            accept_key("dGhlIHNhbXBsZSBub25jZQ=="),
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        );
    }

    #[test]
    fn frame_text_uses_fin_text_and_boundary_lengths() {
        let short = frame_text("hi");
        assert_eq!(short, vec![0x81, 0x02, b'h', b'i']);

        let mid = frame_text(&"a".repeat(126));
        assert_eq!(mid[0], 0x81);
        assert_eq!(mid[1], 126);
        assert_eq!(u16::from_be_bytes([mid[2], mid[3]]), 126);
        assert_eq!(mid.len(), 4 + 126);

        let big = frame_text(&"b".repeat(70_000));
        assert_eq!(big[1], 127);
        assert_eq!(u64::from_be_bytes(big[2..10].try_into().unwrap()), 70_000);
        assert_eq!(big.len(), 10 + 70_000);
    }

    fn masked_frame(opcode: u8, payload: &[u8], mask: [u8; 4]) -> Vec<u8> {
        let mut out = vec![0x80 | opcode];
        let mask_bit = 0x80;
        if payload.len() < 126 {
            out.push(mask_bit | payload.len() as u8);
        } else if payload.len() <= u16::MAX as usize {
            out.push(mask_bit | 126);
            out.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        } else {
            out.push(mask_bit | 127);
            out.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        }
        out.extend_from_slice(&mask);
        out.extend(payload.iter().enumerate().map(|(i, b)| b ^ mask[i & 3]));
        out
    }

    #[test]
    fn decode_masked_client_text_roundtrip() {
        let buf = masked_frame(0x1, b"hello mesh", [1, 2, 3, 4]);
        let (frame, used) = decode_frame(&buf).expect("complete frame");
        assert_eq!(frame, ClientFrame::Data);
        assert_eq!(used, buf.len());
    }

    #[test]
    fn decode_ping_echoes_payload() {
        let mask = [9, 8, 7, 6];
        let buf = masked_frame(0x9, b"ping-payload", mask);
        let (frame, _) = decode_frame(&buf).expect("complete frame");
        assert_eq!(frame, ClientFrame::Ping(b"ping-payload".to_vec()));
    }

    #[test]
    fn decode_close_and_pong() {
        let buf_close = masked_frame(0x8, &[], [0, 0, 0, 0]);
        let (close, _) = decode_frame(&buf_close).expect("close");
        assert_eq!(close, ClientFrame::Close);

        let buf_pong = masked_frame(0xA, b"ok", [5, 5, 5, 5]);
        let (pong, _) = decode_frame(&buf_pong).expect("pong");
        assert_eq!(pong, ClientFrame::Pong);
    }

    #[test]
    fn decode_extended_lengths() {
        let payload = vec![0x41u8; 5000];
        let buf = masked_frame(0x2, &payload, [3, 1, 4, 1]);
        let (_, used) = decode_frame(&buf).expect("extended frame");
        assert_eq!(used, buf.len());
    }

    #[test]
    fn decode_waits_for_full_frame() {
        let buf = masked_frame(0x1, b"hello mesh", [1, 2, 3, 4]);
        assert!(decode_frame(&buf[..4]).is_none());
        assert!(decode_frame(&buf[..buf.len() - 1]).is_none());
    }

    #[test]
    fn oversized_frame_is_close_poison() {
        let mut buf = vec![0x82, 0x80 | 127];
        buf.extend_from_slice(&((MAX_FRAME as u64) + 1).to_be_bytes());
        buf.extend_from_slice(&[0; 4]);
        let (frame, used) = decode_frame(&buf).expect("poison resolves");
        assert_eq!(frame, ClientFrame::Close);
        assert_eq!(used, buf.len());
    }
}
