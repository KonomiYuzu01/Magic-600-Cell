//! Journal frames. A frame is `[payload length: u32, little-endian][its bitwise complement][payload][SHA-256 of the
//! payload]`, and its payload is one canonical `c600-json-1` record (FP.5). One durable action is one frame (R-06).
//! The complement tells a damaged length from a torn append: a header that was written whole must be right.

use c600_digest::sha256;

const HEAD: usize = 8;
const TAIL: usize = 32;
/// The largest payload a frame may carry (an M6 draft limit).
pub const MAX_PAYLOAD: usize = 1 << 24;

/// The bytes of one frame.
pub fn frame(payload: &[u8]) -> Vec<u8> {
    assert!(payload.len() <= MAX_PAYLOAD, "a journal record exceeds the frame limit");
    let mut bytes = Vec::with_capacity(HEAD + payload.len() + TAIL);
    let len = payload.len() as u32;
    bytes.extend_from_slice(&len.to_le_bytes());
    bytes.extend_from_slice(&(!len).to_le_bytes());
    bytes.extend_from_slice(payload);
    bytes.extend_from_slice(&sha256(payload));
    bytes
}

/// What follows the last whole frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tail {
    /// Nothing.
    Clean,
    /// The start of a frame, possibly followed by zero bytes, as a torn append leaves it; opening cuts it off.
    Torn,
    /// A frame whose header or checksum is wrong, at this offset, and that is not a torn append; the session opens
    /// read-only.
    Damaged(u64),
}

/// The whole frames of a journal, the end of the last one, and what follows it.
pub struct Scan {
    pub payloads: Vec<Vec<u8>>,
    pub end: u64,
    pub tail: Tail,
}

pub fn scan(bytes: &[u8]) -> Scan {
    let mut payloads = Vec::new();
    let mut at = 0usize;
    let tail = loop {
        let rest = &bytes[at..];
        if rest.is_empty() {
            break Tail::Clean;
        }
        match whole(rest) {
            Some(payload) => {
                payloads.push(payload.to_vec());
                at += HEAD + payload.len() + TAIL;
            }
            None if torn(rest) => break Tail::Torn,
            None => break Tail::Damaged(at as u64),
        }
    };
    Scan { payloads, end: at as u64, tail }
}

fn field(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// The payload of the frame at the start of `rest` when the frame is whole: its header is right and its checksum
/// matches.
fn whole(rest: &[u8]) -> Option<&[u8]> {
    if rest.len() < HEAD || field(rest, 4) != !field(rest, 0) {
        return None;
    }
    let len = field(rest, 0) as usize;
    if len > MAX_PAYLOAD || rest.len() < HEAD + len + TAIL {
        return None;
    }
    let payload = &rest[HEAD..HEAD + len];
    (sha256(payload)[..] == rest[HEAD + len..HEAD + len + TAIL]).then_some(payload)
}

/// Whether what follows the last whole frame is what an interrupted append leaves (an M6 draft rule): the start of
/// one frame, then nothing or only zero bytes, as a file system that extends a file before its data reaches the disk
/// shows it after a crash. A cut file and a zero-filled one are judged alike: the bytes up to the last nonzero one
/// must be the start of a frame, so a header written in part or whole must be right, and a checksum written in part
/// must be the start of the payload's checksum. Anything else is damage: a wrong header, a checksum that does not
/// match, or other bytes after the frame. A hole inside the frame, as out-of-order sector writes could leave it, is
/// damage too.
fn torn(rest: &[u8]) -> bool {
    let written = rest.iter().rposition(|&b| b != 0).map_or(0, |last| last + 1);
    if written < 4 {
        return true;
    }
    let len = field(rest, 0);
    if len as usize > MAX_PAYLOAD {
        return false;
    }
    let shown = written.min(HEAD);
    if rest[4..shown] != (!len).to_le_bytes()[..shown - 4] {
        return false;
    }
    let body = HEAD + len as usize;
    if written <= body {
        return true;
    }
    let checksum = written - body;
    checksum < TAIL && rest[body..written] == sha256(&rest[HEAD..body])[..checksum]
}
