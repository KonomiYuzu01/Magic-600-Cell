//! CT-EV3-05 (whole; source): EV3.7. Declared parameter, layout and default edits recheck source; an arbitrary drawn
//! part cannot be round-tripped.
//!
//! The round trip runs on the three built-in tools' sources as they ship (`full_600_cell.6cs`, `representations.6cs`
//! and `jumble_checker.6cs`, read from their crates and never changed here). As shipped they declare no editable part,
//! so every field of every declaration, their entry points' declared parameters included, is refused. Each tool's
//! fork, its shipped bytes with a declared parameter, layout and default appended under the draft directive
//! (`rust/README.md`, LP4-7), round-trips each declared field: the edit replaces exactly the field's literal, the
//! edited bytes are the answer's new digest, read back with the proposed value, and are checked again. A drawn view, a
//! computed value, an undeclared field and a missing directive are `EV3:input.round_trip_undeclared`; a value of
//! another type is `LG2:type.mismatch`; a request for other bytes is `EV1:stale.editor_source`.

mod support;

use c600_editor::{
    apply_edits, code, declared_parts, diagnose, edit_declared_part, Document, Envelope, Form, ModuleContext,
    NextAction, PartKind, PartValue, Reason, Service, Undeclared, Units, Verdict, WHOLE,
};
use support::{full, open};

const TOOLS: [(&str, &str); 3] = [
    ("full_600_cell", include_str!("../../c600-tools/src/full_600_cell.6cs")),
    ("representations", include_str!("../../c600-repr/src/representations.6cs")),
    ("jumble_checker", include_str!("../../c600-jumble-tool/src/jumble_checker.6cs")),
];

/// The parts a fork of a built-in tool declares: one of each kind, a nested layout field, a field the directive does
/// not list, and a directive on a computed value.
const PARTS: &str = "
# @editable parameter
fn default_layer() -> Nat effects [] {
    return 2;
}

