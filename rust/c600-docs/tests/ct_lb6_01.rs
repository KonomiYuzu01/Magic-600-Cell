//! CT-LB6-01 (whole: each shipped tool has all PrincipleRecord parts, resolving source and evidence digests and a
//! runnable real-geometry fixture; a part that does not exist reads a typed none; a domain outside the cited scope is
//! named; regeneration of the decomposition accounts for transitive library and kernel calls, and a changed
//! decomposition marks the hand parts stale; expected claims come from the written derivation, not from the module's
//! output). The shipped tools are PHS.22's three built-in tools, the 600-cell-Full tool, the jumbling tool and the
//! representations tool, and the library's twelve modules; their records are the data files under
//! `docs/progress/1.0/principle-records/`. Each record's fixture is a test of this file, `fixture_<module>`, which
//! runs the record's instance on the real geometry and compares what the module answers with the expected claims the
//! record took from its own derivation. A pass is source and fixture evidence for those instances; it proves no
//! universal lemma and records no human comprehension result (EV5.4).

mod common;

use std::collections::BTreeSet;

use c600_docs::code::{DECOMPOSITION_MISMATCH, PRINCIPLE_RECORD_INCOMPLETE};
use c600_docs::decompose::{check_submitted, Edge};
use c600_docs::explain::computed_basis;
use c600_docs::json::{self, Json};
use c600_docs::record::{self, prerequisite_cycle, Generated, Part, State, HAND_PARTS};
use c600_docs::Checked;
use c600_run::value::{q5_of, Value};

use common::run::*;
use common::{subjects, Subject};

// ------------------------------------------------------------------------------------------------- helpers

fn subject(module: &str) -> Subject {
    subjects().into_iter().find(|s| s.module == module).unwrap_or_else(|| panic!("a subject `{module}`"))
}

/// A run value as record JSON: integers, truth values, text, words as letter lists, lists and records in field order.
fn value_json(v: &Value) -> Json {
    match v {
        Value::Bool(b) => Json::Bool(*b),
        Value::Int(n) => Json::Int(n.to_string().parse().expect("an integer that fits in 64 bits")),
        Value::Text(t) | Value::Tag(t) => Json::text(t.as_str()),
        Value::Word(letters) => Json::List(letters.iter().map(|&l| Json::Int(l.into())).collect()),
        Value::List(items) => Json::List(items.items().iter().map(value_json).collect()),
        Value::Record(r) | Value::Receipt(r) => {
            Json::Object(r.fields().iter().map(|(k, v)| (k.as_str().to_string(), value_json(v))).collect())
        }
        other => panic!("no record form for {other:?}"),
    }
}

fn field(v: &Json, name: &str) -> Json {
    v.get(name).cloned().unwrap_or_else(|| panic!("a field `{name}` in {}", v.compact()))
}

fn length(v: &Json) -> Json {
    Json::Int(v.as_list().expect("a list").len() as i64)
}

fn ints(v: &Json) -> Vec<i64> {
    v.as_list().expect("a list").iter().map(|x| x.as_i64().expect("an integer")).collect()
}

fn letters(v: &Json) -> Vec<i32> {
    ints(v).into_iter().map(|x| x as i32).collect()
}

fn word(v: &Json) -> Value {
    Value::word(&letters(v))
}

fn list(v: &Json) -> Value {
    Value::list(ints(v).into_iter().map(Value::int).collect())
}

fn step(letters: &[i32]) -> Value {
    Value::record(vec![
        ("kind", Value::text("word")),
        ("moves", Value::word(letters)),
        ("node", Value::int(0)),
        ("orbit", Value::int(0)),
        ("sign", Value::int(0)),
    ])
}

/// A Q5 value `(a + b√5) / d` from its record form `[a, b, d]`.
fn q5(v: &Json) -> Value {
    let [a, b, d] = ints(v)[..] else { panic!("[a, b, d]") };
    q5_of(&Value::int(a), &Value::int(b), &Value::int(d)).expect("a Q5 value")
}

/// The record's instance input.
fn input(s: &Subject) -> Json {
    s.record().instance.given().expect("an instance").input.clone()
}

