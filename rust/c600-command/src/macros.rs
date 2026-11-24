//! The macro base (EV4.9; JI.18 to JI.20; DT-J1 Outcome E1 and E3; DT-M1 K6; LP4-8; draft): typed macros of both
//! puzzles in one base, written by H-08's `macro.save` ([`MACRO_SAVE_ROW`]).
//!
//! - A macro is the record `macro/<name>` ([`macro_record`]). One base serves both puzzles (EV4.8, DT-J1 E1): a word of
//!   600-cell-Full's generators and a word of the pinned twist menu's letters are saved, listed and reopened the same
//!   way, in either Jumble mode, each with its own identity block.
//! - `macro.save {word, name}` with H-08's `confirm` gate: `word` is a `WordRef` the layer reads inline, `{"recipe":
//!   [...]}` for a Full word in 0.4's recipe form, whose words and stars are its named parts, or `{"menu": [[grip,
//!   item], ...]}` for a word of the pinned menu's letters (JI.19: a grip below [`POLES`] and an item of the menu below
//!   60). Saving neither selects, inserts nor executes the word (H-08's note).
//! - The record ([`Macro::to_payload`]) holds the name; the alphabet, `full` or `menu`; KN1's identity block (the
//!   loaded model's for a Full word, the jumbling block of the model and the pinned menu, with its order, for a menu
//!   word); the literal letters in chronological order, flat (JI.18: a Full word's signed generators, a menu word's
//!   `60*grip + item` codes); the exact expanded length, known before any check, preview or run; the literal digest
//!   (a Full word's is the journal's literal digest of its expansion, the SHA-256 of the letters as little-endian
//!   32-bit integers; a menu word's is JI.19's `ia16-expanded-le/1`, the SHA-256 of the codes as little-endian 16-bit
//!   values); the start digest, the head's state digest when it was saved (labels with Jumble off, poses with Jumble
//!   on); the source as the solver wrote it, which names its parts but never supplies letters; and the key contracts
//!   its word and trace keys are under ([`KeyContracts`]). The provenance block is the row's own
//!   ([`MACRO_PROVENANCE`]): the solver's command wrote it. A stored word, `{"record": r}`, is the exception (BND.32;
//!   draft): its macro keeps the record's block, and its source pins the record's content digest
//!   ([`stored_source`]), so a reopen admits the write only against that record as the journal holds it.
//! - Key computation is not here: `c600-command` has no path to the interpreter (ONE.48), and the keys are its
//!   primitives (`word_key`, `trace_key`). The record pins the contracts they are computed under: LG2.18's
//!   `word-key/1`, and LB1.10's `trace-key/1` under `full-disjoint-support/1` for a Full word or `jumble-locality/1`
//!   for a menu word. [`check_keys`] refuses a comparison of two macros whose key kind, version, alphabet, identity or
//!   independence contract differ with `LB1:input.key_contract_mismatch`, before any key is computed (EV4.9).
//! - Names under `macro/` are the base's: only `macro.save` writes one. `workspace.set` and `worksheet.save` refuse
//!   them with `KN5:input.argument_form`, a program job's output that writes one fails with
//!   `KN2:input.named_object_invalid`, and a reopen admits a write of one only under `macro.save` from the shell with
//!   its stored confirmation and a record that reads (B7).

use c600_jumble::locality::JUMBLE_LOCALITY;
use c600_jumble::model::POLES;
use c600_kernel::json::{self, Value};
use c600_kernel::{IdentityBlock, Puzzle};

use crate::receipt::{code, Error};
use crate::store::{identity_from_json, identity_json};

/// H-08's row that saves a word as a macro.
pub const MACRO_SAVE_ROW: &str = "macro.save";
/// The prefix of a macro's record name.
pub const MACRO_PREFIX: &str = "macro/";
/// The provenance block of a macro's record when the word is inline: the solver's own `macro.save` wrote it. A macro of
/// a stored word keeps that word's block instead (BND.32).
pub const MACRO_PROVENANCE: &str = "macro.save";
/// LG2.18's word equality key and its version.
pub const WORD_KEY: &str = "word-key/1";
/// LB1.10's trace equality key and its version.
pub const TRACE_KEY: &str = "trace-key/1";
/// LB1.10's independence contract of 600-cell-Full's generators.
pub const FULL_DISJOINT_SUPPORT: &str = "full-disjoint-support/1";
/// JI.19's item bound of a menu letter's `ia16` code, `60*grip + item`.
pub const MENU_ITEMS: usize = 60;

