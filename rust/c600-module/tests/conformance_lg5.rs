//! LP2's conformance tests for chapter 02's LG5 rows CT-LG5-01 to CT-LG5-07 against LP3 C4's `c600-module`: module
//! identity by content digest, the manifest as data, the lock and the approval bound to it, model identity, drafts
//! beside an active version, forks, and sharing through local files. Evidence kind: source and fixture.
//!
//! The crate holds no run record, evidence, signature or active-version store, so the rows that need one run in part,
//! and each test's marker names what it leaves out. The engine and the policy pins are the synthetic fixtures of
//! `tests/support`, which the crate's other tests share.

mod support;

use c600_check::{Grant, EFFECT_UNDECLARED, IR_VERSION_UNSUPPORTED, TYPE_MISMATCH};
use c600_digest::sha256;
use c600_ir::{Capability, Program, EXEC_CONTRACT, IR_VERSION};
use c600_module::code::{
    CONTRACT_MISMATCH, DEPENDENCY_MISSING, DIGEST_MISMATCH, MODEL_NOT_SUPPORTED, NONCANONICAL_VALUE, NOT_GRANTED,
    PERSISTENCE_FORMAT, PERSISTENCE_VERSION,
};
use c600_module::{
    approve, load, resolve, verify, Closure, Entry, Files, Identity, Lock, Manifest, Refusal, FULL_MODEL_ID,
};
use c600_run::boundary::{admit_preview, Answer, Place, Store};
use c600_run::meter::Outcome as Ended;
use c600_run::provenance::{self, field, hex, Author, Json, Profile};
use c600_run::value::Value;
use c600_syntax::code::{EDITION_UNSUPPORTED, UNEXPECTED_TOKEN};
use support::*;

/// The body of a root that reads its dependency's value, saves it in the workspace and returns it.
const SAVE_VALUE: &str = "export fn go() -> Int effects [workspace] {\n    let v = constant.value();\n    \
                          let saved = write_workspace<Int>(\"value\", v);\n    return v;\n}\n";

fn refusal<T>(result: Result<T, Refusal>) -> Refusal {
    match result {
        Ok(_) => panic!("expected a refusal"),
        Err(r) => r,
    }
}

fn code<T>(result: Result<T, Refusal>) -> &'static str {
    refusal(result).code
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// A module `constant` whose `value` returns `n`.
fn constant(n: i64) -> String {
    module("constant", &[], &[], &format!("export fn value() -> Int effects [] {{\n    return {n};\n}}\n"))
}

/// A module `name` whose `function` returns `alias.value()` of the given dependency bytes.
fn forwarder(name: &str, function: &str, alias: &str, dep: &str) -> String {
    let body = format!("export fn {function}() -> Int effects [] {{\n    return {alias}.value();\n}}\n");
    module(name, &[], &[(alias, dep)], &body)
}

/// A jumbling identity block: its model digest byte, menu, menu order and contract revision.
fn jumbling(model: u8, menu: &str, order: &str, rev: &str) -> Identity {
    Identity::from_json(&Json::object([
        ("contract_rev", Json::text(rev)),
        ("menu_id", Json::text(menu)),
        ("menu_order", Json::text(order)),
        ("model_id", Json::Text(hex(&[model; 32]))),
        ("puzzle", Json::text("jumbling")),
    ]))
    .unwrap()
}

/// [`package`]'s files with a manifest the caller changes, and the lock resolved for that manifest.
fn package_with(root: &str, deps: &[&str], change: impl FnOnce(&mut Manifest)) -> Package {
    let files = package(root, deps).files;
    let mut manifest = manifest_for(root.as_bytes());
    change(&mut manifest);
    let manifest = manifest.bytes();
    let lock = resolve(&manifest, &files).unwrap().bytes();
    Package { manifest, lock, files }
}

fn lock_with(lock: &Lock, change: impl FnOnce(&mut Lock)) -> Lock {
    let mut lock = lock.clone();
    change(&mut lock);
    lock
}

fn own(function: &str) -> Entry {
    entry(function, Author::Own, Profile::ToolAuthoring)
}

/// An entry whose run grants these capabilities.
fn granted(function: &str, author: Author, profile: Profile, capabilities: &[Capability]) -> Entry {
    Entry { grant: Grant::of(capabilities), ..entry(function, author, profile) }
}

fn edit(json: &Json, key: &str, value: Option<Json>) -> Json {
    let Json::Object(mut map) = json.clone() else { panic!("an object") };
    match value {
        Some(v) => map.insert(key.into(), v),
        None => map.remove(key),
    };
    Json::Object(map)
}

/// The block of a workspace record.
fn saved_block(store: &Store, name: &str) -> Vec<u8> {
    store.records[&Place::Workspace(name.into())].block.clone()
}

/// The source digest a program's block names as its producing module.
fn producer_module(block: &Json) -> String {
    let digest = block.get("producer").and_then(|p| p.get("module")).and_then(|m| m.get("sha256"));
    digest.and_then(Json::as_text).expect("a program's block").to_string()
}

