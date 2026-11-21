//! Conformance tests for chapter 02's LG3 rows and chapter 03's LB7 rows at run time (LP2, M2 and M3): the run-time
//! halves of CT-LG3-02, CT-LG3-04, CT-LG3-05, CT-LG3-06 and CT-LG3-09, and CT-LB7-01 and CT-LB7-02. The static halves
//! are `c600-syntax`'s `conformance_lg3.rs` and `c600-check`'s `effects.rs`. The flow rule stays off (BND.41).

mod conformance;

use c600_check::{admit, check, Grant, Policy, Profile as CheckProfile, AMBIENT_READ, EFFECT_UNDECLARED, NOT_GRANTED};
use c600_ir::{Capability, ReceiptField};
use c600_run::boundary::{Answer, Store};
use c600_run::interp::{NoHost, Options, Request, Sink};
use c600_run::meter::Outcome;
use c600_run::provenance::{self, field, Author, Json, Profile};
use c600_run::value::Value;
use conformance::*;

const FLOW_REFUSED: &str = "LG3:capability.flow_refused";

/// A module that saves words, reads them back and proposes them.
const SAVER: &str = "export fn save(w: Word) -> Unit effects [workspace] {\n    \
    return write_workspace<Word>(\"saved\", w);\n}\n\n\
    export fn resave() -> Unit effects [snapshot, workspace] {\n    \
    return write_workspace<Word>(\"resaved\", read_snapshot<Word>(\"saved\"));\n}\n\n\
    export fn go(w: Word) -> Text effects [preview] {\n    return receipt_id(propose(w, target(\"t\")));\n}\n\n\
    export fn go_saved() -> Text effects [snapshot, preview] {\n    \
    return receipt_id(propose(read_snapshot<Word>(\"saved\"), target(\"t\")));\n}\n\n\
    export fn go_resaved() -> Text effects [snapshot, preview] {\n    \
    return receipt_id(propose(read_snapshot<Word>(\"resaved\"), target(\"t\")));\n}\n\n\
    export fn peek() -> Nat effects [snapshot] {\n    return read_snapshot<Nat>(\"n\");\n}\n";

const BUILT_IN: (Author, Profile) = (Author::BuiltIn, Profile::Solving);
const OWN_TOOL: (Author, Profile) = (Author::Own, Profile::ToolAuthoring);
const RESEARCH: (Author, Profile) = (Author::Own, Profile::Research);

fn word() -> Vec<Value> {
    vec![Value::word(&[1, -2])]
}

/// Runs `entry` of `SAVER` as `who`, with the given arguments and argument blocks.
fn saver(
    entry: &str,
    who: (Author, Profile),
    args: Vec<Value>,
    inputs: Vec<Vec<u8>>,
    e: &mut Engine,
    s: &mut Store,
) -> Ran {
    run_as(&m(SAVER), entry, who, args, inputs, e, s, &Options::new(WIDE))
}

/// The code of the only shown answer, or `None` when the engine previewed.
fn refused(ran: &Ran) -> Option<&'static str> {
    assert_eq!(ran.shell.len(), 1);
    match &ran.shell[0].answer {
        Answer::Previewed(_) => None,
        Answer::Refused(r) => Some(r.code),
    }
}

/// The kept block under `digest`, read.
fn block_of(store: &Store, digest: &[u8; 32]) -> Json {
    provenance::read(&store.blocks[digest]).expect("a kept block reads")
}

