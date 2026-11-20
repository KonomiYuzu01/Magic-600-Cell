//! CT-LB2-02 (whole: a word with identity net effect and moving intermediate prefixes passes Net and fails Strict for
//! the relevant captured lock, and an incomplete prefix check never returns safe) on 600-cell-Full. The entries are
//! library code, `c600-library`'s module `protection` (LB2's row "Net and Strict protection"): three capture a block,
//! identity or position lock from the head through KN2.4's `positions` and `labels` rows, and three judge a captured
//! lock and a recipe through KN2.4's `protection` row and report the verdict with its coverage, input and level
//! (LB2.3). They run on `c600-run` through `Boundary` and the tool's host bridge onto a command layer, in the solving
//! profile, author `built_in`; the bridge serves the three rows (`c600_tools::kernel_rows`). The kernel's own Net and
//! Strict halves on orbit sets are `c600-library/tests/ct_lb2_02.rs`, `net_oversize.rs` and LP2's kernel tests.
//! A pass is source and fixture evidence.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_check::{admit_built_in, Grant, Policy, Row};
use c600_command::{CommandLayer, Secret, Table};
use c600_digest::{sha256, sha256_hex};
use c600_kernel::{Derivation, Kernel, ProtectedSet, Snapshot, Verdict, PIECES, SLOTS};
use c600_library::{
    PROTECTION_CHECKS, PROTECTION_EXPORTS, PROTECTION_LOCKS, PROTECTION_MODULE, PROTECTION_ROWS, PROTECTION_SOURCE,
};
use c600_run::boundary::{Boundary, Context, Store};
use c600_run::interp::{load, Loaded, Options, Run};
use c600_run::meter::{Amounts, Outcome};
use c600_run::provenance::{Author, Producer, Profile};
use c600_run::value::{equal, Value};
use c600_tools::bridge::{base, Bridge};
use c600_tools::kernel_rows::{LABELS, POSITIONS, PROTECTION};
use c600_tools::tool::{checker, contract, lower_source, named_word_block, pins};

const LIMITS: Amounts = Amounts { steps: 1 << 24, kernel_cost: 1 << 40, memory: 1 << 34, output: 1 << 28 };

/// A fixed scramble from solved.
const SCRAMBLE: [i32; 20] =
    [17, -305, 911, 4, -1200, 600, 33, -88, 1021, -7, 450, 2, -999, 316, 75, -640, 1199, -12, 808, 23];

/// The code of a requested assurance whose coverage is incomplete (LB2.3, chapter 13).
const INCOMPLETE: &str = "LB2:protection.incomplete_coverage";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn kernel() -> Arc<Kernel> {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    let kernel = KERNEL.get_or_init(|| match Kernel::load(&repository().join("assets")) {
        Ok(kernel) => Arc::new(kernel),
        Err(error) => panic!("the model in assets/ loads: {error}"),
    });
    kernel.clone()
}

fn loaded() -> &'static Loaded {
    static LOADED: OnceLock<Loaded> = OnceLock::new();
    LOADED.get_or_init(|| {
        load(&lower_source(PROTECTION_SOURCE).expect("the library module lowers")).expect("the library module loads")
    })
}

fn wide(letters: &[i32]) -> Vec<i64> {
    letters.iter().map(|&m| i64::from(m)).collect()
}

/// The state [`SCRAMBLE`] derives from solved.
fn scrambled() -> Snapshot {
    let kernel = kernel();
    let recipe = kernel.word(&wide(&SCRAMBLE)).expect("a well-formed word");
    match kernel.derive(&kernel.solved(), &recipe, &ProtectedSet::none()).expect("no star certificate fails") {
        Derivation::Derived(derived) => derived.state,
        Derivation::Refused(refused) => panic!("refused with no protected orbit: {}", refused.refusal),
    }
}

/// A layer whose session starts at `start`, with no protected orbit.
fn layer_at(start: Snapshot) -> CommandLayer {
    let none = ProtectedSet::none();
    CommandLayer::new(kernel(), Table::h08(), Secret::fixed([5; 32]), Default::default(), "locks", start, none)
}

/// The context of one run of `entry` in the solving profile, built in, with a state read (LG3.13).
fn context(layer: &CommandLayer, entry: &str, inputs: usize) -> Context {
    let producer = Producer {
        contract: contract(),
        checker: Some(checker()),
        author: Author::BuiltIn,
        module: sha256(PROTECTION_SOURCE.as_bytes()),
        entry: entry.into(),
        profile: Profile::Solving,
    };
    let base = base(layer.revision());
    let inputs = vec![named_word_block(); inputs];
    Context { run: "locks".into(), base, producer, reads_state: true, inputs, pins: pins() }
}