/// The fixture's comparison: the module's answers, claim by claim, equal the expected claims of the record, and the
/// fixture observes exactly the claims the record expects.
fn compare(s: &Subject, observed: Vec<(&str, Json)>) {
    let record = s.record();
    let instance = record.instance.given().expect("an instance");
    let expected: BTreeSet<&str> = instance.expected.iter().map(|e| e.claim.as_str()).collect();
    let seen: BTreeSet<&str> = observed.iter().map(|(c, _)| *c).collect();
    assert_eq!(seen, expected, "{}: the fixture observes the record's claims", s.module);
    for e in &instance.expected {
        let (_, got) = observed.iter().find(|(c, _)| *c == e.claim).expect("observed");
        assert_eq!(got, &e.value, "{}: claim `{}`", s.module, e.claim);
    }
}

fn this_file() -> String {
    String::from_utf8(common::resolve("rust/c600-docs/tests/ct_lb6_01.rs").expect("this file")).expect("UTF-8")
}

// ------------------------------------------------------------------------------------------------- the records

/// Every shipped tool and every library module has a record that is complete and current: every part given or a
/// typed none, bound to the module's source digest, its closure digest and its model identity, its evidence files at
/// their recorded digests, its kernel leaves specified with an oracle test that exists, its step map reporting the
/// unmapped items on both sides, its prerequisites glossary terms at or below its level, and its fixture a test of
/// this file. The records' prerequisite records form a DAG.
#[test]
fn ct_lb6_01_every_shipped_tool_and_library_module_has_a_complete_current_record() {
    let file = this_file();
    let mut records = Vec::new();
    for s in subjects() {
        let record = s.record();
        assert_eq!((record.tool.as_str(), record.module.as_str()), (s.tool, s.module));
        assert_eq!(record.entries, s.entries, "{}: the record specifies every exported entry", s.module);
        let admission = s.admit(&record);
        assert_eq!(admission.state, State::Current, "{}: {:#?}\nnow: {}", s.module, admission.findings, s.facts());
        assert_eq!(record.binding.model_identity, s.model_identity(), "{}", s.module);
        let instance = record.instance.given().expect("an instance");
        assert_eq!(instance.model_identity, s.model_identity(), "{}: the instance is on the real geometry", s.module);
        let fixture = format!("rust/c600-docs/tests/ct_lb6_01.rs::fixture_{}", s.module);
        assert_eq!(instance.fixture, fixture, "{}", s.module);
        assert!(
            file.contains(&format!("#[test]\nfn fixture_{}()", s.module)),
            "{}: the fixture runs as a test",
            s.module
        );
        records.push(record);
    }
    assert_eq!(records.len(), 15, "three tools and twelve library modules");
    let refs: Vec<&record::PrincipleRecord> = records.iter().collect();
    assert_eq!(prerequisite_cycle(&refs), None, "the records' prerequisites form a DAG");

    // A cycle is found.
    let mut cyclic = records.clone();
    let (a, b) = (cyclic[3].module.clone(), cyclic[8].module.clone());
    if let Part::Given(p) = &mut cyclic[3].by_hand.prerequisites {
        p.records = vec![b];
    }
    if let Part::Given(p) = &mut cyclic[8].by_hand.prerequisites {
        p.records = vec![a];
    }
    let refs: Vec<&record::PrincipleRecord> = cyclic.iter().collect();
    assert!(prerequisite_cycle(&refs).is_some());
}

