//! CT-LB1-03, the 600-cell-Full half (LB1.8, LB1.9; chapter 03's test table): inverse, conjugate, commutator and
//! powers construct words without state access, application checks each prefix, and an inverse returns an admitted
//! path to its start. The words are built by language code: the library's module `words` (powers) and the IR draft's
//! own operations (concatenation, inverse, conjugate, commutator), run on `c600-run` with no host. The 600-cell-Full
//! kernel read from `assets/` applies them.
//!
//! What runs: the language part (the library's source parses, keeps its canonical layout, lowers to one module that
//! requests no capability and checks with no diagnostic, and `power` has no effect, reads no state, prepares no preview
//! and names no proposal target; `power` and a client that imports the library by digest build the expected letters,
//! literally, with no cancellation and no host); the application part (`Kernel::replay`, fed a library-built word in
//! chunks, finds the first prefix that moves a protected orbit as soon as that prefix is pushed, clears every prefix
//! when no orbit is protected, and ends where `Kernel::derive` ends); and the inverse part (a library-built word and
//! its inverse retrace one path, prefix by prefix, from solved and from a scrambled state).
//!
//! Not run here: the jumbling half, which C8 runs on Rust constructors over one pinned menu
//! (`c600-jumble-tool/tests/ct_lb1_03.rs`) and in the language on its library's menu words
//! (`c600-jumble-tool/tests/library.rs`); and the refusal `LB1:input.incompatible_alphabet`, which the checker gives,
//! not the library, for a menu word where a `Word` is expected. A pass is source and fixture evidence.

mod common;

use std::collections::BTreeSet;

use c600_check::{check, separation, Policy, Separation};
use c600_digest::sha256_hex;
use c600_ir::{Callee, Expr};
use c600_kernel::{codes, Derivation, ProtectedSet, Verdict};
use c600_library::{literal_words, lower_source, lower_sources, source_sha256, EXPORTS, MODULE, SOURCE};
use c600_syntax::{format, parse};
use common::{
    after, chunks, client, client_source, effect, int, kernel, loaded, moved_orbits, power, scrambled, wide, word,
    CLIENT,
};

/// Every node of an IR expression, in path order.
fn nodes<'e>(e: &'e Expr, out: &mut Vec<&'e Expr>) {
    out.push(e);
    for child in e.children() {
        nodes(child, out);
    }
}