/// CT-LG5-01 (whole: changed bytes under the path of the root, of a direct and of a transitive dependency fail the
/// locked digest at verification, as do changed manifest bytes under their lock, and changed modules resolved anew
/// form another closure that does not load under the approval; a locked dependency that is missing or not at its
/// location fails, verification never takes its bytes from another path, and resolution finds content by digest
/// only, never a same-named replacement)
#[test]
fn ct_lg5_01_changed_content_fails_its_digest_and_a_missing_dependency_is_not_replaced() {
    let leaf = constant(1);
    let mid = forwarder("mid", "value", "constant", &leaf);
    let root = forwarder("m", "go", "mid", &mid);
    let p = package(&root, &[&mid, &leaf]);
    let full = Identity::full();
    let (closure, approval) = approved(&p, own("go")).unwrap();

    // LG5.1: changed bytes under each module path fail the locked digest, and the refusal names the bytes found.
    for (path, text) in [("src/root.6cs", &root), ("src/dep0.6cs", &mid), ("src/dep1.6cs", &leaf)] {
        let extra = format!("{text}\nexport fn extra() -> Int effects [] {{\n    return 0;\n}}\n");
        for changed in [format!("{text}\n"), extra] {
            let files = p.files.clone().with(path, changed.as_bytes());
            let r = refusal(verify(&p.manifest, &p.lock, &files, &full));
            assert_eq!(r.code, DIGEST_MISMATCH, "{path}");
            assert!(r.detail.contains(&hex(&sha256(changed.as_bytes()))), "{}", r.detail);
        }
    }
    let mut manifest = Manifest::read(&p.manifest).unwrap();
    manifest.supports.push(jumbling(7, "menu-a", "order-a", "rev-a"));
    assert_eq!(code(verify(&manifest.bytes(), &p.lock, &p.files, &full)), DIGEST_MISMATCH, "the manifest");

    // LG5.1, LG5.10: changed modules resolved anew are another closure, which the approval does not load.
    let leaf2 = constant(2);
    let mid2 = forwarder("mid", "value", "constant", &leaf2);
    let other = package(&forwarder("m", "go", "mid", &mid2), &[&mid2, &leaf2]).verify().unwrap();
    assert_ne!(other.digest, closure.digest);
    assert_eq!(code(load(&other, &approval)), DIGEST_MISMATCH);

    // LG5.3, LG5.9: an unavailable locked dependency fails, and nothing replaces it.
    let mut no_leaf = p.files.clone();
    no_leaf.entries.remove("src/dep1.6cs");
    assert_eq!(code(verify(&p.manifest, &p.lock, &no_leaf, &full)), DEPENDENCY_MISSING, "removed");
    assert_eq!(code(resolve(&p.manifest, &no_leaf)), DEPENDENCY_MISSING, "removed");
    let replaced = no_leaf.clone().with("vendor/constant.6cs", leaf2.as_bytes());
    assert_eq!(code(verify(&p.manifest, &p.lock, &replaced, &full)), DEPENDENCY_MISSING, "a same-named module");
    assert_eq!(code(resolve(&p.manifest, &replaced)), DEPENDENCY_MISSING, "a same-named module");
    let moved = no_leaf.clone().with("vendor/constant.6cs", leaf.as_bytes());
    assert_eq!(code(verify(&p.manifest, &p.lock, &moved, &full)), DEPENDENCY_MISSING, "not at its location");
    assert_eq!(resolve(&p.manifest, &moved).unwrap().closure_digest(), closure.digest, "found again by digest");
    let mut no_mid = p.files.clone();
    no_mid.entries.remove("src/dep0.6cs");
    assert_eq!(code(verify(&p.manifest, &p.lock, &no_mid, &full)), DEPENDENCY_MISSING, "the direct dependency");
    assert_eq!(code(resolve(&p.manifest, &no_mid)), DEPENDENCY_MISSING, "the direct dependency");

    // The addressed bytes still run under their approval.
    let ready = load(&closure, &approval).unwrap();
    let (mut engine, mut store) = (Engine::fixture(), Store::default());
    let out = run(&ready, "run", vec![], vec![], &mut engine, &mut store).unwrap();
    assert_eq!(out.run.output.as_deref(), Some(&encoded(&Value::int(1))[..]));
}