/// A part that does not exist reads a typed none: `{"none": "because ..."}` with its reason. The shipped records hold
/// typed nones (a tool with no menu, an entry with no tie, a pure module with no kernel leaf), each with a reason. A
/// part that is absent, a none without its reason, a none beside other fields and a given part with nothing in it
/// are each `LB6:evidence.principle_record_incomplete`.
#[test]
fn ct_lb6_01_a_part_that_does_not_exist_reads_a_typed_none() {
    let mut nones = 0;
    for s in subjects() {
        let record = s.record();
        let specs = record.specification.given().expect("a specification");
        for spec in specs {
            for part in [&spec.parameters, &spec.budget, &spec.menu, &spec.tie_rule, &spec.empty_answer] {
                if let Part::None(reason) = part {
                    assert!(!reason.trim().is_empty(), "{}", s.module);
                    nones += 1;
                }
            }
        }
        if let Part::None(reason) = &record.by_hand.leaf_specs {
            assert!(!reason.trim().is_empty());
            assert!(s.decompositions(&s.checked()).iter().all(|d| d.kernel_leaves().is_empty()), "{}", s.module);
            nones += 1;
        }
    }
    assert!(nones > 0, "the shipped records hold typed nones");

    let bytes = common::resolve(&subject("words").record_path()).expect("the words record");
    let text = String::from_utf8(bytes).unwrap();
    let codes = |text: &str| -> Vec<(&'static str, String)> {
        record::read(text.as_bytes()).1.into_iter().map(|f| (f.code, f.part)).collect()
    };
    assert!(codes(&text).is_empty());
    let cases = [
        ("\"not_meant\":", "\"not_meant_absent\":", "not meant"),
        ("\"leaf_specs\":", "\"leaf_specs_absent\":", "leaf specs"),
    ];
    for (from, to, part) in cases {
        assert!(text.contains(from), "{from}");
        let changed = text.replacen(from, to, 1);
        let found = codes(&changed);
        assert!(found.iter().any(|(c, p)| *c == PRINCIPLE_RECORD_INCOMPLETE && p == part), "{part}: {found:?}");
    }
    // An empty reason: the leaf specifications' none with its text cut to nothing.
    let doc = json::parse(text.as_bytes()).unwrap();
    let reason = doc.get("by_hand").and_then(|h| h.get("leaf_specs")).and_then(|l| l.get("none")).unwrap();
    let cut = text.replacen(&reason.compact(), "\"\"", 1);
    assert!(codes(&cut).iter().any(|(c, p)| *c == PRINCIPLE_RECORD_INCOMPLETE && p == "leaf specs"));
    // A none beside other fields, and a given list with nothing in it.
    let mixed = text.replacen("\"none\": \"the decomposition", "\"spec\": \"x\", \"none\": \"the decomposition", 1);
    assert!(codes(&mixed).iter().any(|(c, p)| *c == PRINCIPLE_RECORD_INCOMPLETE && p == "leaf specs"));
    let empty = text.replacen(&pretty_list_start(&text, "\"failure\": ["), "\"failure\": [], \"unused\": [", 1);
    assert!(codes(&empty).iter().any(|(c, p)| *c == PRINCIPLE_RECORD_INCOMPLETE && p == "failure"));
}

/// The text from `start` to the end of its line: the opening of a pretty-printed list.
fn pretty_list_start(text: &str, start: &str) -> String {
    let at = text.find(start).expect("the list");
    let end = text[at..].find('\n').map_or(text.len(), |n| at + n);
    text[at..end].to_string()
}

/// A domain outside the cited scope is named. Each specification says what its answer means outside the scope of the
/// statement it cites, or why nothing lies outside; where an argument's evidence is a computation over recorded
/// inputs, the record names the part of the domain those inputs do not cover, and the claim's scope is the recorded
/// set, never the whole domain.
#[test]
fn ct_lb6_01_a_domain_outside_the_cited_scope_is_named() {
    for s in subjects() {
        let record = s.record();
        let computed = record
            .justification
            .given()
            .into_iter()
            .flatten()
            .any(|a| a.evidence.given().is_some_and(|e| computed_basis(&e.basis)));
        for spec in record.specification.given().expect("a specification") {
            match &spec.outside_scope {
                Part::Given(text) => assert!(!text.trim().is_empty()),
                Part::None(reason) => {
                    assert!(!computed, "{}.{}: computed evidence covers recorded inputs only", s.module, spec.entry);
                    assert!(!reason.trim().is_empty());
                }
            }
            assert!(!spec.domain.trim().is_empty(), "{}.{}: the domain", s.module, spec.entry);
        }
        if computed {
            let checked = s.checked();
            let decompositions = s.decompositions(&checked);
            let refs: Vec<_> = decompositions.iter().collect();
            let e = c600_docs::explain::explain(
                &record,
                &State::Current,
                &refs,
                c600_docs::explain::FixtureOutcome::NotRun,
                "built_in",
                None,
                &c600_evidence::Edition::draft_1(),
            );
            for (a, view) in record.justification.given().unwrap().iter().zip(&e.arguments) {
                if let Some(ev) = a.evidence.given().filter(|ev| computed_basis(&ev.basis)) {
                    let phrase = format!("{} of family {} only", c600_docs::explain::RECORDED, ev.path);
                    assert_eq!(view.claim.scope, phrase, "{}: the claim covers the recorded inputs only", s.module);
                }
            }
        }
    }
}