/// CT-LG3-02 (part: the run-time guard; the static half is `c600-check`'s and `c600-syntax`'s `ct_lg3_02_*`; the
/// overlay rows are in `c600-command`'s `tests/layer.rs` and `tests/served_rows.rs`): a running program can ask its
/// host only for a state read, a granted input or a preview, and deliver only to a view, the workspace, a file or an
/// exact sink. Both lists are matched without a wildcard, so a commit request added to the interpreter fails to compile
/// here. A program reads its receipt's fields and nothing else, none of them a token, and a preview in any profile
/// commits nothing.
#[test]
fn ct_lg3_02_no_request_commits_or_obtains_a_token() {
    fn request(r: &Request) -> &'static str {
        match r {
            Request::Snapshot { .. } | Request::SnapshotWith { .. } | Request::Declassified { .. } => "state read",
            Request::FileRead { .. } | Request::Clock => "granted input",
            Request::Propose { .. } | Request::ProposeMenu { .. } => "preview",
        }
    }
    fn sink(s: &Sink) -> &'static str {
        match s {
            Sink::View => "view",
            Sink::Workspace { .. } => "workspace",
            Sink::File { .. } => "file",
            Sink::Exact { .. } => "exact sink",
        }
    }
    fn receipt(f: ReceiptField) -> &'static str {
        match f {
            ReceiptField::Id | ReceiptField::Base => "acknowledgement",
            ReceiptField::Effect
            | ReceiptField::Protection
            | ReceiptField::EndState
            | ReceiptField::N
            | ReceiptField::Error => "outcome",
        }
    }
    let kinds = (request(&Request::Clock), sink(&Sink::View), receipt(ReceiptField::Id));
    assert_eq!(kinds, ("granted input", "view", "acknowledgement"));
    for who in [BUILT_IN, OWN_TOOL, RESEARCH] {
        let (mut engine, mut store) = (Engine::default(), Store::default());
        let hash = engine.state_hash();
        let ran = saver("go", who, word(), vec![manual()], &mut engine, &mut store);
        assert_eq!(ran.run.summary.outcome, Outcome::Completed, "{who:?}");
        assert_eq!((engine.state_hash(), engine.journal.len()), (hash, 0), "{who:?}");
    }
}

/// CT-LG3-04 (part: the run-time half; the static halves are `c600-check`'s and `c600-syntax`'s `ct_lg3_04_*`, and the
/// row is whole with them): a checked file, clock or randomness request that reaches a host without the grant fails
/// with `LG3:capability.not_granted`, and a draw fails without a granted seed. Static admission is no grant.
#[test]
fn ct_lg3_04_an_ungranted_request_fails_at_run_time() {
    let source = m_requiring(
        "file, clock, random",
        "export fn load() -> Nat effects [file] {\n    return file_read<Nat>(\"data\");\n}\n\n\
         export fn save(n: Nat) -> Unit effects [file] {\n    return file_write(\"data\", n);\n}\n\n\
         export fn now() -> Display<Nat> effects [clock] {\n    return read_clock();\n}\n\n\
         export fn draw() -> Nat effects [random] {\n    return random_draw();\n}\n",
    );
    let (_, loaded) = build(&[&source]);
    for (entry, args) in [("load", vec![]), ("save", vec![Value::int(1)]), ("now", vec![]), ("draw", vec![])] {
        let r = loaded.run("m", entry, args, &mut NoHost, &Options::new(WIDE)).unwrap();
        assert_eq!((r.summary.outcome, r.summary.code), (Outcome::Error, Some(NOT_GRANTED)), "{entry}");
    }
}

/// CT-LG3-05 (part: the engine's rechecks behind the runtime boundary, and the command layer's at `operation.execute`
/// in `c600-command/tests/kn5.rs` and `layer.rs`; not run: a checker-admitted program's proposal refused by the layer's
/// `operation.preview` on a moved head, in the other mode or under another model): a program the checker admitted is
/// still refused
/// at run time when the revision is stale, the mode is wrong either way, the model differs, an overlay row is
/// unavailable or a piece would move. A refused preview reaches the shell with the engine's code while the program
/// sees the fixed acknowledgement; a refused snapshot read stops the run with the engine's code.
#[test]
fn ct_lg3_05_static_success_cannot_bypass_a_run_time_refusal() {
    let codes = [STALE, NEEDS_JUMBLE_OFF, NEEDS_JUMBLE_ON, IDENTITY_MISMATCH, OVERLAY_MISSING, PIECE_WOULD_MOVE];
    let (mut engine, mut store) = (Engine::with(&[("n", Value::int(3))]), Store::default());
    let accepted = saver("go", BUILT_IN, word(), vec![manual()], &mut engine, &mut store);
    assert_eq!(refused(&accepted), None);
    let read = saver("peek", BUILT_IN, vec![], vec![], &mut engine, &mut store);
    assert_eq!(format!("{:?}", read.run.value), format!("{:?}", Some(Value::int(3))));
    for code in codes {
        let mut engine = Engine::with(&[("n", Value::int(3))]);
        engine.refuse_previews = Some(code);
        let ran = saver("go", BUILT_IN, word(), vec![manual()], &mut engine, &mut store);
        assert_eq!(refused(&ran), Some(code));
        assert_eq!((&ran.run.summary, &ran.run.output), (&accepted.run.summary, &accepted.run.output), "{code}");
        engine.refuse_reads = Some(code);
        let read = saver("peek", BUILT_IN, vec![], vec![], &mut engine, &mut store);
        assert_eq!((read.run.summary.outcome, read.run.summary.code), (Outcome::Error, Some(code)));
    }
}

