//! CT-RTE-08, source leg (chapter 02 LG5, chapter 07 BND.39 to BND.43 and BND.84, RTE.45): an approval stays bound to
//! the dependency bytes that execute; substituted bytes and dynamic code are refused; a callee that reads state makes
//! its caller non-previewing; a previewer gets only `{base, id}`; a stored word keeps its provenance block and preview
//! admission refuses it. The module, branch, preview-outcome and persisted-word cases keep BM2.
//!
//! The persisted-word cases use the in-memory store of `c600-run`. The durable leg, a word read back from M6's
//! session store, runs in `c600-durable`'s tests (C7, `ct_rte_08_persisted_word.rs`).

mod support;

use c600_digest::sha256;
use c600_ir::{Callee, Expr, Lit, Program, QTy, Ty};
use c600_module::code::{
    CONTRACT_MISMATCH, DEPENDENCY_MISSING, DIGEST_MISMATCH, IR_REJECTED, IR_VERSION_UNSUPPORTED, MODEL_NOT_SUPPORTED,
    NONCANONICAL_VALUE, NOT_GRANTED, PERSISTENCE_FORMAT, PERSISTENCE_VERSION,
};
use c600_module::{approve, load, resolve, verify, Closure, Files, Identity, Lock, Manifest, RunRequest};
use c600_run::boundary::{admit_preview, Answer, Place, Store};
use c600_run::interp::Options;
use c600_run::menu::{MenuAlphabet, MenuIdentity};
use c600_run::meter::Outcome as Ended;
use c600_run::provenance::{self, field, hex, Author, Json, Profile};
use c600_run::value::Value;
use support::*;

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// S02's dependency: a module `constant` whose `value` returns `n`.
fn constant(n: i64) -> String {
    module("constant", &[], &[], &format!("export fn value() -> Int effects [] {{\n    return {n};\n}}\n"))
}

/// The root `m` that calls `constant.value()` from the given dependency bytes.
fn caller(dep: &str) -> String {
    module("m", &[], &[("constant", dep)], "export fn go() -> Int effects [] {\n    return constant.value();\n}\n")
}

fn own_tool(function: &str) -> c600_module::Entry {
    entry(function, Author::Own, Profile::ToolAuthoring)
}

fn solving(function: &str) -> c600_module::Entry {
    entry(function, Author::BuiltIn, Profile::Solving)
}

/// A built-in entry of a package, approved and loaded for the solving profile.
fn previewer(p: &Package, function: &str) -> c600_module::Ready {
    ready(p, solving(function)).unwrap()
}

fn edit(json: &Json, key: &str, value: Option<Json>) -> Json {
    let Json::Object(mut map) = json.clone() else { panic!("an object") };
    match value {
        Some(v) => map.insert(key.into(), v),
        None => map.remove(key),
    };
    Json::Object(map)
}

fn code<T>(result: Result<T, c600_module::Refusal>) -> &'static str {
    match result {
        Ok(_) => panic!("expected a refusal"),
        Err(r) => r.code,
    }
}

/// LG5.1, LG5.3, LG5.6, RTE.45: the approval names every module digest of the closure and the closure digest, and the
/// bytes that were checked are the bytes that run.
#[test]
fn ct_rte_08_approval_binds_every_executed_module() {
    let dep = constant(1);
    let root = caller(&dep);
    let p = package(&root, &[&dep]);
    let (closure, approval) = approved(&p, own_tool("go")).unwrap();
    let mut digests = vec![sha256(root.as_bytes()), sha256(dep.as_bytes())];
    digests.sort();
    assert_eq!(approval.modules, digests);
    assert_eq!(approval.module, sha256(root.as_bytes()));
    assert_eq!((approval.closure, approval.manifest), (closure.digest, sha256(&p.manifest)));
    assert_eq!(approval.closure, Lock::read(&p.lock).unwrap().closure_digest());
    let bytes = approval.bytes();
    assert!(digests.iter().all(|d| contains(&bytes, hex(d).as_bytes())));
    assert!(contains(&bytes, hex(&closure.digest).as_bytes()));
    assert_eq!(approval.digest(), sha256(&bytes));

    let ready = load(&closure, &approval).unwrap();
    let (mut engine, mut store) = (Engine::fixture(), Store::default());
    let out = run(&ready, "run", vec![], vec![], &mut engine, &mut store).unwrap();
    assert_eq!(out.run.output.as_deref(), Some(&encoded(&Value::int(1))[..]));

    // Another dependency body is another closure.
    let other = constant(2);
    assert_ne!(package(&caller(&other), &[&other]).verify().unwrap().digest, closure.digest);
}