# A layout: only `columns` and `title.size` are declared editable.
# @editable layout columns title.size
fn filter_layout() -> record { columns: Nat, order: Text, title: record { size: Nat, text: Text } } effects [] {
    return record { columns = 3, order = \"cell\", title = record { size = 12, text = \"Filter\" } };
}

# @editable default
fn default_scope() -> Text effects [] {
    return \"cell\";
}

# @editable parameter
fn drawn_size() -> Nat effects [] {
    return exact_add(1, 2);
}
";

fn fork(source: &str) -> String {
    format!("{source}{PARTS}")
}

fn int(n: &str) -> PartValue {
    PartValue::Int(n.to_owned())
}

fn text(s: &str) -> PartValue {
    PartValue::Text(s.to_owned())
}

/// The refusal's reason, after checking its code and envelope.
fn undeclared(e: Envelope) -> Undeclared {
    assert_eq!(e.code, code::ROUND_TRIP_UNDECLARED);
    assert_eq!(e.code, "EV3:input.round_trip_undeclared");
    assert_eq!((e.form, e.service, &e.next), (Form::Refusal, Service::EditDeclaredPart, &NextAction::EditSource));
    match e.reason {
        Reason::RoundTripUndeclared { why, .. } => why,
        other => panic!("{other:?}"),
    }
}

#[test]
fn ct_ev3_05_the_shipped_tools_declare_no_editable_part() {
    let context = ModuleContext::new();
    for (name, source) in TOOLS {
        let doc = open(source);
        assert_eq!(declared_parts(&doc, doc.digest()).unwrap(), vec![], "{name}");
        let file = c600_syntax::parse(source.as_bytes());
        let mut asked = 0;
        for decl in file.declarations() {
            let fields = decl.params.iter().map(|p| p.name.text.clone()).chain([WHOLE.to_owned()]);
            for field in fields {
                let refused = edit_declared_part(&doc, doc.digest(), &decl.name.text, &field, &int("1"), &context);
                assert_eq!(undeclared(refused.unwrap_err()), Undeclared::NotEditable, "{name}.{}", decl.name.text);
                asked += 1;
            }
        }
        assert!(asked >= 2, "{name}");
        // The source is unchanged: the document still names the shipped bytes.
        assert_eq!(doc.bytes(), source.as_bytes());
    }
}

#[test]
fn ct_ev3_05_declared_parameter_layout_and_default_edits_recheck() {
    let context = ModuleContext::new();
    let cases: [(&str, &str, PartKind, PartValue); 4] = [
        ("default_layer", WHOLE, PartKind::Parameter, int("5")),
        ("filter_layout", "columns", PartKind::Layout, int("4")),
        ("filter_layout", "title.size", PartKind::Layout, int("18")),
        ("default_scope", WHOLE, PartKind::Default, text("layer \"2\"\n\\ ✓")),
    ];
    for (name, shipped) in TOOLS {
        let source = fork(shipped);
        let doc = open(&source);
        let before = diagnose(&doc, doc.digest(), &context).unwrap();
        assert!(matches!(before.verdict, Verdict::Checked(_)), "{name}: {:#?}", before.diagnostics);

        let parts = declared_parts(&doc, doc.digest()).unwrap();
        let listed: Vec<_> = parts.iter().map(|p| (p.declaration.as_str(), p.kind)).collect();
        assert_eq!(
            listed,
            [
                ("default_layer", PartKind::Parameter),
                ("filter_layout", PartKind::Layout),
                ("default_scope", PartKind::Default)
            ],
            "{name}: the computed `drawn_size` is no declared part"
        );
        let layout = &parts[1];
        let fields: Vec<_> = layout.fields.iter().map(|f| (f.path.as_str(), f.ty.as_str(), f.value.clone())).collect();
        assert_eq!(fields, [("columns", "Nat", int("3")), ("title.size", "Nat", int("12"))]);

        for (declaration, field, kind, value) in &cases {
            let trip = edit_declared_part(&doc, doc.digest(), declaration, field, value, &context).unwrap();
            assert_eq!((trip.kind, trip.field.as_str()), (*kind, *field));
            assert!(trip.subject.names(&doc), "the answer names the bytes it was asked about");
            // One edit, exactly over the field's literal, under the request's digest.
            assert_eq!(trip.edits.len(), 1);
            let edit = &trip.edits[0];
            assert_eq!(edit.range.digest, doc.digest());
            let declared = parts.iter().find(|p| p.declaration == *declaration).unwrap();
            let old = declared.fields.iter().find(|f| f.path == *field).unwrap();
            assert_eq!(edit.range, old.range);
            assert_eq!(edit.text, value.literal());

            // The edited bytes are the new version the answer names; the old document is unchanged.
            let edited = apply_edits(&doc, &trip.edits).unwrap();
            assert_eq!(c600_syntax::sha256_hex(&edited), trip.source_digest);
            assert_ne!(trip.source_digest, doc.digest());
            assert_eq!(doc.bytes(), source.as_bytes());

            // Read back: the field holds the value, and every other field keeps its own.
            let after = Document::open(edited.clone(), Units::Utf8);
            let reread = declared_parts(&after, after.digest()).unwrap();
            assert_eq!(reread.len(), parts.len());
            for (p, q) in parts.iter().zip(&reread) {
                for (f, g) in p.fields.iter().zip(&q.fields) {
                    let expected = if p.declaration == *declaration && f.path == *field { value } else { &f.value };
                    assert_eq!(&g.value, expected, "{name}: {}.{}", p.declaration, f.path);
                }
            }

            // Checked again: the same verdict a fresh diagnosis of the edited bytes gives.
            assert!(trip.diagnostics.is_empty(), "{name}: {:#?}", trip.diagnostics);
            let Verdict::Checked(candidate) = &trip.verdict else { panic!("{name}: {:?}", trip.verdict) };
            assert_eq!(candidate.source, trip.source_digest);
            let fresh = diagnose(&after, after.digest(), &context).unwrap();
            assert_eq!(fresh.verdict, trip.verdict);
        }
    }
}

#[test]
fn ct_ev3_05_the_recheck_reports_what_the_edited_draft_fails() {
    // A session identity the fork's manifest does not claim: the edit is still proposed, and its check refuses the
    // draft with KN1's code, as diagnose does for the edited bytes.
    let (_, shipped) = TOOLS[1];
    let source = fork(shipped);
    let doc = open(&source);
    let mut other = full();
    other.model_id = "4".repeat(64);
    let context = ModuleContext::new().with_session(full(), vec![other]);
    let trip = edit_declared_part(&doc, doc.digest(), "default_layer", WHOLE, &int("7"), &context).unwrap();
    assert_eq!(trip.edits.len(), 1);
    assert_eq!(trip.verdict, Verdict::Refused);
    assert!(trip.diagnostics.iter().any(|e| e.code == code::IDENTITY_MISMATCH), "{:#?}", trip.diagnostics);
    let edited = apply_edits(&doc, &trip.edits).unwrap();
    let after = Document::open(edited, Units::Utf8);
    let fresh = diagnose(&after, after.digest(), &context).unwrap();
    let codes = |d: &[Envelope]| d.iter().map(|e| e.code).collect::<Vec<_>>();
    assert_eq!(codes(&fresh.diagnostics), codes(&trip.diagnostics));
}

#[test]
fn ct_ev3_05_an_arbitrary_drawn_part_cannot_be_round_tripped() {
    let context = ModuleContext::new();
    let (_, shipped) = TOOLS[1];
    let source = fork(shipped);
    let doc = open(&source);
    let ask = |declaration: &str, field: &str, value: PartValue| {
        undeclared(edit_declared_part(&doc, doc.digest(), declaration, field, &value, &context).unwrap_err())
    };
    // A drawn view and its predicate: no directive.
    assert_eq!(ask("filter_counts", "predicate", int("1")), Undeclared::NotEditable);
    assert_eq!(ask("filter_counts", WHOLE, int("1")), Undeclared::NotEditable);
    // A computed value under a directive is never inverted into code.
    assert_eq!(ask("drawn_size", WHOLE, int("4")), Undeclared::NotLiteral);
    // Fields the directive does not list, and fields that do not exist.
    let declared = vec!["columns".to_owned(), "title.size".to_owned()];
    for field in ["order", "title.text", "title", WHOLE, "rows"] {
        assert_eq!(ask("filter_layout", field, text("x")), Undeclared::FieldNotDeclared { declared: declared.clone() });
    }
    assert_eq!(
        ask("default_layer", "value.size", int("1")),
        Undeclared::FieldNotDeclared { declared: vec![WHOLE.to_owned()] }
    );
    // No such declaration.
    assert_eq!(ask("missing", WHOLE, int("1")), Undeclared::NoDeclaration);

    // A directive on a view, an unknown kind, a malformed path and two directives.
    let edge = format!(
        "{shipped}
# @editable layout
export view drawn(n: Nat) -> Unit effects [view] {{
    return publish_view(n);
}}

# @editable colour
fn tint() -> Nat effects [] {{
    return 1;
}}

# @editable parameter 3d
fn depth() -> Nat effects [] {{
    return 1;
}}

# @editable parameter
# @editable default
fn twice() -> Nat effects [] {{
    return 1;
}}

# @editable parameter
fn bound() -> Nat effects [] {{
    let n = 1;
    return n;
}}
"
    );
    let doc = open(&edge);
    let ask = |declaration: &str| {
        undeclared(edit_declared_part(&doc, doc.digest(), declaration, WHOLE, &int("2"), &context).unwrap_err())
    };
    assert_eq!(ask("drawn"), Undeclared::NotLiteral);
    assert_eq!(ask("tint"), Undeclared::Malformed);
    assert_eq!(ask("depth"), Undeclared::Malformed);
    assert_eq!(ask("twice"), Undeclared::Malformed);
    assert_eq!(ask("bound"), Undeclared::NotLiteral);
    assert_eq!(declared_parts(&doc, doc.digest()).unwrap(), vec![]);
}

#[test]
fn ct_ev3_05_a_value_of_another_type_is_a_type_mismatch() {
    let context = ModuleContext::new();
    let (_, shipped) = TOOLS[0];
    let source = fork(shipped);
    let doc = open(&source);
    let cases: [(&str, &str, PartValue, &str, &str); 8] = [
        ("default_layer", WHOLE, int("-1"), "Nat", "Int"),
        ("default_scope", WHOLE, PartValue::Word(vec![1201]), "Text", "malformed Word"),
        ("default_layer", WHOLE, PartValue::Rat { num: "1".into(), den: "0".into() }, "Nat", "malformed Rat"),
        ("default_layer", WHOLE, text("2"), "Nat", "Text"),
        ("default_layer", WHOLE, int("02"), "Nat", "malformed Int"),
        ("filter_layout", "columns", PartValue::Bool(true), "Nat", "Bool"),
        ("default_scope", WHOLE, PartValue::Word(vec![1, -2]), "Text", "Word"),
        ("filter_layout", "title.size", PartValue::Record(vec![]), "Nat", "record"),
    ];
    for (declaration, field, value, expected, supplied) in cases {
        let e = edit_declared_part(&doc, doc.digest(), declaration, field, &value, &context).unwrap_err();
        assert_eq!(e.code, code::TYPE_MISMATCH);
        assert_eq!(e.code, "LG2:type.mismatch");
        assert_eq!((e.form, e.service), (Form::Refusal, Service::EditDeclaredPart));
        assert_eq!(e.next, NextAction::SupplyValue { expected: expected.to_owned() });
        let Reason::ValueType { declaration: d, field: f, expected: x, supplied: s } = &e.reason else {
            panic!("{:?}", e.reason)
        };
        assert_eq!((d.as_str(), f.as_str(), x.as_str(), s.as_str()), (declaration, field, expected, supplied));
        // Like every refusal, it has no span; the field it names is a declared one, with its range.
        assert_eq!(e.span, None);
        let parts = declared_parts(&doc, doc.digest()).unwrap();
        let part = parts.iter().find(|p| p.declaration == declaration).unwrap();
        assert!(part.fields.iter().any(|g| g.path == field));
    }
    // A whole-record value with the declared fields in order conforms; with another order it does not.
    let layout = "# @editable layout\nfn grid() -> record { columns: Nat, rows: Nat } effects [] {\n    \
                  return record { columns = 1, rows = 1 };\n}\n";
    let source = format!("{shipped}\n{layout}");
    let doc = open(&source);
    let good = PartValue::Record(vec![("columns".into(), int("2")), ("rows".into(), int("3"))]);
    let trip = edit_declared_part(&doc, doc.digest(), "grid", WHOLE, &good, &context).unwrap();
    assert_eq!(trip.edits[0].text, "record { columns = 2, rows = 3 }");
    assert!(matches!(trip.verdict, Verdict::Checked(_)), "{:#?}", trip.diagnostics);
    let swapped = PartValue::Record(vec![("rows".into(), int("3")), ("columns".into(), int("2"))]);
    let e = edit_declared_part(&doc, doc.digest(), "grid", WHOLE, &swapped, &context).unwrap_err();
    assert_eq!(e.code, code::TYPE_MISMATCH);
}

#[test]
fn ct_ev3_05_a_request_for_other_bytes_is_stale() {
    let context = ModuleContext::new();
    let (_, shipped) = TOOLS[2];
    let source = fork(shipped);
    let doc = open(&source);
    let old = c600_syntax::sha256_hex(shipped.as_bytes());
    for digest in [old.as_str(), "0000"] {
        let e = edit_declared_part(&doc, digest, "default_layer", WHOLE, &int("1"), &context).unwrap_err();
        assert_eq!(e.code, code::STALE_EDITOR_SOURCE);
        assert_eq!(e.service, Service::EditDeclaredPart);
        assert!(matches!(e.reason, Reason::StaleSource { .. }), "{:?}", e.reason);
        let e = declared_parts(&doc, digest).unwrap_err();
        assert_eq!(e.code, code::STALE_EDITOR_SOURCE);
    }
    // Every literal form round-trips through its canonical spelling.
    let forms = "
# @editable parameter
fn forms() -> record { a: Int, b: Rat, c: Q5, d: Approx<P53>, e: Bool, f: Word } effects [] {
    return record { a = 1, b = 1/2, c = q5(1/2, 1/2), d = ~53\"0.5\", e = true, f = word[+1, -2] };
}
";
    let source = format!("{shipped}{forms}");
    let doc = open(&source);
    let values = PartValue::Record(vec![
        ("a".into(), int("-12")),
        ("b".into(), PartValue::Rat { num: "-3".into(), den: "4".into() }),
        ("c".into(), PartValue::Q5 { one: ("0".into(), "1".into()), sqrt5: ("2".into(), "3".into()) }),
        ("d".into(), PartValue::Approx { precision: 53, decimal: "-1.25e3".into() }),
        ("e".into(), PartValue::Bool(false)),
        ("f".into(), PartValue::Word(vec![1200, -7, 3])),
    ]);
    let trip = edit_declared_part(&doc, doc.digest(), "forms", WHOLE, &values, &context).unwrap();
    assert!(matches!(trip.verdict, Verdict::Checked(_)), "{:#?}", trip.diagnostics);
    let after = Document::open(apply_edits(&doc, &trip.edits).unwrap(), Units::Utf8);
    let reread = declared_parts(&after, after.digest()).unwrap();
    assert_eq!(reread[0].fields[0].value, values);
}