/// Regeneration accounts for transitive library and kernel calls. A client module that imports the library's
/// `recipes` by digest and calls `recipes.effect_report` gets a decomposition that holds the library entry's node,
/// in its own module with its own source digest, and the `effect` row's kernel leaf the library entry reaches; a
/// function value passed to `repeat` is a "may call" edge. A submitted graph that omits the library node, an effect,
/// or the kernel call, or that names other source, is `LB6:evidence.decomposition_mismatch`.
#[test]
fn ct_lb6_01_regeneration_accounts_for_transitive_library_and_kernel_calls() {
    let recipes = c600_library::RECIPES_SOURCE;
    let client = format!(
        "edition \"6cs-draft-1\";\nmodule client requires [];\nimport recipes \"{}\";\n\n\
         export fn moved(w: Word) -> Nat effects [snapshot] {{\n    \
         let step = recipes.word_step(w);\n    \
         return recipes.effect_report(list<record {{ kind: Text, moves: Word, node: Nat, orbit: Nat, sign: Int }}>(step)).moved_pieces;\n}}\n",
        c600_library::recipes_sha256()
    );
    let checked = Checked::new(&[recipes.as_bytes(), client.as_bytes()]).unwrap_or_else(|b| panic!("{b:?}"));
    let d = checked.decompose("client", "moved").expect("a decomposition");
    assert_eq!(d.other_modules(), BTreeSet::from(["recipes".to_string()]));
    for label in ["client.moved", "recipes.effect_report", "recipes.word_step"] {
        assert!(d.node_labels().contains(label), "{label}");
    }
    let library = d.node("recipes.effect_report").unwrap();
    assert_eq!(library.source_sha256, c600_library::recipes_sha256());
    assert_eq!(d.kernel_leaves(), BTreeSet::from(["kernel:read_snapshot(effect)".to_string()]));
    let root = d.node("client.moved").unwrap();
    assert!(root.edges.contains(&Edge::Call { module: "recipes".into(), function: "effect_report".into() }));
    assert!(root.effects.iter().any(|e| e.contains("Snapshot")), "the root inherits the library call's effect");

    // A function value: `seeds.stage_order` passes a closure to `repeat`, whose calls are "may call" edges or calls
    // inside the closure, and every function it reaches is a node.
    let seeds = Checked::new(&[c600_library::SEEDS_SOURCE.as_bytes()]).unwrap();
    let order = seeds.decompose("seeds", "stage_order").unwrap();
    for label in ["seeds.first_ready", "seeds.released", "seeds.in_degrees", "seeds.unplaced"] {
        assert!(order.node_labels().contains(label), "{label}");
    }
    assert!(order.base_parts().contains("base:Repeat"));

    check_submitted(&d, &d).expect("the generated graph matches itself");
    let mut omitted = d.clone();
    omitted.nodes.retain(|n| n.module != "recipes");
    let mut no_kernel = d.clone();
    for n in &mut no_kernel.nodes {
        n.edges.retain(|e| !matches!(e, Edge::Kernel { .. }));
    }
    let mut no_effect = d.clone();
    no_effect.nodes[0].effects.clear();
    let mut other_source = d.clone();
    for n in &mut other_source.nodes {
        if n.module == "recipes" {
            n.source_sha256 = "0".repeat(64);
        }
    }
    let mut other_ir = d.clone();
    other_ir.ir_version = "another".into();
    for (name, submitted) in [
        ("omitted", omitted),
        ("no kernel", no_kernel),
        ("no effect", no_effect),
        ("source", other_source),
        ("ir", other_ir),
    ] {
        let refused = check_submitted(&submitted, &d).expect_err(name);
        assert_eq!(refused.code, DECOMPOSITION_MISMATCH, "{name}");
    }
}

