//! CT-EV5-01 (whole: EV5.2 to EV5.5: generated docs and principle records resolve their pinned source, domains,
//! empty-answer meanings, instances, decomposition, evidence and the parts a person writes; a fork inherits no check),
//! on the three built-in tools of PHS.22 and the library's twelve modules. The reference documentation is generated
//! from each module's checked metadata and links its declarations and its principle record; a broken draft's output
//! stays apart from the active documentation; an explanation consumes the record and links its provenance lazily,
//! down to kernel leaves and base parts, stage-qualified lemmas, imported bytes and word digests; and a fork's
//! explanation keeps the cited statements and inherits no evidence. A pass is source evidence.

mod common;

use std::collections::BTreeSet;

use c600_docs::explain::{self, expand, expand_word, word_link, FixtureOutcome, Link, MEANS_NOT_CERTIFICATE};
use c600_docs::record::{Part, State};
use c600_docs::reference::{self, Status};
use c600_docs::Checked;
use c600_evidence::Edition;

use common::{subjects, Subject};

fn explanation(s: &Subject, fixture: FixtureOutcome, fork_of: Option<&str>) -> explain::Explanation {
    let record = s.record();
    let checked = s.checked();
    let decompositions = s.decompositions(&checked);
    let refs: Vec<_> = decompositions.iter().collect();
    explain::explain(&record, &State::Current, &refs, fixture, "built_in", fork_of, &Edition::draft_1())
}

/// EV5.2: each module's reference is generated from its checked metadata: its source and closure digests are those
/// its record is bound to, its edition, IR version and contract are the program's, its supported identity is the
/// record's model identity, and each function's signature, declared and inferred effects, capabilities, grant rows
/// and state reads are the checker's, with a link to the declaration's range in the source. The page links the
/// principle record by its path and digest and keeps no second signature: every signature line on the page is
/// generated.
#[test]
fn ct_ev5_01_reference_docs_are_generated_from_checked_metadata() {
    for s in subjects() {
        let record = s.record();
        let checked = s.checked();
        let record_bytes = common::resolve(&s.record_path()).unwrap();
        let digest = c600_digest::sha256_hex(&record_bytes);
        let principles = [(s.module, s.record_path(), digest.clone())];
        let principles: Vec<(&str, &str, &str)> =
            principles.iter().map(|(m, p, d)| (*m, p.as_str(), d.as_str())).collect();
        let docs = reference::reference(&checked, std::slice::from_ref(&s.identity), &principles, true).unwrap();
        assert_eq!(docs.status, Status::Active);
        let [module] = &docs.modules[..] else { panic!("one module") };
        assert_eq!(module.source_sha256, record.binding.source_sha256, "{}: the pinned source", s.module);
        assert_eq!(module.closure_sha256, record.binding.closure_sha256, "{}: the pinned closure", s.module);
        assert_eq!(module.edition, "6cs-draft-1");
        assert_eq!(
            (module.ir_version.as_str(), module.exec_contract.as_str()),
            (c600_ir::IR_VERSION, c600_ir::EXEC_CONTRACT)
        );
        assert_eq!(
            module.supports,
            vec![record.binding.model_identity.clone()],
            "{}: the supported identity",
            s.module
        );
        assert_eq!(module.principle, Some((s.record_path(), digest)));
        let exported: Vec<&str> = module.entries.iter().filter(|e| e.exported).map(|e| e.name.as_str()).collect();
        assert_eq!(exported, record.entries.iter().map(String::as_str).collect::<Vec<_>>());
        let ir = checked.lowered.program.modules.iter().find(|m| m.name == s.module).unwrap();
        for (entry, f) in module.entries.iter().zip(&ir.functions) {
            let summary = checked.report.summary(s.module, &f.name).unwrap();
            assert_eq!(entry.name, f.name);
            assert_eq!(entry.params.len(), f.params.len());
            assert_eq!(entry.result, c600_check::type_name(&f.result.ty));
            assert_eq!(entry.effects.len(), summary.effects.len(), "{}.{}", s.module, f.name);
            assert_eq!(entry.rows.len(), summary.rows.len(), "{}.{}", s.module, f.name);
            let source = std::str::from_utf8(s.source.as_bytes()).unwrap();
            let declared = &source[entry.declaration.start..entry.declaration.end];
            assert!(declared.contains(&format!("{}(", f.name)), "{}.{}: the declaration link", s.module, f.name);
            assert!(entry.declaration.line >= 1);
        }
        let page = reference::render(&docs);
        assert!(page.contains("Generated from checked metadata (EV5.2)"));
        assert!(page.contains(&format!("Principle record: `{}`", s.record_path())));
        let signatures = page.lines().filter(|l| l.starts_with("### `")).count();
        assert_eq!(signatures, module.entries.len(), "{}: one generated signature per function", s.module);
    }

    // Documentation comments directly above a declaration are carried as written; a comment separated by a blank line
    // is not.
    let source = "edition \"6cs-draft-1\";\nmodule notes requires [];\n\n# Not this one.\n\n# Twice a word.\n\
                  # Written out letter by letter.\nexport fn twice(w: Word) -> Word effects [] {\n    \
                  return concat(w, w);\n}\n";
    let checked = Checked::new(&[source.as_bytes()]).unwrap_or_else(|b| panic!("{b:?}"));
    let docs = reference::reference(&checked, &[c600_module::Identity::full()], &[], false).unwrap();
    assert_eq!(docs.status, Status::Candidate);
    assert_eq!(docs.modules[0].entries[0].doc, ["Twice a word.", "Written out letter by letter."]);
    assert_eq!(docs.modules[0].principle, None);
    assert!(reference::render(&docs).contains("Principle record: none"));
}