/// CT-LG5-02 (part: a file read in a direct dependency, or in one reached through another module, is the root's
/// requirement: undeclared or behind an effect-free signature it is refused, and declared it is refused until the run
/// grants it, for own, built-in and imported entries; the manifest's capabilities must equal the root header's; a
/// declared and granted network capability stays refused; a session identity outside `supports`
/// is refused, and one inside it grants nothing; the manifest's, a locked module's and a source's edition are checked;
/// a boundary class and a principle record read back as data, are refused in the wrong form, and leave every author's
/// admission of a previewer as it was; not run: a check of a declared boundary class against chapter 07's reviewed
/// classes, since the crate reads only the field's form, its built-in approval takes no class rows, and the checker's
/// built-in admission takes its class rows from the caller, not from a manifest)
#[test]
fn ct_lg5_02_manifest_entries_are_checked_as_data_and_grant_nothing() {
    let full = Identity::full();
    let j = jumbling(7, "menu-a", "order-a", "rev-a");
    let file = [Capability::File];
    let reader = module(
        "reader",
        &["file"],
        &[],
        "export fn notes() -> Text effects [file] {\n    return file_read<Text>(\"notes\");\n}\n",
    );
    let root = |requires: &[&str]| {
        let body = "export fn go() -> Text effects [file] {\n    return reader.notes();\n}\n";
        module("m", requires, &[("reader", &reader)], body)
    };
    let mid = module(
        "mid",
        &["file"],
        &[("reader", &reader)],
        "export fn notes() -> Text effects [file] {\n    return reader.notes();\n}\n",
    );
    let through = |requires: &[&str]| {
        let body = "export fn go() -> Text effects [file] {\n    return mid.notes();\n}\n";
        module("m", requires, &[("mid", &mid)], body)
    };
    let authors = [Author::Own, Author::BuiltIn, Author::Imported];

    // LG5.2, LG5.7, LG3.9: a requirement anywhere in the closure is the root's, an import does not make it pure, and a
    // declaration is no grant.
    let pure = |source: String| source.replace("effects [file]", "effects []");
    let declared = package(&root(&["file"]), &[&reader]);
    let chains = [
        (package(&root(&[]), &[&reader]), package(&pure(root(&[])), &[&reader]), declared.clone()),
        (
            package(&through(&[]), &[&mid, &reader]),
            package(&pure(through(&[])), &[&mid, &reader]),
            package(&through(&["file"]), &[&mid, &reader]),
        ),
    ];
    for (undeclared, hidden, declared) in &chains {
        for author in authors {
            let e = granted("go", author, Profile::ToolAuthoring, &file);
            assert_eq!(code(approved(undeclared, e.clone())), EFFECT_UNDECLARED, "{author:?}");
            assert_eq!(code(approved(hidden, e.clone())), EFFECT_UNDECLARED, "{author:?}: an effect-free signature");
            let ungranted = entry("go", author, Profile::ToolAuthoring);
            assert_eq!(code(approved(declared, ungranted)), NOT_GRANTED, "{author:?}");
            assert!(approved(declared, e).is_ok(), "{author:?}");
        }
    }

    // The manifest's capabilities are the root header's, no more and no fewer.
    let relocked = |m: Manifest| {
        let bytes = m.bytes();
        let lock = resolve(&bytes, &declared.files).unwrap().bytes();
        verify(&bytes, &lock, &declared.files, &full)
    };
    for capabilities in [vec![], vec!["clock".to_string(), "file".to_string()]] {
        let m = Manifest { capabilities, ..manifest_for(root(&["file"]).as_bytes()) };
        assert_eq!(code(relocked(m)), CONTRACT_MISMATCH);
    }

    // LG3.8: a never-granted capability stays refused, declared by the manifest and the header and granted by the run.
    let network = package(&root(&["file", "network"]), &[&reader]);
    assert!(network.verify().is_ok(), "the declaration is well-formed data");
    for author in authors {
        let e = granted("go", author, Profile::ToolAuthoring, &[Capability::File, Capability::Network]);
        assert_eq!(code(approved(&network, e)), NOT_GRANTED, "{author:?}");
    }

    // LG5.2, LG5.5: compatibility is checked against the session, and listing an identity grants nothing.
    assert_eq!(code(verify(&declared.manifest, &declared.lock, &declared.files, &j)), MODEL_NOT_SUPPORTED);
    let both = package_with(&root(&["file"]), &[&reader], |m| m.supports.push(j.clone()));
    for session in [&full, &j] {
        let closure = verify(&both.manifest, &both.lock, &both.files, session).unwrap();
        assert_eq!(code(approve(&closure, own("go"), contract(), checker())), NOT_GRANTED);
        let e = granted("go", Author::Own, Profile::ToolAuthoring, &file);
        assert!(approve(&closure, e, contract(), checker()).is_ok());
    }

    // LG5.2, LG5.3: the edition of the manifest, of a locked module and of a source header.
    let m = Manifest { edition: "6cs-draft-0".into(), ..manifest_for(root(&["file"]).as_bytes()) };
    assert_eq!(code(relocked(m)), CONTRACT_MISMATCH, "the manifest's edition");
    let lock = lock_with(&Lock::read(&declared.lock).unwrap(), |l| l.modules[0].edition = "6cs-draft-0".into());
    let r = verify(&declared.manifest, &lock.bytes(), &declared.files, &full);
    assert_eq!(code(r), CONTRACT_MISMATCH, "a locked edition");
    let old = root(&["file"]).replace(EDITION, "6cs-draft-0");
    let m = Manifest { source: sha256(old.as_bytes()), ..manifest_for(root(&["file"]).as_bytes()) };
    let files = declared.files.clone().with("src/old.6cs", old.as_bytes());
    assert_eq!(code(resolve(&m.bytes(), &files)), EDITION_UNSUPPORTED, "a source's edition");

    // LG5.2: a boundary class and a principle record are data: read back, refused in the wrong form, and no grant.
    let previewer = module(
        "m",
        &[],
        &[],
        "export fn go(w: Word, t: Target) -> Text effects [preview] {\n    return receipt_id(propose(w, t));\n}\n",
    );
    let principle = sha256(b"a principle record");
    let plain = package(&previewer, &[]);
    let declaring = package_with(&previewer, &[], |m| {
        m.boundary = Some("analysis".into());
        m.principle = Some(principle);
    });
    let read = Manifest::read(&declaring.manifest).unwrap();
    assert_eq!((read.boundary.as_deref(), read.principle), (Some("analysis"), Some(principle)));
    for author in [Author::Own, Author::ForkedBuiltIn, Author::Imported, Author::BuiltIn] {
        let admitted = |p: &Package| {
            approved(p, entry("go", author, Profile::Solving)).map(|(_, a)| a.admitted).map_err(|r| r.code)
        };
        let (without, with) = (admitted(&plain), admitted(&declaring));
        assert_eq!(with, without, "{author:?}");
        assert_eq!(with.is_ok(), matches!(author, Author::BuiltIn), "{author:?}");
    }
    let json = manifest_for(previewer.as_bytes()).json();
    for (key, value) in [("boundary", Json::Int(1)), ("principle", Json::text("analysis"))] {
        let payload = edit(json.get("payload").unwrap(), key, Some(value));
        let bytes = provenance::canonical(&edit(&json, "payload", Some(payload)));
        assert_eq!(code(Manifest::read(&bytes)), PERSISTENCE_FORMAT, "{key}");
    }
}

