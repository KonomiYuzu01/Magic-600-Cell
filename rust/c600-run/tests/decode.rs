//! DET.62's byte decoding at run time (LP2 draft until the IR freeze, PHS.49). `text_decode` turns captured bytes into
//! `Text`: one step with the input-byte tariff on every byte and a reservation of 4 bytes per input byte, settled to
//! the text's 4 bytes per scalar (DET.57). Invalid UTF-8 or a surrogate stops the run with
//! `LG4:input.text_scalar_invalid` in the effect phase before any value is bound, and a decoding error inside a child
//! stops the child alone. These are the decoding rows of the frozen vectors `decode-scalar`, `decode-surrogate` and
//! `child-decode-error-releases-reservation` (DET.65); CT-RTE-04 realises the traces through the runtime. Bytes have
//! an exact equality but no canonical form and no key order.

use c600_check::check;
use c600_run::encode::{encode, Noncanonical};
use c600_run::interp::{code, load, Loaded, NoHost, Options, Run};
use c600_run::meter::{self, Amounts, Outcome, Summary, Tariffs};
use c600_run::value::{equal, Value};
use c600_syntax::{lower, parse};

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

fn run(loaded: &Loaded, function: &str) -> Run {
    loaded.run("m", function, Vec::new(), &mut NoHost, &Options::new(WIDE)).unwrap_or_else(|r| panic!("{r:?}"))
}

/// The rows of one operation as (steps, kernel cost, memory, released).
fn rows(summary: &Summary, op: &str) -> Vec<(u128, u128, u128, u128)> {
    let suffix = format!("#{op}");
    summary
        .rows
        .iter()
        .filter(|r| r.at.ends_with(&suffix))
        .map(|r| (r.steps, r.kernel_cost, r.memory, r.released))
        .collect()
}

/// The text of a completed run.
fn decoded(r: &Run, text: &str) {
    assert_eq!(r.summary.outcome, Outcome::Completed, "{:?} {:?}", r.summary.code, r.summary.stop);
    assert!(equal(r.value.as_ref().expect("a completed run"), &Value::text(text)), "{:?}", r.value);
}

/// `decode-scalar`: four bytes of one scalar are one row, with the tariff on 4 bytes, 16 bytes reserved and 12
/// released, since one scalar holds 4. The reservation is the bound of the input's length in scalars, whatever the
/// result is. Bytes bound with `let` are borrowed, so the row releases the reservation's excess alone; an operand the
/// decoding owns, such as a literal, is released in the same row.
#[test]
fn a_decoding_is_one_row_priced_per_input_byte_and_settled_per_scalar() {
    let per_byte = Tariffs::TEST.decode_per_byte;
    let loaded = build(
        "export fn one() -> Text effects [] {\n    let b = bytes(\"f0908080\");\n    return text_decode(b);\n}\n\n\
         export fn mixed() -> Text effects [] {\n    let b = bytes(\"41C3A9e282ac\");\n    return text_decode(b);\n}\n\n\
         export fn empty() -> Text effects [] {\n    let b = bytes(\"\");\n    return text_decode(b);\n}\n\n\
         export fn owned() -> Text effects [] {\n    return text_decode(bytes(\"f0908080\"));\n}\n",
    );
    let r = run(&loaded, "one");
    decoded(&r, "\u{10000}");
    assert_eq!(rows(&r.summary, "text_decode"), [(1, 4 * per_byte, 16, 12)]);
    // "A", "é" and "€": 6 bytes and 3 scalars.
    let r = run(&loaded, "mixed");
    decoded(&r, "A\u{e9}\u{20ac}");
    assert_eq!(rows(&r.summary, "text_decode"), [(1, 6 * per_byte, 24, 12)]);
    let r = run(&loaded, "empty");
    decoded(&r, "");
    assert_eq!(rows(&r.summary, "text_decode"), [(1, 0, 0, 0)]);
    // The literal's four bytes go with the reservation's excess.
    let r = run(&loaded, "owned");
    decoded(&r, "\u{10000}");
    assert_eq!(rows(&r.summary, "text_decode"), [(1, 4 * per_byte, 16, 12 + 4)]);
}

