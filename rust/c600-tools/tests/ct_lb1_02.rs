//! CT-LB1-02 (part: the 600-cell-Full half, the library's entry "Word equality keys" on the pinned model's relation.
//! The jumbling half, the entry `menu_word_keys` in `c600-jumble-tool` with LG2.18's domain-preserving key of menu
//! words on C8's pinned menu matrices, runs with its key and kernel halves in `c600-jumble-tool/tests/lb3.rs`; with
//! them CT-LB1-02 runs whole)
//!
//! The owner's answer of 17 November 2026 gives C5's word keys of 600-cell-Full words LB1.10's draft `trace-key/1`.
//! The entry is language code, the library's module `word_keys` (`c600_library::word_equality_keys`), run on
//! `c600-run` with the relation `full-disjoint-support/1` of the model in `assets/` pinned. A key is the least
//! topological word of the word's letters under that relation: it reorders adjacent independent letters and keeps
//! every letter. On 600-cell-Full words the tests check the test table's three claims:
//!
//! - Keys preserve the end digest: words with equal keys derive the same state from solved and from a scrambled
//!   state, and the library's answer is the relation's own.
//! - At a blocked generator, every orbit its turn moves protected, the literal `a, a⁻¹` and its key's spelling are both
//!   refused at their first letter under Strict protection, and the empty word is admitted. Free reduction, the IR
//!   draft's `word_key`, merges the pair with the empty word, which is why the entry does not use it.
//! - Prefix protection reads literal letters: two words with one key are refused at different prefixes.
//!
//! Evidence kind: source and fixture (the pinned model). No Windows/DirectX, input or performance claim.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use c600_check::{check, separation, Policy, Separation, WORD_KEY_NOT_EXECUTABLE};
use c600_digest::sha256_hex;
use c600_kernel::{Derivation, Kernel, ProtectedSet, Snapshot, ORBITS, PIECES};
use c600_library::{
    lower_source, lower_sources, word_equality_keys, word_keys_sha256, WORD_KEYS_EXPORTS, WORD_KEYS_MODULE,
    WORD_KEYS_SOURCE,
};
use c600_run::interp::{load, Loaded, NoHost, Options};
use c600_run::meter::{Amounts, Outcome};
use c600_run::trace::Relation;
use c600_run::value::Value;
use c600_syntax::{format, parse};
use c600_tools::supports::full_relation;

const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };

/// A scramble of 20 letters across the alphabet, as in CT-LB2-02.
const SCRAMBLE: [i32; 20] =
    [17, -305, 911, 4, -1200, 600, 33, -88, 1021, -7, 450, 2, -999, 316, 75, -640, 1199, -12, 808, 23];

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn kernel() -> &'static Kernel {
    static KERNEL: OnceLock<Kernel> = OnceLock::new();
    KERNEL.get_or_init(|| match Kernel::load(&repository().join("assets")) {
        Ok(kernel) => kernel,
        Err(error) => panic!("the model in assets/ does not load: {error}"),
    })
}

/// The relation of the pinned model, the one a run pins for the entry.
fn relation() -> &'static Relation {
    static RELATION: OnceLock<Relation> = OnceLock::new();
    RELATION.get_or_init(|| full_relation(kernel()).expect("the supports make a relation"))
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

/// The two base states of these tests: solved and scrambled.
fn bases() -> [Snapshot; 2] {
    [kernel().solved(), scrambled()]
}

/// The literal letters replayed from `base` under Strict protection of `protected` (KN2.4 `replay`).
fn replay(base: &Snapshot, protected: &ProtectedSet, letters: &[i32]) -> Derivation {
    let mut replay = kernel().replay(base, protected).expect("the model's base and set");
    replay.push(&wide(letters)).expect("letters of the alphabet");
    replay.finish()
}

/// The first refused prefix of a replay, `None` when it derives.
fn refused_at(derivation: Derivation) -> Option<u64> {
    match derivation {
        Derivation::Derived(_) => None,
        Derivation::Refused(refused) => {
            assert!(refused.first_refused_prefix.is_some(), "Strict names the prefix");
            refused.first_refused_prefix
        }
    }
}

/// The end digest of a word replayed from `base` with nothing protected.
fn end(base: &Snapshot, letters: &[i32]) -> String {
    match replay(base, &ProtectedSet::none(), letters) {
        Derivation::Derived(derived) => derived.state.digest().to_owned(),
        Derivation::Refused(refused) => panic!("refused with no protected orbit: {}", refused.refusal),
    }
}

