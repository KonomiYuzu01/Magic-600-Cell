//! FP.5's canonical read, FP.7's gzip profile, FP.8's references and FP.9's limits (PS1.3 to PS1.11).
//!
//! The JSON value model and FP.5's writer are `c600_run::provenance`'s: [`canonical`] adds the terminal line feed and
//! [`parse`] reads strictly. This module adds the store's read rules on top: the size charges, the depth bound and the
//! byte equality that makes a value canonical.

use c600_digest::sha256;
pub use c600_run::provenance::{canonical, hex, parse, Json};

use super::codes::{CANON, DIGEST, FORMAT, SIZE};
use super::{require, Limit, Refusal};

/// FP.9's `c600-limits-1`: each row's inclusive low and high bound. Read and write bounds are equal.
pub const LIMITS: [(&str, u64, u64); 24] = [
    ("legacy_log_stored", 1, 16_777_216),
    ("legacy_log_decoded", 1, 25_165_824),
    ("legacy_log_events", 0, 10_000),
    ("legacy_log_primitives", 0, 2_000_000),
    ("recipe_steps", 1, 256),
    ("recipe_word_letters", 1, 100_000),
    ("recipe_primitives", 1, 3_000_000),
    ("legacy_part_depth", 0, 64),
    ("legacy_prefs_chars", 0, 500_000),
    ("record_stored", 0, 71_303_168),
    ("record_decoded", 0, 67_108_864),
    ("source_bytes", 0, 8_388_608),
    ("integer_digits", 1, 10_000),
    ("json_depth", 0, 64),
    ("graph_nodes", 0, 1_000_000),
    ("graph_edges", 0, 4_000_000),
    ("array_rank", 1, 8),
    ("array_elements", 0, 16_777_216),
    ("bundle_members", 0, 60_000),
    ("bundle_stored", 0, 2_147_483_648),
    ("bundle_decoded", 0, 4_294_967_296),
    ("archive_events", 0, 1_000_000),
    ("archive_replay", 0, 100_000_000),
    ("ia16_chunk_letters", 1, 524_288),
];

/// FP.7's header: DEFLATE, no flags, mtime zero, XFL zero, OS 255.
pub const GZIP_HEADER: [u8; 10] = [0x1f, 0x8b, 0x08, 0, 0, 0, 0, 0, 0, 0xff];
/// FP.7's profile name, the `encoding` of a gzip member's BlobRef.
pub const GZIP_PROFILE: &str = "c600-gzip-stored-1";
/// FP.7: new members of at most this many decoded bytes are raw; larger ones use the gzip profile.
pub const RAW_MEMBER_MAX: usize = 250_000;
const BLOCK: usize = 65_535;

/// A row's bounds. Panics on a row FP.9 does not have: rows are named by this crate's code, never by input.
pub fn bounds(row: &str) -> (u64, u64) {
    LIMITS.iter().find(|(name, _, _)| *name == row).map(|(_, low, high)| (*low, *high)).expect("an FP.9 row")
}

/// FP.9's charge: `amount` must lie within the row's bounds; otherwise `PS1:resource_limit.persistence_size` with the
/// row, its limit and the attempted amount (PS1.11).
pub fn charge(row: &'static str, amount: u128) -> Result<(), Refusal> {
    let (low, high) = bounds(row);
    if (low as u128..=high as u128).contains(&amount) {
        return Ok(());
    }
    Err(Refusal { code: SIZE, limit: Some(Limit { row, max: high, attempted: amount.to_string() }), length: None })
}

/// The deepest container nesting of JSON text, counted outside strings.
fn depth(raw: &[u8]) -> usize {
    let (mut level, mut deepest, mut in_string, mut escaped) = (0usize, 0usize, false, false);
    for &b in raw {
        if in_string {
            match (escaped, b) {
                (true, _) => escaped = false,
                (false, b'\\') => escaped = true,
                (false, b'"') => in_string = false,
                _ => {}
            }
            continue;
        }
        match b {
            b'"' => in_string = true,
            b'[' | b'{' => {
                level += 1;
                deepest = deepest.max(level);
            }
            b']' | b'}' => level = level.saturating_sub(1),
            _ => {}
        }
    }
    deepest
}

/// FP.5's active read: the stored bytes must be exactly the canonical bytes of the value they parse to. Draft: a value
/// nested deeper than FP.9's `json_depth` is refused for its size before its syntax is read.
pub fn read_canonical(raw: &[u8]) -> Result<Json, Refusal> {
    charge("record_decoded", raw.len() as u128)?;
    charge("json_depth", depth(raw) as u128)?;
    let value = parse(raw).ok_or(Refusal::new(CANON))?;
    require(canonical(&value) == raw, CANON)?;
    Ok(value)
}

