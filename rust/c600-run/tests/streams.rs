//! DET.63's streamed words at run time (LP2 draft until the IR freeze, PHS.49; KN2.10, LG4.6). `word_stream(c, chunk)`
//! streams a compressed word chronologically in semantic chunks fixed by its sealed inputs: one `word_open` row that
//! reserves the cursor, then one `chunk:<position>` row per chunk that preflights its share before it expands a letter
//! and releases its scratch after. Expansion keeps the compressed structure and a stack of parts, never the whole word,
//! and an interruption stops the stream at a chunk boundary. `stream_digest` reads the literal digest, the SHA-256 of
//! the letters as little-endian 32-bit integers, which no chunk size changes. These are the language half of CT-LG4-03
//! and the streaming rows of the frozen vectors `streaming`, `streaming-complete-split` and
//! `streaming-complete-coalesced` (DET.65), which `det65_programs.rs` runs as programs. The kernel half, the literal
//! digest and the Strict prefix under any chunking, is `c600-kernel`'s `ct_lg4_03_*`.

use std::cell::Cell;
use std::sync::Arc;

use c600_check::check;
use c600_digest::sha256_hex;
use c600_run::encode::{encode, Noncanonical};
use c600_run::interp::{code, load, Cancel, Loaded, NoHost, Options, Run};
use c600_run::meter::{self, Amounts, Interruption, Outcome, Summary, Tariffs};
use c600_run::value::{equal, Compressed, Value, WordStream};
use c600_syntax::{lower, parse};
use num_bigint::BigInt;

const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };
const HEADER: &str = "edition \"6cs-draft-1\";\nmodule m requires [];\n\n";

/// The checker's codes for one module `m`.
fn codes(declarations: &str) -> Vec<&'static str> {
    let source = format!("{HEADER}{declarations}");
    let file = parse(source.as_bytes());
    assert!(file.diagnostics.is_empty(), "{source}\n{:#?}", file.diagnostics);
    let lowered = lower(&[file]).unwrap_or_else(|e| panic!("{e}"));
    let report = check(&lowered.program).unwrap_or_else(|e| panic!("{e}"));
    report.diagnostics.iter().map(|d| d.code).collect()
}

/// Parses, lowers, checks and loads one module `m`; the checker must accept it.
fn build(declarations: &str) -> Loaded {
    let source = format!("{HEADER}{declarations}");
    let file = parse(source.as_bytes());
    assert!(file.diagnostics.is_empty(), "{source}\n{:#?}", file.diagnostics);
    let lowered = lower(&[file]).unwrap_or_else(|e| panic!("{e}"));
    let report = check(&lowered.program).unwrap_or_else(|e| panic!("{e}"));
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    load(&lowered.program).unwrap_or_else(|r| panic!("{r:?}"))
}

fn run_with(loaded: &Loaded, function: &str, options: &Options) -> Run {
    loaded.run("m", function, Vec::new(), &mut NoHost, options).unwrap_or_else(|r| panic!("{r:?}"))
}

fn run(loaded: &Loaded, function: &str) -> Run {
    run_with(loaded, function, &Options::new(WIDE))
}

fn listing(summary: &Summary) -> String {
    summary.rows.iter().map(|r| format!("\n  {} {:?}", r.at, (r.steps, r.kernel_cost, r.memory, r.released))).collect()
}

/// The stream's rows, `word_open` and each `chunk:<position>`, as (operation, steps, kernel cost, memory, released).
fn stream_rows(summary: &Summary) -> Vec<(String, u128, u128, u128, u128)> {
    summary
        .rows
        .iter()
        .filter_map(|r| {
            let op = r.at.rsplit_once('#').map(|(_, op)| op)?;
            (op == "word_open" || op.starts_with("chunk:"))
                .then(|| (op.to_string(), r.steps, r.kernel_cost, r.memory, r.released))
        })
        .collect()
}

/// The letters of a word literal `word[…]`.
fn literal(letters: &[i32]) -> String {
    let written: Vec<String> = letters.iter().map(|l| format!("{l:+}")).collect();
    format!("word[{}]", written.join(", "))
}

/// The literal digest of `letters` as the kernel computes it: the SHA-256 of the letters as little-endian 32-bit
/// integers.
fn literal_digest(letters: &[i32]) -> String {
    let bytes: Vec<u8> = letters.iter().flat_map(|l| l.to_le_bytes()).collect();
    sha256_hex(&bytes)
}

/// A caller that builds the compressed word, binds it and passes it with the chunk size to `s`, which streams it into
/// `cursor` and returns. Parameters are borrowed, so the stream releases no operand and `s`'s scope exit releases the
/// cursor alone: the frozen vectors' `close`.
fn streamer(letters: &[i32], chunk: &str) -> String {
    format!(
        "fn s(c: CompressedWord, k: Nat) -> Unit effects [] {{\n    let cursor = word_stream(c, k);\n    \
         return ();\n}}\n\n\
         export fn go() -> Unit effects [] {{\n    let c = compress({});\n    return s(c, {chunk});\n}}\n",
        literal(letters)
    )
}