/// S02 (`approval.json`), LG5.1, LG5.6: the host replaces the dependency's `return 1` with `return 2` after approval.
/// The locked digest refuses the bytes; a root rewritten to import them is another closure, which the old approval
/// refuses at load. The approved bytes still run, and no refusal changes the engine or the store.
#[test]
fn ct_rte_08_substituted_dependency_bytes_never_run() {
    let dep = constant(1);
    let p = package(&caller(&dep), &[&dep]);
    let (closure, approval) = approved(&p, own_tool("go")).unwrap();
    let (mut engine, mut store) = (Engine::fixture(), Store::default());
    let before = (engine.digest(), store_json(&store));

    let swapped = constant(2);
    let files = p.files.clone().with("src/dep0.6cs", swapped.as_bytes());
    assert_eq!(code(verify(&p.manifest, &p.lock, &files, &Identity::full())), DIGEST_MISMATCH);
    assert_eq!(code(resolve(&p.manifest, &files)), DEPENDENCY_MISSING, "no local file has the locked digest");

    let rewritten = package(&caller(&swapped), &[&swapped]);
    let other = rewritten.verify().unwrap();
    assert_ne!(other.digest, closure.digest);
    assert_eq!(code(load(&other, &approval)), DIGEST_MISMATCH);
    assert_eq!((engine.digest(), store_json(&store)), before);

    let ready = load(&closure, &approval).unwrap();
    let out = run(&ready, "run", vec![], vec![], &mut engine, &mut store).unwrap();
    assert_eq!(out.run.output.as_deref(), Some(&encoded(&Value::int(1))[..]));
}

/// LG5.3, LG5.9: content the locked closure needs and the local files lack is refused, by resolution and by
/// verification, and so is a lock that drops the dependency's location or the dependency itself.
#[test]
fn ct_rte_08_a_missing_dependency_is_refused() {
    let dep = constant(1);
    let root = caller(&dep);
    let p = package(&root, &[&dep]);
    let only_root = Files::new().with("src/root.6cs", root.as_bytes());
    assert_eq!(code(resolve(&p.manifest, &only_root)), DEPENDENCY_MISSING);
    assert_eq!(code(verify(&p.manifest, &p.lock, &only_root, &Identity::full())), DEPENDENCY_MISSING);

    let dep_digest = sha256(dep.as_bytes());
    let mut lock = Lock::read(&p.lock).unwrap();
    lock.locations.retain(|l| l.source != dep_digest);
    assert_eq!(code(verify(&p.manifest, &lock.bytes(), &p.files, &Identity::full())), DEPENDENCY_MISSING);
    lock.modules.retain(|m| m.source != dep_digest);
    assert_eq!(code(verify(&p.manifest, &lock.bytes(), &p.files, &Identity::full())), DEPENDENCY_MISSING);
}

