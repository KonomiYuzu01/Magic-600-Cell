//! The parse's additions for the authoring services (LP4-1; EV1.2; M7 deferred them, PHS.21): comments and
//! whitespace kept as trivia, the tokens a host reads, and what the parser recovered of a broken declaration. None of
//! them changes what parses, lowers or formats; the other tests of this crate run unchanged.

use c600_syntax::ast::{BindingKind, DeclKind, TriviaKind};
use c600_syntax::{format, lexemes, lower, parse, pinned, LexemeKind};

const HEADER: &str = "edition \"6cs-draft-1\";\nmodule tool requires [];\n\n";

#[test]
fn comments_and_whitespace_are_kept_as_trivia_outside_the_program() {
    let source = format!("{HEADER}# first\nfn f(x: Nat) -> Nat effects [] {{\n    # inner λ\n    return x;\n}}\n");
    let file = parse(source.as_bytes());
    assert!(file.diagnostics.is_empty());
    let comments: Vec<&str> = file
        .trivia
        .iter()
        .filter(|t| t.kind == TriviaKind::Comment)
        .map(|t| &source[t.span.start..t.span.end])
        .collect();
    assert_eq!(comments, ["# first", "# inner λ"]);
    // Tokens and trivia tile the text between the header and the end.
    let (tokens, trivia) = lexemes(&source);
    let mut pieces: Vec<(usize, usize)> = tokens.iter().map(|t| (t.span.start, t.span.end)).collect();
    pieces.extend(trivia.iter().map(|t| (t.span.start, t.span.end)));
    pieces.sort();
    pieces.retain(|(s, e)| s < e);
    let mut at = 0;
    for (s, e) in pieces {
        assert_eq!(s, at);
        at = e;
    }
    assert_eq!(at, source.len());
    assert_eq!(trivia, file.trivia);
    assert_eq!(tokens.last().map(|t| t.kind), Some(LexemeKind::End));
    let inner = file.trivia.iter().find(|t| source[t.span.start..].starts_with("# inner")).unwrap();
    assert_eq!((inner.span.line, inner.span.column), (6, 5));

    // The program ignores trivia: the same program without comments is the same program, and the layout drops them.
    let bare = parse(source.replace("# first\n", "").replace("    # inner λ\n", "").as_bytes());
    assert!(file.same_program(&bare));
    assert_eq!(format(&file), format(&bare));
    assert!(lower(&[file]).is_ok());
}

#[test]
fn a_broken_declaration_keeps_what_the_parser_read() {
    let source = format!(
        "{HEADER}export command go<T>(w: Word, n: Nat) -> Nat effects [] {{\n    let m = repeat(n, 0, fn(acc: Nat) -> \
         Nat {{ return acc; }});\n    let k = match tag(\"x\") {{ holds(v) => v; }};\n    return m(\n\n\
         fn after() -> Nat effects [] {{\n    return 1;\n}}\n"
    );
    let file = parse(source.as_bytes());
    assert_eq!(file.holes.len(), 1);
    assert_eq!(file.recovered.len(), 1);
    assert_eq!(file.declarations().map(|d| d.name.text.as_str()).collect::<Vec<_>>(), ["after"]);
    let r = &file.recovered[0];
    assert_eq!((r.kind, r.exported), (Some(DeclKind::Command), true));
    assert_eq!(r.name.as_ref().map(|n| n.text.as_str()), Some("go"));
    assert_eq!(r.hole, file.holes[0]);
    let bindings: Vec<(&str, BindingKind)> = r.bindings.iter().map(|b| (b.name.text.as_str(), b.kind)).collect();
    assert_eq!(
        bindings,
        [
            ("T", BindingKind::Generic),
            ("w", BindingKind::Parameter),
            ("n", BindingKind::Parameter),
            ("acc", BindingKind::ClosureParameter),
            ("m", BindingKind::Let),
            ("v", BindingKind::MatchBinder),
            ("k", BindingKind::Let),
        ]
    );
    let scope = |name: &str| {
        let b = r.bindings.iter().find(|b| b.name.text == name).unwrap();
        &source[b.from..b.to]
    };
    // A closed scope ends where it closed; an open one at the hole's end.
    assert_eq!(scope("acc"), ") -> Nat { return acc; }");
    assert_eq!(scope("v"), " => v");
    assert!(scope("m").starts_with("\n    let k") && scope("m").ends_with("return m(\n\n"));
    assert_eq!(r.bindings.iter().find(|b| b.name.text == "w").unwrap().to, r.hole.end);
    assert_eq!(&source[r.stopped.start..r.stopped.end], "fn", "parsing stopped at the next declaration");
}

#[test]
fn the_byte_check_is_the_parser_s() {
    assert_eq!(pinned(b"a\nb"), Ok("a\nb"));
    for bad in [&b"a\xffb"[..], b"a\r\nb", b"\xef\xbb\xbfa"] {
        let refused = pinned(bad).unwrap_err();
        assert_eq!(parse(bad).diagnostics, vec![refused]);
    }
}