/// The letters of a word's trace key, as the relation spells it.
fn spelling(letters: &[i32]) -> Vec<i32> {
    relation().key(letters).expect("letters of the alphabet").letters().to_vec()
}

/// The first generators a < b < c of the model, by ID, with b independent of a and c and a dependent on c, as in
/// CT-LB1-06.
fn triple() -> (i32, i32, i32) {
    let ind = |x: i32, y: i32| relation().independent(x, y);
    (1..=1200)
        .flat_map(|a| (a + 2..=1200).filter(move |&c| !ind(a, c)).map(move |c| (a, c)))
        .find_map(|(a, c)| (a + 1..c).find(|&b| ind(a, b) && ind(b, c)).map(|b| (a, b, c)))
        .expect("three such generators")
}

/// A client of `word_keys` in the canonical layout: it imports the module by the SHA-256 of its source (the
/// placeholder `KEYS`), and `reduced` compares the IR draft's free-reduction keys for contrast.
const CLIENT: &str = r#"edition "6cs-draft-1";
module keys_client requires [];
import word_keys "KEYS";

export fn same(x: Word, y: Word) -> Bool effects [] {
    return word_keys.same_word(x, y);
}

export fn keyed(x: Word, y: Word) -> Bool effects [] {
    return exact_eq(word_keys.key(x), word_keys.key(y));
}

export fn reduced(x: Word, y: Word) -> Bool effects [] {
    return exact_eq(word_key(x), word_key(y));
}
"#;

/// A client that uses a key as letters: it returns one as a `Word` and inverts one.
const MISUSE: &str = r#"edition "6cs-draft-1";
module keys_misuse requires [];
import word_keys "KEYS";

export fn letters(w: Word) -> Word effects [] {
    return word_keys.key(w);
}

export fn undo(w: Word) -> Word effects [] {
    return inverse(word_keys.key(w));
}
"#;

fn client() -> &'static Loaded {
    static CLIENT_LOADED: OnceLock<Loaded> = OnceLock::new();
    CLIENT_LOADED.get_or_init(|| {
        let client = CLIENT.replace("KEYS", &word_keys_sha256());
        let file = parse(client.as_bytes());
        assert!(file.diagnostics.is_empty(), "{:#?}", file.diagnostics);
        assert_eq!(format(&file).as_deref(), Some(client.as_str()), "the client is in the canonical layout");
        let program = lower_sources(&[WORD_KEYS_SOURCE, &client]).expect("the module and the client lower together");
        let report = check(&program).expect("well-formed IR");
        assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
        load(&program).expect("the program loads")
    })
}

/// A client function's answer on two words, with the model's relation pinned.
fn ask(function: &str, x: &[i32], y: &[i32]) -> bool {
    let options = Options { relation: Some(relation()), ..Options::new(WIDE) };
    let args = vec![Value::word(x), Value::word(y)];
    let run = client().run("keys_client", function, args, &mut NoHost, &options).expect("the run starts");
    assert_eq!(run.summary.outcome, Outcome::Completed, "{function}: {:?}", run.summary.stop);
    let output = run.output.expect("a completed run");
    assert!(output == b"true\n" || output == b"false\n", "{function}: {output:?}");
    output == b"true\n"
}

