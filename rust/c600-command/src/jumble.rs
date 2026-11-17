//! Jumble on and Jumble off (A5; ONE.54, ONE.55, ONE.80, ONE.81; JI.25, JI.27; draft): the crossings between
//! 600-cell-Full's retained labels and the jumbling engine's poses, through H-08's `jumble.on` ([`JUMBLE_ON_ROW`]) and
//! `jumble.off` ([`JUMBLE_OFF_ROW`]).
//!
//! An entry needs a retained witness. In this edition it is JI.25's solved witness, since an entry from an exit root
//! is not built yet: the chronological literal retained path from the session's start to the head, which must start
//! at solved.
//! The layer replays it from solved in the jumbling engine under the pinned menu ([`replay_from_solved`]) and enters only
//! when the replay's labels are the current labels. Labels alone are refused with `KN5:input.jumble_on_labels_only`:
//! no witness is generated from labels (ONE.54, JI.25). The journal keeps the crossing's record ([`entry_record`]), not
//! the poses; a reopen replays the witness again.
//!
//! With Jumble on, `jumble.twist.menu` ([`JUMBLE_TWIST_ROW`]) moves the poses (JI.35, FP.31; draft). The layer derives
//! each pose move from the head's value, and replays the jumbling journal flat from solved ([`replay_poses`]) when the
//! chain of derivations grows long or an undo's exact inverse is not an item of the menu.
//!
//! An exit ([`exit`]) is allowed exactly at A5's label checkpoint (ONE.54): every piece on the lattice, every centre in
//! its own chamber, ν = 0 and the head's poses witnessed from solved by the jumbling journal. The exit's labels become
//! the retained state; the journal keeps the exit's record ([`exit_record`]), not the poses, and a reopen replays the
//! journal to the exit and checks the checkpoint again.

use c600_jumble::levels::{self, Base, Derivation, Derived, Partial, NU_LEN};
use c600_jumble::nu;
use c600_jumble::state::EngineError;
use c600_jumble::twist::Twist;
use c600_kernel::json::{self, Value};
use c600_kernel::{IdentityBlock, Kernel, ProtectionMode, Puzzle, Snapshot};

use crate::receipt::{code, Error};
use crate::store::{identity_from_json, identity_json};
use crate::twist::Twists;

/// H-08's row that turns Jumble on.
pub const JUMBLE_ON_ROW: &str = "jumble.on";

/// H-08's row that turns Jumble off at A5's label checkpoint.
pub const JUMBLE_OFF_ROW: &str = "jumble.off";

/// H-08's row that applies an item of the pinned twist menu (JI.35): a retained commit with Jumble off, a pose move
/// with Jumble on.
pub const JUMBLE_TWIST_ROW: &str = "jumble.twist.menu";

/// The entrance of an entry from the solved witness: JI.27's `solved_witness`.
const SOLVED_WITNESS: &str = "solved_witness";

/// The fields of an entry's crossing record.
const ENTRY_FIELDS: [&str; 4] = ["entrance", "identity", "letters", "word"];

/// The fields of an exit's record.
const EXIT_FIELDS: [&str; 6] = ["certificate", "episode", "identity", "journal", "labels", "poses"];

/// The fields of an exit record's certificate.
const CERTIFICATE_FIELDS: [&str; 6] =
    ["c5_convention", "contract_revision", "frame_agrees", "nu", "nu_source", "program"];

/// How a label checkpoint shows ν = 0: the identity labels, a retained witness, or the ν row's value.
const NU_SOURCES: [&str; 3] = ["identity_labels", "retained_witness", "supplied"];