/// CT-LG5-03 (part: the approval a run executes under names the session identity, the closure, manifest and lock
/// digests, every module digest, the IR version and contract and the pins, and the run's stored output names the
/// module that executed and its entry; the closure digest covers each module's edition and IR contract and the
/// resolver, not a location; verifying again refuses a missing dependency, a locked edition, IR version, IR contract
/// or resolver this runtime does not run, and a source in an unsupported edition; the checker, the interpreter and
/// the loader refuse another IR version, and a substitute closure does not load; not run: chapter 05's run record and
/// chapter 09's reproduction key, and a replay from them, which wait for M6's run records)
#[test]
fn ct_lg5_03_the_approval_names_the_closure_and_a_rerun_refuses_missing_or_unsupported_versions() {
    let dep = constant(1);
    let root = module("m", &[], &[("constant", &dep)], SAVE_VALUE);
    let p = package(&root, &[&dep]);
    let full = Identity::full();
    let (closure, approval) = approved(&p, own("go")).unwrap();

    // LG5.4: the approval names the closure the run executes and the contracts it is interpreted under.
    let bytes = approval.bytes();
    let digests =
        [closure.digest, closure.manifest_digest, closure.lock_digest, sha256(root.as_bytes()), sha256(dep.as_bytes())];
    assert!(digests.iter().all(|d| contains(&bytes, hex(d).as_bytes())));
    assert!([IR_VERSION, EXEC_CONTRACT, FULL_MODEL_ID].iter().all(|t| contains(&bytes, t.as_bytes())));
    assert_eq!((&approval.identity, &approval.contract, &approval.checker), (&full, &contract(), &checker()));
    let ready = load(&closure, &approval).unwrap();
    let (mut engine, mut store) = (Engine::fixture(), Store::default());
    let out = run(&ready, "run", vec![], vec![], &mut engine, &mut store).unwrap();
    assert_eq!(out.run.output.as_deref(), Some(&encoded(&Value::int(1))[..]));
    let block = provenance::read(&saved_block(&store, "value")).unwrap();
    assert_eq!(producer_module(&block), hex(&sha256(root.as_bytes())));
    assert_eq!(block.get("producer").and_then(|p| p.get("entry")).and_then(Json::as_text), Some("go"));

    // LG5.3: the closure digest covers editions, IR contracts and the resolver; a location only finds bytes.
    let lock = Lock::read(&p.lock).unwrap();
    assert_eq!(lock.closure_digest(), closure.digest);
    let moved = lock_with(&lock, |l| l.locations[0].path = "vendor/moved.6cs".into());
    assert_eq!(moved.closure_digest(), closure.digest, "a location");
    let unsupported = [
        ("an edition", lock_with(&lock, |l| l.modules[0].edition = "6cs-draft-0".into())),
        ("an IR version", lock_with(&lock, |l| l.modules[0].ir_version = "c600-ir-draft-0".into())),
        ("an IR contract", lock_with(&lock, |l| l.modules[0].exec_contract = "lp1-exec-draft-0".into())),
        ("a resolver", lock_with(&lock, |l| l.resolver = "c600-resolver-draft-0".into())),
    ];
    for (what, l) in unsupported {
        assert_ne!(l.closure_digest(), closure.digest, "{what}");
        assert_eq!(code(verify(&p.manifest, &l.bytes(), &p.files, &full)), CONTRACT_MISMATCH, "{what}");
    }

    // LG5.3, LG5.4: running again refuses missing content and versions this runtime does not run.
    let mut missing = p.files.clone();
    missing.entries.remove("src/dep0.6cs");
    assert_eq!(code(verify(&p.manifest, &p.lock, &missing, &full)), DEPENDENCY_MISSING);
    let old = root.replace(EDITION, "6cs-draft-0");
    let m = Manifest { source: sha256(old.as_bytes()), ..Manifest::read(&p.manifest).unwrap() };
    let files = p.files.clone().with("src/old.6cs", old.as_bytes());
    assert_eq!(code(resolve(&m.bytes(), &files)), EDITION_UNSUPPORTED);
    let program = Program { ir_version: "c600-ir-draft-0".into(), ..closure.program.clone() };
    assert_eq!(code(c600_run::interp::load(&program)), IR_VERSION_UNSUPPORTED, "the interpreter");
    let stale = Closure { program, ..closure.clone() };
    assert_eq!(code(load(&stale, &approval)), IR_VERSION_UNSUPPORTED, "the loader");
    assert_eq!(code(approve(&stale, own("go"), contract(), checker())), IR_VERSION_UNSUPPORTED, "the checker");

    // LG5.4: a substitute module never runs under the approval.
    let two = constant(2);
    let substitute = package(&module("m", &[], &[("constant", &two)], SAVE_VALUE), &[&two]).verify().unwrap();
    assert_eq!(code(load(&substitute, &approval)), DIGEST_MISMATCH);
}