/// CT-LG3-06 (part: the static-only and the run-time variants of capability separation, checked separately; not
/// run: the flow-label variant, not applicable while the flow rule stays off until LQ2 sets it, BND.41, so no pass;
/// the integrator records it under PHS.63). The static-only variant classifies each entry and refuses a preview in a
/// solving session at admission, leaving a state read to the open cell BND.59; it never emits
/// `LG3:capability.flow_refused`. The run-time variant reads the stored blocks at preview admission, which the static
/// variant cannot see: the same built-in entry is refused for a word an own tool saved and previews the same word
/// saved from a hand entry.
#[test]
fn ct_lg3_06_static_only_and_run_time_variants_are_checked_separately() {
    let program = lowered(&[&m(SAVER)]).program;
    let classify = |entry: &str, profile: CheckProfile| {
        let a = admit(&program, "m", entry, profile, &Grant::none(), &Policy::interim()).unwrap();
        assert!(!a.codes().contains(&FLOW_REFUSED), "{entry}");
        (a.codes(), a.separation.reads_state, a.separation.prepares_previews, a.open_cells)
    };
    assert_eq!(classify("go", CheckProfile::Solving), (vec![NOT_GRANTED], false, true, vec![]));
    assert_eq!(classify("go_saved", CheckProfile::Solving), (vec![NOT_GRANTED], true, true, vec!["BND.59"]));
    assert_eq!(classify("peek", CheckProfile::Solving), (vec![], true, false, vec!["BND.59"]));
    for profile in [CheckProfile::ToolAuthoring, CheckProfile::Research] {
        assert_eq!(classify("go_saved", profile), (vec![], true, true, vec![]), "{profile:?}");
    }
    let (mut engine, mut store) = (Engine::default(), Store::default());
    saver("save", OWN_TOOL, word(), vec![manual()], &mut engine, &mut store);
    assert_eq!(refused(&saver("go_saved", BUILT_IN, vec![], vec![], &mut engine, &mut store)), Some(NOT_GRANTED));
    saver("save", BUILT_IN, word(), vec![manual()], &mut engine, &mut store);
    assert_eq!(refused(&saver("go_saved", BUILT_IN, vec![], vec![], &mut engine, &mut store)), None);
    assert_eq!(engine.previews, [vec![1, -2]], "only the hand-entered word reached the engine");
}

