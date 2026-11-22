//! Conformance tests for chapter 02's LG3 rows in source (LP2): CT-LG3-01 to CT-LG3-04, each naming the part it runs.
//! The IR-level cases are `c600-check`'s `effects.rs`; the run-time halves of CT-LG3-04 to CT-LG3-06 and CT-LG3-09
//! are in `c600-run`'s `conformance_lg3.rs`.

use c600_check::{
    admit, check, Grant, Policy, Profile, Report, AMBIENT_READ, EFFECT_UNDECLARED, NEVER_GRANTED, NOT_GRANTED,
};
use c600_ir::{Capability, Effect, Program};
use c600_syntax::{lower, parse, sha256_hex, LowerError, Lowered};

const PROFILES: [Profile; 3] = [Profile::Solving, Profile::ToolAuthoring, Profile::Research];

fn lowered(sources: &[&str]) -> Result<Lowered, LowerError> {
    let files: Vec<_> = sources
        .iter()
        .map(|s| {
            let file = parse(s.as_bytes());
            assert!(file.diagnostics.is_empty(), "{s}\n{:#?}", file.diagnostics);
            file
        })
        .collect();
    lower(&files)
}

fn program(sources: &[&str]) -> Program {
    lowered(sources).unwrap_or_else(|e| panic!("{e}")).program
}

fn report(sources: &[&str]) -> Report {
    check(&program(sources)).unwrap_or_else(|e| panic!("{e}"))
}

/// One module `tool` with the given capability list and declarations.
fn tool(requires: &str, declarations: &str) -> String {
    format!("edition \"6cs-draft-1\";\nmodule tool requires [{requires}];\n\n{declarations}")
}

fn refused_at_lowering(source: &str) -> String {
    match lowered(&[source]) {
        Ok(_) => panic!("lowered: {source}"),
        Err(e) => e.to_string(),
    }
}

/// A library that reads the snapshot and a granted file.
const LIB: &str = "edition \"6cs-draft-1\";\nmodule lib requires [file];\n\n\
    export fn peek() -> Nat effects [snapshot] {\n    return read_snapshot<Nat>(\"n\");\n}\n\n\
    export fn load() -> Nat effects [file] {\n    return file_read<Nat>(\"data\");\n}\n";

/// A client of `LIB` declaring `requires` and giving every function the effect list `effects`. Its functions reach
/// `lib.peek` directly, transitively, through a higher-order parameter and through a generic effect variable, and
/// `lib.load` through an import.
fn app(requires: &str, effects: &str) -> String {
    format!(
        "edition \"6cs-draft-1\";\nmodule app requires [{requires}];\nimport l \"{digest}\";\n\n\
         fn direct() -> Nat effects [{effects}] {{\n    return l.peek();\n}}\n\n\
         fn transitive() -> Nat effects [{effects}] {{\n    return direct();\n}}\n\n\
         fn run(g: fn() -> Nat effects [snapshot]) -> Nat effects [{effects}] {{\n    return g();\n}}\n\n\
         fn higher_order() -> Nat effects [{effects}] {{\n    return run(l.peek);\n}}\n\n\
         fn apply<E>(g: fn() -> Nat effects [E]) -> Nat effects [E] {{\n    return g();\n}}\n\n\
         fn generic() -> Nat effects [{effects}] {{\n    return apply(l.peek);\n}}\n\n\
         fn imported() -> Nat effects [{effects}] {{\n    return l.load();\n}}\n",
        digest = sha256_hex(LIB.as_bytes())
    )
}

/// CT-LG3-01 (part: the source half; the IR half is `c600-check`'s `ct_lg3_01_*` in `effects.rs`, and the row is whole
/// with them): direct, transitive, higher-order, generic and imported calls keep the callee's effects and capability
/// requirements, and an import supplies no capability.
#[test]
fn ct_lg3_01_the_call_graph_keeps_effects_and_capabilities_in_source() {
    let r = report(&[LIB, &app("file", "snapshot, file")]);
    assert!(r.is_admitted(), "{:#?}", r.diagnostics);
    for f in ["direct", "transitive", "higher_order", "generic"] {
        let s = r.summary("app", f).unwrap();
        assert!(s.effects.contains(&Effect::ReadSnapshot), "{f}: {s:?}");
    }
    let imported = r.summary("app", "imported").unwrap();
    assert!(imported.effects.contains(&Effect::GrantedFile) && imported.capabilities.contains(&Capability::File));
    // Without the snapshot effect each of the five reaches is refused where it is declared.
    let r = report(&[LIB, &app("file", "file")]);
    let refused: Vec<_> = r.diagnostics.iter().map(|d| (d.code, d.function.as_str())).collect();
    for f in ["direct", "transitive", "run", "higher_order", "generic"] {
        assert!(refused.contains(&(EFFECT_UNDECLARED, f)), "{f}: {refused:?}");
    }
    assert!(!refused.iter().any(|(_, f)| *f == "imported"), "{refused:?}");
    // The library's `requires [file]` does not cover its client.
    let r = report(&[LIB, &app("", "snapshot, file")]);
    assert!(r.diagnostics.iter().all(|d| d.code == EFFECT_UNDECLARED && d.module == "app"), "{:#?}", r.diagnostics);
    assert!(!r.diagnostics.is_empty());
}