/// CT-LG5-04 (part: a module runs under each identity its manifest lists, and its literal word reaches the engine as
/// written; a session of another puzzle, or one that matches a listed identity except in the model digest, the menu,
/// the menu order or the contract revision, is refused at verification, before any word or constant of the module
/// runs, and an approval made under one listed identity does not load the closure verified under another; not run: a
/// check at the run itself, since a run request names no session identity and a loaded entry runs against whatever
/// engine the caller passes, and keeping a refused module as recorded material for inspection, which no crate builds)
#[test]
fn ct_lg5_04_a_model_or_menu_order_mismatch_refuses_the_module_before_it_runs() {
    let source = module(
        "m",
        &[],
        &[],
        "export fn go(t: Target) -> Text effects [preview] {\n    return receipt_id(propose(word[1, 2], t));\n}\n",
    );
    let j = jumbling(7, "menu-a", "order-a", "rev-a");
    let p = package_with(&source, &[], |m| m.supports.push(j.clone()));
    let built_in = entry("go", Author::BuiltIn, Profile::Solving);

    // Under each listed identity the module runs, and its word reaches the engine as written.
    let mut closures = Vec::new();
    for session in [Identity::full(), j.clone()] {
        let closure = verify(&p.manifest, &p.lock, &p.files, &session).unwrap();
        let approval = approve(&closure, built_in.clone(), contract(), checker()).unwrap();
        let ready = load(&closure, &approval).unwrap();
        let (mut engine, mut store) = (Engine::fixture(), Store::default());
        let out = run(&ready, "run", vec![Value::target("c0")], vec![manual()], &mut engine, &mut store).unwrap();
        assert_eq!(out.run.output.as_deref(), Some(&b"\"run/1\"\n"[..]));
        assert_eq!(engine.previews, [(vec![A, B], "c0".to_string())]);
        closures.push(closure);
    }

    // LG5.5: a session the manifest does not list is refused, whichever field differs.
    let only_j = package_with(&source, &[], |m| m.supports = vec![j.clone()]);
    assert_eq!(code(only_j.verify()), MODEL_NOT_SUPPORTED, "another puzzle");
    let others = [
        ("the model digest", jumbling(8, "menu-a", "order-a", "rev-a")),
        ("the menu", jumbling(7, "menu-b", "order-a", "rev-a")),
        ("the menu order", jumbling(7, "menu-a", "order-b", "rev-a")),
        ("the contract revision", jumbling(7, "menu-a", "order-a", "rev-b")),
    ];
    for (what, session) in others {
        assert_eq!(code(verify(&p.manifest, &p.lock, &p.files, &session)), MODEL_NOT_SUPPORTED, "{what}");
    }

    // An approval under one identity does not load the same closure verified under another.
    let under_full = approve(&closures[0], built_in, contract(), checker()).unwrap();
    assert_eq!(closures[1].digest, closures[0].digest);
    assert_eq!(code(load(&closures[1], &under_full)), MODEL_NOT_SUPPORTED);
}