/// LG5.2, LG5.3, LG5.7: a manifest, lock or source that disagrees on the resolver, the IR contract, an edition, the
/// capabilities, the dependencies, the parameters or the module set is refused; a lock for other manifest bytes is a
/// digest mismatch.
#[test]
fn ct_rte_08_contract_mismatches_are_refused() {
    let dep = constant(1);
    let root = caller(&dep);
    let p = package(&root, &[&dep]);
    let full = Identity::full();
    let with_lock = |lock: &Lock| verify(&p.manifest, &lock.bytes(), &p.files, &full);

    let mut lock = Lock::read(&p.lock).unwrap();
    lock.resolver = "c600-resolver-draft-0".into();
    assert_eq!(code(with_lock(&lock)), CONTRACT_MISMATCH, "resolver");

    for change in 0..3 {
        let mut lock = Lock::read(&p.lock).unwrap();
        let m = &mut lock.modules[0];
        match change {
            0 => m.ir_version = "ir-draft-0".into(),
            1 => m.exec_contract = "exec-draft-0".into(),
            _ => m.edition = "6cs-draft-0".into(),
        }
        assert_eq!(code(with_lock(&lock)), CONTRACT_MISMATCH, "module change {change}");
    }

    // The manifest disagrees with the root header; each manifest gets its own lock, so only the disagreement refuses.
    let declared = |m: Manifest| {
        let bytes = m.bytes();
        let lock = resolve(&bytes, &p.files).unwrap().bytes();
        verify(&bytes, &lock, &p.files, &full)
    };
    let mut m = manifest_for(root.as_bytes());
    m.capabilities = vec!["file".into()];
    assert_eq!(code(declared(m)), CONTRACT_MISMATCH, "capabilities");
    let mut m = manifest_for(root.as_bytes());
    m.dependencies.clear();
    assert_eq!(code(declared(m)), CONTRACT_MISMATCH, "dependencies");
    let mut m = manifest_for(root.as_bytes());
    m.module = "n".into();
    assert_eq!(code(declared(m)), CONTRACT_MISMATCH, "module name");

    // Declared parameters are not drafted.
    let json = manifest_for(root.as_bytes()).json();
    let payload = edit(json.get("payload").unwrap(), "parameters", Some(Json::List(vec![Json::text("x")])));
    let bytes = provenance::canonical(&edit(&json, "payload", Some(payload)));
    assert_eq!(code(Manifest::read(&bytes)), CONTRACT_MISMATCH, "parameters");

    // A locked module the root does not reach.
    let stray = module("stray", &[], &[], "export fn value() -> Int effects [] {\n    return 3;\n}\n");
    let files = p.files.clone().with("src/stray.6cs", stray.as_bytes());
    let mut lock = Lock::read(&p.lock).unwrap();
    let mut extra = lock.modules[0].clone();
    extra.module = "stray".into();
    extra.source = sha256(stray.as_bytes());
    lock.modules.push(extra);
    lock.locations.push(c600_module::Location { path: "src/stray.6cs".into(), source: sha256(stray.as_bytes()) });
    let refusal = verify(&p.manifest, &lock.bytes(), &files, &full).map(|_| ()).unwrap_err();
    assert_eq!(refusal.code, CONTRACT_MISMATCH, "unreached module");
    assert!(refusal.detail.contains("does not reach"), "{}", refusal.detail);

    // A lock resolved for other manifest bytes.
    let three = constant(3);
    let other = package(&caller(&three), &[&three]);
    assert_eq!(code(verify(&p.manifest, &other.lock, &p.files, &full)), DIGEST_MISMATCH);
}

/// LG5.5: a session identity the manifest does not list is refused at verification, and an approval for another
/// identity is refused at load.
#[test]
fn ct_rte_08_an_identity_outside_supports_is_refused() {
    let dep = constant(1);
    let p = package(&caller(&dep), &[&dep]);
    let jumbling = Identity::from_json(&Json::object([
        ("contract_rev", Json::text("jumble-contract-draft")),
        ("menu_id", Json::text("jumble-menu-draft")),
        ("menu_order", Json::text("jumble-order-draft")),
        ("model_id", Json::Text(hex(&[7; 32]))),
        ("puzzle", Json::text("jumbling")),
    ]))
    .unwrap();
    assert_eq!(code(verify(&p.manifest, &p.lock, &p.files, &jumbling)), MODEL_NOT_SUPPORTED);
    let (closure, approval) = approved(&p, own_tool("go")).unwrap();
    let mut other = approval.clone();
    other.identity = jumbling;
    assert_eq!(code(load(&closure, &other)), MODEL_NOT_SUPPORTED);
    assert!(load(&closure, &approval).is_ok());
}