/// A changed decomposition marks the hand parts stale. The `words` module with a changed body (the same calls in
/// another order, then a new helper) is a new source: a record still bound to the old source is refused with
/// `LB6:evidence.decomposition_mismatch`; once its generated binding is refreshed to the new source and closure, the
/// record is complete but every hand-written part is stale, since the decomposition it was written against changed.
#[test]
fn ct_lb6_01_a_changed_decomposition_marks_the_hand_parts_stale() {
    let s = subject("words");
    let record = s.record();
    assert_eq!(s.admit(&record).state, State::Current);
    let reordered = c600_library::SOURCE.replace(
        "true => concat(inverse(w), power(w, exact_add(n, 1)));",
        "true => concat(power(w, exact_add(n, 1)), inverse(w));",
    );
    let helper = format!(
        "{}\nfn twice(w: Word) -> Word effects [] {{\n    return concat(w, w);\n}}\n",
        reordered.replace(
            "_ => concat(w, power(w, exact_add(n, -1)));",
            "_ => concat(twice(word[]), concat(w, power(w, exact_add(n, -1))));"
        )
    );
    for changed in [reordered, helper] {
        assert_ne!(changed, c600_library::SOURCE);
        let moved =
            Subject { source: Box::leak(changed.into_boxed_str()), entries: s.entries.clone(), ..subject("words") };
        let checked = moved.checked();
        let decompositions = moved.decompositions(&checked);
        let generated = Generated {
            source_sha256: c600_digest::sha256_hex(moved.source.as_bytes()),
            closure_sha256: moved.closure_sha256(),
            decompositions: decompositions.iter().collect(),
        };
        let admit = |r: &record::PrincipleRecord| {
            record::admit(r, &generated, &common::glossary(), &common::record_names(), &|p| common::resolve(p))
        };
        let old = admit(&record);
        assert_eq!(old.state, State::Refused);
        assert!(old.codes().contains(DECOMPOSITION_MISMATCH));

        let mut refreshed = record.clone();
        refreshed.binding.source_sha256 = generated.source_sha256.clone();
        refreshed.binding.closure_sha256 = generated.closure_sha256.clone();
        let admission = admit(&refreshed);
        match &admission.state {
            State::Stale { parts, was, now } => {
                assert_eq!(parts, &HAND_PARTS.to_vec());
                assert_eq!(was, &record.binding.decomposition_sha256);
                assert_ne!(was, now);
            }
            other => panic!("stale, not {other:?}: {:#?}", admission.findings),
        }
        let refs: Vec<_> = decompositions.iter().collect();
        let e = c600_docs::explain::explain(
            &refreshed,
            &admission.state,
            &refs,
            c600_docs::explain::FixtureOutcome::NotRun,
            "built_in",
            None,
            &c600_evidence::Edition::draft_1(),
        );
        assert_eq!(e.stale, HAND_PARTS.to_vec());
        assert!(c600_docs::explain::render(&e).contains("Stale: step map, reason"));
    }
}

/// Expected claims come from the written derivation, not from the module's output. Each expected claim names the step
/// of the record's derivation whose result it repeats, and the admission compares the claim with that step, never
/// with a run: a claim changed to another value, a claim whose step was changed, and a claim that names no step are
/// each refused with `LB6:evidence.principle_record_incomplete`. The fixtures then compare the module's answers with
/// the claims (`fixture_<module>`).
#[test]
fn ct_lb6_01_expected_claims_come_from_the_written_derivation() {
    for s in subjects() {
        let record = s.record();
        let steps = record.by_hand.steps.given().expect("steps");
        for e in &record.instance.given().unwrap().expected {
            assert!(e.from_step >= 1 && e.from_step <= steps.len(), "{}: {}", s.module, e.claim);
            assert_eq!(steps[e.from_step - 1].result, e.value, "{}: {}", s.module, e.claim);
        }
    }
    let s = subject("words");
    let record = s.record();
    let mut claim = record.clone();
    if let Part::Given(i) = &mut claim.instance {
        i.expected[0].value = Json::List(vec![Json::Int(1)]);
    }
    let mut steps = record.clone();
    if let Part::Given(st) = &mut steps.by_hand.steps {
        st[0].result = Json::List(Vec::new());
    }
    let mut nowhere = record.clone();
    if let Part::Given(i) = &mut nowhere.instance {
        i.expected[0].from_step = 99;
    }
    for (name, changed) in [("claim", claim), ("step", steps), ("no step", nowhere)] {
        let admission = s.admit(&changed);
        assert_eq!(admission.state, State::Refused, "{name}");
        assert!(
            admission.findings.iter().any(|f| f.code == PRINCIPLE_RECORD_INCOMPLETE && f.part == "instance"),
            "{name}: {:#?}",
            admission.findings
        );
    }
}