/// CT-LG5-05 (part: beside a loaded active version, drafts are refused before they could replace it: source drafts
/// that fail parsing, typing, effects or a capability grant, manifest drafts with an unlisted identity, other
/// capabilities or noncanonical bytes, and dependency drafts with edited, broken or missing content, where the refusal
/// of edited content names the draft's digest and the locked one; the active version then runs under its unchanged
/// approval and its output names its own module digest, and a valid draft is another version that the active approval
/// does not load and whose own approval names its digest; not run: chapter 04's editor services, EV1, which keep the
/// active version and activate a candidate only after its checks, and which no crate builds yet, so the test holds
/// the active version itself)
#[test]
fn ct_lg5_05_broken_drafts_leave_the_active_version_running_under_its_own_approval() {
    let dep = constant(1);
    let root = module("m", &[], &[("constant", &dep)], SAVE_VALUE);
    let p = package(&root, &[&dep]);
    let active = ready(&p, own("go")).unwrap();
    let approval = active.approval().clone();
    let manifest = manifest_for(root.as_bytes());
    let beside = |path: &str, text: &str| p.files.clone().with(path, text.as_bytes());

    // LG5.6: parsing, typing, effects and capability requirements are checked for a source draft.
    let unparsed = root.replace("return v;", "return v");
    let m = Manifest { source: sha256(unparsed.as_bytes()), ..manifest.clone() };
    assert_eq!(code(resolve(&m.bytes(), &beside("drafts/m.6cs", &unparsed))), UNEXPECTED_TOKEN, "parsing");
    let mistyped = module("m", &[], &[("constant", &dep)], &SAVE_VALUE.replace("go() -> Int", "go() -> Text"));
    assert_eq!(code(approved(&package(&mistyped, &[&dep]), own("go"))), TYPE_MISMATCH, "typing");
    let undeclared = module("m", &[], &[("constant", &dep)], &SAVE_VALUE.replace("[workspace]", "[]"));
    assert_eq!(code(approved(&package(&undeclared, &[&dep]), own("go"))), EFFECT_UNDECLARED, "effects");
    let reads = module(
        "m",
        &["file"],
        &[("constant", &dep)],
        "export fn go() -> Int effects [file] {\n    let v = constant.value();\n    \
         let notes = file_read<Text>(\"notes\");\n    return v;\n}\n",
    );
    assert_eq!(code(approved(&package(&reads, &[&dep]), own("go"))), NOT_GRANTED, "a capability");

    // Manifest drafts: an unlisted identity, other capabilities, noncanonical bytes.
    let unlisted = package_with(&root, &[&dep], |m| m.supports = vec![jumbling(7, "menu-a", "order-a", "rev-a")]);
    assert_eq!(code(unlisted.verify()), MODEL_NOT_SUPPORTED, "an identity");
    let claims = package_with(&root, &[&dep], |m| m.capabilities = vec!["file".into()]);
    assert_eq!(code(claims.verify()), CONTRACT_MISMATCH, "capabilities");
    let mut spaced = p.manifest.clone();
    spaced.insert(1, b' ');
    assert_eq!(code(resolve(&spaced, &p.files)), NONCANONICAL_VALUE, "noncanonical bytes");
    assert_eq!(code(verify(&spaced, &p.lock, &p.files, &Identity::full())), NONCANONICAL_VALUE, "noncanonical bytes");

    // Dependency drafts: edited bytes under the locked path, a broken dependency, an import with no local content.
    let edited = constant(2);
    let r = refusal(verify(&p.manifest, &p.lock, &beside("src/dep0.6cs", &edited), &Identity::full()));
    assert_eq!(r.code, DIGEST_MISMATCH, "edited dependency bytes");
    let (draft, locked) = (hex(&sha256(edited.as_bytes())), hex(&sha256(dep.as_bytes())));
    assert!(r.detail.contains(&draft) && r.detail.contains(&locked), "{}", r.detail);
    let broken = edited.replace("return 2;", "return 2");
    let on_broken = module("m", &[], &[("constant", &broken)], SAVE_VALUE);
    let files = beside("drafts/m.6cs", &on_broken).with("drafts/constant.6cs", broken.as_bytes());
    assert_eq!(code(resolve(&manifest_for(on_broken.as_bytes()).bytes(), &files)), UNEXPECTED_TOKEN, "broken");
    let absent = constant(3);
    let on_absent = module("m", &[], &[("constant", &absent)], SAVE_VALUE);
    let files = beside("drafts/m.6cs", &on_absent);
    assert_eq!(code(resolve(&manifest_for(on_absent.as_bytes()).bytes(), &files)), DEPENDENCY_MISSING, "absent");

    // The active version runs under its unchanged approval, and its output names its own module digest.
    assert_eq!(active.approval(), &approval);
    assert_eq!(approval.module, sha256(root.as_bytes()));
    let (mut engine, mut store) = (Engine::fixture(), Store::default());
    let out = run(&active, "run", vec![], vec![], &mut engine, &mut store).unwrap();
    assert_eq!(out.run.output.as_deref(), Some(&encoded(&Value::int(1))[..]));
    let block = provenance::read(&saved_block(&store, "value")).unwrap();
    assert_eq!(producer_module(&block), hex(&approval.module));

    // A valid draft is another version: the active approval does not load it, and its own approval names its digest.
    let next = constant(2);
    let next_root = module("m", &[], &[("constant", &next)], SAVE_VALUE);
    let q = package(&next_root, &[&next]);
    assert_eq!(code(load(&q.verify().unwrap(), &approval)), DIGEST_MISMATCH);
    let (_, next_approval) = approved(&q, own("go")).unwrap();
    assert_eq!(next_approval.module, sha256(next_root.as_bytes()));
    assert_ne!(next_approval.module, approval.module);
}