/// Runs `entry` on `args` through a boundary and a bridge onto `layer`; a read sends no invocation and changes nothing.
fn run(layer: &mut CommandLayer, entry: &str, args: Vec<Value>) -> Run {
    let before = (layer.revision().clone(), layer.store().log.len());
    let context = context(layer, entry, args.len());
    let mut bridge = Bridge::new(layer, "locks");
    let mut store = Store::default();
    let mut boundary = Boundary::new(&mut bridge, &mut store, context);
    let run = loaded().run(PROTECTION_MODULE, entry, args, &mut boundary, &Options::new(LIMITS));
    drop(boundary);
    assert!(bridge.sent.is_empty(), "a read sends no invocation");
    assert_eq!((layer.revision().clone(), layer.store().log.len()), before, "a read changes nothing");
    run.expect("the run starts")
}

/// The answer of a completed run of `entry`.
fn answer(layer: &mut CommandLayer, entry: &str, args: Vec<Value>) -> Value {
    let run = run(layer, entry, args);
    assert_eq!((run.summary.outcome, run.summary.code), (Outcome::Completed, None), "{entry}: {:?}", run.summary.stop);
    run.value.expect("an answer")
}

fn field<'v>(value: &'v Value, name: &str) -> &'v Value {
    let Value::Record(record) = value else { panic!("a record: {value:?}") };
    record.get(name).unwrap_or_else(|| panic!("a field {name}: {value:?}"))
}

fn int(value: &Value) -> i64 {
    let Value::Int(n) = value else { panic!("an integer: {value:?}") };
    n.to_string().parse().expect("an i64")
}

fn text(value: &Value) -> &str {
    let Value::Text(t) = value else { panic!("a text: {value:?}") };
    t.as_str()
}

fn ints(value: &Value) -> Vec<i64> {
    let Value::List(items) = value else { panic!("a list: {value:?}") };
    items.items().iter().map(int).collect()
}

fn texts(value: &Value) -> Vec<String> {
    let Value::List(items) = value else { panic!("a list: {value:?}") };
    items.items().iter().map(|item| text(item).to_owned()).collect()
}

fn nats(items: &[i64]) -> Value {
    Value::list(items.iter().map(|&item| Value::int(item)).collect())
}

/// A recipe of one word step, in the library's step records.
fn steps(word: &[i32]) -> Value {
    Value::list(vec![Value::record(vec![
        ("kind", Value::text("word")),
        ("moves", Value::word(word)),
        ("node", Value::int(0)),
        ("orbit", Value::int(0)),
        ("sign", Value::int(0)),
    ])])
}

/// A captured lock as the entries take it.
fn lock(kind: &str, slots: &[i64], labels: &[i64]) -> Value {
    Value::record(vec![("kind", Value::text(kind)), ("labels", nats(labels)), ("slots", nats(slots))])
}

/// `Digest<literal word>`: the SHA-256 of the letters as little-endian 32-bit integers.
fn literal_digest(letters: &[i32]) -> String {
    sha256_hex(&letters.iter().flat_map(|m| m.to_le_bytes()).collect::<Vec<u8>>())
}

/// The slots generator `m` moves, in increasing order.
fn moved(m: i32) -> Vec<i64> {
    let kernel = kernel();
    let effect = kernel.effect(&kernel.word(&[i64::from(m)]).unwrap()).unwrap();
    effect.source().iter().map(|&slot| i64::from(slot)).collect()
}

/// A report's fields that every report shares, checked against its input, the lock and the recipe, its mode and level
/// (LB2.3), and
/// the coverage rule: a report is `clear` only when every required prefix was checked and nothing conflicts, and an
/// incomplete check without a conflict is `unknown` with `LB2:protection.incomplete_coverage`, never safe.
fn report(value: &Value, lock: &Value, mode: &str, word: &[i32]) -> (String, i64, i64, i64, Vec<i64>) {
    assert!(equal(field(value, "lock"), lock), "the lock");
    assert_eq!(text(field(value, "mode")), mode);
    assert_eq!(text(field(value, "level")), "labels");
    assert_eq!(text(field(value, "expansion_sha256")), literal_digest(word), "the input");
    assert_eq!(int(field(value, "primitive_count")), word.len() as i64);
    let required = if mode == "net" { 1 } else { word.len() as i64 + 1 };
    assert_eq!(int(field(value, "required")), required);
    let (verdict, checked) = (text(field(value, "verdict")).to_owned(), int(field(value, "checked")));
    let (refused, conflicts) = (int(field(value, "refused_prefix")), ints(field(value, "conflicts")));
    let errors = texts(field(value, "errors"));
    match verdict.as_str() {
        "clear" => assert!(checked == required && conflicts.is_empty() && errors.is_empty() && refused == -1),
        "fails" => assert!(!conflicts.is_empty() && errors.is_empty()),
        "unknown" => {
            assert!(checked < required && conflicts.is_empty() && refused == -1);
            assert_eq!(errors, [INCOMPLETE]);
        }
        other => panic!("a verdict: {other}"),
    }
    (verdict, checked, required, refused, conflicts)
}

