//! Stage 18's stored words, read as streams (shared by the stage 18 sub-checkers).
//!
//! The archive stores a step's parts as JSON word files and the lattice phase's words as flat `ia16` members:
//!
//! - a word file is an object `{"word": [[cap, item], ...], "input_digest": ..., "output_digest": ...}`, gzip-compressed
//!   when its name ends in `.gz`; the record states the SHA-256 of the JSON text of its word, as Python's `json.dumps`
//!   writes a list of letters: `[[c, i], [c, i]]`, with `", "` between items (stage 18 `steps.json`);
//! - a core is an object `{"P": [...], "Q": [...]}` naming the word P·Q·P⁻¹; its SHA-256 is that of its JSON text
//!   `{"P": [...], "Q": [...]}` in the same writing;
//! - an `ia16` member holds one letter per two bytes, the code 60·cap + item as a little-endian uint16, gzip-compressed
//!   when its name ends in `.gz`; its SHA-256 is that of the decoded bytes (stage 13's run records).
//!
//! Every reader checks the declared length before it decodes anything: a length past the admission bound of 10⁷
//! letters (LB5.12, gate 1) fails at once, and the decoder may not inflate past the bytes the declared length allows.
//! A word file's letters are then parsed one at a time from the decoded text and hashed as they pass; no list of
//! letters and no expanded word is built here. The only buffer proportional to a word is the decoded member itself,
//! bounded by its declared length.

use c600_jumble::model::{NP, POLES};
use c600_kernel::json::{self, Value};

use crate::claims::{array, at, dot, field, int, keys, reject, Step};

/// I_a's items per cap: a code is 60·cap + item.
pub const ITEMS: usize = 60;

/// LB5.12 gate 1's admission bound: at most 10⁷ literal letters per word.
pub const LITERAL_LIMIT: u64 = 10_000_000;

/// The largest expansion of DEFLATE: 258 bytes from a one-bit literal-length code and a one-bit distance code.
const DEFLATE_RATIO: usize = 1032;

/// The longest text of one letter in a word file, `[599, 59]` with the separator before it.
const LETTER_TEXT: u64 = 12;

/// The room a word file's other keys may take.
const HEADER_TEXT: u64 = 1024;

/// The admission check (LB5.12, gate 1): an exact length before expansion, at most 10⁷ letters.
pub fn admit(letters: u64, claim: &str) -> Step<()> {
    if letters <= LITERAL_LIMIT {
        Ok(())
    } else {
        Err(reject(
            claim,
            format!("{letters} letters exceed the admission bound of {LITERAL_LIMIT}, refused before expansion"),
        ))
    }
}

/// A stored member's bytes, decoded when its name ends in `.gz`. A gzip member may not inflate past `limit` bytes, and a
/// member that could not hold `limit` bytes is not the declared one; a plain member longer than `limit` fails.
pub fn decode(name: &str, bytes: &[u8], limit: usize, claim: &str) -> Step<Vec<u8>> {
    if name.ends_with(".gz") {
        // A gzip member inflates at most 1,032 times its size (RFC 1951), so the bound needs no decoding.
        let ceiling = bytes.len().saturating_mul(DEFLATE_RATIO);
        c600_kernel::archive::gunzip(bytes, limit.min(ceiling.max(1))).map_err(|error| {
            reject(claim, format!("the member {name} does not decode within its declared size: {}", error.0))
        })
    } else if bytes.len() <= limit {
        Ok(bytes.to_vec())
    } else {
        Err(reject(claim, format!("the member {name} holds {} bytes, more than its declared size allows", bytes.len())))
    }
}

/// The bytes a word file of `letters` letters may decode to.
pub fn text_limit(letters: u64, claim: &str) -> Step<usize> {
    letters
        .checked_mul(LETTER_TEXT)
        .and_then(|n| n.checked_add(HEADER_TEXT))
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| reject(claim, "too many letters"))
}

/// The digest fields of a word file and what its letters hashed to.
#[derive(Clone, Debug, Default)]
pub struct WordFile {
    pub letters: u64,
    pub sha256: String,
    pub input_digest: Option<String>,
    pub output_digest: Option<String>,
}

/// The text of one letter as `json.dumps` writes it.
fn letter_text(out: &mut Vec<u8>, cap: usize, item: usize) {
    out.push(b'[');
    out.extend_from_slice(cap.to_string().as_bytes());
    out.extend_from_slice(b", ");
    out.extend_from_slice(item.to_string().as_bytes());
    out.push(b']');
}

/// The JSON text of a list of letters, `[[c, i], ...]`.
pub fn letters_text(letters: &[(usize, usize)]) -> Vec<u8> {
    let mut out = vec![b'['];
    for (k, &(cap, item)) in letters.iter().enumerate() {
        if k > 0 {
            out.extend_from_slice(b", ");
        }
        letter_text(&mut out, cap, item);
    }
    out.push(b']');
    out
}

