//! MPUltimate v1 logs as interchange for 600-cell-Full only, with 0.4's `mpult_log.py` rules: check, import and
//! export against a native profile that 0.4's bridge check produced (`C600-NATIVE-PROFILE-v1`).
//!
//! The caps are 0.4's: at most 16 MiB of file, 20,000 sequence entries (`MAX_ENTRIES`) and 10,000 native moves
//! (`MAX_MOVES`). A log of another puzzle is refused with `KN1:input.identity_mismatch`, a log over a cap with
//! `PS1:resource_limit.persistence_size`, another MPUltimate version with `PS2:input.persistence_version`.
//!
//! Import replays every native move through the profile's verified words, keeps the redo tail after the saved pointer
//! as moves after the saved cursor, and checks the colors at the pointer against the replay. Export writes the line
//! from the start through the saved cursor and on along the last redo branch, with the pointer at the saved cursor and
//! the colors of its state. A transaction that came from an MPUlt import is written with the sequence entries it was
//! read from, so an unchanged import exports to the same entries; any other transaction is written letter by letter
//! through the profile's translations, as 0.4 exports.

use std::collections::HashMap;

use c600_kernel::archive::gunzip;
use c600_kernel::json::{self, Value};
use c600_kernel::{Derivation, Kernel, ProtectedSet, Snapshot, SLOTS};

use crate::compile::Compiled;
use crate::log::{
    Commit, Cursor, CursorAction, Interchange, Log, MoveAction, MoveEntry, MpultOrigin, Origin, Saved, Start,
};
use crate::problem::{code, Gate, Problem};
use crate::record::recipe_value;

/// The origin format name 0.4 records (`native_log.format`).
pub const FORMAT: &str = crate::log::MPULT_ORIGIN;
/// The puzzle name in MPUlt.
pub const NAME: &str = "600-cell-Full";
/// The file extension MPUlt writes, without its dot.
pub const EXTENSION: &str = "log";
/// `MAX_MOVES`: native moves in one log.
pub const MAX_MOVES: usize = 10_000;
/// `MAX_ENTRIES`: sequence entries in one log, macro markers included.
pub const MAX_ENTRIES: usize = 20_000;
/// `MAX_LOG_BYTES`.
pub const MAX_LOG_BYTES: usize = 16 * 1024 * 1024;
/// The largest timer in milliseconds.
pub const MAX_TIMER: u64 = 922_337_203_685_477;
/// The profile's format.
pub const PROFILE_FORMAT: &str = "C600-NATIVE-PROFILE-v1";
/// The puzzle description lines of 600-cell-Full in `native/runtime/MPUlt_puzzles.txt`, stripped.
pub const DESCRIPTION: [&str; 8] = [
    "Dim 4",
    "NAxis 1",
    "Faces 4.236068,1,1,1",
    "Group 0,4.472136,-7.236068,0/0,5.854102,-9.472136,8.09017 0,0,1,0/0,0,0,1 -0.618034,1.618034,1,0/0,0,0,1",
    "Axis 4.236068,1,1,1",
    "Twists 0,1.618034,-0.618034,-1/-0.618034,1,0,1.618034 0,2,-1,-1/0,1,1,-2",
    "Cuts 0.968 -0.968",
    "FixedMask 2",
];

const DIGITS: &[u8; 36] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const STICKERS_PER_FACE: i32 = 433;
const FACES: usize = 600;
const NOTE: &str = "Imported native MPUlt move";
const SCRAMBLE: &str = "recorded-scramble-native-import";
const MANUAL: &str = "manual-native-import";

fn problem(code: &'static str, at: impl Into<String>, message: impl Into<String>) -> Problem {
    Problem::new(code, Gate::Interchange, at, message)
}

fn format_problem(at: impl Into<String>, message: impl Into<String>) -> Problem {
    problem(code::PERSISTENCE_FORMAT, at, message)
}

// ---------------------------------------------------------------------------------------------------------------
// The profile

/// What 0.4's `profile_data` reads from a passed native bridge check.
#[derive(Clone, Debug)]
pub struct NativeProfile {
    model_id: String,
    /// The lab slot of each native sticker.
    native_to_lab: Vec<usize>,
    /// The native face of each lab face.
    inverse_faces: Vec<u16>,
    /// Each native token `axis:twist:angle:mask` (angle reduced) and its lab word.
    token_words: HashMap<String, Vec<i32>>,
    /// The order of each axis and twist: the largest angle plus one.
    orders: HashMap<(i64, i64), i64>,
    /// The native token of each positive lab letter.
    translations: HashMap<i32, [i64; 4]>,
    sha256: String,
}