/// The built-in column admits each entry only when its class lists the rows it reads: the three lock constructors
/// read `labels` or `positions` at the slots or pieces the solver names, and the three checks read `protection` at a
/// record of the lock, the mode and the recipe the solver names (LB3.5). Each reads state and prepares no preview
/// (LG3.13).
#[test]
fn ct_lb2_02_the_entries_are_admitted_with_their_rows() {
    assert_eq!(PROTECTION_ROWS, [LABELS, POSITIONS, PROTECTION]);
    let program = lower_source(PROTECTION_SOURCE).unwrap();
    let admit = |entry: &str, rows: &[&str]| {
        let class: BTreeSet<Row> = rows.iter().map(|row| Row::Read(row.to_string())).collect();
        admit_built_in(&program, PROTECTION_MODULE, entry, &class, &Grant::none(), &Policy::interim()).unwrap()
    };
    for entry in PROTECTION_EXPORTS {
        let own: &[&str] = match entry {
            "position_lock" => &[LABELS],
            "identity_lock" | "block_lock" => &[POSITIONS],
            _ => &[PROTECTION],
        };
        assert!(PROTECTION_LOCKS.contains(&entry) != PROTECTION_CHECKS.contains(&entry), "{entry}");
        for rows in [own, &PROTECTION_ROWS[..]] {
            let admitted = admit(entry, rows);
            assert!(admitted.is_admitted(), "{entry}: {:?}", admitted.diagnostics);
            assert!(admitted.separation.reads_state && !admitted.separation.prepares_previews, "{entry}");
        }
        let others: Vec<&str> = PROTECTION_ROWS.iter().copied().filter(|row| !own.contains(row)).collect();
        assert!(!admit(entry, &others).is_admitted(), "{entry}: its own row in its class");
    }
}

/// The locks capture what the head holds, at the slots or pieces the solver names, in their order: a position lock
/// the labels at its slots, an identity lock each piece's current slots with its stickers' labels (their home slots),
/// and a block lock each piece's home slots with their own labels, the completed block's predicate. From a scrambled
/// head the identity lock holds and the block lock does not where a piece is away from home.
#[test]
fn ct_lb2_02_locks_capture_the_head() {
    let kernel = kernel();
    let head = scrambled();
    let mut layer = layer_at(head.clone());
    let slots = [259_799, 0, 17, 0];
    let position = answer(&mut layer, "position_lock", vec![nats(&slots)]);
    let labels: Vec<i64> = slots.iter().map(|&slot| i64::from(head.labels()[slot as usize])).collect();
    assert!(equal(&position, &lock("position", &slots, &labels)));

    let pieces = [5, 177_119, 0, 64_000];
    let rows = kernel.positions(&head, &pieces).unwrap();
    let flat = |pick: fn(&c600_kernel::PiecePosition) -> &Vec<i32>| -> Vec<i64> {
        rows.iter().flat_map(|row| pick(row).iter().map(|&slot| i64::from(slot))).collect()
    };
    let (home, now) = (flat(|row| &row.home), flat(|row| &row.slots));
    let identity = answer(&mut layer, "identity_lock", vec![nats(&pieces)]);
    assert!(equal(&identity, &lock("identity", &now, &home)));
    let block = answer(&mut layer, "block_lock", vec![nats(&pieces)]);
    assert!(equal(&block, &lock("block", &home, &home)));
    for (index, &slot) in now.iter().enumerate() {
        assert_eq!(i64::from(head.labels()[slot as usize]), home[index], "the identity lock holds at its capture");
    }
}

/// The pieces whose stickers sit in `slots` at `state`, in increasing order, `count` of them.
fn pieces_at(state: &Snapshot, slots: &[i64], count: usize, home_only: bool) -> Vec<i64> {
    let all: Vec<i64> = (0..PIECES as i64).collect();
    let rows = kernel().positions(state, &all).unwrap();
    let wanted = |row: &&c600_kernel::PiecePosition| {
        let at_home = row.slots == row.home;
        row.slots.iter().any(|&slot| slots.binary_search(&i64::from(slot)).is_ok()) && (at_home || !home_only)
    };
    rows.iter().filter(wanted).take(count).map(|row| row.piece as i64).collect()
}