/// EV5.2 and EV1.7: a broken draft's reference output stays apart from the active tool's documentation. A draft of
/// the 600-cell-Full tool that does not check is a broken draft: its reference holds its findings and no entry, and
/// its page says that it is not the active tool's documentation.
#[test]
fn ct_ev5_01_a_broken_draft_stays_apart_from_the_active_documentation() {
    let source = c600_tools::tool::SOURCE.replace("effects [preview]", "effects []");
    assert_ne!(source, c600_tools::tool::SOURCE);
    let broken = Checked::new(&[source.as_bytes()]).expect_err("the draft does not check");
    assert!(!broken.findings.is_empty());
    let docs = reference::broken(&broken);
    assert!(matches!(&docs.status, Status::Broken { findings } if !findings.is_empty()));
    assert!(docs.modules.is_empty(), "a broken draft has no checked metadata");
    let page = reference::render(&docs);
    assert!(page.contains("broken draft, not the active tool's documentation"));
    assert!(page.contains(&broken.findings[0].0));
    let active = Checked::new(&[c600_tools::tool::SOURCE.as_bytes()]).unwrap();
    let active =
        reference::render(&reference::reference(&active, &[c600_module::Identity::full()], &[], true).unwrap());
    assert!(active.contains("# Reference (active)") && !active.contains("broken"));
}

/// EV5.3: a tool's explanation consumes its record: the exact function and domain, the meaning of an empty or
/// negative answer, the supporting arguments with their evidence and claims, the instance with its fixture's claim,
/// the decomposition's roots, and every part a person writes. It says that it supplies the means of understanding and
/// certifies none, and that permission is decided at invocation.
#[test]
fn ct_ev5_01_records_resolve_domains_empty_answers_instances_evidence_and_hand_parts() {
    for s in subjects() {
        let record = s.record();
        assert_eq!(s.admit(&record).state, State::Current, "{}", s.module);
        let e = explanation(&s, FixtureOutcome::Passed, None);
        assert_eq!((e.tool.as_str(), e.module.as_str()), (s.tool, s.module));
        assert_eq!(e.specification, record.specification);
        for spec in e.specification.given().expect("a specification") {
            assert!(!spec.function.is_empty() && !spec.domain.is_empty(), "{}.{}", s.module, spec.entry);
            for part in [&spec.empty_answer, &spec.negative_scope] {
                match part {
                    Part::Given(text) | Part::None(text) => assert!(!text.trim().is_empty(), "{}", spec.entry),
                }
            }
        }
        let arguments = record.justification.given().expect("a justification");
        assert_eq!(e.arguments.len(), arguments.len());
        for (view, a) in e.arguments.iter().zip(arguments) {
            assert_eq!((view.kind, &view.text, &view.evidence), (a.kind, &a.text, &a.evidence));
            if let Part::Given(statement) = &a.statement {
                assert_eq!(&view.claim.statement, statement);
            }
        }
        let instance = e.instance.as_ref().expect("the instance's claim");
        assert_eq!(instance.scope, "on one instance", "{}: the fixture's claim is that instance only", s.module);
        assert_eq!(e.decomposition.len(), record.entries.len(), "{}: one root per entry", s.module);
        assert_eq!(e.by_hand, record.by_hand, "{}: the parts a person writes", s.module);
        assert!(e.stale.is_empty());
        assert_eq!(e.note, MEANS_NOT_CERTIFICATE);
        assert_eq!(e.permission, "decided at invocation; an explanation grants nothing");
        let page = explain::render(&e);
        assert!(page.contains(MEANS_NOT_CERTIFICATE) && page.contains("## Instance"));
    }

    // A completed empty answer concerns the named scope only, and a job cut off by its limit has no completed empty
    // answer (EV5.3, KN7.4).
    let setups = subjects().into_iter().find(|s| s.module == "guarded_setups").unwrap().record();
    let spec = &setups.specification.given().unwrap()[0];
    let Part::Given(empty) = &spec.empty_answer else { panic!("a given meaning") };
    assert!(empty.contains("concerns the named position") && empty.contains("never says that no solving step exists"));
    let protection = subjects().into_iter().find(|s| s.module == "protection").unwrap().record();
    let within = protection.specification.given().unwrap().iter().find(|s| s.entry == "strict_check_within").unwrap();
    assert!(matches!(&within.empty_answer, Part::None(r) if r.contains("never a completed empty answer")));
}