/// CT-LG3-09 (part: R-12 for refused proposals, and saving through the runtime's store; the command layer's rejected
/// record write is in `c600-command`'s `profile.rs`, and a failed disk write of a record that carries a block is in
/// `c600-session`'s `record_objects.rs`; not run: the session's assistance provenance record, which no crate keeps): a
/// refused
/// proposal changes no stored payload or block; a fork's output and a research result
/// keep their blocks when saved and saved again, so a solving preview of either is refused; an imported word keeps its
/// import block (BND.23) among the parts when saved, and a solver's tool that saves it makes it that tool's output.
#[test]
fn ct_lg3_09_saving_grants_no_author_class_or_solving_permission() {
    // R-12: refused by the engine and refused at admission, the stored records stay as they were.
    let (mut engine, mut store) = (Engine::default(), Store::default());
    saver("save", OWN_TOOL, word(), vec![manual()], &mut engine, &mut store);
    let before = records(&store);
    engine.refuse_previews = Some(PIECE_WOULD_MOVE);
    let shown = refused(&saver("go", BUILT_IN, word(), vec![manual()], &mut engine, &mut store));
    assert_eq!(shown, Some(PIECE_WOULD_MOVE));
    assert_eq!(refused(&saver("go_saved", BUILT_IN, vec![], vec![], &mut engine, &mut store)), Some(NOT_GRANTED));
    assert_eq!(records(&store), before);

    // A fork's output and a research result, saved and saved again by a built-in.
    for (who, author) in [((Author::ForkedBuiltIn, Profile::ToolAuthoring), "forked_built_in"), (RESEARCH, "own")] {
        let (mut engine, mut store) = (Engine::default(), Store::default());
        saver("save", who, word(), vec![manual()], &mut engine, &mut store);
        let first = block_at(&store, &ws("saved"));
        assert_eq!((field::author(&first), field::profile(&first)), (author, who.1.name()));
        saver("resave", BUILT_IN, vec![], vec![], &mut engine, &mut store);
        let again = block_at(&store, &ws("resaved"));
        assert_eq!(field::author(&again), "built_in");
        let cited = field::parts(&again).iter().any(|p| block_of(&store, p) == first);
        assert!(cited, "{who:?}: the saved block is cited unchanged");
        for entry in ["go_saved", "go_resaved"] {
            let ran = saver(entry, BUILT_IN, vec![], vec![], &mut engine, &mut store);
            assert_eq!(refused(&ran), Some(NOT_GRANTED), "{who:?} {entry}");
        }
        assert!(engine.previews.is_empty(), "{who:?}");
    }

    // An imported word saved by a built-in keeps its import block and previews; saved by an own tool it is refused.
    let (mut engine, mut store) = (Engine::default(), Store::default());
    saver("save", BUILT_IN, word(), vec![imported()], &mut engine, &mut store);
    let saved = block_at(&store, &ws("saved"));
    let import = field::parts(&saved).iter().map(|p| block_of(&store, p)).find(|b| field::producer(b) == "import");
    assert_eq!(import.as_ref().map(field::author), Some("own"), "BND.23: an import is the solver's own");
    assert_eq!(refused(&saver("go_saved", BUILT_IN, vec![], vec![], &mut engine, &mut store)), None);
    saver("save", OWN_TOOL, word(), vec![imported()], &mut engine, &mut store);
    assert_eq!(field::author(&block_at(&store, &ws("saved"))), "own");
    assert_eq!(refused(&saver("go_saved", BUILT_IN, vec![], vec![], &mut engine, &mut store)), Some(NOT_GRANTED));
}

/// Saves a word to the workspace and a file, and proposes either copy.
const KEEPER: &str = "export fn keep(w: Word) -> Unit effects [workspace, file] {\n    \
    let a = write_workspace<Word>(\"kept\", w);\n    let b = file_write(\"kept.6cw\", w);\n    return ();\n}\n\n\
    export fn go_kept() -> Text effects [snapshot, preview] {\n    \
    return receipt_id(propose(read_snapshot<Word>(\"kept\"), target(\"t\")));\n}\n\n\
    export fn go_file() -> Text effects [file, preview] {\n    \
    return receipt_id(propose(file_read<Word>(\"kept.6cw\"), target(\"t\")));\n}\n";

/// CT-LB7-01 (part: views, save/load and import; not run: the undoable action of an invoked workspace command, which
/// waits for the session store, M6, and the command layer, LP3): a view cannot write a record; research output saved
/// through the workspace and a file and loaded by a later solving run keeps its block, so its preview is refused at
/// admission; an imported word saved the same way keeps its import block and previews.
#[test]
fn ct_lb7_01_views_write_nothing_and_saved_research_output_previews_nothing() {
    let view = m("view v(w: Word) -> Unit effects [workspace] {\n    return write_workspace<Word>(\"k\", w);\n}\n");
    let codes = check(&lowered(&[&view]).program).unwrap().codes();
    assert!(!codes.is_empty() && codes.iter().all(|c| *c == EFFECT_UNDECLARED), "{codes:?}");
    let source = m_requiring("file", KEEPER);
    let options = Options::new(WIDE);
    for (input, part, who, shown) in
        [(manual(), "manual", RESEARCH, Some(NOT_GRANTED)), (imported(), "import", BUILT_IN, None)]
    {
        let (mut engine, mut store) = (Engine::default(), Store::default());
        let kept = run_as(&source, "keep", who, word(), vec![input], &mut engine, &mut store, &options);
        assert_eq!(kept.run.summary.outcome, Outcome::Completed);
        let (in_workspace, in_file) = (block_at(&store, &ws("kept")), block_at(&store, &file("kept.6cw")));
        assert_eq!(in_workspace, in_file);
        assert_eq!(field::profile(&in_workspace), who.1.name());
        let parts: Vec<_> = field::parts(&in_workspace).iter().map(|p| block_of(&store, p)).collect();
        assert_eq!(parts.iter().map(field::producer).collect::<Vec<_>>(), [part]);
        for entry in ["go_kept", "go_file"] {
            let ran = run_as(&source, entry, BUILT_IN, vec![], vec![], &mut engine, &mut store, &options);
            assert_eq!(refused(&ran), shown, "{who:?} {entry}");
        }
    }
}