/// The library's word-key source parses with no diagnostic, is in the formatter's canonical layout with LF line
/// endings, and lowers to one module, `word_keys`, that requests no capability and exports `key` and `same_word`. The
/// checker reports no diagnostic, and both exports are pure. The catalogue row is LB1.8's "Word equality keys", its
/// Effect field is the checker's effects and its digest is the source's. A key is never letters: a client that returns
/// one as a `Word` or inverts one is refused with `LG2:type.word_key_not_executable` (LG2.13, LB1.9).
#[test]
fn ct_lb1_02_the_word_keys_source_checks_and_is_pure() {
    let file = parse(WORD_KEYS_SOURCE.as_bytes());
    assert!(file.diagnostics.is_empty() && file.holes.is_empty(), "{:#?}", file.diagnostics);
    assert_eq!(format(&file).as_deref(), Some(WORD_KEYS_SOURCE), "the source is in the canonical layout");
    assert!(!WORD_KEYS_SOURCE.contains('\r') && WORD_KEYS_SOURCE.ends_with('\n'), "LF line endings");

    let program = lower_source(WORD_KEYS_SOURCE).expect("the source lowers");
    let [module] = &program.modules[..] else { panic!("one module, not {}", program.modules.len()) };
    assert_eq!(module.name, WORD_KEYS_MODULE);
    assert!(module.capabilities.is_empty(), "the module requests no capability");
    let exports: Vec<&str> = module.functions.iter().filter(|f| f.exported).map(|f| f.name.as_str()).collect();
    assert_eq!(exports, WORD_KEYS_EXPORTS);

    let report = check(&program).expect("well-formed IR");
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    let row = word_equality_keys();
    for function in WORD_KEYS_EXPORTS {
        let summary = report.summary(WORD_KEYS_MODULE, function).expect("a summary");
        assert!(summary.effects.is_empty(), "{function}: {:?}", summary.effects);
        assert!(summary.capabilities.is_empty() && summary.vars.is_empty() && summary.unresolved.is_empty());
        assert!(summary.state_reads.is_empty() && summary.rows.is_empty(), "{function} reads no state");
        assert_eq!(
            separation(summary, &Policy::interim()),
            Separation { reads_state: false, prepares_previews: false }
        );
        assert_eq!(row.effects.iter().copied().collect::<BTreeSet<_>>(), summary.effects, "Effect: pure");
    }
    assert_eq!(row.entry, "Word equality keys");
    assert!(row.level.starts_with("600-cell-Full words"));
    assert!(row.notes.contains("an inverse pair is not the empty word"));
    assert_eq!((row.module, row.functions), (WORD_KEYS_MODULE, &WORD_KEYS_EXPORTS[..]));
    assert_eq!(row.source_sha256, sha256_hex(WORD_KEYS_SOURCE.as_bytes()));
    assert_eq!(row.source_sha256, word_keys_sha256());

    let misuse = MISUSE.replace("KEYS", &word_keys_sha256());
    let program = lower_sources(&[WORD_KEYS_SOURCE, &misuse]).expect("the module and the client lower together");
    let report = check(&program).expect("well-formed IR");
    let codes: Vec<&str> = report.diagnostics.iter().map(|d| d.code).collect();
    assert_eq!(codes, [WORD_KEY_NOT_EXECUTABLE; 2], "{:#?}", report.diagnostics);
}

/// For the generators a < b < c of [`triple`]: words that differ by swaps of adjacent independent letters, also inside
/// a scramble and against a key's own spelling, have equal keys and the same end digest from solved and from the
/// scrambled state. Words with a dependent swap, or an inverse pair against the empty word, keep unequal keys. Keys
/// are sound, not complete: the inverse pair ends where the empty word does. The library's `same_word` and its keys'
/// equality agree with the relation's spelling on every pair.
#[test]
fn ct_lb1_02_equal_keys_keep_the_end_digest() {
    let (a, b, c) = triple();
    eprintln!("a, b, c = {a}, {b}, {c}");
    let then = |head: &[i32], tail: &[i32]| [head, tail].concat();
    let pairs: [(Vec<i32>, Vec<i32>, bool); 9] = [
        (vec![b, c, a], vec![c, a, b], true),
        (vec![b, -a], vec![-a, b], true),
        (then(&SCRAMBLE, &[b, c, a]), then(&SCRAMBLE, &[c, a, b]), true),
        (then(&[b, -a], &SCRAMBLE), then(&[-a, b], &SCRAMBLE), true),
        (SCRAMBLE.to_vec(), spelling(&SCRAMBLE), true),
        (then(&[c, a, b], &SCRAMBLE), spelling(&then(&[c, a, b], &SCRAMBLE)), true),
        (vec![a, b, c], vec![b, c, a], false),
        (vec![a, c], vec![c, a], false),
        (vec![a, -a], Vec::new(), false),
    ];
    let bases = bases();
    for (x, y, equal) in &pairs {
        assert_eq!(spelling(x) == spelling(y), *equal, "the relation's keys of {x:?} and {y:?}");
        assert_eq!(ask("same", x, y), *equal, "same_word({x:?}, {y:?})");
        assert_eq!(ask("keyed", x, y), *equal, "key({x:?}) = key({y:?})");
        if *equal {
            for base in &bases {
                assert_eq!(end(base, x), end(base, y), "{x:?} and {y:?} from {}", base.digest());
            }
        }
    }
    for base in &bases {
        assert_eq!(end(base, &[a, -a]), end(base, &[]), "the inverse pair ends where the empty word does");
    }
}

