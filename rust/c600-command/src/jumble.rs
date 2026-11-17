//! Jumble on (A5; ONE.54, ONE.55, ONE.80, ONE.81; JI.25, JI.27; draft): the crossing that takes a session from
//! 600-cell-Full's retained labels into the jumbling engine's poses, through H-08's `jumble.on` ([`JUMBLE_ON_ROW`]).
//!
//! An entry needs a retained witness. With no exit in the session (`jumble.off` is not built yet), it is JI.25's solved
//! witness: the chronological literal retained path from the session's start to the head, which must start at solved.
//! The layer replays it from solved in the jumbling engine under the pinned menu ([`replay_from_solved`]) and enters only
//! when the replay's labels are the current labels. Labels alone are refused with `KN5:input.jumble_on_labels_only`:
//! no witness is generated from labels (ONE.54, JI.25). The journal keeps the crossing's record ([`entry_record`]), not
//! the poses; a reopen replays the witness again.

use c600_jumble::levels::{self, Base, Derivation, Derived, Partial};
use c600_jumble::state::EngineError;
use c600_kernel::json::{self, Value};
use c600_kernel::{IdentityBlock, ProtectionMode, Puzzle};

use crate::receipt::{code, Error};
use crate::store::{identity_from_json, identity_json};
use crate::twist::Twists;

/// H-08's row that turns Jumble on.
pub const JUMBLE_ON_ROW: &str = "jumble.on";

/// The entrance of an entry from the solved witness: JI.27's `solved_witness`.
const SOLVED_WITNESS: &str = "solved_witness";

/// The fields of an entry's crossing record.
const ENTRY_FIELDS: [&str; 4] = ["entrance", "identity", "letters", "word"];

/// `Digest<literal word>` of 600-cell-Full's primitive letters, the kernel's convention: the SHA-256 of the letters as
/// little-endian int32.
pub(crate) fn word_digest(letters: &[i32]) -> String {
    let bytes: Vec<u8> = letters.iter().flat_map(|letter| letter.to_le_bytes()).collect();
    c600_digest::sha256_hex(&bytes)
}

/// An entry's crossing record (draft): `{"entrance": "solved_witness", "identity", "letters", "word"}`, with the
/// jumbling identity block the entry was made under, which names the pinned menu, and the solved witness's letter count
/// and `Digest<literal word>`. The witness's letters are the move tree's, which the journal keeps already.
pub(crate) fn entry_record(identity: &IdentityBlock, letters: &[i32]) -> Value {
    json::object([
        ("entrance", Value::String(SOLVED_WITNESS.to_owned())),
        ("identity", identity_json(identity)),
        ("letters", Value::Int(letters.len() as i64)),
        ("word", Value::String(word_digest(letters))),
    ])
}

/// Whether `value` reads as a crossing record: [`entry_record`]'s four fields and no other, a jumbling identity block, a
/// natural letter count and a SHA-256 in lower-case hex.
pub(crate) fn crossing_reads(value: &Value) -> bool {
    let Some(fields) = value.as_object() else {
        return false;
    };
    let hex = |text: &str| text.len() == 64 && text.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    fields.len() == ENTRY_FIELDS.len()
        && ENTRY_FIELDS.iter().all(|field| fields.contains_key(*field))
        && value.get("entrance").and_then(Value::as_str) == Some(SOLVED_WITNESS)
        && value.get("identity").and_then(identity_from_json).is_some_and(|block| block.puzzle == Puzzle::Jumbling)
        && value.get("letters").and_then(Value::as_i64).is_some_and(|n| n >= 0)
        && value.get("word").and_then(Value::as_str).is_some_and(hex)
}

/// A solved witness replayed in the jumbling engine: the derived value at its end, that value's pose digest, the
/// identity block of the pinned menu and the entry's crossing record.
pub(crate) struct Entry {
    pub(crate) derived: Derived,
    pub(crate) digest: String,
    pub(crate) identity: IdentityBlock,
    pub(crate) record: Value,
}

/// Replays `letters`, 600-cell-Full's signed primitive letters, from solved in the jumbling engine under the pinned menu
/// (`levels::derive` with no protected piece), and checks that the replay's labels have the digest `labels`, the
/// current labels' (A5, JI.25). A word that does not replay, or whose labels are not total or are not the current
/// ones, is not a valid witness: `KN5:input.jumble_on_labels_only`. An engine fault is
/// `KN5:consistency.state_hash_mismatch`.
pub(crate) fn replay_from_solved(twists: &Twists, letters: &[i32], labels: &str) -> Result<Entry, Error> {
    let ctx = &*twists.context;
    let engine = |error: EngineError| Error::new(code::STATE_HASH_MISMATCH, format!("the jumbling engine: {error}"));
    let labels_only = |detail: String| Error::new(code::JUMBLE_ON_LABELS_ONLY, detail);
    let word = letters.iter().map(|&letter| levels::primitive(ctx, i64::from(letter)));
    let word = word.collect::<Result<Vec<_>, _>>().map_err(engine)?;
    let identity = ctx.identity(Some(&twists.menu));
    let solved = levels::Snapshot::solved(ctx, Some(twists.menu.clone()));
    let derivation = levels::derive(ctx, Base::Snapshot(&solved), &identity, &word, ProtectionMode::Net, &[]);
    let derived = match derivation.map_err(engine)? {
        Err(refusal) => return Err(refusal.into()),
        Ok(Derivation::Refused(refused)) => {
            let message = refused.refusal.message;
            return Err(labels_only(format!("the retained word from solved does not replay as a witness: {message}")));
        }
        Ok(Derivation::Derived(derived)) => derived,
    };
    match levels::labels(ctx, derived.state()).map_err(engine)? {
        Partial::Total(total) if total.digest() == labels => {}
        Partial::Total(_) => return Err(labels_only("the witness's replay does not reach the current labels".into())),
        _ => return Err(labels_only("the witness's replay has no total labels".into())),
    }
    let digest = derived.digest(ctx);
    let record = entry_record(&identity, letters);
    Ok(Entry { derived, digest, identity, record })
}