impl NativeProfile {
    /// Reads a profile's bytes (its canonical JSON) and checks it as 0.4 does, plus its format and `profile_sha256`.
    pub fn read(kernel: &Kernel, bytes: &[u8]) -> Result<NativeProfile, Problem> {
        let value = json::parse(bytes).map_err(|_| format_problem("profile", "The native profile is not JSON"))?;
        let Value::Object(fields) = &value else {
            return Err(format_problem("profile", "The native profile is not an object"));
        };
        match fields.get("format").and_then(Value::as_str) {
            Some(PROFILE_FORMAT) => {}
            Some(_) => {
                return Err(problem(code::PERSISTENCE_VERSION, "profile.format", "Unknown native profile format"))
            }
            None => return Err(format_problem("profile.format", "The native profile names no format")),
        }
        // 0.4 hashes `canonical(profile)` before adding `profile_sha256` and writes the same text with it added. Its
        // `translations` keys were integers then, sorted as numbers, so the digest is checked on the file's own bytes
        // with the member taken out, not on a re-encoding.
        let claimed = fields.get("profile_sha256").and_then(Value::as_str).unwrap_or_default().to_owned();
        let member = format!("\"profile_sha256\":\"{claimed}\"");
        let text = std::str::from_utf8(bytes).unwrap_or_default();
        let without = [format!(",{member}"), format!("{member},")]
            .iter()
            .find(|m| text.matches(m.as_str()).count() == 1)
            .map(|m| text.replacen(m.as_str(), "", 1));
        if without.is_none_or(|text| c600_digest::sha256_hex(text.as_bytes()) != claimed) {
            return Err(problem(code::DIGEST_MISMATCH, "profile.profile_sha256", "The native profile digest differs"));
        }
        let required = "MPUlt log IO requires the current full native bridge to pass first";
        let model_id = fields.get("model_id").and_then(Value::as_str).unwrap_or_default();
        if model_id != kernel.identity().model_id {
            return Err(problem(code::IDENTITY_MISMATCH, "profile.model_id", required));
        }
        if fields.get("matched_stickers").and_then(Value::as_i64) != Some(SLOTS as i64)
            || fields.get("matched_generators").and_then(Value::as_i64) != Some(1200)
        {
            return Err(format_problem("profile", required));
        }
        let invalid = || format_problem("profile", "Invalid native bridge mapping");
        let permutation = |key: &str, n: usize| -> Result<Vec<usize>, Problem> {
            let items = fields.get(key).and_then(Value::as_array).ok_or_else(invalid)?;
            let values: Vec<usize> = items
                .iter()
                .map(|v| v.as_i64().and_then(|v| usize::try_from(v).ok()).filter(|&v| v < n))
                .collect::<Option<_>>()
                .ok_or_else(invalid)?;
            let mut seen = vec![false; n];
            if values.len() != n || values.iter().any(|&v| std::mem::replace(&mut seen[v], true)) {
                return Err(invalid());
            }
            Ok(values)
        };
        let native_to_lab = permutation("native_to_lab", SLOTS)?;
        let faces = permutation("native_face_to_lab", FACES)?;
        let mut inverse_faces = vec![0u16; FACES];
        for (native, &lab) in faces.iter().enumerate() {
            inverse_faces[lab] = native as u16;
        }
        let table = fields.get("token_words").and_then(Value::as_object).ok_or_else(invalid)?;
        let mut token_words = HashMap::with_capacity(table.len());
        let mut orders: HashMap<(i64, i64), i64> = HashMap::new();
        for (token, word) in table {
            let [axis, twist, angle, _] = token_parts(token).ok_or_else(invalid)?;
            let order = orders.entry((axis, twist)).or_insert(0);
            *order = (*order).max(angle + 1);
            let word: Vec<i32> = word
                .as_array()
                .ok_or_else(invalid)?
                .iter()
                .map(|m| m.as_i64().and_then(|m| i32::try_from(m).ok()))
                .collect::<Option<_>>()
                .ok_or_else(invalid)?;
            token_words.insert(token.clone(), word);
        }
        let mut translations = HashMap::new();
        for (letter, token) in fields.get("translations").and_then(Value::as_object).ok_or_else(invalid)? {
            let letter: i32 = letter.parse().map_err(|_| invalid())?;
            let part = |key: &str| token.get(key).and_then(Value::as_i64).ok_or_else(invalid);
            translations.insert(letter, [part("axis")?, part("twist")?, part("angle")?, part("mask")?]);
        }
        Ok(NativeProfile {
            model_id: model_id.to_owned(),
            native_to_lab,
            inverse_faces,
            token_words,
            orders,
            translations,
            sha256: claimed,
        })
    }

    /// The profile's `profile_sha256`.
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    /// 0.4's `native_colors`: the native face color of every native sticker in a state.
    pub fn colors(&self, state: &Snapshot) -> Vec<u16> {
        let labels = state.labels();
        self.native_to_lab.iter().map(|&slot| self.inverse_faces[(labels[slot] / STICKERS_PER_FACE) as usize]).collect()
    }