/// Evidence and source digests resolve: a record whose evidence file has other bytes than the recorded digest, whose
/// evidence file does not exist, or whose fixture or leaf test names no function, is refused.
#[test]
fn ct_lb6_01_source_and_evidence_digests_resolve() {
    let s = subject("full_600_cell");
    let record = s.record();
    let mut digest = record.clone();
    let mut missing = record.clone();
    let mut fixture = record.clone();
    let mut leaf = record.clone();
    if let Part::Given(args) = &mut digest.justification {
        let e = args.iter_mut().find_map(|a| match &mut a.evidence {
            Part::Given(e) => Some(e),
            Part::None(_) => None,
        });
        e.expect("evidence").sha256 = "0".repeat(64);
    }
    if let Part::Given(args) = &mut missing.justification {
        for a in args.iter_mut() {
            if let Part::Given(e) = &mut a.evidence {
                e.path = "rust/no-such-file.rs".into();
            }
        }
    }
    if let Part::Given(i) = &mut fixture.instance {
        i.fixture = "rust/c600-docs/tests/ct_lb6_01.rs::fixture_no_such_module".into();
    }
    if let Part::Given(specs) = &mut leaf.by_hand.leaf_specs {
        specs[0].test = "rust/c600-tools/tests/bridge.rs::no_such_test".into();
    }
    let mut source = record.clone();
    source.binding.source_sha256 = "0".repeat(64);
    for (name, changed, code) in [
        ("evidence digest", digest, PRINCIPLE_RECORD_INCOMPLETE),
        ("evidence file", missing, PRINCIPLE_RECORD_INCOMPLETE),
        ("fixture", fixture, PRINCIPLE_RECORD_INCOMPLETE),
        ("leaf test", leaf, PRINCIPLE_RECORD_INCOMPLETE),
        ("source", source, DECOMPOSITION_MISMATCH),
    ] {
        let admission = s.admit(&changed);
        assert_eq!(admission.state, State::Refused, "{name}");
        assert!(admission.codes().contains(code), "{name}: {:#?}", admission.findings);
    }
}

// ------------------------------------------------------------------------------------------------- the fixtures

/// The 600-cell-Full tool's instance: the solver names [1, -1] at solved; the tool runs inside the command the solver
/// invoked and proposes the word; the layer's preview of it is the kernel's complete effect.
#[test]
fn fixture_full_600_cell() {
    let s = subject("full_600_cell");
    let input = input(&s);
    let (layer, output) = full_tool_preview(&letters(&field(&input, "word")));
    let preview = layer.preview().expect("a preview");
    let ack = json::parse(&output).expect("the acknowledgement is JSON");
    assert_eq!(ack.get("base").and_then(Json::as_str), Some("fixture@1/e0"));
    let end = preview.end.as_ref().expect("no protected orbit moves");
    compare(
        &s,
        vec![
            ("primitive_count", Json::Int(preview.effect.primitive_count as i64)),
            ("moved_pieces", Json::Int(preview.effect.moved_pieces as i64)),
            ("support_rows", Json::Int(preview.effect.support.len() as i64)),
            ("end_is_base", Json::Bool(end.digest() == preview.base.digest())),
        ],
    );
}

/// The jumbling tool's instance under the pinned menu of W-J fixture I_a, at solved: the named word [0, 12], [0, 45]
/// and the named twist [7, 12], each checked as a whole word by the jumbling engine.
#[test]
fn fixture_jumble_checker() {
    let s = subject("jumble_checker");
    let input = input(&s);
    let flat = |v: &Json| -> Vec<i32> { v.as_list().unwrap().iter().flat_map(letters).collect() };
    let named = jumble_check(c600_jumble_tool::tool::CHECK_WORD, &flat(&field(&input, "word")));
    let twist = jumble_check(c600_jumble_tool::tool::CHECK_TWIST, &flat(&field(&input, "twist")));
    let number = |c: &c600_kernel::json::Value, f: &str| Json::Int(c.get(f).and_then(|v| v.as_i64()).expect(f));
    let truth = |c: &c600_kernel::json::Value, f: &str| {
        Json::Bool(matches!(c.get(f), Some(c600_kernel::json::Value::Bool(true))))
    };
    let (w, t) = (&named.checks[0], &twist.checks[0]);
    compare(
        &s,
        vec![
            ("word_letters", number(w, "letters")),
            ("word_admissible", truth(w, "admissible")),
            ("word_off_after", number(w, "off_after")),
            ("twist_letters", number(t, "letters")),
            ("twist_cap", number(t, "cap")),
        ],
    );
}

/// The representations tool's instance: `compare_cells(7, 7)` publishes the view of the two named cells and their
/// face-step distance, read from the `structure.compare` row at solved.
#[test]
fn fixture_representations() {
    let s = subject("representations");
    let input = input(&s);
    let cells = ints(&field(&input, "cells"));
    let mut layer = layer_at(common::kernel().solved(), "fixture");
    let args = vec![Value::int(cells[0]), Value::int(cells[1])];
    let ran = on_layer(c600_repr::tool::SOURCE, s.module, "compare_cells", args, &mut layer, None);
    let [view] = &ran.views[..] else { panic!("one view") };
    let view = json::parse(view).expect("a view is JSON");
    let int = |f: &str| {
        let text = view.get(f).and_then(|v| v.get("int")).and_then(Json::as_str).expect(f);
        Json::Int(text.parse().expect("an integer"))
    };
    compare(&s, vec![("first", int("first")), ("second", int("second")), ("steps", int("steps"))]);
}

