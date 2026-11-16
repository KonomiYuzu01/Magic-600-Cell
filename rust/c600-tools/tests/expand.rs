//! The source route's recipe expansion over the frozen oracle cases O01 to O07 (LP3, C5, phase A): each step's
//! recipe is expanded through the kernel, outside any program, and checked against the case's record.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;
use c600_tools::case::{read_document, steps};
use c600_tools::expand::{expand, literal_digest, Expansion};

const CASES: [&str; 7] = ["O01", "O02", "O03", "O04", "O05", "O06", "O07"];

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn kernel() -> &'static Kernel {
    static KERNEL: OnceLock<Kernel> = OnceLock::new();
    KERNEL.get_or_init(|| match Kernel::load(&repository().join("assets")) {
        Ok(kernel) => kernel,
        Err(error) => panic!("the model in assets/ does not load: {error}"),
    })
}

fn case(name: &str) -> Value {
    let path = repository().join("work/experiments/language-lx/inputs/oracle").join(format!("{name}.json.gz"));
    read_document(&path).unwrap_or_else(|error| panic!("{name}: {error}"))
}

/// Every step of O01 to O07, 3,201 in all, expands without a state read. A step the oracle accepted or refused for
/// protection expands to a literal word whose digest and length are the step's recorded `expansion_sha256` and
/// `primitive_count`. Of O07's invalid-input steps, the four single words of integer moves (steps 0, 1, 2 and 8) pass
/// unchanged with the kernel's refusal beside them; the other six have no literal word and keep only the refusal.
#[test]
fn every_case_step_expands_to_the_recorded_literal_word() {
    let (mut total, mut letters) = (0, 0);
    let (mut unchecked, mut refused) = (Vec::new(), Vec::new());
    for name in CASES {
        let case = case(name);
        for step in steps(&case).unwrap() {
            let expect = step.expect.unwrap_or_else(|| panic!("{name} {}: no expect", step.index));
            let expansion = expand(kernel(), step.recipe);
            let at = (name, step.index);
            match expect.get("error").and_then(Value::as_str) {
                Some("invalid-input") => match &expansion {
                    Expansion::Unchecked { letters, refusal } => {
                        assert!(refusal.code.starts_with("KN2:"), "{at:?}: {refusal}");
                        let moves = &step.recipe.as_array().unwrap()[0].get("moves").unwrap().canonical();
                        assert_eq!(moves, &json::parse(format!("{letters:?}").as_bytes()).unwrap().canonical());
                        unchecked.push(at);
                    }
                    Expansion::Refused(refusal) => {
                        assert!(refusal.code.starts_with("KN2:"), "{at:?}: {refusal}");
                        refused.push(at);
                    }
                    Expansion::Literal(_) => panic!("{at:?}: the kernel accepted an invalid input"),
                },
                _ => {
                    let Expansion::Literal(word) = &expansion else { panic!("{at:?}: {expansion:?}") };
                    assert_eq!(
                        Some(literal_digest(word).as_str()),
                        expect.get("expansion_sha256").and_then(Value::as_str)
                    );
                    assert_eq!(
                        Some(word.len() as i64),
                        expect.get("primitive_count").and_then(Value::as_i64),
                        "{at:?}"
                    );
                    letters += word.len();
                }
            }
            total += 1;
        }
    }
    assert_eq!(total, 3201);
    assert_eq!(letters, 660_047);
    assert_eq!(unchecked, [("O07", 0), ("O07", 1), ("O07", 2), ("O07", 8)]);
    assert_eq!(refused, [("O07", 3), ("O07", 4), ("O07", 5), ("O07", 6), ("O07", 7), ("O07", 9)]);
}

/// The expansion is the kernel's: a star expands to its recorded path, a word to its letters, and the digest is the
/// literal digest the kernel records (`Digest<literal word>` of `[1, -1]`, O06 step 3).
#[test]
fn the_expansion_is_the_kernels_literal() {
    let parse = |text: &str| json::parse(text.as_bytes()).unwrap();
    let word = expand(kernel(), &parse(r#"[{"kind": "word", "moves": [1, -1]}]"#));
    assert_eq!(word, Expansion::Literal(vec![1, -1]));
    assert_eq!(literal_digest(&[1, -1]), "b15348c8f462384c01e83b6d499c6faf3f96808f5aa07c6bab4b65b36b4445d4");
    let star = parse(r#"[{"kind": "star", "orbit": 2, "node": 0, "sign": 1}]"#);
    let recipe = kernel().recipe(&star).unwrap();
    assert_eq!(expand(kernel(), &star), Expansion::Literal(kernel().letters(&recipe).unwrap()));
    assert_eq!(expand(kernel(), &star).letters().map(<[i32]>::len), Some(9230));
    for (text, passes) in [
        (r#"[{"kind": "word", "moves": [0]}]"#, true),
        (r#"[{"kind": "word", "moves": [1]}, {"kind": "word", "moves": [0]}]"#, false),
        (r#"[{"kind": "star", "orbit": 35, "node": 0, "sign": 1}]"#, false),
        (r#"[{"kind": "word", "moves": [4294967296]}]"#, false),
        (r#"[{"kind": "word", "moves": [1.5]}]"#, false),
        (r#"[]"#, false),
    ] {
        let expansion = expand(kernel(), &parse(text));
        assert_eq!(matches!(expansion, Expansion::Unchecked { .. }), passes, "{text}");
        assert_eq!(matches!(expansion, Expansion::Refused(_)), !passes, "{text}");
    }
}