    /// The lab word of a native move token, with its angle reduced by the twist's order; `None` when the token is not
    /// a supported, verified move.
    fn word(&self, token: &str) -> Option<&[i32]> {
        let [axis, twist, angle, mask] = token_parts(token)?;
        if [axis, twist, angle, mask].iter().any(|v| !(-32768..=32767).contains(v)) {
            return None;
        }
        let order = *self.orders.get(&(axis, twist))?;
        if !(order == 2 || order == 3) || ![0, 1, 4, 5].contains(&mask) {
            return None;
        }
        let normal = format!("{axis}:{twist}:{}:{mask}", angle.rem_euclid(order));
        self.token_words.get(&normal).map(Vec::as_slice)
    }

    /// The native token of one lab letter, as 0.4's export writes it.
    fn token(&self, letter: i32) -> Result<String, Problem> {
        let [axis, twist, mut angle, mask] = *self
            .translations
            .get(&letter.abs())
            .ok_or_else(|| format_problem("profile", "Native bridge lacks an export generator"))?;
        if letter < 0 {
            let order = *self.orders.get(&(axis, twist)).unwrap_or(&1);
            angle = (-angle).rem_euclid(order);
        }
        let token = format!("{axis}:{twist}:{angle}:{mask}");
        if !self.token_words.contains_key(&token) {
            return Err(format_problem("profile", "Unsupported native export token"));
        }
        Ok(token)
    }
}

/// The four integers of `a:t:angle:mask`, each `-?[0-9]+`.
fn token_parts(token: &str) -> Option<[i64; 4]> {
    let mut parts = token.split(':');
    let mut out = [0i64; 4];
    for slot in &mut out {
        let part = parts.next()?;
        let digits = part.strip_prefix('-').unwrap_or(part);
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        // A field outside i64 is outside the signed 16-bit range too.
        *slot = part.parse().unwrap_or(i64::MAX);
    }
    parts.next().is_none().then_some(out)
}

// ---------------------------------------------------------------------------------------------------------------
// The wire form

/// 0.4's `pack`: four signed 16-bit fields in one 64-bit word.
fn pack(parts: [i64; 4]) -> u64 {
    parts.iter().enumerate().fold(0u64, |acc, (i, &v)| acc | (((v as u64) & 0xffff) << (16 * i)))
}

/// 0.4's `checksum`: a multiplicative hash over the counters, the sequence and the colors, bit-reversed.
fn checksum(lseq: u64, shuffle: u64, ptr: u64, timer: u64, sequence: &[i64], colors: &[u16]) -> u64 {
    let step = |value: u64, item: u64| value.wrapping_mul(305_419_893).wrapping_add(item);
    let mut value = 0u64;
    for item in [SLOTS as u64, lseq, shuffle, ptr, timer] {
        value = step(value, item);
    }
    for &item in sequence {
        value = step(value, item as u64);
    }
    for &item in colors {
        value = step(value, u64::from(item));
    }
    value.reverse_bits()
}

fn sequence_entry(token: &str) -> i64 {
    match token {
        "m[" => -2,
        "m]" => -1,
        _ => token_parts(token).map_or(0, |parts| pack(parts) as i64),
    }
}

/// 0.4's `serialize`: the exact v1 wire form of colors, sequence entries (without the boundary marker, which goes
/// before entry `shuffle`), pointer and timer, with its checksum. It checks nothing; [`check`] does.
pub fn serialize(colors: &[u16], tokens: &[String], shuffle: usize, ptr: usize, timer: u64) -> Vec<u8> {
    let sequence: Vec<i64> = tokens.iter().map(|t| sequence_entry(t)).collect();
    let mut lines = vec![format!("MPUltimate v1 {NAME} {} {shuffle} {ptr}", tokens.len()), format!("Puzzle {NAME}")];
    lines.extend(DESCRIPTION.iter().map(|line| line.to_string()));
    lines.push("EndPuzzle".to_owned());
    lines.push(format!("2 {SLOTS}"));
    let encoded: Vec<u8> =
        colors.iter().flat_map(|&c| [DIGITS[(c % 36) as usize], DIGITS[(c / 36) as usize]]).collect();
    lines.extend(encoded.chunks(256).map(|chunk| String::from_utf8_lossy(chunk).into_owned()));
    let crc = checksum(tokens.len() as u64, shuffle as u64, ptr as u64, timer, &sequence, colors);
    lines.extend([format!("#timer {timer}"), format!("#CRC {crc}"), "*".to_owned()]);
    for start in (0..tokens.len()).step_by(16) {
        let mut chunk = Vec::new();
        for (index, token) in tokens.iter().enumerate().skip(start).take(16) {
            if index == shuffle {
                chunk.push("m|");
            }
            chunk.push(token.as_str());
        }
        let end = if start + 16 >= tokens.len() { " ." } else { "" };
        lines.push(format!("{}{end}", chunk.join(" ")));
    }
    let mut text = lines.join("\n");
    text.push('\n');
    text.into_bytes()
}