/// At a blocked generator a, every orbit its turn moves protected: the literal word a, a⁻¹ is refused at its first
/// letter under Strict protection, its key's spelling is the same two letters and is refused there too, and the empty
/// word is admitted and derives the base. Net protection admits the pair, whose net map is the identity, so it is the
/// literal prefixes that refuse it. The library tells the pair from the empty word; free reduction does not.
#[test]
fn ct_lb1_02_at_a_blocked_generator_the_inverse_pair_and_its_key_are_refused() {
    let (a, _, _) = triple();
    let kernel = kernel();
    let solved = kernel.solved();
    let touched: Vec<i64> = (0..ORBITS as i64)
        .filter(|&orbit| {
            let set = kernel.protected_set(&[orbit]).expect("an orbit of the model");
            refused_at(replay(&solved, &set, &[a])) == Some(1)
        })
        .collect();
    assert!(!touched.is_empty(), "the turn of {a} moves some orbit");
    let blocked = kernel.protected_set(&touched).expect("orbits of the model");
    let pair = [a, -a];
    assert_eq!(spelling(&pair), pair, "the key keeps both letters: nothing cancels");
    for base in bases() {
        assert_eq!(refused_at(replay(&base, &blocked, &pair)), Some(1), "the literal pair");
        assert_eq!(refused_at(replay(&base, &blocked, &spelling(&pair))), Some(1), "its key's spelling");
        match replay(&base, &blocked, &[]) {
            Derivation::Derived(derived) => assert_eq!(derived.state.digest(), base.digest(), "the empty word"),
            Derivation::Refused(refused) => panic!("the empty word is refused: {}", refused.refusal),
        }
        let recipe = kernel.word(&wide(&pair)).expect("a well-formed word");
        match kernel.derive(&base, &recipe, &blocked).expect("no star certificate fails") {
            Derivation::Derived(derived) => assert_eq!(derived.state.digest(), base.digest(), "Net: the identity"),
            Derivation::Refused(refused) => panic!("Net refuses the identity: {}", refused.refusal),
        }
    }
    assert!(!ask("same", &pair, &[]), "the library's key keeps the pair");
    assert!(ask("reduced", &pair, &[]), "free reduction merges the pair with the empty word");
}

/// For independent generators a < b, the words b, a and a, b have one key, spelled a, b. Protect two pieces whose
/// stickers sit on slots b's turn moves (so none that a's turn moves): under Strict protection the literal b, a is
/// refused at prefix 1 and a, b at prefix 2, from solved and from the scrambled state. Equal keys, different prefixes:
/// prefix protection reads the literal letters, never the key.
#[test]
fn ct_lb1_02_prefix_protection_reads_the_literal_letters() {
    let (a, b, _) = triple();
    let kernel = kernel();
    assert!(a < b && relation().independent(a, b));
    assert_eq!(spelling(&[b, a]), [a, b], "the key puts the smaller generator first");
    assert!(ask("same", &[b, a], &[a, b]));
    let support = |m: i32| -> Vec<i32> {
        kernel.effect(&kernel.word(&[i64::from(m)]).expect("a word")).expect("an effect").source().to_vec()
    };
    let (moved_a, moved_b) = (support(a), support(b));
    let all: Vec<i64> = (0..PIECES as i64).collect();
    for base in bases() {
        let rows = kernel.positions(&base, &all).expect("the pieces of the model");
        let on = |slots: &[i32], moved: &[i32]| slots.iter().any(|slot| moved.binary_search(slot).is_ok());
        let pieces: Vec<i64> = rows
            .iter()
            .filter(|row| on(&row.slots, &moved_b) && !on(&row.slots, &moved_a))
            .take(2)
            .map(|row| row.piece as i64)
            .collect();
        assert_eq!(pieces.len(), 2, "two pieces on slots that b's turn moves");
        let protected = kernel.protected_pieces(&pieces).expect("pieces of the model");
        let first = |letters: &[i32]| {
            let recipe = kernel.word(&wide(letters)).expect("a well-formed word");
            kernel.strict_piece_protection(&base, &recipe, &protected).expect("the model's base").first_refused_prefix
        };
        assert_eq!(first(&[b, a]), Some(1), "the literal b, a");
        assert_eq!(first(&[a, b]), Some(2), "the literal a, b");
    }
}