/// FP.5, FP.10, FP.11: the manifest and the lock read as FP.11 records. An unknown format or version and a required
/// extension are `PS2`; a missing header, an unknown or missing field and a non-null identity are `PS1`; bytes out of
/// canonical form and an unsorted set are noncanonical; an optional extension is kept and has no effect.
#[test]
fn ct_rte_08_record_forms_are_checked_before_any_use() {
    let dep = constant(1);
    let root = caller(&dep);
    let json = manifest_for(root.as_bytes()).json();
    let read = |j: &Json| Manifest::read(&provenance::canonical(j)).map(|_| ());
    assert!(read(&json).is_ok());
    let extension = |required: bool| {
        Json::List(vec![Json::object([
            ("name", Json::text("x-note")),
            ("required", Json::Bool(required)),
            ("value", Json::Null),
        ])])
    };
    let cases: [(&str, Json, &str); 7] = [
        ("version 2", edit(&json, "version", Some(Json::Int(2))), PERSISTENCE_VERSION),
        ("another format", edit(&json, "format", Some(Json::text("c600-module-x"))), PERSISTENCE_VERSION),
        ("no version", edit(&json, "version", None), PERSISTENCE_FORMAT),
        ("unknown field", edit(&json, "note", Some(Json::Null)), PERSISTENCE_FORMAT),
        ("no provenance", edit(&json, "provenance", None), PERSISTENCE_FORMAT),
        ("identity", edit(&json, "identity", Some(Identity::full().json().clone())), PERSISTENCE_FORMAT),
        ("required extension", edit(&json, "extensions", Some(extension(true))), PERSISTENCE_VERSION),
    ];
    for (what, j, expected) in cases {
        assert_eq!(code(read(&j)), expected, "{what}");
    }
    assert!(read(&edit(&json, "extensions", Some(extension(false)))).is_ok(), "an optional extension");

    let unsorted = Json::List(vec![Json::text("file"), Json::text("clock")]);
    let payload = edit(json.get("payload").unwrap(), "capabilities", Some(unsorted));
    assert_eq!(code(read(&edit(&json, "payload", Some(payload)))), NONCANONICAL_VALUE);
    let mut spaced = provenance::canonical(&json);
    spaced.insert(1, b' ');
    assert_eq!(code(Manifest::read(&spaced)), NONCANONICAL_VALUE);

    let p = package(&root, &[&dep]);
    let lock = Lock::read(&p.lock).unwrap().json();
    assert_eq!(
        code(Lock::read(&provenance::canonical(&edit(&lock, "version", Some(Json::Int(2)))))),
        PERSISTENCE_VERSION
    );
    assert_eq!(code(Lock::read(&p.manifest)), PERSISTENCE_VERSION, "a manifest is not a lock");
}

/// LG5.1, LG5.9, PS1.15: a moved file changes the lock's bytes, never the closure digest, and the approval still loads
/// and runs the same bytes.
#[test]
fn ct_rte_08_a_moved_file_keeps_the_closure_and_its_approval() {
    let dep = constant(1);
    let root = caller(&dep);
    let p = package(&root, &[&dep]);
    let (closure, approval) = approved(&p, own_tool("go")).unwrap();
    let files = Files::new().with("src/root.6cs", root.as_bytes()).with("vendor/constant.6cs", dep.as_bytes());
    let lock = resolve(&p.manifest, &files).unwrap().bytes();
    assert_ne!(lock, p.lock);
    let moved = verify(&p.manifest, &lock, &files, &Identity::full()).unwrap();
    assert_eq!(moved.digest, closure.digest);
    let ready = load(&moved, &approval).unwrap();
    let (mut engine, mut store) = (Engine::fixture(), Store::default());
    let out = run(&ready, "run", vec![], vec![], &mut engine, &mut store).unwrap();
    assert_eq!(out.run.output.as_deref(), Some(&encoded(&Value::int(1))[..]));
}