/// Python's `str.splitlines()`.
fn splitlines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        let boundary = matches!(
            c,
            '\n' | '\r' | '\u{b}' | '\u{c}' | '\u{1c}' | '\u{1d}' | '\u{1e}' | '\u{85}' | '\u{2028}' | '\u{2029}'
        );
        if boundary {
            lines.push(&text[start..i]);
            let mut end = i + c.len_utf8();
            if c == '\r' {
                if let Some(&(j, '\n')) = chars.peek() {
                    chars.next();
                    end = j + 1;
                }
            }
            start = end;
        }
    }
    if start < text.len() {
        lines.push(&text[start..]);
    }
    lines
}

/// Python's whitespace for `str.split()` and `str.strip()`.
fn is_space(c: char) -> bool {
    c.is_whitespace() || matches!(c, '\u{1c}'..='\u{1f}')
}

fn strip(text: &str) -> &str {
    text.trim_matches(is_space)
}

fn split(text: &str) -> Vec<&str> {
    text.split(is_space).filter(|part| !part.is_empty()).collect()
}

/// 0.4's `normalize_description`: whitespace joined by one space and every decimal number in its shortest exact
/// spelling, so that trailing zeros never change the comparison.
fn normalize_description(line: &str) -> Result<String, Problem> {
    let joined = split(line).join(" ");
    let bytes = joined.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        match number_at(bytes, i) {
            Some(end) => {
                out.push_str(&canonical_decimal(&joined[i..end])?);
                i = end;
            }
            None => {
                let c = joined[i..].chars().next().unwrap_or_default();
                out.push(c);
                i += c.len_utf8().max(1);
            }
        }
    }
    Ok(out)
}

/// The end of `[-+]?(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][-+]?[0-9]+)?` matched at `start`.
fn number_at(bytes: &[u8], start: usize) -> Option<usize> {
    let digits = |mut i: usize| {
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        i
    };
    let mut i = start;
    if matches!(bytes.get(i), Some(b'-' | b'+')) {
        i += 1;
    }
    let integer_end = digits(i);
    if integer_end > i {
        i = integer_end;
        if bytes.get(i) == Some(&b'.') {
            i = digits(i + 1);
        }
    } else if bytes.get(i) == Some(&b'.') && digits(i + 1) > i + 1 {
        i = digits(i + 1);
    } else {
        return None;
    }
    if matches!(bytes.get(i), Some(b'e' | b'E')) {
        let mut j = i + 1;
        if matches!(bytes.get(j), Some(b'-' | b'+')) {
            j += 1;
        }
        let end = digits(j);
        if end > j {
            i = end;
        }
    }
    Some(i)
}