/// `decode-surrogate`: an encoded surrogate, like any invalid UTF-8, stops the run in the effect phase after its row
/// is admitted, with no value bound; the reservation stays live until cleanup, which returns it.
#[test]
fn invalid_utf8_or_a_surrogate_stops_before_any_value_is_bound() {
    let per_byte = Tariffs::TEST.decode_per_byte;
    // A surrogate, a lone continuation byte, a byte that never starts a scalar, an overlong form, a truncated scalar
    // and a value past U+10FFFF.
    for hex in ["eda080", "80", "ff", "c0af", "f09080", "f4908080"] {
        let loaded =
            build(&format!("export fn d() -> Text effects [] {{\n    return text_decode(bytes(\"{hex}\"));\n}}\n"));
        let r = run(&loaded, "d");
        let n = hex.len() as u128 / 2;
        let refused = (Outcome::Error, Some(meter::code::TEXT_SCALAR_INVALID));
        assert_eq!((r.summary.outcome, r.summary.code), refused, "{hex}");
        let stop = r.summary.stop.as_ref().expect("a stop");
        assert!(stop.at.ends_with("#text_decode") && stop.phase == "effect", "{hex}: {stop:?}");
        assert!(r.value.is_none(), "{hex}");
        assert_eq!(rows(&r.summary, "text_decode"), [(1, n * per_byte, 4 * n, 0)], "{hex}");
        assert!(r.summary.live_before_cleanup >= 4 * n, "{hex}");
        assert_eq!(r.summary.live_after_cleanup, 0, "{hex}");
    }
}

/// A run that cannot pay for a decoding stops before it reads a byte: the row is admitted first (DET.56).
#[test]
fn a_decoding_beyond_its_budget_stops_before_any_byte_is_read() {
    let loaded = build("export fn d() -> Text effects [] {\n    return text_decode(bytes(\"eda080\"));\n}\n");
    let whole = run(&loaded, "d").summary.used;
    let tight = Amounts { kernel_cost: whole.kernel_cost - 1, ..WIDE };
    let r = loaded.run("m", "d", Vec::new(), &mut NoHost, &Options::new(tight)).unwrap();
    // The surrogate would be an error, but the cut-off comes first.
    assert_eq!((r.summary.outcome, r.summary.code), (Outcome::CutOff, Some(meter::code::KERNEL_COST)));
    assert!(r.summary.stop.as_ref().unwrap().at.ends_with("#text_decode"));
    assert_eq!(r.summary.used.kernel_cost + 3 * Tariffs::TEST.decode_per_byte, whole.kernel_cost);
}

/// `child-decode-error-releases-reservation`: a decoding error inside a child stops the child alone, in the effect
/// phase at its decoding; the parent takes the alternative and completes, and cleanup leaves nothing live.
#[test]
fn a_child_decoding_error_stops_the_child_alone() {
    let loaded = build(
        "export fn c() -> Text effects [budget] {\n    \
         return child(record { steps = 200, kernel_cost = 200, memory = 200, output = 0 }, \
         fn() -> Text { return text_decode(bytes(\"ff\")); }, fn() -> Text { return \"none\"; });\n}\n",
    );
    let r = run(&loaded, "c");
    decoded(&r, "none");
    let [child] = r.summary.children.as_slice() else { panic!("one child: {:?}", r.summary.children) };
    assert_eq!((child.outcome, child.code), (Outcome::Error, Some(meter::code::TEXT_SCALAR_INVALID)));
    let stop = child.stop.as_ref().expect("the child's stop");
    assert!(stop.at.ends_with("#text_decode") && stop.phase == "effect", "{stop:?}");
    assert_eq!(rows(&r.summary, "text_decode"), [(1, Tariffs::TEST.decode_per_byte, 4, 0)]);
    assert_eq!(r.summary.live_after_cleanup, 0);
}