/// A letter's cap and item: a cap below 600 and an item below 60.
fn valid(cap: u64, item: u64, claim: &str) -> Step<(usize, usize)> {
    if cap < POLES as u64 && item < ITEMS as u64 {
        Ok((cap as usize, item as usize))
    } else {
        Err(reject(claim, format!("a letter's cap is 0 to {} and its item 0 to {}", POLES - 1, ITEMS - 1)))
    }
}

/// A cursor over a word file's decoded text.
struct Cursor<'t> {
    text: &'t [u8],
    at: usize,
    claim: &'t str,
}

impl<'t> Cursor<'t> {
    fn ws(&mut self) {
        while matches!(self.text.get(self.at), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.at += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.ws();
        self.text.get(self.at).copied()
    }

    fn expect(&mut self, byte: u8) -> Step<()> {
        if self.peek() == Some(byte) {
            self.at += 1;
            Ok(())
        } else {
            Err(reject(self.claim, format!("the word file is not of its form at byte {}", self.at)))
        }
    }

    /// A string without escapes (the keys and the hex digests of a word file).
    fn string(&mut self) -> Step<&'t str> {
        self.expect(b'"')?;
        let start = self.at;
        while let Some(&b) = self.text.get(self.at) {
            match b {
                b'"' => {
                    self.at += 1;
                    return std::str::from_utf8(&self.text[start..self.at - 1])
                        .map_err(|_| reject(self.claim, "a string of the word file is not UTF-8"));
                }
                b'\\' | 0..=0x1f => break,
                _ => self.at += 1,
            }
        }
        Err(reject(self.claim, format!("the word file has no plain string at byte {start}")))
    }

    /// A nonnegative integer in canonical decimal form.
    fn integer(&mut self) -> Step<u64> {
        self.ws();
        let start = self.at;
        let mut value: u64 = 0;
        while let Some(&b) = self.text.get(self.at) {
            if !b.is_ascii_digit() {
                break;
            }
            value = value
                .checked_mul(10)
                .and_then(|v| v.checked_add(u64::from(b - b'0')))
                .ok_or_else(|| reject(self.claim, "an integer of the word file is too large"))?;
            self.at += 1;
        }
        let digits = self.at - start;
        if digits == 0 || (digits > 1 && self.text[start] == b'0') {
            return Err(reject(self.claim, format!("the word file has no canonical integer at byte {start}")));
        }
        Ok(value)
    }
}

/// Streams a word file's decoded text: each letter is checked, hashed as the JSON text of the word and passed to
/// `each(position, cap, item)` in order, and the count may not pass `declared`. The keys are `word` and the digests
/// named in `digests`, each at most once; `word` is required.
pub fn stream_word_file(
    text: &[u8],
    claim: &str,
    declared: u64,
    digests: &[&str],
    mut each: impl FnMut(u64, usize, usize) -> Step<()>,
) -> Step<WordFile> {
    let mut cursor = Cursor { text, at: 0, claim };
    let mut file = WordFile::default();
    let mut seen_word = false;
    cursor.expect(b'{')?;
    if cursor.peek() == Some(b'}') {
        return Err(reject(claim, "the word file has no word"));
    }
    loop {
        let key = cursor.string()?;
        cursor.expect(b':')?;
        match key {
            "word" if !seen_word => {
                seen_word = true;
                let mut hash = c600_digest::Sha256::new();
                let mut buffer: Vec<u8> = Vec::with_capacity(1 << 16);
                buffer.push(b'[');
                cursor.expect(b'[')?;
                if cursor.peek() == Some(b']') {
                    cursor.at += 1;
                } else {
                    loop {
                        if file.letters == declared {
                            return Err(reject(
                                claim,
                                format!("the word holds more than its declared {declared} letters"),
                            ));
                        }
                        let path = at(&dot(claim, "word"), file.letters as usize);
                        cursor.expect(b'[')?;
                        let cap = cursor.integer()?;
                        cursor.expect(b',')?;
                        let item = cursor.integer()?;
                        cursor.expect(b']')?;
                        let (cap, item) = valid(cap, item, &path)?;
                        if file.letters > 0 {
                            buffer.extend_from_slice(b", ");
                        }
                        letter_text(&mut buffer, cap, item);
                        if buffer.len() >= 1 << 16 {
                            hash.update(&buffer);
                            buffer.clear();
                        }
                        each(file.letters, cap, item)?;
                        file.letters += 1;
                        match cursor.peek() {
                            Some(b',') => cursor.at += 1,
                            Some(b']') => {
                                cursor.at += 1;
                                break;
                            }
                            _ => {
                                return Err(reject(
                                    claim,
                                    format!("the word is not a list of letters at byte {}", cursor.at),
                                ))
                            }
                        }
                    }
                }
                buffer.push(b']');
                hash.update(&buffer);
                file.sha256 = c600_digest::hex(&hash.finish());
            }
            "input_digest" if digests.contains(&key) && file.input_digest.is_none() => {
                file.input_digest = Some(cursor.string()?.to_owned());
            }
            "output_digest" if digests.contains(&key) && file.output_digest.is_none() => {
                file.output_digest = Some(cursor.string()?.to_owned());
            }
            other => return Err(reject(claim, format!("the word file's key {other} is unknown or repeated"))),
        }
        match cursor.peek() {
            Some(b',') => cursor.at += 1,
            Some(b'}') => {
                cursor.at += 1;
                break;
            }
            _ => return Err(reject(claim, format!("the word file is not an object at byte {}", cursor.at))),
        }
    }
    cursor.ws();
    if cursor.at != text.len() {
        return Err(reject(claim, "the word file has text after its object"));
    }
    if !seen_word {
        return Err(reject(claim, "the word file has no word"));
    }
    for key in digests {
        let present = match *key {
            "input_digest" => file.input_digest.is_some(),
            _ => file.output_digest.is_some(),
        };
        if !present {
            return Err(reject(claim, format!("the word file has no {key}")));
        }
    }
    Ok(file)
}