fn reaches_primitives(d: &c600_docs::decompose::Decomposition, label: &str, seen: &mut BTreeSet<String>) {
    if !seen.insert(label.to_string()) {
        return;
    }
    for link in expand(d, label).unwrap_or_else(|| panic!("{label} expands")) {
        match link {
            Link::Function { label, .. } => reaches_primitives(d, &label, seen),
            Link::Kernel { label } => assert!(label.starts_with("kernel:")),
            Link::Base { label } => assert!(label.starts_with("base:")),
            Link::MayCall { label } => assert!(d.node(&label).is_some(), "a may-call edge names a node: {label}"),
            other => panic!("{other:?} in a decomposition"),
        }
    }
}

/// EV5.4: provenance expands lazily. Each root is one collapsed link with its edge count; expanding it gives its
/// calls collapsed, its kernel leaves and its base parts, and expanding every call in turn ends at kernel leaves and
/// base parts, transitive library calls included. A stage-qualified, commit-pinned statement is a lemma link. A long
/// word is a digest and its length until expanded, and expansion reads letters only from bytes with that digest.
#[test]
fn ct_ev5_01_provenance_expands_lazily_down_to_primitives() {
    for s in subjects() {
        let e = explanation(&s, FixtureOutcome::NotRun, None);
        let checked = s.checked();
        for (root, d) in e.decomposition.iter().zip(s.decompositions(&checked)) {
            let Link::Function { label, source_sha256, edges } = root else { panic!("a collapsed root") };
            assert_eq!(source_sha256, &c600_digest::sha256_hex(s.source.as_bytes()));
            assert_eq!(*edges, d.node(label).unwrap().edges.len());
            let mut seen = BTreeSet::new();
            reaches_primitives(&d, label, &mut seen);
            assert_eq!(seen, d.node_labels(), "{}: every node is reached", s.module);
        }
    }

    // Through a library call: the client's root expands to the library entry, collapsed, which expands to the
    // `effect` row's kernel leaf.
    let client = format!(
        "edition \"6cs-draft-1\";\nmodule client requires [];\nimport recipes \"{}\";\n\n\
         export fn moved(w: Word) -> Nat effects [snapshot] {{\n    let step = recipes.word_step(w);\n    \
         return recipes.effect_report(list<record {{ kind: Text, moves: Word, node: Nat, orbit: Nat, sign: Int }}>(step)).moved_pieces;\n}}\n",
        c600_library::recipes_sha256()
    );
    let checked = Checked::new(&[c600_library::RECIPES_SOURCE.as_bytes(), client.as_bytes()]).unwrap();
    let d = checked.decompose("client", "moved").unwrap();
    let top = expand(&d, "client.moved").unwrap();
    assert!(top.iter().any(|l| matches!(l, Link::Function { label, .. } if label == "recipes.effect_report")));
    let library = expand(&d, "recipes.effect_report").unwrap();
    assert!(library.contains(&Link::Kernel { label: "kernel:read_snapshot(effect)".into() }));

    // A stage-qualified, source-commit-pinned lemma.
    let commit = "b01099e3".repeat(5);
    let named = format!("lean@{commit}:Full600.inverse_cancels");
    assert!(matches!(explain::statement_id(&named), c600_evidence::StatementId::Named { .. }));
    let mut record = common::subjects()[0].record();
    if let Part::Given(args) = &mut record.justification {
        args[1].statement = Part::Given(named.clone());
    }
    let s = &common::subjects()[0];
    let checked = s.checked();
    let ds = s.decompositions(&checked);
    let refs: Vec<_> = ds.iter().collect();
    let e = explain::explain(
        &record,
        &State::Current,
        &refs,
        FixtureOutcome::NotRun,
        "built_in",
        None,
        &Edition::draft_1(),
    );
    assert_eq!(e.arguments[1].link, Some(Link::Lemma { statement: named }));

    // A long word: the seed of orbit 0 in the model's seed atlas.
    let atlas = c600_kernel::json::parse(&common::resolve("assets/seed_atlas.json").unwrap()).unwrap();
    let letters: Vec<i32> = atlas.as_array().unwrap()[0]
        .get("word")
        .and_then(|w| w.as_array())
        .unwrap()
        .iter()
        .map(|l| l.as_i64().unwrap() as i32)
        .collect();
    assert!(letters.len() > 2000);
    let link = word_link(&letters);
    let Link::Word { digest, length } = &link else { panic!("a word link") };
    assert_eq!((digest.len(), *length), (64, letters.len()), "a digest and a length, no letters");
    assert_eq!(expand_word(&link, &letters, 100, 5).as_deref(), Some(&letters[100..105]));
    let mut other = letters.clone();
    other[0] = -other[0];
    assert_eq!(expand_word(&link, &other, 100, 5), None, "letters of other bytes are refused");
    assert_eq!(expand_word(&link, &letters, letters.len() - 2, 5), None, "positions past the end");
}