/// `words.power` on [1, -2] with n = 2, -1 and 0.
#[test]
fn fixture_words() {
    let s = subject("words");
    let input = input(&s);
    let w = field(&input, "w");
    let power = |n: i64| value_json(&pure(&[s.source], s.module, "power", vec![word(&w), Value::int(n)], None));
    compare(&s, vec![("power_2", power(2)), ("power_minus_1", power(-1)), ("power_0", power(0))]);
}

/// `quadratic` on the golden ratio φ = (1 + √5)/2 of the 600-cell's vertex coordinates.
#[test]
fn fixture_quadratic() {
    let s = subject("quadratic");
    let input = input(&s);
    let run = |entry: &str, args: Vec<Value>| value_json(&pure(&[s.source], s.module, entry, args, None));
    compare(
        &s,
        vec![
            ("compare_phi_one", run("compare_q5", vec![q5(&field(&input, "phi")), q5(&field(&input, "one"))])),
            ("sign_one_minus_phi", run("sign_q5", vec![q5(&field(&input, "one_minus_phi"))])),
        ],
    );
}

/// `word_keys.same_word` under the model's trace relation on generators 1 and 45, whose supports are disjoint, and
/// on generators 1 and 2.
#[test]
fn fixture_word_keys() {
    let s = subject("word_keys");
    let input = input(&s);
    let relation = relation();
    let same = |a: &str, b: &str| {
        let args = vec![word(&field(&input, a)), word(&field(&input, b))];
        value_json(&pure(&[s.source], s.module, "same_word", args, Some(&relation)))
    };
    compare(&s, vec![("same_x_y", same("x", "y")), ("same_x_z", same("x", "z"))]);
}

/// `linalg` over F5 on the C5 coordinates (orbits 17 and 28) of corrections 27 and 30 of the I_a correction library.
#[test]
fn fixture_linalg() {
    let s = subject("linalg");
    let input = input(&s);
    let rows = Value::list(field(&input, "rows").as_list().unwrap().iter().map(list).collect());
    let rank = value_json(&pure(&[s.source], s.module, "rank_f5", vec![rows.clone()], None));
    let solved = value_json(&pure(&[s.source], s.module, "solve_f5", vec![rows, list(&field(&input, "rhs"))], None));
    compare(
        &s,
        vec![("rank_f5", rank), ("solvable", field(&solved, "solvable")), ("solution", field(&solved, "solution"))],
    );
}

/// `trees` on 2, on φ and on the solved pose, the identity matrix.
#[test]
fn fixture_trees() {
    let s = subject("trees");
    let input = input(&s);
    let run = |entry: &str, args: Vec<Value>| value_json(&pure(&[s.source], s.module, entry, args, None));
    let matrix = Value::list(
        field(&input, "solved_pose")
            .as_list()
            .unwrap()
            .iter()
            .map(|row| Value::list(row.as_list().unwrap().iter().map(q5).collect()))
            .collect(),
    );
    let tree = run("pose_tree", vec![matrix]);
    compare(
        &s,
        vec![
            ("v2_two", run("v2", vec![q5(&field(&input, "two"))])),
            ("v2_phi", run("v2", vec![q5(&field(&input, "phi"))])),
            ("dist_l", field(&tree, "dist_l")),
            ("dist_r", field(&tree, "dist_r")),
            ("branch_l", field(&tree, "branch_l")),
        ],
    );
}

/// `recipes`: a star word from a setup, a relocation and a seed, and the effect report of [5, -5] at solved.
#[test]
fn fixture_recipes() {
    let s = subject("recipes");
    let input = input(&s);
    let star = pure(
        &[s.source],
        s.module,
        "star_word",
        vec![word(&field(&input, "setup")), word(&field(&input, "relocation")), word(&field(&input, "seed"))],
        None,
    );
    let mut layer = layer_at(common::kernel().solved(), "fixture");
    let recipe = Value::list(vec![step(&letters(&field(&input, "recipe_word")))]);
    let report = value_json(&on_layer(s.source, s.module, "effect_report", vec![recipe], &mut layer, None).value);
    compare(
        &s,
        vec![
            ("star_word", value_json(&star)),
            ("effect_primitive_count", field(&report, "primitive_count")),
            ("effect_moved_pieces", field(&report, "moved_pieces")),
            ("effect_support_rows", length(&field(&report, "support"))),
        ],
    );
}