/// `text_decode_hex` decodes bytes the code holds (LP2 draft; DET.57): they are part of the operation, so its one row is
/// all it charges, with no literal row and no slot for an operand, and it is priced and refused as `text_decode` is.
/// In a child its body's rows are its call, whose frame has no slot, and its decoding: the two rows of the child in
/// `child-decode-error-releases-reservation`, whose stop leaves nothing live in the parent.
#[test]
fn bytes_the_code_holds_decode_in_one_row() {
    let per_byte = Tariffs::TEST.decode_per_byte;
    let loaded = build(
        "export fn held() -> Text effects [] {\n    return text_decode_hex(\"f0908080\");\n}\n\n\
         export fn bad() -> Text effects [] {\n    return text_decode_hex(\"eda080\");\n}\n\n\
         export fn c() -> Text effects [budget] {\n    \
         return child(record { steps = 20, kernel_cost = 20, memory = 20, output = 0 }, \
         fn() -> Text { return text_decode_hex(\"ff\"); }, fn() -> Text { return \"none\"; });\n}\n",
    );
    let r = run(&loaded, "held");
    decoded(&r, "\u{10000}");
    assert_eq!(rows(&r.summary, "text_decode"), [(1, 4 * per_byte, 16, 12)]);
    assert!(rows(&r.summary, "value").is_empty(), "no literal row");
    assert_eq!(loaded.frame_size(loaded.function("m", "held").expect("held")), 16, "no slot");
    let r = run(&loaded, "bad");
    assert_eq!((r.summary.outcome, r.summary.code), (Outcome::Error, Some(meter::code::TEXT_SCALAR_INVALID)));
    assert_eq!(rows(&r.summary, "text_decode"), [(1, 3 * per_byte, 12, 0)]);
    assert_eq!(r.summary.live_after_cleanup, 0);
    let r = run(&loaded, "c");
    decoded(&r, "none");
    let [child] = r.summary.children.as_slice() else { panic!("one child: {:?}", r.summary.children) };
    assert_eq!((child.outcome, child.code), (Outcome::Error, Some(meter::code::TEXT_SCALAR_INVALID)));
    assert_eq!((child.used.steps, child.used.kernel_cost, child.used.memory), (2, per_byte, 16 + 4));
    assert_eq!((child.returned_memory, child.live_after_return), (0, 0));
    for odd in ["0", "zz"] {
        let source = format!("{HEADER}export fn b() -> Text effects [] {{\n    return text_decode_hex(\"{odd}\");\n}}\n");
        assert!(lower(&[parse(source.as_bytes())]).is_err(), "{odd}");
    }
}

/// Bytes have an exact equality but no canonical form, so no output, goal or evidence holds them, and no key order,
/// so no set element or map key does; `text_decode` takes bytes only, and a `bytes` literal is hex digit pairs.
#[test]
fn bytes_have_no_canonical_form_or_key_and_only_bytes_decode() {
    let b = Value::Bytes(vec![0xf0, 0x90].into());
    assert_eq!((b.size(), b.bound()), (2, 0));
    assert_eq!(encode(&b, &mut Vec::new()), Err(Noncanonical));
    assert!(equal(&b, &Value::Bytes(vec![0xf0, 0x90].into())));
    assert!(!equal(&b, &Value::Bytes(vec![0xf0].into())));
    let loaded = build("export fn out() -> Bytes effects [] {\n    return bytes(\"00\");\n}\n");
    let r = run(&loaded, "out");
    assert_eq!((r.summary.outcome, r.summary.code), (Outcome::Error, Some(code::NONCANONICAL_VALUE)));
    let refused = codes("export fn d() -> Text effects [] {\n    return text_decode(\"abc\");\n}\n");
    assert_eq!(refused, ["LG2:type.mismatch"]);
    // A set of bytes is not even a well-formed type (LG2.20).
    let source =
        format!("{HEADER}export fn s() -> Set<Bytes> effects [] {{\n    return set<Bytes>(bytes(\"00\"));\n}}\n");
    let lowered = lower(&[parse(source.as_bytes())]).expect("lowered");
    let keyed = check(&lowered.program).expect_err("a set of bytes");
    assert!(keyed.to_string().contains("LG2.20"), "{keyed}");
    for odd in ["0", "zz", "0g"] {
        let source = format!("{HEADER}export fn b() -> Bytes effects [] {{\n    return bytes(\"{odd}\");\n}}\n");
        assert!(lower(&[parse(source.as_bytes())]).is_err(), "{odd}");
    }
}
