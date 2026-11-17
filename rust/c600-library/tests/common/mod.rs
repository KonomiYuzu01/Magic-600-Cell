//! Shared fixtures: the library lowered together with a client module that imports it by digest, runs of both on
//! `c600-run` with no host, and the 600-cell-Full kernel read from `assets/`. Every value here is a source or fixture
//! value.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use c600_check::check;
use c600_kernel::{Derivation, Effect, Kernel, ProtectedSet, Snapshot};
use c600_library::{lower_sources, source_sha256, MODULE, SOURCE};
use c600_run::interp::{load, Loaded, NoHost, Options};
use c600_run::meter::{Amounts, Outcome};
use c600_run::value::Value;

/// Run limits wide enough for every word these tests build.
pub const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };

/// The client module's name.
pub const CLIENT: &str = "client";

/// A client of the library, in the canonical layout: it imports `words` by the SHA-256 of the library's source (the
/// placeholder `LIBRARY`) and builds words with `words.power` and the IR draft's own word operations.
const CLIENT_SOURCE: &str = r#"edition "6cs-draft-1";
module client requires [];
import words "LIBRARY";

export fn inverse_of(w: Word) -> Word effects [] {
    return inverse(w);
}

export fn conjugate_of(s: Word, w: Word) -> Word effects [] {
    return conjugate(s, w);
}

export fn commutator_of(a: Word, b: Word) -> Word effects [] {
    return commutator(a, b);
}

export fn commutator_power(a: Word, b: Word, n: Int) -> Word effects [] {
    return words.power(commutator(a, b), n);
}

export fn there_and_back(w: Word, n: Int) -> Word effects [] {
    let there = words.power(w, n);
    return concat(there, inverse(there));
}
"#;

/// The client's source with the library's digest in its import.
pub fn client_source() -> String {
    CLIENT_SOURCE.replace("LIBRARY", &source_sha256())
}

/// The library and the client, lowered together, checked with no diagnostic and loaded.
pub fn loaded() -> Loaded {
    let client = client_source();
    let program = lower_sources(&[SOURCE, &client]).expect("the library and the client lower together");
    let report = check(&program).expect("well-formed IR");
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    load(&program).expect("the program loads")
}

/// Runs `module.function` on `args` with no host, so no state exists for the run to read, and returns the word it
/// builds.
pub fn build(loaded: &Loaded, module: &str, function: &str, args: Vec<Value>) -> Vec<i32> {
    let run = loaded.run(module, function, args, &mut NoHost, &Options::new(WIDE)).expect("the run starts");
    assert_eq!(run.summary.outcome, Outcome::Completed, "{module}.{function}: {:?}", run.summary.code);
    match run.value {
        Some(Value::Word(letters)) => letters.to_vec(),
        other => panic!("{module}.{function} built no word: {other:?}"),
    }
}

/// The library's `power(w, n)`.
pub fn power(loaded: &Loaded, w: &[i32], n: i64) -> Vec<i32> {
    build(loaded, MODULE, "power", vec![Value::word(w), Value::int(n)])
}

/// A function of the client.
pub fn client(loaded: &Loaded, function: &str, args: Vec<Value>) -> Vec<i32> {
    build(loaded, CLIENT, function, args)
}

pub fn word(letters: &[i32]) -> Value {
    Value::word(letters)
}

pub fn int(n: i64) -> Value {
    Value::int(n)
}

pub fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// The 600-cell-Full kernel over the model in `assets/`.
pub fn kernel() -> &'static Kernel {
    static KERNEL: OnceLock<Kernel> = OnceLock::new();
    KERNEL.get_or_init(|| match Kernel::load(&repository().join("assets")) {
        Ok(kernel) => kernel,
        Err(error) => panic!("the model in assets/ does not load: {error}"),
    })
}

/// A word's letters as the kernel takes them.
pub fn wide(letters: &[i32]) -> Vec<i64> {
    letters.iter().map(|&m| i64::from(m)).collect()
}

/// The effect row of a word of at least one letter.
pub fn effect(letters: &[i32]) -> Effect {
    let kernel = kernel();
    kernel.effect(&kernel.word(&wide(letters)).expect("a well-formed word")).expect("no star certificate fails")
}

/// The orbits a word of at least one letter moves.
pub fn moved_orbits(letters: &[i32]) -> Vec<i64> {
    effect(letters).support.iter().map(|row| i64::from(row.orbit)).collect()
}

/// The state `letters` derive from `base` with no protected orbit; a word of no letters leaves `base`.
pub fn after(base: &Snapshot, letters: &[i32]) -> Snapshot {
    if letters.is_empty() {
        return base.clone();
    }
    let kernel = kernel();
    let recipe = kernel.word(&wide(letters)).expect("a well-formed word");
    match kernel.derive(base, &recipe, &ProtectedSet::none()).expect("no star certificate fails") {
        Derivation::Derived(derived) => derived.state,
        Derivation::Refused(refused) => panic!("refused with no protected orbit: {}", refused.refusal),
    }
}

/// A fixed scramble from solved.
pub const SCRAMBLE: [i32; 20] =
    [17, -305, 911, 4, -1200, 600, 33, -88, 1021, -7, 450, 2, -999, 316, 75, -640, 1199, -12, 808, 23];

/// The state [`SCRAMBLE`] derives from solved.
pub fn scrambled() -> Snapshot {
    let kernel = kernel();
    let state = after(&kernel.solved(), &SCRAMBLE);
    assert_ne!(state.digest(), kernel.solved().digest(), "the scramble moves stickers");
    state
}

/// A word's letters in chunks of growing size, after an empty chunk: 0, 1, 2, 3 and so on letters, the last chunk
/// holding what is left.
pub fn chunks(letters: &[i64]) -> Vec<&[i64]> {
    let mut out = vec![&letters[..0]];
    let (mut start, mut size) = (0, 1);
    while start < letters.len() {
        let end = (start + size).min(letters.len());
        out.push(&letters[start..end]);
        (start, size) = (end, size + 1);
    }
    out
}