/// Python's `format(Decimal(token), 'f')` with trailing zeros and a trailing point removed, and zero as `0`.
fn canonical_decimal(token: &str) -> Result<String, Problem> {
    if token.len() > 64 {
        return Err(format_problem("description", "Oversized native description number"));
    }
    let (negative, rest) = match token.as_bytes()[0] {
        b'-' => (true, &token[1..]),
        b'+' => (false, &token[1..]),
        _ => (false, token),
    };
    let (mantissa, exponent) = match rest.find(['e', 'E']) {
        Some(at) => (&rest[..at], rest[at + 1..].parse::<i64>().unwrap_or(i64::MAX)),
        None => (rest, 0),
    };
    let (integer, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let exponent = exponent.saturating_sub(fraction.len() as i64);
    if exponent.unsigned_abs() > 100 {
        return Err(format_problem("description", "Native description exponent out of range"));
    }
    let all = format!("{integer}{fraction}");
    let digits = all.trim_start_matches('0');
    if digits.is_empty() {
        return Ok("0".to_owned());
    }
    let mut fixed = if exponent >= 0 {
        format!("{digits}{}", "0".repeat(exponent as usize))
    } else {
        let point = digits.len() as i64 + exponent;
        let text = if point <= 0 {
            format!("0.{}{digits}", "0".repeat((-point) as usize))
        } else {
            format!("{}.{}", &digits[..point as usize], &digits[point as usize..])
        };
        text.trim_end_matches('0').trim_end_matches('.').to_owned()
    };
    if negative {
        fixed.insert(0, '-');
    }
    Ok(fixed)
}

// ---------------------------------------------------------------------------------------------------------------
// Check and import

/// One transaction of an MPUlt log: one native move of at least one letter.
#[derive(Clone, Debug)]
pub struct MpultEvent {
    /// The sequence entries that produced it: macro markers and moves of no letter before it, then its own token.
    pub tokens: Vec<String>,
    pub word: Vec<i32>,
    pub pre: String,
    pub post: String,
    pub assistance: String,
}

/// What 0.4's `import_native` returns.
#[derive(Clone, Debug)]
pub struct MpultPlan {
    pub events: Vec<MpultEvent>,
    /// The state at the saved pointer.
    pub state: Snapshot,
    /// The transactions before the pointer; the rest are the redo tail.
    pub selected_count: usize,
    pub primitive_count: u64,
    pub sequence_entries: u64,
    pub pointer: u64,
    pub shuffle: u64,
    pub timer: u64,
    /// The sequence entries after the last transaction.
    pub trailing: Vec<String>,
}

/// A decimal counter; `None` when it does not fit 64 bits, which is past every limit.
fn counter(text: &str) -> Option<u64> {
    text.parse().ok()
}

/// The header's counters: sequence entries, shuffle boundary and pointer.
type Counters = (Option<u64>, Option<u64>, Option<u64>);

/// The header's three counters, or the code that refuses it.
fn header(line: &str) -> Result<Counters, Problem> {
    let unsupported = "Only MPUltimate v1 full 600-cell logs are supported";
    let parts: Vec<&str> = line.split(' ').collect();
    let number = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    match parts.as_slice() {
        ["MPUltimate", "v1", NAME, a, b, c] if number(a) && number(b) && number(c) => {
            Ok((counter(a), counter(b), counter(c)))
        }
        ["MPUltimate", "v1", puzzle, ..] if *puzzle != NAME => {
            Err(problem(code::IDENTITY_MISMATCH, "header", unsupported))
        }
        ["MPUltimate", version, ..] if *version != "v1" => {
            Err(problem(code::PERSISTENCE_VERSION, "header", unsupported))
        }
        _ => Err(format_problem("header", unsupported)),
    }
}

/// Checks an MPUlt log's bytes against a profile and replays it, changing nothing.
pub fn check(kernel: &Kernel, profile: &NativeProfile, bytes: &[u8]) -> Result<MpultPlan, Problem> {
    if bytes.is_empty() || bytes.len() > MAX_LOG_BYTES {
        return Err(problem(code::PERSISTENCE_SIZE, "bytes", "MPUlt log must contain 1 byte..16 MiB"));
    }
    if profile.model_id != kernel.identity().model_id {
        return Err(problem(code::IDENTITY_MISMATCH, "profile", "The native profile belongs to another model"));
    }
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
    let text = std::str::from_utf8(bytes).map_err(|_| format_problem("bytes", "MPUlt log must be UTF-8 text"))?;
    let lines = splitlines(text);
    if lines.len() < 12 {
        return Err(format_problem("lines", "Incomplete MPUlt log"));
    }
    let counters = header(strip(lines[0]))?;
    let lseq = match counters.0 {
        Some(lseq) if lseq <= MAX_ENTRIES as u64 => lseq,
        _ => return Err(problem(code::PERSISTENCE_SIZE, "header", "Invalid or oversized native history counters")),
    };
    let (Some(shuffle), Some(ptr)) = (counters.1.filter(|&s| s <= lseq), counters.2.filter(|&p| p <= lseq)) else {
        return Err(format_problem("header", "Invalid or oversized native history counters"));
    };

    let mut expected = vec![format!("Puzzle {NAME}")];
    expected.extend(DESCRIPTION.iter().map(|line| line.to_string()));
    expected.push("EndPuzzle".to_owned());
    expected.push(format!("2 {SLOTS}"));
    let actual: Vec<&str> = lines[1..1 + expected.len()].iter().map(|line| strip(line)).collect();
    let n = expected.len();
    if actual[0] != expected[0] || actual[n - 2..] != expected[n - 2..] {
        return Err(problem(
            code::IDENTITY_MISMATCH,
            "description",
            "Native log puzzle name or sticker dimensions differ from the retained full model",
        ));
    }
    for (line, wanted) in actual[1..n - 2].iter().zip(&expected[1..n - 2]) {
        let equal = if matches!(wanted.split(' ').next(), Some("Dim" | "NAxis" | "FixedMask")) {
            split(line).join(" ") == *wanted
        } else {
            normalize_description(line)? == normalize_description(wanted)?
        };
        if !equal {
            return Err(problem(
                code::IDENTITY_MISMATCH,
                "description",
                "Native log puzzle description differs from the retained full model",
            ));
        }
    }

    let mut index = 1 + n;
    let color_lines = SLOTS.div_ceil(128);
    let data = lines.get(index..index + color_lines).unwrap_or(&[]);
    let lengths_ok = data.len() == color_lines
        && data.iter().enumerate().all(|(i, line)| line.chars().count() == 256.min(2 * (SLOTS - 128 * i)));
    if !lengths_ok {
        return Err(format_problem("colors", "Native log color record length mismatch"));
    }
    let encoded: Vec<u8> = data.iter().flat_map(|line| line.bytes()).collect();
    if !encoded.iter().all(|b| b.is_ascii_digit() || b.is_ascii_uppercase()) {
        return Err(format_problem("colors", "Invalid native base36 colors"));
    }
    let digit = |b: u8| DIGITS.iter().position(|&d| d == b).unwrap_or(0) as u16;
    let colors: Vec<u16> = encoded.chunks(2).map(|pair| digit(pair[0]) + 36 * digit(pair[1])).collect();
    if colors.iter().any(|&c| c as usize >= FACES) {
        return Err(format_problem("colors", "Native color is outside the 600-cell palette"));
    }
    index += color_lines;

    if lines.len() < index + 3 {
        return Err(format_problem("timer", "Native timer/checksum delimiter missing"));
    }
    let field = |line: &str, prefix: &str| {
        line.strip_prefix(prefix).filter(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit())).map(counter)
    };
    let (Some(timer), Some(claimed)) = (field(lines[index], "#timer "), field(lines[index + 1], "#CRC ")) else {
        return Err(format_problem("timer", "Native timer/checksum delimiter invalid"));
    };
    if lines[index + 2] != "*" {
        return Err(format_problem("timer", "Native timer/checksum delimiter invalid"));
    }
    let (Some(timer), Some(claimed)) = (timer.filter(|&t| t <= MAX_TIMER), claimed) else {
        return Err(format_problem("timer", "Native timer or checksum is out of range"));
    };

    let joined = lines[index + 3..].join(" ");
    let mut tail = strip(&joined);
    if lseq > 0 {
        tail = tail
            .strip_suffix('.')
            .ok_or_else(|| format_problem("sequence", "Native move sequence terminator is missing"))?;
    } else if !tail.is_empty() {
        return Err(format_problem("sequence", "Empty native history has unexpected trailing data"));
    }

    // The sequence: every entry but the boundary marker, its packed value, and the lab word of each move.
    let mut entries: Vec<(&str, Option<&[i32]>)> = Vec::new();
    let mut sequence = Vec::new();
    let mut depth = 0i64;
    let mut boundary = None;
    let mut moves = 0usize;
    for token in split(tail) {
        match token {
            "m|" => {
                if boundary.is_some() {
                    return Err(format_problem("sequence", "Duplicate native shuffle boundary"));
                }
                boundary = Some(sequence.len() as u64);
                continue;
            }
            "m[" | "m]" => {
                depth += if token == "m[" { 1 } else { -1 };
                if depth < 0 {
                    return Err(format_problem("sequence", "Unbalanced native macro group"));
                }
                sequence.push(sequence_entry(token));
                entries.push((token, None));
                continue;
            }
            _ => {}
        }
        let Some(parts) = token_parts(token) else {
            return Err(format_problem("sequence", "Invalid native move token"));
        };
        if parts.iter().any(|v| !(-32768..=32767).contains(v)) {
            return Err(format_problem("sequence", "Native move fields exceed signed 16-bit range"));
        }
        let order = profile.orders.get(&(parts[0], parts[1])).copied();
        if !matches!(order, Some(2 | 3)) || ![0, 1, 4, 5].contains(&parts[3]) {
            return Err(format_problem("sequence", "Unsupported native axis/twist/layer mask"));
        }
        let Some(word) = profile.word(token) else {
            return Err(format_problem("sequence", "Native move is absent from the verified bridge"));
        };
        sequence.push(pack(parts) as i64);
        entries.push((token, Some(word)));
        moves += 1;
        if moves > MAX_MOVES {
            return Err(problem(code::PERSISTENCE_SIZE, "sequence", "MPUlt import is capped at 10,000 native moves"));
        }
    }
    let boundary_ok = if shuffle < lseq { boundary == Some(shuffle) } else { boundary.is_none() };
    if depth != 0 || sequence.len() as u64 != lseq || !boundary_ok {
        return Err(format_problem("sequence", "Native sequence count, macro group or shuffle boundary mismatch"));
    }
    if checksum(lseq, shuffle, ptr, timer, &sequence, &colors) != claimed {
        return Err(problem(code::DIGEST_MISMATCH, "checksum", "Native log CRC mismatch"));
    }

    // Replay every move, the redo tail included; select the state at the pointer.
    let none = ProtectedSet::none();
    let mut state = kernel.solved();
    let mut events = Vec::new();
    let mut pending = Vec::new();
    let mut selected = (ptr == 0).then(|| (0, state.clone()));
    for (index, (token, word)) in entries.iter().enumerate() {
        pending.push(token.to_string());
        if let Some(word) = word.filter(|word| !word.is_empty()) {
            let letters: Vec<i64> = word.iter().map(|&m| i64::from(m)).collect();
            let recipe = kernel.word(&letters).map_err(|r| problem(r.code, "sequence", r.message))?;
            let pre = state.digest().to_owned();
            let derivation = kernel
                .derive(&state, &recipe, &none)
                .map_err(|error| problem(code::CHECKPOINT_REPLAY, "sequence", error.to_string()))?;
            state = match derivation {
                Derivation::Derived(derived) => derived.state,
                Derivation::Refused(refused) => {
                    return Err(problem(refused.refusal.code, "sequence", refused.refusal.message))
                }
            };
            let assistance = if (index as u64) < shuffle { SCRAMBLE } else { MANUAL };
            events.push(MpultEvent {
                tokens: std::mem::take(&mut pending),
                word: word.to_vec(),
                pre,
                post: state.digest().to_owned(),
                assistance: assistance.to_owned(),
            });
        }
        if index as u64 + 1 == ptr {
            selected = Some((events.len(), state.clone()));
        }
    }
    let Some((selected_count, selected_state)) = selected else {
        return Err(format_problem("pointer", "Native active pointer was not reached"));
    };
    if profile.colors(&selected_state) != colors {
        return Err(problem(
            code::CHECKPOINT_REPLAY,
            "colors",
            "Native colors disagree with the complete legal move replay at Ptr",
        ));
    }
    Ok(MpultPlan {
        primitive_count: events.iter().map(|e| e.word.len() as u64).sum(),
        events,
        state: selected_state,
        selected_count,
        sequence_entries: lseq,
        pointer: ptr,
        shuffle,
        timer,
        trailing: pending,
    })
}