/// CT-LB1-03, the language part: construction reads no state. The library's source parses with no diagnostic, is in
/// the formatter's canonical layout with LF line endings, and lowers to one module, `words`, that requests no
/// capability and exports `power` only. The checker reports no diagnostic; `power`'s summary has no effect,
/// capability, effect variable, state read, grant-table row or unresolved call, so under capability separation it
/// neither reads state nor prepares previews, and its body proposes nothing. It uses the IR draft's `concat` and
/// `inverse` and exact integer operations, and calls only itself, as the catalogue row's Backing says.
#[test]
fn ct_lb1_03_the_library_source_checks_and_power_reads_no_state() {
    let file = parse(SOURCE.as_bytes());
    assert!(file.diagnostics.is_empty() && file.holes.is_empty(), "{:#?}", file.diagnostics);
    assert_eq!(format(&file).as_deref(), Some(SOURCE), "the source is in the canonical layout");
    assert!(!SOURCE.contains('\r') && SOURCE.ends_with('\n'), "LF line endings");

    let program = lower_source(SOURCE).expect("the library lowers");
    let [module] = &program.modules[..] else { panic!("one module, not {}", program.modules.len()) };
    assert_eq!(module.name, MODULE);
    assert!(module.capabilities.is_empty(), "the module requests no capability");
    let exports: Vec<&str> = module.functions.iter().filter(|f| f.exported).map(|f| f.name.as_str()).collect();
    assert_eq!(exports, EXPORTS);

    let report = check(&program).expect("well-formed IR");
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    let summary = report.summary(MODULE, "power").expect("a summary of power");
    assert!(summary.is_pure(), "{summary:?}");
    assert!(summary.effects.is_empty() && summary.capabilities.is_empty() && summary.vars.is_empty());
    assert!(summary.state_reads.is_empty() && summary.rows.is_empty() && summary.unresolved.is_empty());
    let separated = separation(summary, &Policy::interim());
    assert_eq!(separated, Separation { reads_state: false, prepares_previews: false });

    let power_fn = module.functions.iter().find(|f| f.name == "power").expect("the module defines power");
    let mut all = Vec::new();
    nodes(&power_fn.body, &mut all);
    assert!(!all.iter().any(|e| matches!(e, Expr::Propose { .. })), "power names no proposal target");
    let ops: BTreeSet<String> = all
        .iter()
        .filter_map(|e| match e {
            Expr::Prim { op, .. } => Some(format!("{op:?}")),
            _ => None,
        })
        .collect();
    assert_eq!(ops, BTreeSet::from(["Concat", "ExactAdd", "ExactEq", "ExactLess", "Inverse"].map(String::from)));
    let calls: BTreeSet<(String, String)> = all
        .iter()
        .filter_map(|e| match e {
            Expr::Call { callee: Callee::Fn { module, name }, .. } => {
                Some((module.clone().unwrap_or_else(|| MODULE.into()), name.clone()))
            }
            Expr::Call { .. } => panic!("a call of a function value"),
            _ => None,
        })
        .collect();
    assert_eq!(calls, BTreeSet::from([(MODULE.to_string(), "power".to_string())]));

    // The catalogue row: LB1.2's five fields, with the checker's effects and the source the module was built from.
    let row = literal_words();
    assert_eq!(row.entry, "Literal words");
    assert_eq!(row.effects.iter().copied().collect::<BTreeSet<_>>(), summary.effects, "Effect: pure");
    assert!(row.level.starts_with("600-cell-Full signed generators"));
    assert!(row.backing.contains("`words.power`") && row.notes.contains("checks no state"));
    assert_eq!((row.module, row.functions), (MODULE, &EXPORTS[..]));
    assert_eq!(row.source_sha256, sha256_hex(SOURCE.as_bytes()));
    assert_eq!(row.source_sha256, source_sha256());
}

/// CT-LB1-03, the language part: inverse, conjugate, commutator and powers construct words without state access.
/// Each word is built on `c600-run` with no host, so the run has no state to read; it completes, and its letters are
/// the literal words, with no cancellation: `power` writes `w` out `n` times, or `inverse(w)` out `-n` times, and
/// `power(w, 0)` is `word[]`. A client lowered with the library imports it by the digest of its source and combines
/// `words.power` with the IR draft's conjugate and commutator; the checker infers no effect for any of its functions.
#[test]
fn ct_lb1_03_words_are_built_letter_by_letter_with_no_host() {
    let loaded = loaded();
    assert_eq!(power(&loaded, &[1, -2], 3), [1, -2, 1, -2, 1, -2]);
    assert_eq!(power(&loaded, &[1, -1], 2), [1, -1, 1, -1], "nothing cancels");
    assert_eq!(power(&loaded, &[1, -2], 1), [1, -2]);
    assert_eq!(power(&loaded, &[1, -2], 0), [0i32; 0]);
    assert_eq!(power(&loaded, &[], 4), [0i32; 0]);
    assert_eq!(power(&loaded, &[1, -2], -1), [2, -1]);
    assert_eq!(power(&loaded, &[1, -2], -2), [2, -1, 2, -1]);
    assert_eq!(power(&loaded, &[1200, -1200], -3), [1200, -1200, 1200, -1200, 1200, -1200]);

    assert_eq!(client(&loaded, "inverse_of", vec![word(&[3, -7, 11])]), [-11, 7, -3]);
    assert_eq!(client(&loaded, "conjugate_of", vec![word(&[3]), word(&[1, -2])]), [3, 1, -2, -3]);
    assert_eq!(client(&loaded, "commutator_of", vec![word(&[1]), word(&[1])]), [1, 1, -1, -1], "nothing cancels");
    assert_eq!(client(&loaded, "commutator_of", vec![word(&[1, 2]), word(&[-3])]), [1, 2, -3, -2, -1, 3]);
    let squared = client(&loaded, "commutator_power", vec![word(&[1]), word(&[2]), int(2)]);
    assert_eq!(squared, [1, 2, -1, -2, 1, 2, -1, -2]);
    let inverted = client(&loaded, "commutator_power", vec![word(&[1]), word(&[2]), int(-1)]);
    assert_eq!(inverted, [2, 1, -2, -1]);
    assert_eq!(client(&loaded, "there_and_back", vec![word(&[5]), int(1)]), [5, -5], "nothing cancels");
    assert_eq!(client(&loaded, "there_and_back", vec![word(&[3, -7]), int(-2)]), [7, -3, 7, -3, 3, -7, 3, -7]);

    // The client's functions read no state either, and reach the library's power by its digest.
    let client_source = client_source();
    let program = lower_sources(&[SOURCE, &client_source]).unwrap();
    let report = check(&program).unwrap();
    for function in ["inverse_of", "conjugate_of", "commutator_of", "commutator_power", "there_and_back"] {
        let summary = report.summary(CLIENT, function).unwrap();
        assert!(summary.is_pure() && summary.state_reads.is_empty() && summary.rows.is_empty(), "{function}");
    }
    let reaches = &report.summary(CLIENT, "there_and_back").unwrap().reaches;
    assert!(reaches.contains(&(MODULE.to_string(), "power".to_string())), "{reaches:?}");
    // Without the library, or with another library source, the import does not resolve.
    assert!(lower_sources(&[&client_source]).is_err());
    let edited = SOURCE.replace("word[]", "word[+1, -1]");
    assert!(lower_sources(&[&edited, &client_source]).is_err(), "the import names the library's digest");
}