/// S01 (`approval.json`), RTE.45, LG3.2: dynamic code is refused. The surface cannot write it (an unbound name is a
/// lowering error); an unresolved call that reaches the checker is refused before any other finding, and the runtime
/// refuses to load it.
#[test]
fn ct_rte_08_dynamic_code_is_refused() {
    let surface =
        module("m", &[], &[], "export fn go() -> Int effects [] {\n    return eval(\"constant arithmetic\");\n}\n");
    let files = Files::new().with("src/root.6cs", surface.as_bytes());
    let manifest = manifest_for(surface.as_bytes()).bytes();
    let lock = resolve(&manifest, &files).unwrap().bytes();
    assert_eq!(code(verify(&manifest, &lock, &files, &Identity::full())), IR_REJECTED);

    let plain = module("m", &[], &[], "export fn go() -> Int effects [] {\n    return 2;\n}\n");
    let closure = package(&plain, &[]).verify().unwrap();
    let mut program = closure.program.clone();
    let go = program.modules.iter_mut().flat_map(|m| m.functions.iter_mut()).find(|f| f.name == "go").unwrap();
    go.body = Expr::Call {
        callee: Callee::Unresolved { name: "eval".into(), result: QTy { ty: Ty::Int, display: false } },
        args: vec![Expr::Lit(Lit::Text("constant arithmetic".into()))],
    };
    let dynamic = Closure { program: program.clone(), ..closure.clone() };
    for author in [Author::Own, Author::BuiltIn] {
        let refused = approve(&dynamic, entry("go", author, Profile::ToolAuthoring), contract(), checker());
        let refusal = refused.unwrap_err();
        assert_eq!(refusal.code, NOT_GRANTED, "{author:?}");
        assert!(refusal.detail.contains("dynamic code"), "{}", refusal.detail);
    }
    assert_eq!(code(c600_run::interp::load(&program)), IR_REJECTED);
}

/// LG4.1, LG5.4: a closure whose program asks for another IR version or execution contract is refused with the
/// checker's `LG4:input.ir_version_unsupported` in every author column, the built-in one included, before the entry is
/// looked up; the interpreter refuses to load it with the same code.
#[test]
fn ct_rte_08_another_ir_version_is_refused_in_every_author_column() {
    let plain = module("m", &[], &[], "export fn go() -> Int effects [] {\n    return 2;\n}\n");
    let closure = package(&plain, &[]).verify().unwrap();
    let programs = [
        Program { ir_version: "c600-ir-draft-0".into(), ..closure.program.clone() },
        Program { exec_contract: "lp1-exec-draft-0".into(), ..closure.program.clone() },
    ];
    for program in programs {
        let stale = Closure { program: program.clone(), ..closure.clone() };
        for author in [Author::Own, Author::ForkedBuiltIn, Author::Imported, Author::BuiltIn] {
            let refused = approve(&stale, entry("go", author, Profile::ToolAuthoring), contract(), checker());
            let asked = format!("{} with {}", program.ir_version, program.exec_contract);
            assert_eq!(code(refused), IR_VERSION_UNSUPPORTED, "{author:?}, {asked}");
        }
        assert_eq!(code(c600_run::interp::load(&program)), IR_VERSION_UNSUPPORTED);
    }
}

