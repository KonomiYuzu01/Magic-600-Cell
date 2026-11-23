//! CT-EV1-05 (whole; synthetic): EV1.10. Multi-unit characters, insertion ranges and end positions round-trip through
//! the coordinate convention: UTF-8 byte offsets on scalar boundaries, converted from and to a host's declared `utf8`,
//! `utf16` or `scalar` units without rounding. Malformed UTF-8 (`61 FF 62` and the truncated `61 F0 9F 98`) uses
//! ERR.10's raw context without a span or a host conversion; even `[0, 0)` is no valid source range for either input.

use c600_editor::{
    code, complete, diagnose, parse, Document, HostRange, ModuleContext, NextAction, Reason, Service, SourceRange,
    Units,
};

/// EV1.10's scalars `a`, U+1F600, U+00E9, LF, `z`.
const SAMPLE: &str = "a\u{1F600}\u{E9}\nz";
/// EV1.10's boundary triples `(utf8, utf16, scalar)`.
const TRIPLES: [(usize, usize, usize); 6] = [(0, 0, 0), (1, 1, 1), (5, 3, 2), (7, 4, 3), (8, 5, 4), (9, 6, 5)];

fn host(triple: (usize, usize, usize), units: Units) -> usize {
    match units {
        Units::Utf8 => triple.0,
        Units::Utf16 => triple.1,
        Units::Scalar => triple.2,
    }
}

/// CT-EV1-05 (the boundary triples): every boundary converts to its byte offset from each declared unit and back, and
/// every pair of boundaries round-trips as a range, insertions included.
#[test]
fn ct_ev1_05_boundary_triples_round_trip() {
    assert_eq!(SAMPLE.len(), 9);
    for units in Units::ALL {
        let doc = Document::open(SAMPLE, units);
        for t in TRIPLES {
            assert_eq!(doc.offset(doc.digest(), host(t, units)).unwrap(), t.0, "{units:?} {t:?}");
        }
        for a in TRIPLES {
            for b in TRIPLES.iter().filter(|b| b.0 >= a.0) {
                let range = doc.range(doc.digest(), host(a, units), host(*b, units)).unwrap();
                assert_eq!((range.start, range.end, range.digest.as_str()), (a.0, b.0, doc.digest()));
                let back = doc.host(&range).unwrap();
                assert_eq!(back, HostRange { start: host(a, units), end: host(*b, units), units });
            }
        }
    }
}

/// CT-EV1-05 (EV1.10's examples): UTF-8 `[1, 5)` selects U+1F600 and is UTF-16 `[1, 3)`; `[5, 5)` inserts after it at
/// `[3, 3)`; `[9, 9)` inserts at the end, `[6, 6)`; UTF-8 offset 2 and UTF-16 offset 2 refuse.
#[test]
fn ct_ev1_05_the_examples_of_ev1_10() {
    let utf8 = Document::open(SAMPLE, Units::Utf8);
    let utf16 = Document::open(SAMPLE, Units::Utf16);
    let emoji = SourceRange::new(utf8.digest(), 1, 5);
    assert_eq!(&SAMPLE[emoji.start..emoji.end], "\u{1F600}");
    assert_eq!(utf16.host(&emoji).unwrap(), HostRange { start: 1, end: 3, units: Units::Utf16 });
    let after = SourceRange::new(utf8.digest(), 5, 5);
    assert!(after.is_insertion());
    assert_eq!(utf16.host(&after).unwrap(), HostRange { start: 3, end: 3, units: Units::Utf16 });
    let end = SourceRange::new(utf8.digest(), 9, 9);
    assert_eq!(utf16.host(&end).unwrap(), HostRange { start: 6, end: 6, units: Units::Utf16 });
    assert_eq!(utf16.range(utf16.digest(), 1, 3).unwrap(), emoji);

    for (doc, units) in [(&utf8, Units::Utf8), (&utf16, Units::Utf16)] {
        let refused = doc.offset(doc.digest(), 2).unwrap_err();
        assert_eq!(refused.code, code::SOURCE_RANGE_INVALID);
        assert_eq!(refused.reason, Reason::InsideScalar { offset: 2, units });
        assert_eq!(refused.next, NextAction::ChoosePosition);
    }
    let inside = SourceRange::new(utf8.digest(), 2, 5);
    assert_eq!(utf16.host(&inside).unwrap_err().reason, Reason::InsideScalar { offset: 2, units: Units::Utf8 });
    // Scalar 2 is UTF-8 5; no scalar position falls inside a scalar, but past the end is refused.
    let scalar = Document::open(SAMPLE, Units::Scalar);
    assert_eq!(scalar.offset(scalar.digest(), 2).unwrap(), 5);
    let past = scalar.offset(scalar.digest(), 6).unwrap_err();
    assert_eq!(past.reason, Reason::OutOfBounds { offset: 6, units: Units::Scalar, length: 5 });
}