/// Whether `text` is a SHA-256 in lower-case hex.
fn hex(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Whether `value` is an object with exactly the fields `names`.
fn fields_are(value: &Value, names: &[&str]) -> bool {
    value.as_object().is_some_and(|fields| fields.len() == names.len() && names.iter().all(|n| fields.contains_key(*n)))
}

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
    fields_are(value, &ENTRY_FIELDS)
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

/// Replays `word`, the jumbling engine's journal from solved to a pose move (draft), flat from solved under the pinned
/// menu with no protected piece, and returns the value at its end with that value's pose digest. The word is the
/// layer's own journal of moves it admitted already, so a refusal or an engine fault is
/// `KN5:consistency.state_hash_mismatch`.
pub(crate) fn replay_poses(twists: &Twists, word: &[Twist]) -> Result<(Derived, String), Error> {
    let ctx = &*twists.context;
    let mismatch = |detail: String| Error::new(code::STATE_HASH_MISMATCH, detail);
    let identity = ctx.identity(Some(&twists.menu));
    let solved = levels::Snapshot::solved(ctx, Some(twists.menu.clone()));
    let derivation = levels::derive(ctx, Base::Snapshot(&solved), &identity, word, ProtectionMode::Net, &[]);
    match derivation.map_err(|error| mismatch(format!("the jumbling engine: {error}")))? {
        Ok(Derivation::Derived(derived)) => {
            let digest = derived.digest(ctx);
            Ok((derived, digest))
        }
        Ok(Derivation::Refused(refused)) => {
            Err(mismatch(format!("the journal does not replay from solved: {}", refused.refusal.message)))
        }
        Err(refusal) => Err(mismatch(format!("the journal does not replay from solved: {}", refusal.message))),
    }
}

/// An exit's record (draft): the episode it ends, named by the entry's event; the jumbling identity block; the
/// jumbling journal from solved to the exit's poses, its letter count and `Digest<literal word>` (JI.25's source
/// derivation); the pose digest; `Digest<labels>` of the exit's labels; and A5's certificate: ν = 0 with its source and
/// the C5 convention, the frame agreement, the certificate program's SHA-256 and the contract revision. The poses
/// themselves are the journal's replay, which a reopen repeats.
pub(crate) fn exit_record(
    identity: &IdentityBlock,
    episode: &str,
    journal: &[Twist],
    poses: &str,
    labels: &str,
    nu_source: &str,
    contract: &str,
) -> Value {
    let text = |text: &str| Value::String(text.to_owned());
    let certificate = json::object([
        ("c5_convention", text(nu::C5_CONVENTION)),
        ("contract_revision", text(contract)),
        ("frame_agrees", Value::Bool(true)),
        ("nu", Value::Array(vec![Value::Int(0); NU_LEN])),
        ("nu_source", text(nu_source)),
        ("program", text(nu::CERTIFICATE_PROGRAM_SHA256)),
    ]);
    let derivation =
        json::object([("letters", Value::Int(journal.len() as i64)), ("word", text(&levels::word_digest(journal)))]);
    json::object([
        ("certificate", certificate),
        ("episode", text(episode)),
        ("identity", identity_json(identity)),
        ("journal", derivation),
        ("labels", text(labels)),
        ("poses", text(poses)),
    ])
}

/// Whether `value` reads as an exit's record: [`exit_record`]'s fields and no other, the episode an event name
/// `e<order>`, a jumbling identity block, a journal of a natural letter count and a SHA-256, two SHA-256 digests, and a
/// certificate whose ν is 45 zeros from a known source, whose frame agrees, and whose convention, program and contract
/// revision are named. The values are checked where the exit is replayed.
pub(crate) fn exit_reads(value: &Value) -> bool {
    fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
        value.get(key).and_then(Value::as_str)
    }
    fn event(name: &str) -> bool {
        name.strip_prefix('e').is_some_and(|order| !order.is_empty() && order.bytes().all(|b| b.is_ascii_digit()))
    }
    let (Some(certificate), Some(journal)) = (value.get("certificate"), value.get("journal")) else {
        return false;
    };
    let named = |key: &str| text(certificate, key).is_some_and(|name| !name.is_empty());
    let nu = certificate.get("nu").and_then(Value::as_array);
    fields_are(value, &EXIT_FIELDS)
        && fields_are(certificate, &CERTIFICATE_FIELDS)
        && fields_are(journal, &["letters", "word"])
        && text(value, "episode").is_some_and(event)
        && value.get("identity").and_then(identity_from_json).is_some_and(|block| block.puzzle == Puzzle::Jumbling)
        && journal.get("letters").and_then(Value::as_i64).is_some_and(|n| n >= 0)
        && text(journal, "word").is_some_and(hex)
        && text(value, "labels").is_some_and(hex)
        && text(value, "poses").is_some_and(hex)
        && nu.is_some_and(|nu| nu.len() == NU_LEN && nu.iter().all(|x| x.as_i64() == Some(0)))
        && text(certificate, "nu_source").is_some_and(|source| NU_SOURCES.contains(&source))
        && certificate.get("frame_agrees") == Some(&Value::Bool(true))
        && text(certificate, "program").is_some_and(hex)
        && named("c5_convention")
        && named("contract_revision")
}

/// An exit at A5's label checkpoint: the retained state at the exit's labels and the exit's record.
pub(crate) struct Exit {
    pub(crate) state: Snapshot,
    pub(crate) record: Value,
}