/// CT-LB2-02, Net against Strict for each captured lock. Two words have identity net effect, `[5, -5]` and
/// `[1, 2, 1, 2, -2, -1, -2, -1]`, and their first letter moves stickers of the lock. From solved and from a scrambled
/// head, a position lock on slots the first letter moves (with one it does not, out of order), an identity lock on
/// two pieces with a sticker there, and a block lock on completed pieces there (one or two; a scrambled head has few): Net judges the end of the complete
/// operation and clears each lock, and Strict refuses the first prefix, whose conflicts are the lock's slots the first
/// letter moves, in the lock's order. The identity lock's verdicts are the kernel's own for its pieces
/// (`Kernel::piece_protection`, `Kernel::strict_piece_protection`).
#[test]
fn ct_lb2_02_an_identity_word_passes_net_and_fails_strict_for_each_captured_lock() {
    let kernel = kernel();
    for start in [kernel.solved(), scrambled()] {
        let mut layer = layer_at(start.clone());
        for word in [vec![5, -5], vec![1, 2, 1, 2, -2, -1, -2, -1]] {
            let first = moved(word[0]);
            let still = (0..SLOTS as i64).find(|slot| first.binary_search(slot).is_err()).unwrap();
            let slots = [first[2], still, first[0], first[1]];
            let pieces = pieces_at(&start, &first, 2, false);
            let block = pieces_at(&start, &first, 2, true);
            assert!(pieces.len() == 2 && !block.is_empty(), "{word:?}: pieces to lock: {pieces:?} {block:?}");
            let locks = [
                ("position", answer(&mut layer, "position_lock", vec![nats(&slots)])),
                ("identity", answer(&mut layer, "identity_lock", vec![nats(&pieces)])),
                ("block", answer(&mut layer, "block_lock", vec![nats(&block)])),
            ];
            for (kind, captured) in &locks {
                let at = format!("{word:?} {kind}");
                let net = answer(&mut layer, "net_check", vec![captured.clone(), steps(&word)]);
                let (verdict, checked, ..) = report(&net, captured, "net", &word);
                assert_eq!((verdict.as_str(), checked), ("clear", 1), "{at}: Net judges the end only");

                let strict = answer(&mut layer, "strict_check", vec![captured.clone(), steps(&word)]);
                let (verdict, checked, _, refused, conflicts) = report(&strict, captured, "strict", &word);
                assert_eq!((verdict.as_str(), refused, checked), ("fails", 1, 2), "{at}: the first prefix");
                let locked = ints(field(captured, "slots"));
                let expected: Vec<i64> =
                    locked.iter().copied().filter(|slot| first.binary_search(slot).is_ok()).collect();
                assert_eq!(conflicts, expected, "{at}: the locked slots the first letter moves, in the lock's order");
            }

            let recipe = kernel.word(&wide(&word)).unwrap();
            let protected = kernel.protected_pieces(&pieces).unwrap();
            let effect = kernel.effect(&recipe).unwrap();
            assert_eq!(kernel.piece_protection(&start, &effect, &protected).unwrap().verdict, Verdict::Clear);
            let strict = kernel.strict_piece_protection(&start, &recipe, &protected).unwrap();
            assert_eq!(strict.first_refused_prefix, Some(1), "{word:?}: the kernel's own Strict row");
            let mut moved_by_kernel: Vec<i64> = strict
                .rows
                .iter()
                .flat_map(|row| row.before.iter().zip(&row.after).filter(|(b, a)| b != a).map(|(&b, _)| i64::from(b)))
                .collect();
            moved_by_kernel.sort_unstable();
            let identity = answer(&mut layer, "strict_check", vec![locks[1].1.clone(), steps(&word)]);
            let mut conflicts = ints(field(&identity, "conflicts"));
            conflicts.sort_unstable();
            assert_eq!(conflicts, moved_by_kernel, "{word:?}: the kernel's moved stickers of the pieces");
        }
    }
}

