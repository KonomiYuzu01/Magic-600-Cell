//! CT-LB2-02, the Strict half as the 600-cell-Full kernel judges it on words the library built (LB2.3: Net tests the
//! end of the complete operation, Strict every executed prefix; KN2.7, KN2.11). A word with identity net effect whose
//! intermediate prefixes move a protected orbit passes Net and fails Strict, and an incomplete prefix check never
//! returns safe. The words are built by language code, `words.power` and the IR draft's `concat` and `inverse`, run on
//! `c600-run` with no host; the protected set is the kernel's set of protected orbits.
//!
//! Both halves also run through the command layer in `rust/c600-tools/tests/net_oversize.rs` (the Strict half on the
//! layer's Strict policy, LP2's pull request 198), and LP2's `rust/c600-kernel/tests/kernel.rs` runs the kernel's own
//! half on the literal word `[1, -1]`. Not built: the captured lock of LB2's row "Net and Strict protection" (block,
//! identity and position locks) and LB2.3's coverage report with `LB2:protection.incomplete_coverage`, which are
//! library entries over kernel rows; the tool's host bridge answers a snapshot read with an argument for KN2.4's
//! `positions`, `labels` and `effect` rows, not `protection` (`c600_tools::kernel_rows`).
//! A pass is source and fixture evidence.

mod common;

use c600_digest::sha256_hex;
use c600_kernel::{codes, Derivation, ProtectionMode, Verdict};
use common::{client, effect, int, kernel, loaded, moved_orbits, scrambled, wide, word};

/// `Digest<literal word>`: the SHA-256 of the letters as little-endian 32-bit integers.
fn literal_digest(letters: &[i32]) -> String {
    sha256_hex(&letters.iter().flat_map(|m| m.to_le_bytes()).collect::<Vec<u8>>())
}

/// CT-LB2-02, Net against Strict. Three library-built words have identity net effect: `concat(word[+5],
/// inverse(word[+5]))`, a power followed by its inverse, and a negative power followed by its inverse. From solved and
/// from a scrambled state, with orbit 7 protected or with every orbit the word's first letter moves, some intermediate
/// prefix moves a protected sticker (each prefix judged through the effect row). `Kernel::derive` (Net) derives the
/// word with a clear row and no conflict, and its end state is the base. `Kernel::derive_strict` refuses it with
/// `KN2:protection.orbit_would_move` at the first such prefix, leaves the base as it was, reports the same net effect,
/// and gives one certificate for each protected orbit that prefix moves, each a move the prefix makes. On the shipped
/// model every generator moves a sticker of every orbit, so the first refused prefix is the first letter.
#[test]
fn ct_lb2_02_an_identity_word_from_the_library_passes_net_and_fails_strict() {
    let loaded = loaded();
    let kernel = kernel();
    let words = [
        client(&loaded, "there_and_back", vec![word(&[5]), int(1)]),
        client(&loaded, "there_and_back", vec![word(&[1, 2]), int(2)]),
        client(&loaded, "there_and_back", vec![word(&[3, -7]), int(-3)]),
    ];
    assert_eq!(words[0], [5, -5]);
    assert_eq!(words[1], [1, 2, 1, 2, -2, -1, -2, -1]);
    for base in [kernel.solved(), scrambled()] {
        for w in &words {
            let recipe = kernel.word(&wide(w)).unwrap();
            assert_eq!(effect(w).moved_stickers(), 0, "{w:?}: identity net effect");
            for orbits in [vec![7], moved_orbits(&w[..1])] {
                let protected = kernel.protected_set(&orbits).unwrap();
                let moving: Vec<usize> = (1..w.len())
                    .filter(|&k| kernel.protection(&effect(&w[..k]), &protected).verdict == Verdict::Moves)
                    .collect();
                let first = *moving.first().unwrap_or_else(|| panic!("{w:?}: no intermediate prefix moves"));
                assert_eq!(first, 1, "{w:?}: every generator moves every orbit on the shipped model");

                let Ok(Derivation::Derived(net)) = kernel.derive(&base, &recipe, &protected) else {
                    panic!("{w:?}: Net refused an identity word");
                };
                assert_eq!(net.state.digest(), base.digest(), "{w:?}: the end state is the base");
                let row = &net.protection;
                assert_eq!((row.mode, row.verdict, row.conflicts.len()), (ProtectionMode::Net, Verdict::Clear, 0));
                assert_eq!(row.first_refused_prefix, None, "Net judges only the end of the complete operation");

                let Ok(Derivation::Refused(strict)) = kernel.derive_strict(&base, &recipe, &protected) else {
                    panic!("{w:?}: Strict cleared a word with a moving prefix");
                };
                assert_eq!(strict.first_refused_prefix, Some(first as u64), "{w:?}");
                assert_eq!(strict.protection.first_refused_prefix, Some(first as u64));
                let row = &strict.protection;
                assert_eq!((row.mode, row.verdict), (ProtectionMode::Strict, Verdict::Moves));
                assert_eq!(strict.refusal.code, codes::ORBIT_WOULD_MOVE);
                assert_eq!((strict.base.as_str(), &strict.word), (base.digest(), &literal_digest(w)));
                assert_eq!(strict.effect, net.effect, "{w:?}: the same net effect under both modes");

                let prefix = effect(&w[..first]);
                let moved: Vec<usize> = prefix
                    .support
                    .iter()
                    .filter_map(|row| usize::try_from(row.orbit).ok())
                    .filter(|orbit| protected.orbits().contains(orbit))
                    .collect();
                let named: Vec<usize> = strict.certificates.iter().map(|certificate| certificate.orbit).collect();
                assert_eq!(named, moved, "{w:?}: one certificate per protected orbit the prefix moves");
                for certificate in &strict.certificates {
                    let at = prefix.source().binary_search(&certificate.source).expect("the prefix moves the source");
                    assert_eq!(prefix.destination()[at], certificate.destination);
                }
            }
        }
    }
}