/// `streaming-complete-split` and `streaming-complete-coalesced` (DET.63): five letters in chunks of two are an open
/// row of one step, the setup tariff and the 2-byte cursor, then chunks at positions 0, 2 and 4, each one step with
/// `expand_per_letter` and `replay_per_letter` per letter and 2 bytes of scratch per letter, released in the same
/// row; the last chunk is short and has its own share. The scope exit that drops the cursor releases its 2 bytes.
/// The kernel shares sum to the full tariff, and the setup is charged once, at the open.
#[test]
fn a_stream_opens_once_and_charges_each_declared_chunk() {
    let t = Tariffs::TEST;
    let per_letter = t.expand_per_letter + t.replay_per_letter;
    let loaded = build(&streamer(&[1, -2, 3, 4, -5], "2"));
    let r = run(&loaded, "go");
    let list = listing(&r.summary);
    assert_eq!(r.summary.outcome, Outcome::Completed, "{:?}{list}", r.summary.stop);
    let rows = stream_rows(&r.summary);
    let want = [
        ("word_open", 1, t.stream_setup, 2, 0),
        ("chunk:0", 1, 2 * per_letter, 4, 4),
        ("chunk:2", 1, 2 * per_letter, 4, 4),
        ("chunk:4", 1, per_letter, 2, 2),
    ];
    let want: Vec<_> = want.iter().map(|(op, s, k, m, r)| (op.to_string(), *s, *k, *m, *r)).collect();
    assert_eq!(rows, want, "{list}");
    assert_eq!(want.iter().map(|w| w.2).sum::<u128>(), t.stream_setup + 5 * per_letter);
    // The vectors' rows, with `close` the scope exit of `s`.
    let at = |suffix: &str| r.summary.rows.iter().position(|row| row.at.ends_with(suffix)).expect(suffix);
    let close = &r.summary.rows[at("m.s:root#scope_exit")];
    assert_eq!((close.steps, close.kernel_cost, close.memory, close.released), (1, 0, 0, 2), "{list}");
    assert!(at("#word_open") < at("#chunk:0") && at("#chunk:4") < at("m.s:root#scope_exit"));
}

/// The schedule depends on the sealed inputs alone (DET.63): a zero-length stream has its open row only, a chunk
/// larger than the word is one short chunk with the word's share, a chunk of one letter gives one row per letter, and
/// a chunk size of 0 is a pointwise failure before any step.
#[test]
fn the_schedule_comes_from_the_length_and_the_chunk_size() {
    let t = Tariffs::TEST;
    let per_letter = t.expand_per_letter + t.replay_per_letter;
    let r = run(&build(&streamer(&[], "3")), "go");
    assert_eq!(r.summary.outcome, Outcome::Completed);
    assert_eq!(stream_rows(&r.summary), [("word_open".to_string(), 1, t.stream_setup, 2, 0)]);

    let r = run(&build(&streamer(&[7, 8, 9], "1000000000000000000000000000000")), "go");
    assert_eq!(r.summary.outcome, Outcome::Completed);
    let rows = stream_rows(&r.summary);
    assert_eq!(rows[1..], [("chunk:0".to_string(), 1, 3 * per_letter, 6, 6)], "{}", listing(&r.summary));

    let r = run(&build(&streamer(&[7, 8, 9], "1")), "go");
    let ops: Vec<String> = stream_rows(&r.summary).into_iter().map(|row| row.0).collect();
    assert_eq!(ops, ["word_open", "chunk:0", "chunk:1", "chunk:2"]);

    let r = run(&build(&streamer(&[7, 8, 9], "0")), "go");
    assert_eq!(r.summary.outcome, Outcome::Error);
    assert_eq!(r.summary.code, Some(meter::code::POINTWISE_FAILED));
    let stop = r.summary.stop.as_ref().expect("a stop");
    assert!(stop.at.ends_with("#word_stream"), "{}", stop.at);
    assert!(stream_rows(&r.summary).is_empty(), "no step before the schedule is known");
}