/// CT-LB7-02 (part: grants, profiles, storage and forks; that a key binding adds no token or capability to a tool
/// row's job runs in `c600-input/tests/routing.rs`):
/// the same module is admitted or refused by the run's grant and profile only, since admission takes no author and no
/// place of storage; a copy of the module kept in the workspace and read back is the same program with the same
/// admissions. At the boundary a fork's or an own tool's preview is refused where a built-in's passes, and each gets
/// the same acknowledgement and no token.
#[test]
fn ct_lb7_02_grants_and_profiles_decide_and_storage_or_a_fork_grants_nothing() {
    let declarations = "export fn go(w: Word) -> Text effects [preview] {\n    \
                        return receipt_id(propose(w, target(\"t\")));\n}\n\n\
                        export fn go_file(w: Word) -> Text effects [file, preview] {\n    \
                        let n = file_read<Nat>(\"n\");\n    return receipt_id(propose(w, target(\"t\")));\n}\n";
    let source = m_requiring("file", declarations);
    let table = |program: &c600_ir::Program| {
        let mut rows = Vec::new();
        for profile in [CheckProfile::Solving, CheckProfile::ToolAuthoring, CheckProfile::Research] {
            for grant in [Grant::none(), Grant::of(&[Capability::File])] {
                let a = admit(program, "m", "go_file", profile, &grant, &Policy::interim()).unwrap();
                rows.push((profile, grant.capabilities.len(), a.codes()));
            }
        }
        rows
    };
    // An ungranted file read is an ambient read (owner, 16 November 2026, PHS.52); the solving profile's other refusal
    // keeps `not_granted`, with or without the grant.
    let expected = vec![
        (CheckProfile::Solving, 0, vec![AMBIENT_READ, NOT_GRANTED]),
        (CheckProfile::Solving, 1, vec![NOT_GRANTED]),
        (CheckProfile::ToolAuthoring, 0, vec![AMBIENT_READ]),
        (CheckProfile::ToolAuthoring, 1, vec![]),
        (CheckProfile::Research, 0, vec![AMBIENT_READ]),
        (CheckProfile::Research, 1, vec![]),
    ];
    let original = lowered(&[&source]).program;
    assert_eq!(table(&original), expected);
    // A copy kept in the workspace and read back.
    let (mut engine, mut store) = (Engine::default(), Store::default());
    let keep = m("export fn keep(t: Text) -> Unit effects [workspace] {\n    \
                  return write_workspace<Text>(\"module\", t);\n}\n");
    let args = vec![Value::text(&source)];
    run_as(&keep, "keep", OWN_TOOL, args, vec![], &mut engine, &mut store, &Options::new(WIDE));
    let Value::Text(copy) = &store.records[&ws("module")].value else { panic!("a text") };
    assert_eq!(copy.as_str(), source);
    let reread = lowered(&[copy.as_str()]).program;
    assert_eq!(reread, original);
    assert_eq!(table(&reread), expected);
    // At the boundary only the built-in's preview passes.
    for (author, shown) in
        [(Author::BuiltIn, None), (Author::ForkedBuiltIn, Some(NOT_GRANTED)), (Author::Own, Some(NOT_GRANTED))]
    {
        let mut engine = Engine::default();
        let options = Options::new(WIDE);
        let ran = run_as(
            &source,
            "go",
            (author, Profile::Solving),
            word(),
            vec![manual()],
            &mut engine,
            &mut store,
            &options,
        );
        assert_eq!(refused(&ran), shown, "{author:?}");
        assert_eq!(ran.run.output.as_deref(), Some(&b"\"run/1\"\n"[..]), "{author:?}");
    }
}