/// The module case of BM2 (BND.39, LG3.13; LX-3 MR11): a callee in another module reads the state, so its caller's
/// proposal is refused at the preview point; the same caller with a callee that reads nothing previews.
#[test]
fn ct_rte_08_a_callee_that_reads_state_makes_its_caller_non_previewing() {
    let reader = module(
        "selector",
        &[],
        &[],
        "export fn select() -> Word effects [snapshot] {\n    \
         let nu = read_snapshot<record { i0: Int, i1: Int }>(\"nu2\");\n    \
         return if(exact_eq(nu.i0, 1), word[1], word[2]);\n}\n",
    );
    let pure = module("selector", &[], &[], "export fn select() -> Word effects [] {\n    return word[1];\n}\n");
    let root = |dep: &str, effects: &str| {
        let body = format!(
            "export fn go(t: Target) -> Text effects [{effects}] {{\n    \
             return receipt_id(propose(selector.select(), t));\n}}\n"
        );
        module("m", &[], &[("selector", dep)], &body)
    };

    let p = package(&root(&reader, "snapshot, preview"), &[&reader]);
    let ready = ready(&p, solving("go")).unwrap();
    assert!(ready.approval().admitted.separation.reads_state, "the callee's read is the caller's (LG3.13)");
    let (mut engine, mut store) = (Engine::fixture(), Store::default());
    let out = run(&ready, "run", vec![Value::target("c0")], vec![manual()], &mut engine, &mut store).unwrap();
    assert_eq!(out.run.output.as_deref(), Some(&b"\"run/1\"\n"[..]), "the fixed acknowledgement");
    assert!(matches!(&out.shell[0].answer, Answer::Refused(r) if r.code == NOT_GRANTED));
    assert_eq!(out.separated, ["run/1"]);
    assert!(engine.previews.is_empty());
    assert_eq!(engine.pending, Engine::fixture().pending);

    // An own tool of the same shape is checked by `c600-check::admit` and refused.
    assert_eq!(code(approved(&p, entry("go", Author::Own, Profile::Solving))), NOT_GRANTED);

    let q = package(&root(&pure, "preview"), &[&pure]);
    let ready = previewer(&q, "go");
    let (mut engine, mut store) = (Engine::fixture(), Store::default());
    let out = run(&ready, "run", vec![Value::target("c0")], vec![manual()], &mut engine, &mut store).unwrap();
    assert!(matches!(&out.shell[0].answer, Answer::Previewed(_)));
    assert!(out.separated.is_empty());
    assert_eq!(engine.previews, [(vec![A], "c0".to_string())]);
}

/// BND.42, BND.43: in the solving profile the program gets `{base, id}` whether the engine built the preview or
/// refused it, and the shell keeps the full answer. Reading the outcome is a state read, so a program that reads it
/// has its proposal refused at the preview point and stops at the read.
#[test]
fn ct_rte_08_a_previewer_gets_only_base_and_id() {
    let root = module(
        "m",
        &[],
        &[],
        "export fn go(w: Word, t: Target) -> record { base: Text, id: Text } effects [preview] {\n    \
         let r = propose(w, t);\n    return record { base = receipt_base(r), id = receipt_id(r) };\n}\n\n\
         export fn outcome(w: Word, t: Target) -> Text effects [snapshot, preview] {\n    \
         return receipt_effect(propose(w, t));\n}\n",
    );
    let p = package(&root, &[]);
    let ready = previewer(&p, "go");
    let mut outputs = Vec::new();
    for letter in [A, 4] {
        let (mut engine, mut store) = (Engine::fixture(), Store::default());
        let args = vec![Value::word(&[letter]), Value::target("c0")];
        let out = run(&ready, "run", args, vec![manual(), manual()], &mut engine, &mut store).unwrap();
        match (letter, &out.shell[0].answer) {
            (A, Answer::Previewed(Value::Receipt(r))) => {
                assert!(r.get("effect").is_some() && r.get("protection").is_some(), "the shell keeps every field");
            }
            (4, Answer::Refused(r)) => assert_eq!(r.code, PROTECTION),
            (_, answer) => panic!("{letter}: {answer:?}"),
        }
        outputs.push((out.run.output.clone(), out.run.summary.clone()));
    }
    let expected = encoded(&Value::record(vec![("base", Value::text("rev-7")), ("id", Value::text("run/1"))]));
    assert_eq!(outputs[0].0.as_deref(), Some(&expected[..]));
    assert_eq!(outputs[0], outputs[1], "one acknowledgement, one charge, whatever the engine answered");

    let (mut engine, mut store) = (Engine::fixture(), Store::default());
    let reads = previewer(&p, "outcome");
    let args = vec![Value::word(&[A]), Value::target("c0")];
    let out = run(&reads, "run", args, vec![manual(), manual()], &mut engine, &mut store).unwrap();
    assert_eq!((out.run.summary.outcome, out.run.summary.code), (Ended::Error, Some(NOT_GRANTED)));
    assert_eq!(out.separated, ["run/1"]);
    assert!(engine.previews.is_empty());
}

