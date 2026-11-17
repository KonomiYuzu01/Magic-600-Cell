//! FP.13's flat menu words (PS1.18 to PS1.21): literal `ia16` letters in mandatory chunks, `same_as` and
//! `inverse_of` annotations over half-open spans, and the X3 expansion bound.
//!
//! [`admit`] ports the format packet's `word_admission` exactly: dispatch, the cycle guard, both menu pins against the
//! independently pinned current menu, the payload fields, every chunk, the concatenated digest, then the parts and
//! the annotations in order. Annotations never generate letters and never relink a definition: a relation must match
//! the literal span it names.

use std::collections::BTreeMap;

use c600_digest::sha256_hex;

use super::codec::{canonical, charge, content_ref, int_value, nat, read_blob, sha_of, store_blob, Json};
use super::codes::{CANON, DIGEST, EXPANSION, FORMAT, IDENTITY};
use super::record::{has_exactly, supported};
use super::{require, Refusal};

/// A menu letter: `(grip, item)`, grip below 600 and item below 60.
pub type Letter = (u16, u16);

/// FP.13's payload fields of a menu word.
pub const PAYLOAD: [&str; 7] = ["alphabet", "start", "length", "letters", "chunks", "parts", "annotations"];

/// The current menu a word is admitted against: its identity digest and order digest, pinned independently of the
/// stored word (a stored pin is never its own expected value).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Menu {
    pub id: String,
    pub order: String,
}

/// What admission reads besides the record: stored chunk bytes by their stored digest, included word records by
/// content digest, the pinned inverse of each menu item, and the current menu.
pub struct Store<'a> {
    pub blobs: &'a BTreeMap<String, Vec<u8>>,
    pub records: &'a BTreeMap<String, Json>,
    pub inverse: &'a BTreeMap<u16, u16>,
    pub menu: &'a Menu,
}

/// FP.13's chunk size in letters, FP.9's `ia16_chunk_letters`.
pub fn chunk_letters() -> usize {
    super::codec::bounds("ia16_chunk_letters").1 as usize
}

/// Little-endian `60 * grip + item` per letter.
pub fn encode_ia16(letters: &[Letter]) -> Result<Vec<u8>, Refusal> {
    let mut raw = Vec::with_capacity(2 * letters.len());
    for &(grip, item) in letters {
        require(grip < 600 && item < 60, CANON)?;
        raw.extend_from_slice(&(60 * grip + item).to_le_bytes());
    }
    Ok(raw)
}

pub fn decode_ia16(raw: &[u8]) -> Result<Vec<Letter>, Refusal> {
    require(raw.len() % 2 == 0, CANON)?;
    raw.chunks_exact(2)
        .map(|pair| {
            let code = u16::from_le_bytes([pair[0], pair[1]]);
            require(code < 36_000, CANON).map(|()| (code / 60, code % 60))
        })
        .collect()
}

/// The mandatory chunks of a word's `ia16` bytes: full chunks of 524,288 letters and a final one of 1 to 524,288.
pub fn word_chunks(raw: &[u8]) -> Result<Vec<Vec<u8>>, Refusal> {
    require(raw.len() % 2 == 0, CANON)?;
    Ok(raw.chunks(2 * chunk_letters()).map(<[u8]>::to_vec).collect())
}

/// FP.13's chunk read: the order pin, each chunk's size and codes, the exact length and the concatenated digest.
pub fn read_chunks(
    chunks: &[Vec<u8>],
    length: i128,
    digest: &str,
    recorded_order: &str,
    current_order: &str,
) -> Result<Vec<Letter>, Refusal> {
    require(recorded_order == current_order, IDENTITY)?;
    for (i, part) in chunks.iter().enumerate() {
        charge("ia16_chunk_letters", (part.len() / 2) as u128)?;
        require(i + 1 == chunks.len() || part.len() == 2 * chunk_letters(), CANON)?;
        decode_ia16(part)?;
    }
    let raw = chunks.concat();
    require(raw.len() as i128 == 2 * length, CANON)?;
    require(sha256_hex(&raw) == digest, DIGEST)?;
    decode_ia16(&raw)
}

fn text(value: Option<&Json>) -> Option<&str> {
    value.and_then(Json::as_text)
}