/// A core `{"P": [...], "Q": [...]}`: its letters P and Q and the SHA-256 of its JSON text.
pub fn read_core(bytes: &[u8], claim: &str) -> Step<(Vec<(usize, usize)>, Vec<(usize, usize)>, String)> {
    let doc = json::parse(bytes).map_err(|error| reject(claim, format!("the core is not JSON: {error}")))?;
    keys(&doc, claim, &["P", "Q"], &[])?;
    let list = |key: &str| -> Step<Vec<(usize, usize)>> {
        let path = dot(claim, key);
        array(field(&doc, key, claim)?, &path)?
            .iter()
            .enumerate()
            .map(|(k, value)| pair(value, &at(&path, k)))
            .collect()
    };
    let (p, q) = (list("P")?, list("Q")?);
    let mut text = b"{\"P\": ".to_vec();
    text.extend_from_slice(&letters_text(&p));
    text.extend_from_slice(b", \"Q\": ");
    text.extend_from_slice(&letters_text(&q));
    text.push(b'}');
    Ok((p, q, c600_digest::sha256_hex(&text)))
}

/// A `[cap, item]` letter of the I_a menu.
pub fn pair(value: &Value, claim: &str) -> Step<(usize, usize)> {
    let items = array(value, claim)?;
    if items.len() != 2 {
        return Err(reject(claim, "a letter is [cap, item]"));
    }
    let (cap, item) = (int(&items[0], claim)?, int(&items[1], claim)?);
    match (u64::try_from(cap), u64::try_from(item)) {
        (Ok(cap), Ok(item)) => valid(cap, item, claim),
        _ => Err(reject(claim, "a letter's cap and item are nonnegative")),
    }
}

/// A unit `{"pieces": [...]}`: distinct piece IDs, at least one.
pub fn read_unit(bytes: &[u8], claim: &str) -> Step<Vec<u32>> {
    let doc = json::parse(bytes).map_err(|error| reject(claim, format!("the unit is not JSON: {error}")))?;
    keys(&doc, claim, &["pieces"], &[])?;
    let path = dot(claim, "pieces");
    let mut pieces = Vec::new();
    for (k, value) in array(field(&doc, "pieces", claim)?, &path)?.iter().enumerate() {
        let piece = int(value, &at(&path, k))?;
        match u32::try_from(piece) {
            Ok(p) if (p as usize) < NP => pieces.push(p),
            _ => return Err(reject(at(&path, k), format!("a piece is 0 to {}", NP - 1))),
        }
    }
    if pieces.is_empty() {
        return Err(reject(&path, "the unit is empty"));
    }
    let mut sorted = pieces.clone();
    sorted.sort_unstable();
    sorted.dedup();
    if sorted.len() != pieces.len() {
        return Err(reject(&path, "the unit names a piece twice"));
    }
    Ok(pieces)
}

/// An `ia16` member of `letters` letters: admitted (at most 10⁷) before anything is decoded, decoded within exactly
/// its declared size, every code naming a cap and an I_a item. Returns the decoded bytes and their SHA-256.
pub fn ia16_member(name: &str, bytes: &[u8], letters: u64, claim: &str) -> Step<(Vec<u8>, String)> {
    admit(letters, &dot(claim, "letters"))?;
    let size = usize::try_from(letters * 2).map_err(|_| reject(claim, "too many letters"))?;
    let decoded = decode(name, bytes, size, &dot(claim, "letters"))?;
    if decoded.len() != size {
        return Err(reject(dot(claim, "letters"), format!("the word holds {} bytes, not {size}", decoded.len())));
    }
    if let Some(k) = decoded.chunks_exact(2).position(|b| u16::from_le_bytes([b[0], b[1]]) as usize >= POLES * ITEMS) {
        return Err(reject(at(&dot(claim, "word"), k), "a letter code names no cap and item"));
    }
    let digest = c600_digest::sha256_hex(&decoded);
    Ok((decoded, digest))
}

/// The code of a letter.
pub fn code(cap: usize, item: usize) -> u16 {
    (cap * ITEMS + item) as u16
}

/// The letter of a code.
pub fn letter_of(code: u16) -> (usize, usize) {
    (code as usize / ITEMS, code as usize % ITEMS)
}