/// CT-LB1-03, application checks each prefix (LB1.9: Strict protection uses the literal word, intermediate states
/// included). Three library-built words, from solved and from a scrambled state, are fed to `Kernel::replay` in
/// chunks of growing size after an empty one. With a protected orbit (orbit 7 alone, or every orbit the word's first
/// letter moves), the first refused prefix is the first prefix whose net map moves a protected sticker, found prefix by
/// prefix through the effect row: it is reported as soon as it has been pushed, never before, and `finish()` refuses
/// there with `Kernel::derive_strict`'s prefix, certificates and protection row. On the shipped model every generator
/// moves a sticker of every orbit, so that prefix is the first. With no protected orbit no prefix is refused, and the
/// replay ends in the state `Kernel::derive` derives, with the same `Digest<literal word>`. The empty word
/// `power(w, 0)` has no prefix to check, and its replay derives the base state even with every orbit protected.
#[test]
fn ct_lb1_03_application_checks_each_prefix_of_a_library_built_word() {
    let loaded = loaded();
    let kernel = kernel();
    let words = [
        power(&loaded, &[3, -7], 3),
        client(&loaded, "commutator_power", vec![word(&[1]), word(&[2]), int(2)]),
        client(&loaded, "there_and_back", vec![word(&[5, 11]), int(-2)]),
    ];
    for base in [kernel.solved(), scrambled()] {
        for w in &words {
            let letters = wide(w);
            let recipe = kernel.word(&letters).unwrap();
            for orbits in [vec![7], moved_orbits(&w[..1])] {
                let protected = kernel.protected_set(&orbits).unwrap();
                let first = (1..=w.len())
                    .find(|&k| kernel.protection(&effect(&w[..k]), &protected).verdict == Verdict::Moves)
                    .map(|k| k as u64);
                assert_eq!(first, Some(1), "{w:?}: every generator moves every orbit on the shipped model");
                let mut replay = kernel.replay(&base, &protected).unwrap();
                for chunk in chunks(&letters) {
                    replay.push(chunk).unwrap();
                    let pushed = replay.position();
                    assert_eq!(replay.first_refused_prefix(), first.filter(|&p| p <= pushed), "{w:?} at {pushed}");
                }
                assert_eq!(replay.position(), w.len() as u64);
                let Derivation::Refused(refused) = replay.finish() else { panic!("{w:?}: Strict cleared it") };
                let Ok(Derivation::Refused(strict)) = kernel.derive_strict(&base, &recipe, &protected) else {
                    panic!("{w:?}: derive_strict cleared it");
                };
                assert_eq!((refused.first_refused_prefix, strict.first_refused_prefix), (first, first));
                assert_eq!(refused.refusal.code, codes::ORBIT_WOULD_MOVE);
                assert_eq!((&refused.certificates, &refused.protection), (&strict.certificates, &strict.protection));
                assert_eq!((refused.base.as_str(), &refused.word), (base.digest(), &strict.word));
                assert!(!refused.certificates.is_empty());
                assert!(refused.certificates.iter().all(|c| protected.orbits().contains(&c.orbit)));
            }

            let mut replay = kernel.replay(&base, &ProtectedSet::none()).unwrap();
            for chunk in chunks(&letters) {
                replay.push(chunk).unwrap();
                assert_eq!(replay.first_refused_prefix(), None, "{w:?}: no orbit is protected");
            }
            let Derivation::Derived(end) = replay.finish() else { panic!("{w:?}: refused with no protected orbit") };
            let Ok(Derivation::Derived(net)) = kernel.derive(&base, &recipe, &ProtectedSet::none()) else {
                panic!("{w:?}: derive refused it");
            };
            assert_eq!((end.state.digest(), &end.word), (net.state.digest(), &net.word));
            assert_eq!((end.protection.verdict, end.protection.first_refused_prefix), (Verdict::Clear, None));
        }

        // `power(w, 0)` is `word[]`: it has no prefix to check, and its replay derives the base state, even with every
        // orbit protected.
        let empty = power(&loaded, &[3, -7], 0);
        let every = kernel.protected_set(&(0..35).collect::<Vec<i64>>()).unwrap();
        let mut replay = kernel.replay(&base, &every).unwrap();
        replay.push(&wide(&empty)).unwrap();
        let Derivation::Derived(still) = replay.finish() else { panic!("the empty word is refused") };
        assert_eq!((still.state.digest(), still.word.as_str()), (base.digest(), sha256_hex(&[]).as_str()));
    }
}