/// FP.11's Nat and FP.6's integer text: `0` or an optionally negative decimal without leading zeros, charged against
/// `integer_digits`. Draft: a value beyond `i128` saturates. Every FP.9 bound is far below that range, so saturation
/// keeps each comparison's outcome; a saturated value is never written back.
pub fn int_value(value: &Json) -> Result<i128, Refusal> {
    let text = value.as_text().ok_or(Refusal::new(CANON))?;
    let digits = text.strip_prefix('-').unwrap_or(text);
    let decimal = !digits.is_empty()
        && digits.bytes().all(|b| b.is_ascii_digit())
        && (digits == "0" && !text.starts_with('-') || !digits.starts_with('0'));
    require(decimal, CANON)?;
    charge("integer_digits", digits.len() as u128)?;
    let magnitude = digits.bytes().fold(0i128, |n, b| n.saturating_mul(10).saturating_add((b - b'0') as i128));
    Ok(if text.starts_with('-') { -magnitude } else { magnitude })
}

/// A Nat field: [`int_value`] that is not negative.
pub fn nat(value: &Json) -> Result<u128, Refusal> {
    let n = int_value(value)?;
    require(n >= 0, CANON)?;
    Ok(n as u128)
}

/// FP.6's Nat text of a count.
pub fn nat_text(n: u128) -> Json {
    Json::Text(n.to_string())
}

/// FP.7's compressor: stored DEFLATE blocks of at most 65,535 bytes, then CRC32 and size modulo 2^32.
pub fn pack_gzip(raw: &[u8]) -> Vec<u8> {
    let mut out = GZIP_HEADER.to_vec();
    let mut start = 0;
    loop {
        let end = (start + BLOCK).min(raw.len());
        let part = &raw[start..end];
        out.push((end == raw.len()) as u8);
        out.extend_from_slice(&(part.len() as u16).to_le_bytes());
        out.extend_from_slice(&(part.len() as u16 ^ 0xFFFF).to_le_bytes());
        out.extend_from_slice(part);
        if end == raw.len() {
            break;
        }
        start = end;
    }
    out.extend_from_slice(&c600_kernel::archive::crc32(raw).to_le_bytes());
    out.extend_from_slice(&(raw.len() as u32).to_le_bytes());
    out
}

/// FP.7's strict reader: the exact header, stored blocks only, every non-final block full, the decoded size bounded
/// before each block is copied, one member and no trailing bytes, the CRC and size, and the exact profile bytes.
pub fn unpack_gzip(packed: &[u8]) -> Result<Vec<u8>, Refusal> {
    charge("record_stored", packed.len() as u128)?;
    require(packed.len() >= 10 && packed[..10] == GZIP_HEADER, CANON)?;
    let end = packed.len().saturating_sub(8);
    let (mut offset, mut out) = (10usize, Vec::new());
    loop {
        require(offset + 5 <= end, CANON)?;
        let flag = packed[offset];
        let count = u16::from_le_bytes([packed[offset + 1], packed[offset + 2]]);
        let inverse = u16::from_le_bytes([packed[offset + 3], packed[offset + 4]]);
        require(flag <= 1 && inverse == count ^ 0xFFFF, CANON)?;
        require(flag == 1 || count as usize == BLOCK, CANON)?;
        require(offset + 5 + count as usize <= end, CANON)?;
        charge("record_decoded", (out.len() + count as usize) as u128)?;
        out.extend_from_slice(&packed[offset + 5..offset + 5 + count as usize]);
        offset += 5 + count as usize;
        if flag == 1 {
            break;
        }
    }
    require(offset + 8 == packed.len(), CANON)?;
    let mut trailer = c600_kernel::archive::crc32(&out).to_le_bytes().to_vec();
    trailer.extend_from_slice(&(out.len() as u32).to_le_bytes());
    require(packed[offset..] == trailer[..], DIGEST)?;
    require(pack_gzip(&out) == packed, CANON)?;
    Ok(out)
}

/// FP.8's `DigestRef` of bytes under `role`.
pub fn digest_ref(role: &str, data: &[u8]) -> Json {
    c600_run::provenance::digest_ref(role, &sha256(data))
}

/// The SHA-256 of FP.5 bytes as a `content` DigestRef: a record's content identity.
pub fn content_ref(value: &Json) -> Json {
    digest_ref("content", &canonical(value))
}

/// The `sha256` text of a DigestRef-shaped value, if it has one.
pub fn sha_of(reference: &Json) -> Option<&str> {
    reference.get("sha256").and_then(Json::as_text)
}