fn list(value: Option<&Json>) -> Result<&[Json], Refusal> {
    match value {
        Some(Json::List(items)) => Ok(items),
        _ => Err(Refusal::new(FORMAT)),
    }
}

/// Python truthiness, as the reference reads an annotation's `relation`.
fn truthy(value: &Json) -> bool {
    match value {
        Json::Null | Json::Bool(false) | Json::Int(0) => false,
        Json::Text(s) => !s.is_empty(),
        Json::List(items) => !items.is_empty(),
        Json::Object(map) => !map.is_empty(),
        _ => true,
    }
}

/// FP.13's admission: the record's literal letters, or the refusal of the first rule it breaks.
pub fn admit(record: &Json, store: &Store) -> Result<Vec<Letter>, Refusal> {
    admit_within(record, store, &[])
}

fn admit_within(record: &Json, store: &Store, visiting: &[String]) -> Result<Vec<Letter>, Refusal> {
    supported(record)?;
    let digest = sha256_hex(&canonical(record));
    require(!visiting.contains(&digest), FORMAT)?;
    let identity = record.get("identity").ok_or(Refusal::new(FORMAT))?;
    require(text(identity.get("menu_id")) == Some(store.menu.id.as_str()), IDENTITY)?;
    require(text(identity.get("menu_order")) == Some(store.menu.order.as_str()), IDENTITY)?;
    let payload = record.get("payload").ok_or(Refusal::new(FORMAT))?;
    require(has_exactly(payload, &PAYLOAD), FORMAT)?;
    require(text(payload.get("alphabet")) == Some("menu"), FORMAT)?;
    let mut chunks = Vec::new();
    for reference in list(payload.get("chunks"))? {
        let stored = reference.get("stored").and_then(sha_of).ok_or(Refusal::new(FORMAT))?;
        let raw = store.blobs.get(stored).ok_or(Refusal::new(FORMAT))?;
        chunks.push(read_blob(reference, raw)?);
    }
    let letters_digest = payload.get("letters").and_then(sha_of).ok_or(Refusal::new(FORMAT))?;
    let letters = read_chunks(
        &chunks,
        int_value(payload.get("length").expect("a checked field"))?,
        letters_digest,
        text(identity.get("menu_order")).unwrap_or(""),
        &store.menu.order,
    )?;
    let mut parts: Vec<(&str, &Json)> = Vec::new();
    for part in list(payload.get("parts"))? {
        let id = text(part.get("id")).ok_or(Refusal::new(FORMAT))?;
        parts.push((id, part.get("word").ok_or(Refusal::new(FORMAT))?));
    }
    require(parts.windows(2).all(|w| w[0].0 < w[1].0), FORMAT)?;
    let mut inner = visiting.to_vec();
    inner.push(digest);
    let external = |reference: &Json| -> Result<Vec<Letter>, Refusal> {
        supported(reference)?;
        let content = reference.get("content");
        let sha = content.and_then(sha_of).ok_or(Refusal::new(FORMAT))?;
        let target = store.records.get(sha).ok_or(Refusal::new(FORMAT))?;
        supported(target)?;
        let same_pair =
            target.get("format") == reference.get("format") && target.get("version") == reference.get("version");
        require(same_pair, FORMAT)?;
        require(Some(&content_ref(target)) == content, DIGEST)?;
        require(target.get("identity") == Some(identity), IDENTITY)?;
        admit_within(target, store, &inner)
    };
    for (_, reference) in &parts {
        external(reference)?; // Included definitions need closure even if no annotation uses them.
    }
    let mut local: BTreeMap<&str, Vec<Letter>> = BTreeMap::new();
    for annotation in list(payload.get("annotations"))? {
        require(has_exactly(annotation, &["name", "start", "end", "relation"]), FORMAT)?;
        let start = int_value(annotation.get("start").expect("a checked field"))?;
        let end = int_value(annotation.get("end").expect("a checked field"))?;
        let name = text(annotation.get("name")).ok_or(Refusal::new(FORMAT))?;
        let in_range = 0 <= start && start <= end && end <= letters.len() as i128;
        require(in_range && !local.contains_key(name), FORMAT)?;
        let span = letters[start as usize..end as usize].to_vec();
        let relation = annotation.get("relation").expect("a checked field");
        if truthy(relation) {
            let kind = text(relation.get("kind"));
            let shaped = has_exactly(relation, &["kind", "word"]) && matches!(kind, Some("same_as" | "inverse_of"));
            require(shaped, FORMAT)?;
            let (tag, value) = match relation.get("word") {
                Some(Json::Object(target)) if target.len() == 1 => target.iter().next().expect("one entry"),
                _ => return Err(Refusal::new(FORMAT)),
            };
            let mut expected = match tag.as_str() {
                "local" => local.get(value.as_text().unwrap_or("\u{0}")).cloned().ok_or(Refusal::new(FORMAT))?,
                "part" => {
                    let id = value.as_text().unwrap_or("\u{0}");
                    let (_, reference) = parts.iter().find(|(p, _)| *p == id).ok_or(Refusal::new(FORMAT))?;
                    external(reference)?
                }
                "ref" => external(value)?,
                _ => return Err(Refusal::new(FORMAT)),
            };
            if kind == Some("inverse_of") {
                require(expected.iter().all(|(_, item)| store.inverse.contains_key(item)), FORMAT)?;
                expected = expected.iter().rev().map(|(grip, item)| (*grip, store.inverse[item])).collect();
            }
            require(span == expected, CANON)?;
        }
        local.insert(name, span);
    }
    Ok(letters)
}