/// CT-LG5-06 (part: a fork with a changed body has a new source, manifest and closure digest, and the parent's
/// approval does not load it; as a forked built-in it previews nothing where the parent previews, its file read is
/// refused without the run's grant and approved with it, its stored output names the fork's digest and author class
/// and is refused at preview admission where the parent's output passes, and naming a principle record in its
/// manifest changes none of this; not run: evidence and chapter 06's checks, which the crate neither holds nor depends
/// on, so inheriting no evidence and refusing the parent's check on the fork's subject are not exercised, and chapter
/// 07's classification of a fork, which the caller supplies as the entry's author)
#[test]
fn ct_lg5_06_a_changed_fork_is_a_new_module_that_needs_its_own_grants() {
    let source = |place: &str| {
        let body = format!(
            "export fn preview_word(w: Word, t: Target) -> Text effects [preview] {{\n    \
             return receipt_id(propose(w, t));\n}}\n\n\
             export fn save(w: Word) -> Unit effects [workspace] {{\n    \
             return write_workspace<Word>(\"{place}\", w);\n}}\n\n\
             export fn notes() -> Text effects [file] {{\n    return file_read<Text>(\"notes\");\n}}\n"
        );
        module("m", &["file"], &[], &body)
    };
    let (parent, fork) = (source("saved"), source("kept"));
    let (pp, fp) = (package(&parent, &[]), package(&fork, &[]));
    let (parent_closure, fork_closure) = (pp.verify().unwrap(), fp.verify().unwrap());
    assert_ne!(sha256(fork.as_bytes()), sha256(parent.as_bytes()));
    assert_ne!(sha256(&fp.manifest), sha256(&pp.manifest));
    assert_ne!(fork_closure.digest, parent_closure.digest);
    let previewer = entry("preview_word", Author::BuiltIn, Profile::Solving);
    let parent_approval = approve(&parent_closure, previewer, contract(), checker()).unwrap();
    assert_eq!(code(load(&fork_closure, &parent_approval)), DIGEST_MISMATCH, "the parent's approval");

    // The parent previews the word the solver names; the fork, a forked built-in, prepares no preview.
    let parent_ready = load(&parent_closure, &parent_approval).unwrap();
    let (mut engine, mut store) = (Engine::fixture(), Store::default());
    let args = vec![Value::word(&[A]), Value::target("c0")];
    let out = run(&parent_ready, "run", args, vec![manual(), manual()], &mut engine, &mut store).unwrap();
    assert!(matches!(&out.shell[0].answer, Answer::Previewed(_)));
    assert_eq!(engine.previews, [(vec![A], "c0".to_string())]);
    let forked = entry("preview_word", Author::ForkedBuiltIn, Profile::Solving);
    assert_eq!(code(approve(&fork_closure, forked.clone(), contract(), checker())), NOT_GRANTED);

    // LG5.8: running the fork still needs the run's grant.
    for (closure, author) in [(&parent_closure, Author::BuiltIn), (&fork_closure, Author::ForkedBuiltIn)] {
        let refused = approve(closure, entry("notes", author, Profile::ToolAuthoring), contract(), checker());
        assert_eq!(code(refused), NOT_GRANTED, "{author:?}");
    }
    let file = granted("notes", Author::ForkedBuiltIn, Profile::ToolAuthoring, &[Capability::File]);
    assert!(approve(&fork_closure, file, contract(), checker()).is_ok());

    // The fork's output names the fork and its class, and preview admission refuses it; the parent's output passes.
    let save = |p: &Package, author: Author, place: &str| {
        let ready = ready(p, entry("save", author, Profile::ToolAuthoring)).unwrap();
        let mut store = Store::default();
        let args = vec![Value::word(&[A])];
        let out = run(&ready, "save", args, vec![manual()], &mut Engine::fixture(), &mut store).unwrap();
        assert_eq!(out.run.summary.outcome, Ended::Completed, "{author:?}");
        (saved_block(&store, place), store)
    };
    let (block, store) = save(&pp, Author::BuiltIn, "saved");
    assert!(admit_preview(&block, &store, &pins()).is_ok(), "the parent's output");
    let (block, store) = save(&fp, Author::ForkedBuiltIn, "kept");
    let read = provenance::read(&block).unwrap();
    assert_eq!((field::author(&read), producer_module(&read)), ("forked_built_in", hex(&sha256(fork.as_bytes()))));
    assert_eq!(code(admit_preview(&block, &store, &pins())), NOT_GRANTED, "the fork's output");

    // Naming a principle record in the fork's manifest grants nothing.
    let cited = package_with(&fork, &[], |m| m.principle = Some(sha256(b"the parent's principle record")));
    assert_eq!(code(approved(&cited, forked)), NOT_GRANTED);
}