/// CT-LG4-03 (part: the interpreter's half, a stream under a small memory grant; the literal digest and the
/// cancellation are the next tests, the prefix under any chunking the kernel's; not run: a stream fed chunk by chunk
/// into the kernel's replay, since a run proposes no stream (no owner yet)): a word of 3·2¹⁶ letters, built by sixteen
/// doublings, streams in chunks of 64 letters under a grant far below its expanded size: the run holds the compressed
/// word, the cursor and one chunk's scratch at a time (KN2.10, LG4.6), and the same run with its memory limit at the
/// peak it used completes. An expansion of the whole word would reserve 2 bytes per letter.
#[test]
fn a_long_word_streams_under_a_small_memory_grant() {
    let doublings: String = (0..16).map(|k| format!("    let w{} = concat(w{k}, w{k});\n", k + 1)).collect();
    let loaded = build(&format!(
        "fn s(c: CompressedWord) -> Text effects [] {{\n    let cursor = word_stream(c, 64);\n    \
         return stream_digest(cursor);\n}}\n\n\
         export fn go() -> Text effects [] {{\n    let w0 = compress(word[+1, -2, +3]);\n{doublings}    return s(w16);\n}}\n"
    ));
    let r = run(&loaded, "go");
    assert_eq!(r.summary.outcome, Outcome::Completed, "{:?}", r.summary.stop);
    let length = 3u128 << 16;
    let chunks = stream_rows(&r.summary).len() as u128 - 1;
    assert_eq!(chunks, length / 64);
    let peak = r.summary.used.memory;
    assert!(peak < 2 * length / 64, "a peak of {peak} bytes for {length} letters");
    let letters: Vec<i32> = [1, -2, 3].iter().copied().cycle().take(length as usize).collect();
    assert!(equal(r.value.as_ref().expect("a value"), &Value::text(&literal_digest(&letters))));
    let tight = Options::new(Amounts { memory: peak, ..WIDE });
    let r = run_with(&loaded, "go", &tight);
    assert_eq!(r.summary.outcome, Outcome::Completed, "{:?}", r.summary.stop);
}

/// CT-LG4-03 (part: the literal digest under every chunking): the literal digest (KI.13) is the kernel's: the SHA-256
/// of the letters as little-endian 32-bit integers, the same for every chunk size and for every shape of the same
/// letters, so chunking changes no literal digest (LG4.6).
#[test]
fn every_chunking_keeps_the_literal_digest() {
    let letters = [5, -6, 7, 8, -9, 10, 11];
    let mut digests = Vec::new();
    for chunk in ["1", "2", "3", "7", "64"] {
        let loaded = build(&format!(
            "fn s(c: CompressedWord) -> Text effects [] {{\n    let cursor = word_stream(c, {chunk});\n    \
             return stream_digest(cursor);\n}}\n\n\
             export fn flat() -> Text effects [] {{\n    let c = compress({});\n    return s(c);\n}}\n\n\
             export fn joined() -> Text effects [] {{\n    let a = compress({});\n    \
             let b = concat(a, compress({}));\n    return s(concat(b, compress({})));\n}}\n",
            literal(&letters),
            literal(&letters[..2]),
            literal(&letters[2..5]),
            literal(&letters[5..])
        ));
        for entry in ["flat", "joined"] {
            let r = run(&loaded, entry);
            assert_eq!(r.summary.outcome, Outcome::Completed, "{entry}, chunks of {chunk}: {:?}", r.summary.stop);
            digests.push(r.value.expect("a digest"));
        }
    }
    let want = Value::text(&literal_digest(&letters));
    assert!(digests.iter().all(|d| equal(d, &want)), "{digests:?}");
    assert!(!equal(&want, &Value::text(&literal_digest(&letters[1..]))));
    // `stream_digest` is one step that reserves its 64 hex digits and costs no kernel units.
    let r = run(
        &build(&streamer(&letters, "2").replace("return ();", "let d = stream_digest(cursor);\n    return ();")),
        "go",
    );
    let digest = r.summary.rows.iter().find(|row| row.at.ends_with("#stream_digest")).expect("a digest row");
    assert_eq!((digest.steps, digest.kernel_cost, digest.memory), (1, 0, Value::text(&"0".repeat(64)).size()));
}

/// The cursor's chunked reading gives the word's letters in chronological order for every chunk size, as `expand`
/// gives them, for a doubling chain, a word joined to itself and a word whose first part is a concatenation.
#[test]
fn a_chunked_reading_is_the_expansion_in_order() {
    let leaf = |l: &[i32]| Arc::new(Compressed::leaf(l.into()));
    let a = leaf(&[1, 2, 3]);
    let b = Compressed::concat(&a, &leaf(&[4]));
    let c = Compressed::concat(&b, &b);
    let d = Compressed::concat(&c, &leaf(&[5, 6, 7, 8, 9]));
    let e = Compressed::concat(&leaf(&[10]), &d);
    for word in [a, b, c, d, e] {
        let length = usize::try_from(word.length()).expect("a short word");
        let whole = word.expand(&BigInt::from(0), length).expect("the whole word");
        for chunk in 1..=length + 1 {
            let mut reading = word.letters();
            let mut out = Vec::new();
            while out.len() < length {
                let before = out.len();
                reading.next_chunk(chunk, &mut out);
                assert_eq!(out.len() - before, chunk.min(length - before), "a chunk of its declared size");
            }
            reading.next_chunk(chunk, &mut out);
            assert_eq!(out, whole, "chunks of {chunk}");
        }
    }
}