/// The literal bytes a stored word holds, charged against `bundle_decoded`: storage budgets count every literal byte,
/// even when a small source graph generates them (PS1.20).
pub fn stored_bytes(record: &Json) -> Result<u128, Refusal> {
    let length = nat(record.get("payload").and_then(|p| p.get("length")).ok_or(Refusal::new(FORMAT))?)?;
    let bytes = length.saturating_mul(2);
    charge("bundle_decoded", bytes)?;
    Ok(bytes)
}

/// Replay admission (PS1.21): a word longer than the X3 bound is refused with its exact length before any chunk is
/// read; then the storage charge and FP.13's admission. Nothing is truncated.
pub fn expand(record: &Json, store: &Store, bound: u128) -> Result<Vec<Letter>, Refusal> {
    supported(record)?;
    let length = record.get("payload").and_then(|p| p.get("length")).ok_or(Refusal::new(FORMAT))?;
    if nat(length)? > bound {
        let mut refusal = Refusal::new(EXPANSION);
        refusal.length = length.as_text().map(str::to_string);
        return Err(refusal);
    }
    stored_bytes(record)?;
    admit(record, store)
}

/// A menu word record and its chunk bytes by stored digest. `annotations` are FP.13 annotation objects and `parts`
/// the `(id, Ref)` pairs in ID order.
pub fn write(
    identity: &Json,
    start: &Json,
    letters: &[Letter],
    parts: &[(String, Json)],
    annotations: &[Json],
    provenance: &Json,
) -> Result<(Json, BTreeMap<String, Vec<u8>>), Refusal> {
    let raw = encode_ia16(letters)?;
    let mut blobs = BTreeMap::new();
    let mut chunks = Vec::new();
    for chunk in word_chunks(&raw)? {
        let (reference, stored) = store_blob(&chunk);
        let name = reference.get("stored").and_then(sha_of).expect("store_blob writes a stored digest");
        blobs.insert(name.to_string(), stored);
        chunks.push(reference);
    }
    let parts = parts.iter().map(|(id, word)| Json::object([("id", Json::text(id)), ("word", word.clone())])).collect();
    let payload = Json::object([
        ("alphabet", Json::text("menu")),
        ("start", start.clone()),
        ("length", super::codec::nat_text(letters.len() as u128)),
        ("letters", super::codec::digest_ref("bytes", &raw)),
        ("chunks", Json::List(chunks)),
        ("parts", Json::List(parts)),
        ("annotations", Json::List(annotations.to_vec())),
    ]);
    let record = Json::object([
        ("format", Json::text("c600-word")),
        ("version", Json::Int(1)),
        ("identity", identity.clone()),
        ("payload", payload),
        ("provenance", provenance.clone()),
    ]);
    Ok((record, blobs))
}