/// `completion.exact_completion` on piece 0 at solved.
#[test]
fn fixture_completion() {
    let s = subject("completion");
    let input = input(&s);
    let mut layer = layer_at(common::kernel().solved(), "fixture");
    let args = vec![list(&field(&input, "pieces")), list(&field(&input, "home_slots"))];
    let complete = on_layer(s.source, s.module, "exact_completion", args, &mut layer, None).value;
    compare(&s, vec![("complete", value_json(&complete))]);
}

/// `protection`: a position lock of slots 0 and 1 at solved and the Net check of [5, -5] against it.
#[test]
fn fixture_protection() {
    let s = subject("protection");
    let input = input(&s);
    let mut layer = layer_at(common::kernel().solved(), "fixture");
    let lock = on_layer(s.source, s.module, "position_lock", vec![list(&field(&input, "slots"))], &mut layer, None);
    let recipe = Value::list(vec![step(&letters(&field(&input, "recipe_word")))]);
    let net = on_layer(s.source, s.module, "net_check", vec![lock.value.clone(), recipe], &mut layer, None);
    let (lock, net) = (value_json(&lock.value), value_json(&net.value));
    compare(
        &s,
        vec![
            ("lock_labels", field(&lock, "labels")),
            ("net_conflicts", length(&field(&net, "conflicts"))),
            ("net_verdict", field(&net, "verdict")),
        ],
    );
}

/// `guarded_setups.setups_for` at buffer A of orbit 33, with the certified setup tables.
#[test]
fn fixture_guarded_setups() {
    let s = subject("guarded_setups");
    let input = input(&s);
    let table = setups();
    let goal = Value::record(vec![
        ("orbit", Value::int(field(&input, "orbit").as_i64().unwrap())),
        ("position", Value::int(field(&input, "position").as_i64().unwrap())),
    ]);
    let mut layer = layer_at(common::kernel().solved(), "fixture");
    let answer = value_json(&on_layer(s.source, s.module, "setups_for", vec![goal], &mut layer, Some(table)).value);
    let buffers = field(&answer, "buffers");
    let position = field(&answer, "position");
    compare(
        &s,
        vec![
            ("position_is_buffer_a", Json::Bool(buffers.as_list().unwrap().first() == Some(&position))),
            ("setups", length(&field(&answer, "setups"))),
        ],
    );
}

/// `macros.macro_comparison` of [5, -5] and [7, -7] at solved.
#[test]
fn fixture_macros() {
    let s = subject("macros");
    let input = input(&s);
    let mut layer = layer_at(common::kernel().solved(), "fixture");
    let recipe = |name: &str| Value::list(vec![step(&letters(&field(&input, name)))]);
    let args = vec![Value::text("first"), recipe("first"), Value::text("second"), recipe("second")];
    let c = value_json(&on_layer(s.source, s.module, "macro_comparison", args, &mut layer, None).value);
    compare(
        &s,
        vec![
            ("same_net", field(&c, "same_net")),
            ("same_expansion", field(&c, "same_expansion")),
            ("both", field(&c, "both")),
            ("only_first", field(&c, "only_first")),
            ("only_second", field(&c, "only_second")),
        ],
    );
}

/// `seeds.stage_order` on the seed atlas's rows of orbits 30 and 26, listed in that order.
#[test]
fn fixture_seeds() {
    let s = subject("seeds");
    let input = input(&s);
    let rows = field(&input, "rows")
        .as_list()
        .unwrap()
        .iter()
        .map(|r| {
            Value::record(vec![
                ("collateral", list(&field(r, "collateral"))),
                ("orbit", Value::int(field(r, "orbit").as_i64().unwrap())),
            ])
        })
        .collect();
    let answer = value_json(&pure(&[s.source], s.module, "stage_order", vec![Value::list(rows)], None));
    compare(&s, vec![("order", field(&answer, "order")), ("cycle", field(&answer, "cycle"))]);
}

/// `orbits.orbit_catalogue` of orbit -1, the fixed cell centres, at solved.
#[test]
fn fixture_orbits() {
    let s = subject("orbits");
    let input = input(&s);
    let mut layer = layer_at(common::kernel().solved(), "fixture");
    let answer = value_json(
        &on_layer(s.source, s.module, "orbit_catalogue", vec![list(&field(&input, "named"))], &mut layer, None).value,
    );
    let rows = answer.as_list().expect("rows");
    compare(
        &s,
        vec![
            ("rows", Json::Int(rows.len() as i64)),
            ("orbit", field(&rows[0], "orbit")),
            ("pieces", length(&field(&rows[0], "pieces"))),
        ],
    );
}