/// LB1.8's menu words (LP2 draft) pass the same gate as literal words (BND.39, BND.42, LG3.13): a built-in previews the
/// menu word the solver named, the shell and the canonical result keep its letters and alphabet, and reading the
/// outcome makes the menu-word proposal non-previewing.
#[test]
fn ct_rte_08_a_menu_word_proposal_passes_the_same_gate() {
    let root = module(
        "m",
        &[],
        &[],
        "export fn go(m: MenuWord, t: Target) -> Text effects [preview] {\n    \
         return receipt_id(propose(m, t));\n}\n\n\
         export fn outcome(m: MenuWord, t: Target) -> Text effects [snapshot, preview] {\n    \
         return receipt_effect(propose(m, t));\n}\n",
    );
    let p = package(&root, &[]);
    let identity = MenuIdentity {
        puzzle: "600-cell-jumbled".into(),
        model_id: "model-1".into(),
        menu_id: "menu-a".into(),
        menu_order: "order-1".into(),
        contract_rev: "rev-1".into(),
    };
    let alphabet = MenuAlphabet::new(identity, 4, vec![0, 2, 1]).expect("an alphabet");
    let run_on = |ready: &c600_module::Ready, letters: &[[u32; 2]], engine: &mut Engine| {
        let word = Value::menu_word(alphabet.shared_identity(), letters).expect("indices below 600");
        let request = RunRequest {
            run: "run",
            base: "rev-7",
            args: vec![word, Value::target("c0")],
            inputs: vec![manual(), manual()],
            pins: pins(),
            options: Options { menu: Some(&alphabet), ..Options::new(WIDE) },
        };
        ready.run(request, engine, &mut Store::default()).expect("a run")
    };

    let ready = previewer(&p, "go");
    let mut engine = Engine::fixture();
    let out = run_on(&ready, &[[0, 1], [3, 2]], &mut engine);
    assert_eq!(out.run.output.as_deref(), Some(&b"\"run/1\"\n"[..]), "the fixed acknowledgement");
    assert!(matches!(&out.shell[0].answer, Answer::Previewed(_)) && out.separated.is_empty());
    assert_eq!(engine.menu_previews, [(vec![[0, 1], [3, 2]], "c0".to_string())]);
    let shown = out.shell[0].menu.as_ref().map(|w| w.letters().to_vec());
    assert!(out.shell[0].word.is_empty() && shown == Some(vec![[0, 1], [3, 2]]));
    let other = run_on(&ready, &[[0, 1], [3, 1]], &mut Engine::fixture());
    assert_ne!(out.canonical(), other.canonical(), "the canonical result keeps the menu word");
    assert!(contains(&out.canonical(), b"menu-a"), "and its alphabet");

    let reads = previewer(&p, "outcome");
    let mut engine = Engine::fixture();
    let out = run_on(&reads, &[[0, 1]], &mut engine);
    assert_eq!((out.run.summary.outcome, out.run.summary.code), (Ended::Error, Some(NOT_GRANTED)));
    assert!(matches!(&out.shell[0].answer, Answer::Refused(r) if r.code == NOT_GRANTED));
    assert_eq!(out.separated, ["run/1"]);
    assert!(engine.menu_previews.is_empty() && engine.previews.is_empty() && engine.traps == 0);
}

/// The branch case of BM2 (BND.39; LX-3 MR09, MR10): a state-conditioned choice between two constant words never
/// reaches a preview, on either branch.
#[test]
fn ct_rte_08_a_state_conditioned_branch_cannot_preview() {
    let root = module(
        "m",
        &[],
        &[],
        "export fn go(t: Target) -> Text effects [snapshot, preview] {\n    \
         let n = read_snapshot<record { c0: Int, c1: Int, c2: Int }>(\"counts\");\n    \
         return receipt_id(if(exact_less(0, n.c0), propose(word[1], t), propose(word[2], t)));\n}\n",
    );
    let p = package(&root, &[]);
    let ready = previewer(&p, "go");
    for first in [2, 0] {
        let mut engine = Engine::fixture();
        engine.rows.insert(
            "counts".into(),
            Value::record(vec![("c0", Value::int(first)), ("c1", Value::int(6)), ("c2", Value::int(6))]),
        );
        let mut store = Store::default();
        let out = run(&ready, "run", vec![Value::target("c0")], vec![manual()], &mut engine, &mut store).unwrap();
        let shown = &out.shell[0];
        assert_eq!(shown.word, if first > 0 { vec![A] } else { vec![B] });
        assert!(matches!(&shown.answer, Answer::Refused(r) if r.code == NOT_GRANTED));
        assert_eq!(out.separated, ["run/1"]);
        assert!(engine.previews.is_empty());
    }
}