/// CT-LB2-02, an incomplete prefix check never returns safe. A Strict check within a limit judges the empty prefix
/// and the prefixes up to the limit. On `[1, 2, -2, -1]`, whose second letter is the first to move the locked slot,
/// a limit of 0 or 1 answers `unknown` with `LB2:protection.incomplete_coverage`, never `clear`, and a limit of 2 or
/// more refuses the second prefix. On `[1, -1]`, which never moves it, a limit below 2 answers `unknown` and a limit
/// of 2 or more `clear`, as the check without a limit does. Net clears both: their net effect is the identity.
#[test]
fn ct_lb2_02_an_incomplete_prefix_check_never_returns_safe() {
    let mut layer = layer_at(kernel().solved());
    let (one, two) = (moved(1), moved(2));
    let slot = *two.iter().find(|slot| one.binary_search(slot).is_err()).expect("a slot 2 moves and 1 does not");
    let captured = answer(&mut layer, "position_lock", vec![nats(&[slot])]);
    for (word, first) in [(vec![1, 2, -2, -1], Some(2)), (vec![1, -1], None)] {
        for limit in 0..=word.len() + 1 {
            let within = answer(
                &mut layer,
                "strict_check_within",
                vec![captured.clone(), steps(&word), Value::int(limit as u64)],
            );
            let (verdict, checked, required, refused, conflicts) = report(&within, &captured, "strict", &word);
            let at = format!("{word:?} within {limit}");
            match first {
                Some(first) if limit >= first => {
                    assert_eq!((verdict.as_str(), refused, checked), ("fails", first as i64, first as i64 + 1), "{at}");
                    assert_eq!(conflicts, [slot], "{at}");
                }
                _ if (limit as i64) + 1 < required => {
                    assert_eq!((verdict.as_str(), checked), ("unknown", limit as i64 + 1), "{at}: never safe");
                }
                _ => assert_eq!((verdict.as_str(), checked), ("clear", required), "{at}"),
            }
        }
        let whole = answer(&mut layer, "strict_check", vec![captured.clone(), steps(&word)]);
        let (verdict, ..) = report(&whole, &captured, "strict", &word);
        assert_eq!(verdict, if first.is_some() { "fails" } else { "clear" });
        let net = answer(&mut layer, "net_check", vec![captured.clone(), steps(&word)]);
        assert_eq!(report(&net, &captured, "net", &word).0, "clear");
    }
}

/// Net judges only the end of the complete operation, and Strict every executed prefix from the empty one. A word
/// that moves the locked slot fails both: Net with the slot it changes and no refused prefix, Strict at the first
/// prefix. A lock that does not hold at the head fails Strict at the empty prefix, with the slots that fail there,
/// whatever the word; Net clears it when the word's end state holds it and refuses it when it does not.
#[test]
fn ct_lb2_02_net_judges_the_end_and_strict_every_prefix() {
    let kernel = kernel();
    let mut layer = layer_at(kernel.solved());
    let two = moved(2);
    let slot = two[0];
    let captured = answer(&mut layer, "position_lock", vec![nats(&[slot])]);
    let net = answer(&mut layer, "net_check", vec![captured.clone(), steps(&[2])]);
    let (verdict, checked, _, refused, conflicts) = report(&net, &captured, "net", &[2]);
    assert_eq!((verdict.as_str(), checked, refused, conflicts), ("fails", 1, -1, vec![slot]));
    let strict = answer(&mut layer, "strict_check", vec![captured.clone(), steps(&[2])]);
    let (verdict, _, _, refused, conflicts) = report(&strict, &captured, "strict", &[2]);
    assert_eq!((verdict.as_str(), refused, conflicts), ("fails", 1, vec![slot]));

    // Letter 2 takes the sticker in `slot` to `next`; a lock that wants it in `next` fails at the solved head.
    let effect = kernel.effect(&kernel.word(&[2]).unwrap()).unwrap();
    let at = effect.source().binary_search(&(slot as i32)).unwrap();
    let next = i64::from(effect.destination()[at]);
    let wanted = lock("position", &[next], &[slot]);
    for word in [vec![2], vec![1, -1]] {
        let strict = answer(&mut layer, "strict_check", vec![wanted.clone(), steps(&word)]);
        let (verdict, checked, _, refused, conflicts) = report(&strict, &wanted, "strict", &word);
        assert_eq!((verdict.as_str(), checked, refused, conflicts), ("fails", 1, 0, vec![next]), "{word:?}");
    }
    let ends = answer(&mut layer, "net_check", vec![wanted.clone(), steps(&[2])]);
    assert_eq!(report(&ends, &wanted, "net", &[2]).0, "clear", "the end state holds the lock");
    let stays = answer(&mut layer, "net_check", vec![wanted.clone(), steps(&[1, -1])]);
    assert_eq!(report(&stays, &wanted, "net", &[1, -1]).4, [next], "the end state does not");
}