/// The record name of macro `name`.
pub fn macro_record(name: &str) -> String {
    format!("{MACRO_PREFIX}{name}")
}

/// The source of a macro of the stored word in record `record`, whose value has content digest `digest` (BND.32;
/// draft): `{"digest": d, "record": r}`.
pub fn stored_source(record: &str, digest: &str) -> Value {
    json::object([("digest", Value::String(digest.to_owned())), ("record", Value::String(record.to_owned()))])
}

/// A word's alphabet (EV4.9: each word is typed by its alphabet).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alphabet {
    /// 600-cell-Full's signed generators.
    Full,
    /// The pinned twist menu's letters `[grip, item]`.
    Menu,
}

impl Alphabet {
    pub fn name(self) -> &'static str {
        match self {
            Alphabet::Full => "full",
            Alphabet::Menu => "menu",
        }
    }

    fn from_name(name: &str) -> Option<Alphabet> {
        [Alphabet::Full, Alphabet::Menu].into_iter().find(|alphabet| alphabet.name() == name)
    }

    /// The key contracts of a word of this alphabet.
    pub fn contracts(self) -> KeyContracts {
        let relation = match self {
            Alphabet::Full => FULL_DISJOINT_SUPPORT,
            Alphabet::Menu => JUMBLE_LOCALITY,
        };
        KeyContracts { word: WORD_KEY.to_owned(), trace: TRACE_KEY.to_owned(), relation: relation.to_owned() }
    }
}

/// The contracts a macro's word and trace keys are computed under: the key kinds with their versions and the trace
/// key's independence contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyContracts {
    pub word: String,
    pub trace: String,
    pub relation: String,
}

impl KeyContracts {
    fn to_json(&self) -> Value {
        let text = |s: &str| Value::String(s.to_owned());
        json::object([("relation", text(&self.relation)), ("trace", text(&self.trace)), ("word", text(&self.word))])
    }

    fn from_json(value: &Value) -> Option<KeyContracts> {
        let fields = value.as_object().filter(|fields| fields.len() == 3)?;
        let text = |key: &str| fields.get(key)?.as_str().map(str::to_owned);
        Some(KeyContracts { word: text("word")?, trace: text("trace")?, relation: text("relation")? })
    }
}

/// A saved macro, as its record holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Macro {
    pub name: String,
    pub alphabet: Alphabet,
    /// KN1's identity block of the word's alphabet.
    pub identity: IdentityBlock,
    /// The literal letters in chronological order, flat: signed generators, or `60*grip + item` codes.
    pub letters: Vec<i64>,
    /// The exact expanded length: the number of literal letters.
    pub length: u64,
    /// The literal digest ([`literal_digest`]).
    pub literal: String,
    /// The head's state digest when the macro was saved.
    pub start: String,
    /// The `WordRef` as the solver wrote it.
    pub source: Value,
    pub keys: KeyContracts,
}

impl Macro {
    /// The macro's record payload.
    pub fn to_payload(&self) -> Value {
        json::object([
            ("alphabet", Value::String(self.alphabet.name().to_owned())),
            ("identity", identity_json(&self.identity)),
            ("keys", self.keys.to_json()),
            ("length", Value::Int(self.length as i64)),
            ("letters", Value::Array(self.letters.iter().map(|&letter| Value::Int(letter)).collect())),
            ("literal", Value::String(self.literal.clone())),
            ("name", Value::String(self.name.clone())),
            ("source", self.source.clone()),
            ("start", Value::String(self.start.clone())),
        ])
    }

    /// The record and content digest a macro of a stored word names ([`stored_source`]); `None` for an inline word.
    pub fn stored(&self) -> Option<(&str, &str)> {
        let fields = self.source.as_object().filter(|fields| fields.len() == 2)?;
        Some((fields.get("record")?.as_str()?, fields.get("digest")?.as_str()?))
    }