/// An interruption that becomes pending on the `trip`-th check, one check before each operation.
struct Trip {
    seen: Cell<usize>,
    trip: usize,
}

impl Cancel for Trip {
    fn pending(&self) -> Option<Interruption> {
        self.seen.set(self.seen.get() + 1);
        (self.seen.get() >= self.trip).then_some(Interruption::ByUser)
    }
}

/// CT-LG4-03 (part: cancellation at declared boundaries) and CT-DET-30 (part: cancellation boundaries on RT1):
/// cancellation does not wait for full expansion (LG4.6, DET.63): an interruption pending before any operation of the
/// run stops it before that operation, so a stream stops at its open or at a chunk boundary, with every chunk before it
/// complete. The stopped chunk's locator names the coverage: the letters of the completed chunks.
#[test]
fn a_cancelled_stream_stops_at_a_chunk_boundary() {
    let loaded = build(&streamer(&[1, -2, 3, 4, -5, 6, 7], "3"));
    let probe = run(&loaded, "go");
    assert_eq!(probe.summary.outcome, Outcome::Completed);
    let mut stopped_in_stream = Vec::new();
    for trip in 1..=probe.summary.rows.len() {
        let cancel = Trip { seen: Cell::new(0), trip };
        let mut options = Options::new(WIDE);
        options.cancel = Some(&cancel);
        let r = run_with(&loaded, "go", &options);
        assert_eq!(r.summary.outcome, Outcome::Cancelled, "trip {trip}");
        assert_eq!(r.summary.code, Some(meter::code::CANCELLED_BY_USER));
        let stop = r.summary.stop.as_ref().expect("a stop");
        assert_eq!(stop.phase, "interruption");
        assert_eq!(r.summary.rows[..], probe.summary.rows[..trip - 1], "trip {trip}: the operations before it");
        let op = stop.at.rsplit_once('#').map_or("", |(_, op)| op);
        if let Some(position) = op.strip_prefix("chunk:") {
            let rows = stream_rows(&r.summary);
            assert!(rows[1..].iter().all(|row| row.3 == row.4), "complete chunks release their scratch");
            let covered: u128 = rows[1..].iter().map(|row| row.3 / 2).sum();
            assert_eq!(position, covered.to_string(), "the locator names the coverage");
            stopped_in_stream.push(position.to_string());
        }
    }
    assert_eq!(stopped_in_stream, ["0", "3", "6"]);
}

/// The checker's rules for streams (LP2 draft): `word_stream` takes a compressed word and an exact `Nat`, and
/// `stream_digest` a stream; a stream has no equality, no key order and no canonical form, so it never enters a set,
/// a map's keys, a goal or evidence, and a run that would output one stops.
#[test]
fn a_stream_is_typed_and_stays_in_its_run() {
    let refused = codes("export fn f() -> WordStream effects [] {\n    return word_stream(word[+1], 1);\n}\n");
    assert_eq!(refused, ["LG2:type.mismatch"]);
    let refused =
        codes("export fn f() -> WordStream effects [] {\n    return word_stream(compress(word[+1]), \"1\");\n}\n");
    assert_eq!(refused, ["LG2:type.mismatch"]);
    let refused = codes("export fn f() -> Text effects [] {\n    return stream_digest(compress(word[+1]));\n}\n");
    assert_eq!(refused, ["LG2:type.mismatch"]);
    let refused = codes(
        "export fn f() -> Bool effects [] {\n    let s = word_stream(compress(word[+1]), 1);\n    \
         return exact_eq(s, s);\n}\n",
    );
    assert_eq!(refused, ["LG2:type.mismatch"]);
    let source = format!(
        "{HEADER}export fn f() -> Set<WordStream> effects [] {{\n    \
         return set<WordStream>(word_stream(compress(word[+1]), 1));\n}}\n"
    );
    let lowered = lower(&[parse(source.as_bytes())]).expect("lowered");
    let keyed = check(&lowered.program).expect_err("a set of streams");
    assert!(keyed.to_string().contains("LG2.20"), "{keyed}");

    let stream = Value::WordStream(Arc::new(WordStream::new(BigInt::from(1), [0; 32])));
    assert_eq!(encode(&stream, &mut Vec::new()), Err(Noncanonical));
    let loaded = build("export fn f() -> WordStream effects [] {\n    return word_stream(compress(word[+1]), 1);\n}\n");
    let r = run(&loaded, "f");
    assert_eq!(r.summary.code, Some(code::NONCANONICAL_VALUE), "{:?}", r.summary.stop);
}