/// CT-LG5-07 (part: shared module files verify to the sender's closure under the sender's lock at the same paths,
/// and at other paths the sender's lock is refused while a new resolution finds the same closure by digest; a copy
/// changed in transit, a lock that fixes other bytes or names another resolver, a manifest edited under its lock and
/// a session identity the manifest does not list are refused; an imported module gets no grant from the sender, and
/// the recipient's approval needs the recipient's grant; a signature-shaped optional manifest extension is kept as
/// data and changes no approval decision, and a required one is refused; not run: a module signature and its
/// verification, which no crate defines, and the evidence half, since the crate holds no evidence)
#[test]
fn ct_lg5_07_sharing_through_local_files_keeps_every_check_and_a_signature_grants_nothing() {
    let dep = constant(1);
    let body = "export fn go() -> Int effects [] {\n    return constant.value();\n}\n\n\
                export fn notes() -> Text effects [file] {\n    return file_read<Text>(\"notes\");\n}\n";
    let root = module("m", &["file"], &[("constant", &dep)], body);
    let shared = package(&root, &[&dep]);
    let full = Identity::full();
    let file = [Capability::File];
    let (_, sent) = approved(&shared, granted("notes", Author::Own, Profile::ToolAuthoring, &file)).unwrap();

    // LG5.9: the recipient's local files, at the sender's paths and at others.
    let same = Files::new().with("src/root.6cs", root.as_bytes()).with("src/dep0.6cs", dep.as_bytes());
    assert_eq!(verify(&shared.manifest, &shared.lock, &same, &full).unwrap().digest, sent.closure);
    let incoming = Files::new().with("incoming/m.6cs", root.as_bytes()).with("incoming/constant.6cs", dep.as_bytes());
    assert_eq!(code(verify(&shared.manifest, &shared.lock, &incoming, &full)), DEPENDENCY_MISSING);
    let lock = resolve(&shared.manifest, &incoming).unwrap().bytes();
    assert_ne!(lock, shared.lock);
    let received = verify(&shared.manifest, &lock, &incoming, &full).unwrap();
    assert_eq!((received.digest, received.manifest_digest), (sent.closure, sent.manifest));

    // The digest, lock and identity checks hold for shared files.
    let tampered = constant(7);
    let changed = same.clone().with("src/dep0.6cs", tampered.as_bytes());
    assert_eq!(code(verify(&shared.manifest, &shared.lock, &changed, &full)), DIGEST_MISMATCH, "changed in transit");
    let changed_incoming = incoming.clone().with("incoming/constant.6cs", tampered.as_bytes());
    assert_eq!(code(resolve(&shared.manifest, &changed_incoming)), DEPENDENCY_MISSING, "changed in transit");
    let (old, new) = (sha256(dep.as_bytes()), sha256(tampered.as_bytes()));
    let swapped = lock_with(&Lock::read(&shared.lock).unwrap(), |l| {
        l.modules.iter_mut().filter(|m| m.source == old).for_each(|m| m.source = new);
        l.locations.iter_mut().filter(|x| x.source == old).for_each(|x| x.source = new);
    });
    let r = verify(&shared.manifest, &swapped.bytes(), &changed, &full);
    assert_eq!(code(r), DEPENDENCY_MISSING, "a lock that fixes other bytes");
    let renamed = lock_with(&Lock::read(&shared.lock).unwrap(), |l| l.resolver = "another-resolver".into());
    assert_eq!(code(verify(&shared.manifest, &renamed.bytes(), &same, &full)), CONTRACT_MISMATCH, "a resolver");
    let edited = Manifest { capabilities: Vec::new(), ..Manifest::read(&shared.manifest).unwrap() };
    assert_eq!(code(verify(&edited.bytes(), &shared.lock, &same, &full)), DIGEST_MISMATCH, "an edited manifest");
    let session = jumbling(7, "menu-a", "order-a", "rev-a");
    assert_eq!(code(verify(&shared.manifest, &shared.lock, &same, &session)), MODEL_NOT_SUPPORTED, "an identity");

    // LG5.9, LG3.9: the recipient's approval of an imported module needs the recipient's grant.
    let ungranted = entry("notes", Author::Imported, Profile::ToolAuthoring);
    assert_eq!(code(approve(&received, ungranted.clone(), contract(), checker())), NOT_GRANTED);
    let imported = granted("notes", Author::Imported, Profile::ToolAuthoring, &file);
    let accepted = approve(&received, imported.clone(), contract(), checker()).unwrap();
    assert_eq!((accepted.closure, accepted.lock), (sent.closure, sha256(&lock)));

    // A signature-shaped optional extension is kept as data and changes no decision; a required one is refused.
    let json = Manifest::read(&shared.manifest).unwrap().json();
    let signed = |required: bool| {
        let signature = Json::object([
            ("name", Json::text("x-signature")),
            ("required", Json::Bool(required)),
            ("value", Json::text("a detached signature over the manifest")),
        ]);
        provenance::canonical(&edit(&json, "extensions", Some(Json::List(vec![signature]))))
    };
    let manifest = signed(false);
    assert_eq!(Manifest::read(&manifest).unwrap(), Manifest::read(&shared.manifest).unwrap());
    let lock = resolve(&manifest, &incoming).unwrap().bytes();
    let closure = verify(&manifest, &lock, &incoming, &full).unwrap();
    for e in [ungranted, imported] {
        let decide = |c: &Closure| approve(c, e.clone(), contract(), checker()).map(|a| a.admitted).map_err(|r| r.code);
        assert_eq!(decide(&closure), decide(&received));
    }
    assert_eq!(code(Manifest::read(&signed(true))), PERSISTENCE_VERSION);
}