/// CT-LB1-03, an inverse returns an admitted path to its start (LB1.9: the inverse of an admitted word is admitted
/// from its end). On 600-cell-Full every letter of a well-formed word applies at every state, so a word is admitted
/// unless protection refuses a prefix; on the shipped model any protected orbit refuses every word at its first
/// prefix, so the words here are admitted with no protected orbit. A power and a commutator built by the library are
/// admitted from solved and from a scrambled state; the IR draft's `inverse`, which equals the library's
/// `power(w, -1)`, is admitted from the word's end, letter by letter through `Kernel::replay`; after `k` letters of
/// the inverse the state is the state after `|w| - k` letters of the word, so the inverse retraces the path, and it
/// ends at the start.
#[test]
fn ct_lb1_03_an_inverse_returns_an_admitted_path_to_its_start() {
    let loaded = loaded();
    let kernel = kernel();
    let none = ProtectedSet::none();
    let words = [power(&loaded, &[3, -7], 3), client(&loaded, "commutator_of", vec![word(&[1, 2]), word(&[-29])])];
    for base in [kernel.solved(), scrambled()] {
        for w in &words {
            let Ok(Derivation::Derived(there)) = kernel.derive_strict(&base, &kernel.word(&wide(w)).unwrap(), &none)
            else {
                panic!("{w:?} is not admitted");
            };
            let end = there.state;
            assert_ne!(end.digest(), base.digest(), "{w:?} moves stickers");
            let inverse = client(&loaded, "inverse_of", vec![word(w)]);
            assert_eq!(inverse, power(&loaded, w, -1));
            assert_eq!(inverse.len(), w.len());

            let mut replay = kernel.replay(&end, &none).unwrap();
            for k in 0..=inverse.len() {
                if k > 0 {
                    replay.push(&wide(&inverse[k - 1..k])).unwrap();
                    assert_eq!(replay.first_refused_prefix(), None, "{w:?}: the inverse's prefix of {k} letters");
                }
                let retraced = after(&end, &inverse[..k]);
                assert_eq!(retraced.digest(), after(&base, &w[..w.len() - k]).digest(), "{w:?}: {k} letters back");
            }
            let Derivation::Derived(back) = replay.finish() else { panic!("{w:?}: the inverse is refused") };
            assert_eq!(back.state.digest(), base.digest(), "{w:?}: the inverse ends at the start");
            assert_eq!(back.effect.primitive_count, w.len());
        }
    }
}