/// CT-LB2-02, an incomplete prefix check never returns safe. `Kernel::replay` gives no verdict while a word is pushed:
/// `first_refused_prefix` reports a refusal as soon as its prefix is in, and its `None` says only that no prefix pushed
/// so far is refused. Before any letter of a library-built identity word is pushed it is `None`, although the whole
/// word is refused. Only `finish()` concludes, and it concludes on exactly the letters pushed: a replay finished
/// before any letter derives the base state under the digest of the empty word, never the whole word's, so its clear
/// row is no verdict on the word. The replay pushed whole, in chunks, refuses where `Kernel::derive_strict` refuses.
#[test]
fn ct_lb2_02_an_incomplete_prefix_check_never_returns_safe() {
    let loaded = loaded();
    let kernel = kernel();
    let w = client(&loaded, "there_and_back", vec![word(&[1, 2]), int(2)]);
    let letters = wide(&w);
    let base = kernel.solved();
    let protected = kernel.protected_set(&[7]).unwrap();
    let recipe = kernel.word(&letters).unwrap();
    let Ok(Derivation::Refused(whole)) = kernel.derive_strict(&base, &recipe, &protected) else {
        panic!("Strict cleared the word");
    };
    assert_eq!(whole.word, literal_digest(&w));

    // Nothing pushed: no refusal yet, and no verdict.
    let mut early = kernel.replay(&base, &protected).unwrap();
    early.push(&[]).unwrap();
    assert_eq!((early.position(), early.first_refused_prefix()), (0, None));
    let Derivation::Derived(empty) = early.finish() else { panic!("the empty word is refused") };
    assert_eq!(empty.state.digest(), base.digest());
    assert_eq!(empty.word, literal_digest(&[]));
    assert_ne!(empty.word, whole.word, "the early finish judged another word");

    // Pushed in parts: the refusal is known as soon as its prefix is in, and stays; `finish()` concludes the word.
    let mut replay = kernel.replay(&base, &protected).unwrap();
    replay.push(&[]).unwrap();
    assert_eq!(replay.first_refused_prefix(), None);
    for part in letters.chunks(3) {
        replay.push(part).unwrap();
        assert_eq!(replay.first_refused_prefix(), whole.first_refused_prefix, "at {}", replay.position());
    }
    let Derivation::Refused(refused) = replay.finish() else { panic!("the replay cleared the word") };
    assert_eq!((refused.first_refused_prefix, &refused.word), (whole.first_refused_prefix, &whole.word));
    assert_eq!((&refused.certificates, &refused.protection), (&whole.certificates, &whole.protection));
}