/// CT-LG3-02 (part: program effects in source; the overlay rows are in `c600-command`'s `tests/layer.rs` and
/// `tests/served_rows.rs`): no
/// effect or operation commits or obtains a token, a preview gives a receipt, and a helper cannot hide a preview under
/// another effect.
#[test]
fn ct_lg3_02_no_program_effect_commits_or_obtains_a_token() {
    for word in ["commit", "token", "execute"] {
        let source = tool("", &format!("fn f() -> Unit effects [{word}] {{\n    return ();\n}}\n"));
        assert!(refused_at_lowering(&source).contains("not an effect"), "{word}");
    }
    for name in ["commit", "execute", "user_action_token"] {
        let source = tool("", &format!("fn f(w: Word) -> Unit effects [preview] {{\n    return {name}(w);\n}}\n"));
        assert!(refused_at_lowering(&source).contains("not bound"), "{name}");
    }
    let preview = |effects: &str| {
        tool(
            "",
            &format!("fn f(w: Word) -> Receipt effects [{effects}] {{\n    return propose(w, target(\"t\"));\n}}\n"),
        )
    };
    let r = report(&[&preview("preview")]);
    assert!(r.is_admitted());
    assert_eq!(r.summary("tool", "f").unwrap().effects.iter().copied().collect::<Vec<_>>(), [Effect::PreparePreview]);
    for other in ["view", "workspace", "snapshot, view, workspace, budget"] {
        assert_eq!(report(&[&preview(other)]).codes(), [EFFECT_UNDECLARED], "{other}");
    }
}

/// CT-LG3-03 (part: the static half in source; not run: one undoable action per invoked workspace change, which waits
/// for the session store (M6) and the command layer (LP3)): a view refuses workspace and preview effects, declared or
/// reached through a helper or a higher-order argument, and may publish what it reads.
#[test]
fn ct_lg3_03_views_refuse_workspace_and_preview_effects_in_source() {
    let helpers = "fn save(w: Word) -> Unit effects [workspace] {\n    return write_workspace<Word>(\"k\", w);\n}\n\n\
                   fn prepare(w: Word) -> Receipt effects [preview] {\n    return propose(w, target(\"t\"));\n}\n\n\
                   fn apply<E>(g: fn(Word) -> Unit effects [E], w: Word) -> Unit effects [E] {\n    \
                   return g(w);\n}\n\n";
    let views = [
        "view v(w: Word) -> Unit effects [workspace] {\n    return write_workspace<Word>(\"k\", w);\n}\n",
        "view v(w: Word) -> Unit effects [] {\n    return write_workspace<Word>(\"k\", w);\n}\n",
        "view v(w: Word) -> Receipt effects [preview] {\n    return propose(w, target(\"t\"));\n}\n",
        "view v(w: Word) -> Unit effects [workspace] {\n    return save(w);\n}\n",
        "view v(w: Word) -> Receipt effects [preview] {\n    return prepare(w);\n}\n",
        "view v(w: Word) -> Unit effects [workspace] {\n    return apply(save, w);\n}\n",
    ];
    for view in views {
        let codes = report(&[&tool("", &format!("{helpers}{view}"))]).codes();
        assert!(!codes.is_empty() && codes.iter().all(|c| *c == EFFECT_UNDECLARED), "{view}: {codes:?}");
    }
    let publish =
        "view v() -> Unit effects [snapshot, view] {\n    return publish_view(read_snapshot<Nat>(\"n\"));\n}\n";
    assert!(report(&[&tool("", &format!("{helpers}{publish}"))]).is_admitted());
}

/// CT-LG3-04 (part: the static half in source; the IR half is `c600-check`'s `ct_lg3_04_*` and the run-time refusal is
/// `c600-run`'s `ct_lg3_04_an_ungranted_request_fails_at_run_time`, and the row is whole with them): file, clock and
/// randomness need a declaration and a grant in every profile; network, process and session files are refused even
/// when the manifest requests them and the run grants them. An ungranted clock or file read and a network request are
/// `LG3:capability.ambient_read`, the rest `LG3:capability.not_granted` (owner, 16 November 2026, PHS.52).
#[test]
fn ct_lg3_04_requests_need_a_declaration_and_a_grant_in_source() {
    let requests = [
        (
            "file",
            Capability::File,
            "fn f() -> Nat effects [file] {\n    return file_read<Nat>(\"data\");\n}\n",
            AMBIENT_READ,
        ),
        (
            "clock",
            Capability::Clock,
            "fn f() -> Display<Nat> effects [clock] {\n    return read_clock();\n}\n",
            AMBIENT_READ,
        ),
        (
            "random",
            Capability::Randomness,
            "fn f() -> Nat effects [random] {\n    return random_draw();\n}\n",
            NOT_GRANTED,
        ),
    ];
    for (word, cap, declaration, code) in requests {
        for profile in PROFILES {
            let run = |requires: &str, grant: &Grant| {
                let p = program(&[&tool(requires, declaration)]);
                admit(&p, "tool", "f", profile, grant, &Policy::interim()).unwrap().codes()
            };
            let case = format!("{word} in {profile:?}");
            assert_eq!(run("", &Grant::none()), [EFFECT_UNDECLARED, code], "{case}");
            assert_eq!(run("", &Grant::of(&[cap])), [EFFECT_UNDECLARED], "{case}");
            assert_eq!(run(word, &Grant::none()), [code], "{case}");
            assert!(run(word, &Grant::of(&[cap])).is_empty(), "{case}");
        }
    }
    let codes = [AMBIENT_READ, NOT_GRANTED, NOT_GRANTED];
    for ((word, cap), code) in ["network", "process", "session_file"].into_iter().zip(NEVER_GRANTED).zip(codes) {
        let p = program(&[&tool(word, "fn f() -> Nat effects [] {\n    return 1;\n}\n")]);
        assert_eq!(p.modules[0].capabilities.iter().copied().collect::<Vec<_>>(), [cap]);
        for profile in PROFILES {
            let a = admit(&p, "tool", "f", profile, &Grant::of(&NEVER_GRANTED), &Policy::interim()).unwrap();
            assert_eq!(a.codes(), [code], "{word} in {profile:?}");
        }
    }
}