/// CT-EV1-05 (invalid endpoints and units): a reversed range, an end past the bytes, units other than the declared
/// ones and units the protocol does not know are `EV1:input.source_range_invalid`; another digest is
/// `EV1:stale.editor_source`.
#[test]
fn ct_ev1_05_invalid_endpoints_units_and_digests_refuse() {
    let doc = Document::open(SAMPLE, Units::Utf16);
    assert_eq!(doc.range(doc.digest(), 3, 1).unwrap_err().reason, Reason::Reversed { start: 3, end: 1 });
    let past = SourceRange::new(doc.digest(), 0, 10);
    assert!(matches!(doc.validate(&past).unwrap_err().reason, Reason::OutOfBounds { offset: 10, .. }));
    let undeclared = doc.host_in(&SourceRange::new(doc.digest(), 1, 5), Units::Scalar).unwrap_err();
    assert_eq!(undeclared.code, code::SOURCE_RANGE_INVALID);
    assert_eq!(undeclared.reason, Reason::UndeclaredUnits { declared: Some(Units::Utf16), requested: "scalar".into() });
    let unknown = Document::open_declared(SAMPLE, "utf32").unwrap_err();
    assert_eq!((unknown.code, unknown.service), (code::SOURCE_RANGE_INVALID, Service::Open));
    assert_eq!(unknown.reason, Reason::UndeclaredUnits { declared: None, requested: "utf32".into() });
    assert_eq!(Document::open_declared(SAMPLE, "scalar").unwrap().units(), Units::Scalar);
    let stale = doc.host(&SourceRange::new("0".repeat(64), 1, 5)).unwrap_err();
    assert_eq!(stale.code, code::STALE_EDITOR_SOURCE);
    // A CRLF buffer is not the pinned LF document: it has no range, so a host maps its coordinates first.
    let crlf = Document::open("a\r\nz", Units::Utf16);
    let refused = crlf.offset(crlf.digest(), 0).unwrap_err();
    assert_eq!(refused.code, code::SOURCE_RANGE_INVALID);
    assert!(matches!(refused.reason, Reason::NotPinned { .. }));
}

/// CT-EV1-05 (malformed UTF-8): `61 FF 62` and `61 F0 9F 98` have no source range, not even `[0, 0)`, in any units;
/// parse and diagnose give ERR.10's encoding diagnostic with no span and the raw context: the digest of the raw bytes
/// and the offending interval, never converted to host units.
#[test]
fn ct_ev1_05_malformed_utf8_has_no_source_range() {
    let inputs: [(&[u8], usize, usize); 2] = [(&[0x61, 0xFF, 0x62], 1, 2), (&[0x61, 0xF0, 0x9F, 0x98], 1, 4)];
    for (bytes, start, end) in inputs {
        for units in Units::ALL {
            let doc = Document::open(bytes.to_vec(), units);
            assert!(doc.text().is_none());
            for refused in [
                doc.range(doc.digest(), 0, 0).unwrap_err(),
                doc.offset(doc.digest(), 0).unwrap_err(),
                doc.validate(&SourceRange::new(doc.digest(), 0, 0)).unwrap_err(),
                doc.host(&SourceRange::new(doc.digest(), 0, 0)).unwrap_err(),
                complete(&doc, doc.digest(), 0, &ModuleContext::new()).unwrap_err(),
            ] {
                assert_eq!(refused.code, code::SOURCE_RANGE_INVALID, "{bytes:?}");
                let Reason::NotPinned { raw } = &refused.reason else { panic!("{refused:?}") };
                assert_eq!((raw.digest.as_str(), raw.start, raw.end), (doc.digest(), start, end));
                assert!(refused.span.is_none());
            }
            let parsed = parse(&doc, c600_syntax::EDITION);
            assert_eq!(parsed.diagnostics.len(), 1);
            let e = &parsed.diagnostics[0];
            assert_eq!((e.code, e.span.is_none()), (code::INVALID_ENCODING, true));
            let Reason::Encoding { raw } = &e.reason else { panic!("{e:?}") };
            assert_eq!(
                (raw.digest.as_str(), raw.start, raw.end),
                (c600_syntax::sha256_hex(bytes).as_str(), start, end)
            );
            assert!(!parsed.subject.pinned);
            let d = diagnose(&doc, doc.digest(), &ModuleContext::new()).unwrap();
            assert_eq!((d.diagnostics.len(), d.diagnostics[0].code), (1, code::INVALID_ENCODING));
            assert!(d.diagnostics[0].span.is_none());
        }
    }
}