/// FP.8's raw `BlobRef` of a member's bytes.
pub fn blob(data: &[u8]) -> Json {
    Json::object([
        ("stored", digest_ref("bytes", data)),
        ("decoded", digest_ref("bytes", data)),
        ("stored_bytes", nat_text(data.len() as u128)),
        ("decoded_bytes", nat_text(data.len() as u128)),
        ("encoding", Json::text("raw")),
    ])
}

/// FP.7 and FP.8's writer: a new member's BlobRef and its stored bytes, raw up to 250,000 decoded bytes and gzip
/// above.
pub fn store_blob(data: &[u8]) -> (Json, Vec<u8>) {
    if data.len() <= RAW_MEMBER_MAX {
        return (blob(data), data.to_vec());
    }
    let packed = pack_gzip(data);
    let reference = Json::object([
        ("stored", digest_ref("bytes", &packed)),
        ("decoded", digest_ref("bytes", data)),
        ("stored_bytes", nat_text(packed.len() as u128)),
        ("decoded_bytes", nat_text(data.len() as u128)),
        ("encoding", Json::text(GZIP_PROFILE)),
    ]);
    (reference, packed)
}

/// FP.8's reader: the declared encoding, sizes and both digests must agree with the stored bytes. A `captured`
/// member needs its legacy decoder and is not read here.
pub fn read_blob(reference: &Json, stored: &[u8]) -> Result<Vec<u8>, Refusal> {
    let fields = ["stored", "decoded", "stored_bytes", "decoded_bytes", "encoding"];
    let shaped =
        matches!(reference, Json::Object(m) if m.len() == fields.len() && fields.iter().all(|f| m.contains_key(*f)));
    require(shaped, FORMAT)?;
    let field = |name: &str| reference.get(name).expect("a checked field");
    for (name, role) in [("stored", "bytes"), ("decoded", "bytes")] {
        let r = field(name);
        require(r.get("role").and_then(Json::as_text) == Some(role) && sha_of(r).is_some(), FORMAT)?;
    }
    let stored_bytes = nat(field("stored_bytes"))?;
    let decoded_bytes = nat(field("decoded_bytes"))?;
    charge("record_stored", stored_bytes)?;
    charge("record_decoded", decoded_bytes)?;
    require(stored.len() as u128 == stored_bytes, DIGEST)?;
    require(Some(hex(&sha256(stored)).as_str()) == sha_of(field("stored")), DIGEST)?;
    let decoded = match field("encoding").as_text() {
        Some("raw") => stored.to_vec(),
        Some(GZIP_PROFILE) => unpack_gzip(stored)?,
        _ => return Err(Refusal::new(FORMAT)),
    };
    require(decoded.len() as u128 == decoded_bytes, DIGEST)?;
    require(Some(hex(&sha256(&decoded)).as_str()) == sha_of(field("decoded")), DIGEST)?;
    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CT-PS1-06 (part: the product's FP.7 writer and its strict one-member reader round-trip 0 to 250,001 bytes across
    /// the 65,535-byte block boundaries with FP.7's overhead and refuse a second member; not run: the product writer's
    /// bytes against the three frozen vectors (header and stored digest), and a tampered stored or decoded digest in
    /// `read_blob`).
    #[test]
    fn gzip_round_trips_across_block_boundaries() {
        for size in [0usize, 1, 65_535, 65_536, 250_000, 250_001] {
            let raw = vec![b'a'; size];
            let packed = pack_gzip(&raw);
            assert_eq!(packed.len(), 18 + size + 5 * size.div_ceil(BLOCK).max(1), "FP.7's overhead at {size}");
            assert_eq!(unpack_gzip(&packed).unwrap(), raw);
            let mut twice = packed.clone();
            twice.extend_from_slice(&packed);
            assert_eq!(unpack_gzip(&twice).unwrap_err().code, CANON, "a second member");
        }
    }

    #[test]
    fn int_value_reads_only_decimal_text() {
        assert_eq!(int_value(&Json::text("0")), Ok(0));
        assert_eq!(int_value(&Json::text("-12")), Ok(-12));
        for bad in ["", "-", "-0", "01", "1.0", "+1", " 1"] {
            assert_eq!(int_value(&Json::text(bad)).unwrap_err().code, CANON, "{bad:?}");
        }
        assert_eq!(int_value(&Json::Int(1)).unwrap_err().code, CANON);
        let refusal = int_value(&Json::Text("1".repeat(10_001))).unwrap_err();
        assert_eq!(refusal.code, SIZE);
        assert_eq!(refusal.limit.unwrap().row, "integer_digits");
        assert_eq!(int_value(&Json::Text("9".repeat(10_000))), Ok(i128::MAX), "saturates");
    }
}