    /// Reads a record's payload ([`Macro::to_payload`]). `None` unless it has exactly those fields, an alphabet with
    /// its own identity's puzzle and key contracts, letters of the alphabet, and the length and literal digest of
    /// those letters.
    pub fn from_payload(payload: &Value) -> Option<Macro> {
        let fields = payload.as_object().filter(|fields| fields.len() == 9)?;
        let text = |key: &str| fields.get(key)?.as_str().map(str::to_owned);
        let alphabet = Alphabet::from_name(fields.get("alphabet")?.as_str()?)?;
        let identity = identity_from_json(fields.get("identity")?)?;
        let puzzle = match alphabet {
            Alphabet::Full => Puzzle::Full,
            Alphabet::Menu => Puzzle::Jumbling,
        };
        let keys = KeyContracts::from_json(fields.get("keys")?)?;
        let letters: Vec<i64> = fields.get("letters")?.as_array()?.iter().map(Value::as_i64).collect::<Option<_>>()?;
        let length = u64::try_from(fields.get("length")?.as_i64()?).ok()?;
        let read = Macro {
            name: text("name")?,
            alphabet,
            identity,
            literal: text("literal")?,
            start: text("start")?,
            source: fields.get("source")?.clone(),
            length,
            letters,
            keys,
        };
        let letters_fit = match alphabet {
            Alphabet::Full => read.letters.iter().all(|&letter| letter != 0 && i32::try_from(letter).is_ok()),
            Alphabet::Menu => read.letters.iter().all(|&code| (0..(POLES * MENU_ITEMS) as i64).contains(&code)),
        };
        (read.identity.puzzle == puzzle
            && read.keys == alphabet.contracts()
            && letters_fit
            && read.length == read.letters.len() as u64
            && read.literal == literal_digest(alphabet, &read.letters))
        .then_some(read)
    }
}

/// The literal digest of `letters` (lowercase hex): for a Full word the SHA-256 of the letters as little-endian 32-bit
/// integers, the journal's literal digest of an expansion; for a menu word JI.19's `ia16-expanded-le/1`, the SHA-256 of
/// the codes as little-endian 16-bit values, with no header, padding or chunk boundary.
pub fn literal_digest(alphabet: Alphabet, letters: &[i64]) -> String {
    let mut bytes = Vec::with_capacity(4 * letters.len());
    for &letter in letters {
        match alphabet {
            Alphabet::Full => bytes.extend_from_slice(&(letter as i32).to_le_bytes()),
            Alphabet::Menu => bytes.extend_from_slice(&(letter as u16).to_le_bytes()),
        }
    }
    c600_digest::sha256_hex(&bytes)
}

/// A menu word's letters `[[grip, item], ...]` as JI.19's codes `60*grip + item`, under a menu of `items` items. A
/// letter of another form, a grip of [`POLES`] or more, or an item outside the menu or of [`MENU_ITEMS`] or more is
/// refused with `KN5:input.argument_form`.
pub fn menu_codes(letters: &Value, items: usize) -> Result<Vec<i64>, Error> {
    let bound = items.min(MENU_ITEMS);
    let refused = || {
        let detail = format!("a menu word is `[[grip, item], ...]`, a grip below {POLES} and an item below {bound}");
        Error::new(code::ARGUMENT_FORM, detail)
    };
    let letters = letters.as_array().ok_or_else(refused)?;
    let code = |letter: &Value| -> Option<i64> {
        let pair = letter.as_array().filter(|pair| pair.len() == 2)?;
        let grip = usize::try_from(pair[0].as_i64()?).ok().filter(|&grip| grip < POLES)?;
        let item = usize::try_from(pair[1].as_i64()?).ok().filter(|&item| item < bound)?;
        Some((grip * MENU_ITEMS + item) as i64)
    };
    letters.iter().map(code).collect::<Option<Vec<i64>>>().ok_or_else(refused)
}

/// Whether the keys of macros `a` and `b` can be compared (EV4.9, LG2.23): the same alphabet, identity block (its menu
/// and order included) and key contracts. Otherwise refused with `LB1:input.key_contract_mismatch`, naming the first
/// field that differs, so no equality or ordering result is returned. Equal contracts say nothing about whether the
/// words are equal: that is the keys' comparison, which the interpreter computes.
pub fn check_keys(a: &Macro, b: &Macro) -> Result<(), Error> {
    let field = if a.alphabet != b.alphabet {
        "alphabet"
    } else if a.identity != b.identity {
        "identity"
    } else if a.keys.word != b.keys.word {
        "word key"
    } else if a.keys.trace != b.keys.trace {
        "trace key"
    } else if a.keys.relation != b.keys.relation {
        "independence contract"
    } else {
        return Ok(());
    };
    let detail = format!("`{}` and `{}` differ in their {field}", a.name, b.name);
    Err(Error::new(code::KEY_CONTRACT_MISMATCH, detail))
}