/// The preview-outcome case of BM2 (BND.42; LX-3 MR15, MR16): a program that would let a preview's protection or
/// error choose a later word has its first proposal refused, and its outcome read stops the run.
#[test]
fn ct_rte_08_a_preview_outcome_cannot_choose_a_later_preview() {
    let root = module(
        "m",
        &[],
        &[],
        "export fn go(t: Target) -> Text effects [snapshot, preview] {\n    \
         let probe = propose(word[3], t);\n    \
         return match receipt_protection(probe) {\n        \
         tag(\"clear\") => receipt_id(propose(word[1], t));\n        \
         _ => receipt_id(propose(word[2], t));\n    };\n}\n",
    );
    let p = package(&root, &[]);
    let ready = previewer(&p, "go");
    let (mut engine, mut store) = (Engine::fixture(), Store::default());
    let out = run(&ready, "run", vec![Value::target("c0")], vec![manual()], &mut engine, &mut store).unwrap();
    assert_eq!((out.run.summary.outcome, out.run.summary.code), (Ended::Error, Some(NOT_GRANTED)));
    assert_eq!(out.shell.len(), 1, "no later proposal");
    assert_eq!(out.separated, ["run/1"]);
    assert!(engine.previews.is_empty());
    assert_eq!(engine.pending, Engine::fixture().pending);
}

/// The persisted-word case of BM2 (BND.22, BND.84; CT-LG3-07's saved-word half): a word an own tool stored keeps the
/// host's block, preview admission refuses it with `LG3:capability.not_granted`, and a solving run that proposes it is
/// refused at admission, before the split. The durable leg (M6's session store) is not run here.
#[test]
fn ct_rte_08_a_persisted_tool_word_is_refused_at_preview_admission() {
    let root = module(
        "m",
        &[],
        &[],
        "export fn save(w: Word) -> Unit effects [workspace] {\n    \
         return write_workspace<Word>(\"saved\", w);\n}\n\n\
         export fn go(t: Target) -> Text effects [snapshot, preview] {\n    \
         return receipt_id(propose(read_snapshot<Word>(\"saved\"), t));\n}\n",
    );
    let p = package(&root, &[]);
    let (mut engine, mut store) = (Engine::fixture(), Store::default());
    let saver = ready(&p, own_tool("save")).unwrap();
    let saved = run(&saver, "save", vec![Value::word(&[A])], vec![manual()], &mut engine, &mut store).unwrap();
    assert_eq!(saved.run.summary.outcome, Ended::Completed);
    let block = store.records[&Place::Workspace("saved".into())].block.clone();
    let read = provenance::read(&block).unwrap();
    assert_eq!((field::author(&read), field::producer(&read)), ("own", "program"));
    assert_eq!(field::parts(&read), [sha256(&manual())]);
    assert_eq!(code(admit_preview(&block, &store, &pins())), NOT_GRANTED);

    let ready = previewer(&p, "go");
    let out = run(&ready, "run", vec![Value::target("c0")], vec![manual()], &mut engine, &mut store).unwrap();
    assert!(matches!(&out.shell[0].answer, Answer::Refused(r) if r.code == NOT_GRANTED));
    assert!(out.separated.is_empty(), "admission refused it first (BND.84)");
    assert!(engine.previews.is_empty());
    assert_eq!(store.records[&Place::Workspace("saved".into())].block, block, "the stored block is unchanged");
}