/// Imports an MPUlt log as a 1.0 log: one commit per transaction, then, when the pointer stands before the end, a
/// roll of the cursor back to it, so the redo tail stays as moves after the saved cursor.
pub fn import(kernel: &Kernel, profile: &NativeProfile, bytes: &[u8]) -> Result<Log, Problem> {
    let plan = check(kernel, profile, bytes)?;
    let mut log = Log::new(kernel.identity(), plan.state.digest());
    let mut parent: Option<String> = None;
    let mut ids = Vec::new();
    for (index, event) in plan.events.iter().enumerate() {
        let id = (index + 1).to_string();
        let letters: Vec<i64> = event.word.iter().map(|&m| i64::from(m)).collect();
        let recipe = kernel.word(&letters).map_err(|r| problem(r.code, "sequence", r.message))?;
        log.moves.push(MoveEntry {
            id: id.clone(),
            time: None,
            action: MoveAction::Commit(Commit {
                parent: parent.clone(),
                origin: "import".to_owned(),
                name: String::new(),
                note: NOTE.to_owned(),
                assistance: event.assistance.clone(),
                recipe: recipe_value(&recipe),
                primitive_count: event.word.len() as u64,
                stars: 0,
                pre: event.pre.clone(),
                post: event.post.clone(),
                interchange: Interchange::MPUlt { tokens: event.tokens.clone() },
            }),
        });
        ids.push(id.clone());
        parent = Some(id);
    }
    let head = plan.selected_count.checked_sub(1).map(|i| ids[i].clone());
    if plan.selected_count < plan.events.len() {
        let id = (plan.events.len() + 1).to_string();
        let cursor = Cursor { action: CursorAction::Roll, to: head.clone() };
        log.moves.push(MoveEntry { id, time: None, action: MoveAction::Cursor(cursor) });
    }
    log.saved = Saved { moves: head, layout: None, state: plan.state.digest().to_owned() };
    log.origin = Origin::MPUlt(MpultOrigin {
        source: c600_digest::sha256_hex(bytes),
        timer: plan.timer,
        shuffle: plan.shuffle,
        pointer: plan.pointer,
        trailing: plan.trailing,
        transactions: plan.events.len() as u64,
    });
    Ok(log)
}