/// EV5.4: an imported result traces to its stored bytes and source label, and the trace says that its producer is
/// absent: the jumbling tool's replay argument cites J1's W-J fixture, produced by the research engine outside this
/// build.
#[test]
fn ct_ev5_01_an_imported_result_traces_to_its_bytes_and_source_label() {
    let s = subjects().into_iter().find(|s| s.module == "jumble_checker").unwrap();
    let e = explanation(&s, FixtureOutcome::NotRun, None);
    let path = "research/jumbling/fixtures/wj-I_a.json";
    let bytes = c600_digest::sha256_hex(&common::resolve(path).unwrap());
    // The record's other links (its card-pinned lemma) are not imported results.
    let is_imported = |l: &&Link| matches!(l, Link::Imported { .. });
    let imported: Vec<&Link> = e.arguments.iter().filter_map(|a| a.link.as_ref()).filter(is_imported).collect();
    let [Link::Imported { path: p, bytes_sha256, source_label, producer }] = &imported[..] else {
        panic!("one imported link: {imported:?}")
    };
    assert_eq!((p.as_str(), bytes_sha256, *producer), (path, &bytes, "absent"));
    let view = e.arguments.iter().find(|a| a.link.as_ref().is_some_and(|l| is_imported(&l))).unwrap();
    assert!(view.claim.source_labels.contains(source_label));
    assert!(view.claim.check_references.contains(&bytes));
}

/// EV5.5: a fork inherits no evidence. Its explanation keeps each cited statement and its scope as a reference, but
/// records none of the parent's evidence: no basis, no check and no job outcome reach the fork's claims, and its
/// words are not the parent's. A changed module is not certified because its parent was checked: the parent's record
/// is refused against the fork's source.
#[test]
fn ct_ev5_01_a_fork_inherits_no_evidence() {
    for s in subjects() {
        let parent = explanation(&s, FixtureOutcome::Passed, None);
        let fork = explanation(&s, FixtureOutcome::NotRun, Some(s.module));
        assert_eq!(fork.fork_of.as_deref(), Some(s.module));
        for (p, f) in parent.arguments.iter().zip(&fork.arguments) {
            assert_eq!(f.claim.statement, p.claim.statement, "{}: the cited statement is kept", s.module);
            assert_eq!(f.claim.scope, p.claim.scope, "{}: and its scope", s.module);
            assert!(f.claim.bases.is_empty() && f.claim.checks.is_empty(), "{}: no inherited check", s.module);
            assert_eq!(f.claim.job_outcome, None);
            assert!(f.claim.check_references.is_empty());
        }
        for (p, f) in parent.arguments.iter().zip(&fork.arguments) {
            if !p.claim.checks.is_empty() {
                assert_ne!(p.claim.word.word, f.claim.word.word, "{}: a fork's word is not its parent's", s.module);
            }
        }
        let instance = fork.instance.as_ref().unwrap();
        assert!(instance.bases.is_empty() && instance.job_outcome.is_none(), "the fork's fixture has not run");
        assert!(explain::render(&fork).contains("no evidence is inherited"));
    }
    let s = subjects().into_iter().find(|s| s.module == "words").unwrap();
    let changed = c600_library::SOURCE.replace("true => word[];", "true => word[];  # forked");
    let fork = Subject { source: Box::leak(changed.into_boxed_str()), entries: s.entries.clone(), ..s };
    let admission = fork.admit(&fork.record());
    assert_eq!(admission.state, State::Refused, "the parent's record does not certify the fork");
    assert!(admission.codes().contains(c600_docs::code::DECOMPOSITION_MISMATCH));
}
