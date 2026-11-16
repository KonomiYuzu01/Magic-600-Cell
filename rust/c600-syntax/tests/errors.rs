//! CT-ERR-05's encoding half beyond the two ERR.12 examples in `syntax.rs`: ERR.10's raw context for each shape of
//! malformed input, bound to the bytes as received, with no replacement character, no validated source digest and no
//! program (chapter 13, ERR.10; LG1.3).

use c600_syntax::code::INVALID_ENCODING;
use c600_syntax::{lower, parse, sha256_hex};

/// CT-LG1-02's canonical fixture: valid pinned bytes.
const BASE: &[u8] = include_bytes!("fixtures/canonical.6cs");

/// Asserts that `bytes` are refused with ERR.10's encoding diagnostic alone, over the byte interval `[start, end)`.
fn refused(bytes: &[u8], start: usize, end: usize, case: &str) {
    let file = parse(bytes);
    let codes: Vec<&str> = file.diagnostics.iter().map(|d| d.code).collect();
    assert_eq!(codes, [INVALID_ENCODING], "{case}");
    let d = &file.diagnostics[0];
    assert_eq!(d.span, None, "{case}: an encoding diagnostic has no span");
    let raw = d.raw.as_ref().expect("an encoding diagnostic has a raw context");
    assert_eq!((raw.start, raw.end), (start, end), "{case}");
    assert!(raw.start < raw.end && raw.end <= bytes.len(), "{case}: the interval is nonempty and inside the input");
    assert_eq!(raw.digest, sha256_hex(bytes), "{case}: the digest binds the bytes as received");
    assert!(!d.message.contains('\u{fffd}'), "{case}: no replacement character stands for the bytes");
    assert_eq!(file.digest, None, "{case}: refused bytes have no validated source digest");
    assert!(
        file.edition.is_none() && file.module.is_none() && file.items.is_empty() && file.holes.is_empty(),
        "{case}: nothing is read from refused bytes"
    );
    assert!(lower(std::slice::from_ref(&file)).is_err(), "{case}: refused bytes never lower");
}

/// CT-ERR-05 (part: the LG1 encoding half; not run: the V06 envelope that carries the diagnostic, which needs LP3's
/// command layer, and EV1's active version, which invalid draft bytes must not replace, which needs LP4's editor
/// services): every shape of malformed UTF-8, and a byte-order mark or CR before LF pinning, gives one
/// `LG1:syntax.invalid_encoding` diagnostic with no span, the SHA-256 of the raw input and the first offending byte
/// interval, nonempty and inside the input. A sequence cut by the end of the input runs to the end.
#[test]
fn ct_err_05_every_malformed_shape_keeps_the_raw_context() {
    let cases: [(&str, &[u8], usize, usize); 12] = [
        ("lone continuation byte", &[0x80], 0, 1),
        ("byte never in UTF-8", &[0x61, 0xf5, 0x62], 1, 2),
        ("overlong encoding", &[0x61, 0xc0, 0xaf], 1, 2),
        ("UTF-16 surrogate", &[0xed, 0xa0, 0x80], 0, 1),
        ("scalar above U+10FFFF", &[0xf4, 0x90, 0x80, 0x80], 0, 1),
        ("sequence cut by an ASCII byte", &[0xe2, 0x82, 0x41], 0, 2),
        ("sequence cut by the end of the input", &[0xe2, 0x82], 0, 2),
        ("the first of two malformed sequences", &[0x61, 0xff, 0x62, 0xfe], 1, 2),
        ("malformed byte after a byte-order mark", &[0xef, 0xbb, 0xbf, 0xff], 3, 4),
        ("malformed byte after a CR", &[0x61, 0x0d, 0x0a, 0xff], 3, 4),
        ("byte-order mark", &[0xef, 0xbb, 0xbf, 0x61], 0, 3),
        ("CR before LF pinning", &[0x61, 0x0d, 0x0a, 0x62], 1, 2),
    ];
    for (case, bytes, start, end) in cases {
        refused(bytes, start, end, case);
    }
}

/// CT-ERR-05 (part: the LG1 encoding half; not run: as above): the canonical program parses clean, and one corrupted
/// scalar inside it, an invalid byte or a truncated four-byte sequence, refuses the whole file at that scalar's bytes,
/// keeps no declaration of the valid rest and does not lower.
#[test]
fn ct_err_05_a_corrupted_program_keeps_nothing() {
    let clean = parse(BASE);
    assert!(clean.diagnostics.is_empty() && clean.digest.is_some() && !clean.items.is_empty());
    let at = BASE.windows(5).position(|w| w == b"\"red\"").expect("the fixture has a string literal") + 2;
    for (case, bad, end) in
        [("invalid byte", &[0xff][..], at + 1), ("truncated scalar", &[0xf0, 0x9f, 0x98][..], at + 3)]
    {
        let mut bytes = BASE.to_vec();
        bytes.splice(at..at + 1, bad.iter().copied());
        refused(&bytes, at, end, case);
    }
}