// ---------------------------------------------------------------------------------------------------------------
// Export

/// The line an export writes: the start to the saved cursor, then on along the node added last after each node.
fn export_line(log: &Log) -> (Vec<&MoveEntry>, usize) {
    let mut line = log.line(log.saved.moves.as_deref());
    let selected = line.len();
    let mut at = log.saved.moves.as_deref();
    loop {
        let next = log.moves.iter().rev().find(|e| e.action.is_node() && e.action.parent() == Some(at));
        let Some(next) = next else { break };
        line.push(next);
        at = Some(next.id.as_str());
    }
    (line, selected)
}

/// The lab letters of a commit's witness.
fn letters(kernel: &Kernel, commit: &Commit) -> Result<Vec<i32>, Problem> {
    let recipe = kernel.recipe(&commit.recipe).map_err(|r| problem(r.code, "recipe", r.message))?;
    kernel.letters(&recipe).map_err(|r| problem(r.code, "recipe", r.message))
}

/// Exports a compiled log as an MPUlt v1 log. `timer` defaults to the imported log's timer, or 0.
pub fn export(
    kernel: &Kernel,
    profile: &NativeProfile,
    compiled: &Compiled,
    timer: Option<u64>,
) -> Result<Vec<u8>, Problem> {
    let (log, state) = crate::c600::bound(compiled)?;
    if profile.model_id != log.identity.model_id {
        return Err(problem(code::IDENTITY_MISMATCH, "profile", "The native profile belongs to another model"));
    }
    if log.start != Start::Solved {
        return Err(format_problem("payload.log.start", "an MPUlt log starts at the solved state"));
    }
    let (line, selected) = export_line(log);
    // The entries of each node on the line, whether they are its recorded MPUlt entries, and its assistance.
    let mut groups: Vec<(Vec<String>, bool, &str)> = Vec::new();
    for entry in &line {
        match &entry.action {
            MoveAction::Commit(commit) => {
                let letters = letters(kernel, commit)?;
                if let Interchange::MPUlt { tokens } = &commit.interchange {
                    let words: Option<Vec<i32>> = tokens
                        .iter()
                        .filter(|t| !matches!(t.as_str(), "m[" | "m]"))
                        .map(|t| profile.word(t).map(<[i32]>::to_vec))
                        .collect::<Option<Vec<_>>>()
                        .map(|words| words.concat());
                    if words.as_deref() == Some(letters.as_slice()) {
                        groups.push((tokens.clone(), true, &commit.assistance));
                        continue;
                    }
                }
                let tokens = letters.iter().map(|&m| profile.token(m)).collect::<Result<_, _>>()?;
                groups.push((tokens, false, &commit.assistance));
            }
            MoveAction::PartialUndo(undo) => {
                let Some(MoveEntry { action: MoveAction::Commit(commit), .. }) = log.node(&undo.entry) else {
                    return Err(format_problem("payload.log.history.moves", "a partial undo names no commit"));
                };
                let letters = letters(kernel, commit)?;
                let tokens =
                    letters[..undo.keep as usize].iter().map(|&m| profile.token(m)).collect::<Result<_, _>>()?;
                groups.push((tokens, false, &commit.assistance));
            }
            _ => groups.push((Vec::new(), false, "")),
        }
    }
    let origin = match &log.origin {
        Origin::MPUlt(origin) => Some(origin),
        _ => None,
    };
    let unchanged = origin
        .is_some_and(|o| groups.iter().all(|(_, recorded, _)| *recorded) && groups.len() as u64 == o.transactions);

    let mut tokens: Vec<String> = Vec::new();
    let mut ends = Vec::with_capacity(groups.len());
    let mut shuffle = 0usize;
    let mut still_scramble = true;
    for (group, _, assistance) in &groups {
        tokens.extend(group.iter().cloned());
        ends.push(tokens.len());
        if still_scramble && assistance.contains("scramble") {
            shuffle = tokens.len();
        } else {
            still_scramble = false;
        }
    }
    let mut ptr = if selected == 0 { 0 } else { ends[selected - 1] };
    if let (true, Some(origin)) = (unchanged, origin) {
        tokens.extend(origin.trailing.iter().cloned());
        shuffle = origin.shuffle as usize;
        // The recorded pointer, when it selects the same transaction as the saved cursor.
        let recorded = origin.pointer as usize;
        if ends.iter().filter(|&&end| end <= recorded).count() == selected && recorded <= tokens.len() {
            ptr = recorded;
        }
    } else {
        let mut depth = 0i64;
        let balanced = tokens.iter().all(|t| {
            depth += match t.as_str() {
                "m[" => 1,
                "m]" => -1,
                _ => 0,
            };
            depth >= 0
        }) && depth == 0;
        if !balanced {
            // Drop every macro marker, keeping the pointer and the boundary on the same moves.
            let keep = |end: usize, tokens: &[String]| {
                tokens[..end].iter().filter(|t| !matches!(t.as_str(), "m[" | "m]")).count()
            };
            ptr = keep(ptr, &tokens);
            shuffle = keep(shuffle, &tokens);
            tokens.retain(|t| t != "m[" && t != "m]");
        }
    }
    let move_count = tokens.iter().filter(|t| !matches!(t.as_str(), "m[" | "m]")).count();
    if move_count > MAX_MOVES || tokens.len() > MAX_ENTRIES {
        return Err(problem(
            code::PERSISTENCE_SIZE,
            "moves",
            "MPUlt log export is capped at 10,000 primitive moves; save a C600 proof log for longer histories",
        ));
    }
    let timer = timer.or(origin.map(|o| o.timer)).unwrap_or(0);
    if timer > MAX_TIMER {
        return Err(format_problem("timer", "Invalid native log timer"));
    }
    let out = serialize(&profile.colors(state), &tokens, shuffle, ptr, timer);
    if out.len() > MAX_LOG_BYTES {
        return Err(problem(code::PERSISTENCE_SIZE, "bytes", "MPUlt log exceeds 16 MiB"));
    }
    Ok(out)
}

/// Reads a native profile stored as gzip (as the fixtures keep it) or as its JSON bytes.
pub fn profile_bytes(stored: &[u8]) -> Result<Vec<u8>, Problem> {
    if stored.starts_with(&[0x1f, 0x8b]) {
        gunzip(stored, 64 * 1024 * 1024).map_err(|e| format_problem("profile", e.0))
    } else {
        Ok(stored.to_vec())
    }
}