/// A5's exit from `value`, the head's value with Jumble on, ending the episode that the entry `episode` began (ONE.54,
/// JI.25, JI.27; draft). `journal` is the jumbling journal from solved to the head, along which the head's value is
/// derived, so its reachability claim is witnessed at its own poses.
///
/// A5's label checkpoint (`levels::label_checkpoint`) decides. ν comes from the identity labels or a retained witness
/// where they show it, and otherwise from the ν row (`c600_jumble::nu::nu`). A piece off the lattice is
/// `KN5:input.jumble_off_pieces_off_lattice`, a nonzero ν `KN5:input.jumble_off_nu_nonzero`, and a displaced centre,
/// two pieces in one chamber or poses not shown reachable `KN5:input.jumble_off_not_shown_reachable`; each refusal
/// names the fact only and selects no correction. At a checkpoint the labels become 600-cell-Full's state, which must
/// have the checkpoint's `Digest<labels>`. An engine fault is `KN5:consistency.state_hash_mismatch`; the ν row's own
/// refusal keeps its code.
pub(crate) fn exit(
    twists: &Twists,
    kernel: &Kernel,
    value: &Derived,
    episode: &str,
    journal: &[Twist],
) -> Result<Exit, Error> {
    let ctx = &*twists.context;
    let mismatch = |detail: &str| Error::new(code::STATE_HASH_MISMATCH, detail);
    let engine = |error: EngineError| Error::new(code::STATE_HASH_MISMATCH, format!("the jumbling engine: {error}"));
    let checkpoint = |shown: Option<&[i64]>| levels::label_checkpoint(ctx, value.state(), value.reachability(), shown);
    let mut verdict = checkpoint(None).map_err(engine)?;
    if matches!(verdict, levels::Verdict::Unknown(_)) {
        // Neither the identity labels nor a retained witness show ν here: the ν row computes it.
        if let Partial::Total(row) = nu::nu(ctx, Base::Derived(value)).map_err(engine)?? {
            verdict = checkpoint(Some(&row.values()[..])).map_err(engine)?;
        }
    }
    let evidence = match verdict {
        levels::Verdict::Yes(evidence) => evidence,
        levels::Verdict::No(fact) => return Err(refusal(&fact)),
        levels::Verdict::Unknown(reason) => {
            let detail = format!("Jumble off: the poses are not shown reachable ({reason})");
            return Err(Error::new(code::JUMBLE_OFF_NOT_SHOWN_REACHABLE, detail));
        }
    };
    let labels = match levels::labels(ctx, value.state()).map_err(engine)? {
        Partial::Total(labels) => labels,
        _ => return Err(mismatch("a label checkpoint's labels are not total")),
    };
    let digest = labels.digest();
    // `levels::labels` lists a total value's slots in order, so another order is an engine fault.
    let in_order = labels.slots.iter().enumerate().all(|(slot, &s)| s as usize == slot);
    if !in_order || evidence.get("labels_digest").and_then(Value::as_str) != Some(digest.as_str()) {
        return Err(mismatch("the checkpoint's labels are not the labels of its poses"));
    }
    let state = kernel.state_from_labels(labels.labels).filter(|state| state.digest() == digest);
    let state = state.ok_or_else(|| mismatch("the checkpoint's labels are not a state of 600-cell-Full"))?;
    let text = |key: &str| evidence.get(key).and_then(Value::as_str).unwrap_or_default().to_owned();
    let (poses, source, contract) = (text("pose_digest"), text("nu_source"), text("contract_revision"));
    let record = exit_record(value.identity(), episode, journal, &poses, &digest, &source, &contract);
    if !exit_reads(&record) {
        return Err(mismatch("the checkpoint's evidence does not form an exit record"));
    }
    Ok(Exit { state, record })
}

