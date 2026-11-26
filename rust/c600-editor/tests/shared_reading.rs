//! A document's parse is made once and shared by the services that ask about it (no CT ID of its own; LX-4's editor
//! latency, 26 November 2026). Sharing it changes no answer: parse, complete and diagnose answer a document that an
//! earlier service has read exactly as they answer a fresh one, in either order, for complete, broken, importing and
//! unpinned sources.

mod support;

use c600_editor::{complete, diagnose, parse, Document, ModuleContext, ObligationRole, Units};
use support::{importer, open, tool, LIB};

/// The answers of every service, as text, on one document, asked in `order`.
fn answers(document: &Document, context: &ModuleContext, positions: &[usize], order: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for service in order {
        match *service {
            "parse" => out.push(format!("{:?}", parse(document, &context.edition))),
            "diagnose" => out.push(format!("{:?}", diagnose(document, document.digest(), context))),
            "complete" => {
                for &p in positions {
                    out.push(format!("{p}: {:?}", complete(document, document.digest(), p, context)));
                }
            }
            other => panic!("{other}"),
        }
    }
    out
}

/// Each service's answers on a fresh document per service, in the order `order` names them.
fn fresh(bytes: &[u8], context: &ModuleContext, positions: &[usize], order: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for service in order {
        let document = Document::open(bytes.to_vec(), Units::Utf8);
        out.extend(answers(&document, context, positions, &[service]));
    }
    out
}

fn same_answers(bytes: &[u8], context: &ModuleContext) {
    let positions: Vec<usize> = (0..=bytes.len()).step_by(7).chain([bytes.len()]).collect();
    for order in
        [["parse", "complete", "diagnose"], ["diagnose", "complete", "parse"], ["complete", "diagnose", "parse"]]
    {
        let shared = Document::open(bytes.to_vec(), Units::Utf8);
        let once = answers(&shared, context, &positions, &order);
        assert_eq!(once, fresh(bytes, context, &positions, &order), "{order:?}");
        assert_eq!(answers(&shared, context, &positions, &order), once, "{order:?}, asked again");
    }
}

/// The shared parse keeps a document sendable and shareable between threads, as its bytes alone made it.
#[test]
fn a_document_stays_send_and_sync() {
    fn send_and_sync<T: Send + Sync>() {}
    send_and_sync::<Document>();
}

#[test]
fn a_shared_parse_gives_the_answers_of_a_fresh_one() {
    let context = ModuleContext::new().with_obligation("f", ObligationRole::Evidence);
    let clean = tool(
        "fn f<T>(r: record { a: Nat, b: Text }, t: T) -> Nat effects [snapshot] {\n    # a comment\n    \
         let s = \"text\";\n    return r.a;\n}\n\nfn g(n: Nat) -> Nat effects [] {\n    return exact_add(n, f);\n}\n",
    );
    same_answers(clean.as_bytes(), &context);
    let broken = tool(
        "fn helper(a: Nat) -> Nat effects [] {\n    return a;\n}\n\n\
         command go(w: Word, n: Nat) -> Nat effects [] {\n    let m = helper(n);\n    return he",
    );
    same_answers(broken.as_bytes(), &context);

    let mut with_lib = ModuleContext::new();
    let lib = with_lib.add_dependency(LIB.as_bytes().to_vec(), None);
    let importing = importer(&lib, "fn f(n: Nat) -> Nat effects [] {\n    return lib.d(n);\n}\n");
    same_answers(importing.as_bytes(), &with_lib);
    same_answers(importing.as_bytes(), &ModuleContext::new());

    let unpinned = format!("{}\r\n", tool("fn f() -> Nat effects [] {\n    return 1;\n}"));
    assert!(open(&unpinned).text().is_none(), "a carriage return is not pinned source");
    same_answers(unpinned.as_bytes(), &context);
    let mut not_utf8 = clean.as_bytes().to_vec();
    not_utf8.push(0xFF);
    same_answers(&not_utf8, &context);
    let marked = format!("\u{FEFF}{clean}");
    assert!(open(&marked).text().is_none(), "a byte-order mark is not pinned source");
    same_answers(marked.as_bytes(), &context);

    // An edition this parser does not read is refused, before and after the parse is shared.
    let mut other = ModuleContext::new();
    other.edition = "6cs-draft-0".to_owned();
    same_answers(clean.as_bytes(), &other);
}

/// The shared parse is a cache: a document's debug text is the same before and after a service has made it.
#[test]
fn a_shared_parse_leaves_the_debug_text_alone() {
    let source = tool("fn f() -> Nat effects [] {\n    return 1;\n}\n");
    let fresh = open(&source);
    let expected = format!("{fresh:?} {fresh:#?}");
    let read = open(&source);
    let _ = diagnose(&read, read.digest(), &ModuleContext::new());
    assert_eq!(format!("{read:?} {read:#?}"), expected);
    assert!(expected.starts_with("Document { content: "), "{expected}");
}