/// The refusal for a label checkpoint's no (A5, JI.27): the fact only, with no correction.
fn refusal(fact: &Value) -> Error {
    let count = |key: &str| fact.get(key).and_then(Value::as_i64).unwrap_or_default();
    let not_shown = |detail: String| Error::new(code::JUMBLE_OFF_NOT_SHOWN_REACHABLE, detail);
    match fact.get("fact").and_then(Value::as_str) {
        Some("off_lattice") => Error::new(
            code::JUMBLE_OFF_PIECES_OFF_LATTICE,
            format!("Jumble off: pieces off the lattice: {}", count("off")),
        ),
        Some("nu_nonzero") => Error::new(code::JUMBLE_OFF_NU_NONZERO, "Jumble off: ν is not zero"),
        Some("centre_displaced") => {
            let centres = fact.get("centres").and_then(Value::as_array).map_or(0, <[Value]>::len);
            not_shown(format!("Jumble off: centre pieces outside their own chambers: {centres}"))
        }
        Some("pieces_collide") => {
            not_shown(format!("Jumble off: two pieces share a chamber (collisions: {})", count("collisions")))
        }
        other => not_shown(format!("Jumble off: the label checkpoint answers no ({})", other.unwrap_or("no fact"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(puzzle: Puzzle) -> IdentityBlock {
        let some = |text: &str| Some(text.to_owned());
        let (menu_id, menu_order, contract_rev) = match puzzle {
            Puzzle::Jumbling => (some("menu"), some("order"), some("contract")),
            Puzzle::Full => (None, None, None),
        };
        IdentityBlock { puzzle, model_id: "model".into(), menu_id, menu_order, contract_rev }
    }

    /// `value` with the field at `path` set to `new`, or removed when `new` is `None`.
    fn with(value: &Value, path: &[&str], new: Option<Value>) -> Value {
        let Value::Object(fields) = value else { panic!("an object") };
        let mut fields = fields.clone();
        match (path, new) {
            ([key], Some(new)) => {
                fields.insert((*key).to_owned(), new);
            }
            ([key], None) => {
                fields.remove(*key);
            }
            ([key, rest @ ..], new) => {
                let inner = with(&fields[*key], rest, new);
                fields.insert((*key).to_owned(), inner);
            }
            ([], _) => panic!("a path"),
        }
        Value::Object(fields)
    }

    #[test]
    fn a_checkpoints_no_names_its_fact_and_code() {
        let fact = |name: &str, key: &str, value: Value| {
            json::object([("fact", Value::String(name.to_owned())), (key, value)])
        };
        let ints = |values: &[i64]| Value::Array(values.iter().map(|&x| Value::Int(x)).collect());
        let not_shown = code::JUMBLE_OFF_NOT_SHOWN_REACHABLE;
        let cases = [
            (fact("off_lattice", "off", Value::Int(3)), code::JUMBLE_OFF_PIECES_OFF_LATTICE, "off the lattice: 3"),
            (fact("nu_nonzero", "nu", ints(&[0, 1])), code::JUMBLE_OFF_NU_NONZERO, "ν is not zero"),
            (fact("centre_displaced", "centres", ints(&[7])), not_shown, "own chambers: 1"),
            (fact("pieces_collide", "collisions", Value::Int(2)), not_shown, "collisions: 2"),
            (fact("another_fact", "x", Value::Null), not_shown, "another_fact"),
        ];
        for (fact, expected, detail) in cases {
            let error = refusal(&fact);
            assert_eq!(error.code, expected, "{}", fact.canonical());
            assert!(error.detail.contains(detail), "{}: {}", fact.canonical(), error.detail);
        }
    }

    #[test]
    fn an_exit_record_reads_and_no_other_form_does() {
        let digest = "0123456789abcdef".repeat(4);
        let record = exit_record(&block(Puzzle::Jumbling), "e7", &[], &digest, &digest, "supplied", "contract");
        assert!(exit_reads(&record), "{}", record.canonical());
        assert!(!crossing_reads(&record));
        let entry = entry_record(&block(Puzzle::Jumbling), &[1, -2]);
        assert!(crossing_reads(&entry) && !exit_reads(&entry));
        let text = |text: &str| Some(Value::String(text.to_owned()));
        let zeros = |n: usize| vec![Value::Int(0); n];
        let cases: [(&[&str], Option<Value>); 16] = [
            (&["episode"], text("7")),
            (&["episode"], text("e")),
            (&["episode"], text("e7x")),
            (&["identity"], Some(identity_json(&block(Puzzle::Full)))),
            (&["labels"], text(&digest.to_uppercase())),
            (&["poses"], text("abc")),
            (&["journal", "letters"], Some(Value::Int(-1))),
            (&["journal", "word"], None),
            (&["certificate", "nu"], Some(Value::Array(zeros(NU_LEN - 1)))),
            (&["certificate", "nu"], Some(Value::Array([vec![Value::Int(1)], zeros(NU_LEN - 1)].concat()))),
            (&["certificate", "nu_source"], text("guessed")),
            (&["certificate", "frame_agrees"], Some(Value::Bool(false))),
            (&["certificate", "program"], text("not a digest")),
            (&["certificate", "contract_revision"], text("")),
            (&["certificate", "extra"], Some(Value::Null)),
            (&["extra"], Some(Value::Null)),
        ];
        for (path, new) in cases {
            let changed = with(&record, path, new);
            assert!(!exit_reads(&changed), "{path:?}: {}", changed.canonical());
        }
        assert!(!exit_reads(&with(&record, &["certificate"], None)));
    }
}
