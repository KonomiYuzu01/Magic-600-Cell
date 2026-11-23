//! Word references (EV2.5, EV2.11; LP4-6): CT-EV2-06 whole. A compressed long word resolves cited letter positions
//! lazily, under H7's zero-based half-open intervals, keeps its alphabet and identity block, and a path names one
//! expansion occurrence.

mod support;

use std::sync::Arc;
use std::time::Instant;

use c600_debug::code::{REPLAY_DEPENDENCY_MISSING, WORD_RANGE_INVALID};
use c600_debug::profile::Alphabet;
use c600_debug::words::{literal_digest, Carrier, Letter, Node, Occurrence, Step, WordRef};
use c600_run::value::{Compressed, Part};
use support::identity;

fn g(letters: &[i32]) -> Vec<Letter> {
    letters.iter().map(|&l| Letter::Generator(l)).collect()
}

fn cite(carrier: &Carrier, start: u128, end: u128, path: Vec<Step>, offset: u128) -> WordRef {
    WordRef { occurrence: Some(Occurrence { path, offset }), ..carrier.reference(start, end) }
}

/// The chapter's example: in `repeat([a, b], 3)`, iteration 1 at offset 1 cites the `b` at `[3, 4)`, not the first
/// `b` at `[1, 2)`; a letter at i is `[i, i+1)` and `[length, length)` is the completed cursor.
#[test]
fn ct_ev2_06_a_repeat_iteration_cites_its_own_letters() {
    let ab = Node::letters(&g(&[1, 2]));
    let root = Node::repeat(ab, 3).unwrap();
    let digest = literal_digest(&g(&[1, 2, 1, 2, 1, 2]));
    let word = Carrier::new(root, Alphabet::RetainedGenerators, identity(), &digest, None).unwrap();
    assert_eq!(word.length(), 6);

    let b = word.resolve(&cite(&word, 3, 4, vec![Step::Iteration(1)], 1)).unwrap();
    assert_eq!((b.start, b.end, b.letters.as_slice()), (3, 4, &g(&[2])[..]));
    let first_b = word.resolve(&cite(&word, 1, 2, vec![Step::Iteration(1)], 1)).unwrap_err();
    assert_eq!(first_b.code, WORD_RANGE_INVALID, "iteration 1's offset 1 is not [1, 2)");
    assert_eq!(word.resolve(&cite(&word, 1, 2, vec![Step::Iteration(0)], 1)).unwrap().letters, g(&[2]));
    assert_eq!(word.resolve(&cite(&word, 6, 6, vec![Step::Iteration(3)], 0)).unwrap_err().code, WORD_RANGE_INVALID);

    let done = word.resolve(&word.reference(6, 6)).unwrap();
    assert!(done.letters.is_empty(), "the completed cursor");
    assert_eq!(word.resolve(&word.reference(0, 6)).unwrap().letters, g(&[1, 2, 1, 2, 1, 2]));
    for (start, end) in [(4, 3), (0, 7), (7, 7)] {
        assert_eq!(word.resolve(&word.reference(start, end)).unwrap_err().code, WORD_RANGE_INVALID, "[{start}, {end})");
    }
}

/// One part used twice has two paths; an inverse part's offsets count its emitted letters, each inverted.
#[test]
fn ct_ev2_06_paths_tell_occurrences_and_inverse_traversals_apart() {
    let part = Node::letters(&g(&[7, 8]));
    let tail = Node::inverse(Node::letters(&g(&[4, 5])));
    let root = Node::concat(vec![part.clone(), part, tail]).unwrap();
    let expanded = g(&[7, 8, 7, 8, -5, -4]);
    let word = Carrier::new(root, Alphabet::RetainedGenerators, identity(), &literal_digest(&expanded), None).unwrap();
    assert_eq!(word.resolve(&word.reference(0, 6)).unwrap().letters, expanded);

    let once = word.resolve(&cite(&word, 1, 2, vec![Step::Child(0)], 1)).unwrap();
    let twice = word.resolve(&cite(&word, 3, 4, vec![Step::Child(1)], 1)).unwrap();
    assert_eq!((once.letters.clone(), twice.letters.clone()), (g(&[8]), g(&[8])));
    assert_ne!((once.start, once.end), (twice.start, twice.end));
    assert_eq!(word.resolve(&cite(&word, 3, 4, vec![Step::Child(0)], 1)).unwrap_err().code, WORD_RANGE_INVALID);

    let inverse = word.resolve(&cite(&word, 4, 6, vec![Step::Child(2), Step::Inverse], 0)).unwrap();
    assert_eq!(inverse.letters, g(&[-5, -4]));
    assert_eq!(word.resolve(&cite(&word, 5, 6, vec![Step::Child(2), Step::Inverse], 1)).unwrap().letters, g(&[-4]));
    // The inverse part's own occurrence emits the same letters; a step it does not have, or a child that does not
    // exist, names no occurrence.
    assert_eq!(word.resolve(&cite(&word, 4, 5, vec![Step::Child(2)], 0)).unwrap().letters, g(&[-5]));
    for path in [vec![Step::Child(2), Step::Child(0)], vec![Step::Child(3)], vec![Step::Inverse]] {
        assert_eq!(word.resolve(&cite(&word, 4, 5, path, 0)).unwrap_err().code, WORD_RANGE_INVALID);
    }
}

/// A compressed long step of 3·2^70 letters resolves a cited interval at once, keeps its alphabet and identity, and
/// refuses another digest, identity, alphabet or length, and a part whose expansion is not held.
#[test]
fn ct_ev2_06_a_compressed_long_step_resolves_lazily() {
    let mut parts = vec![Part::Leaf(Arc::from(&[1, 2, 3][..]))];
    for i in 0..70 {
        parts.push(Part::Concat(i, i));
    }
    let compressed = Compressed::from_parts(parts).unwrap();
    let digest = "d".repeat(64);
    let word = Carrier::of_compressed(&compressed, identity(), &digest).unwrap();
    let length = 3u128 << 70;
    assert_eq!(word.length(), length);

    let started = Instant::now();
    let tail = word.resolve(&word.reference(length - 4, length)).unwrap();
    assert_eq!(tail.letters, g(&[3, 1, 2, 3]));
    assert_eq!((tail.alphabet, &tail.identity), (Alphabet::RetainedGenerators, &identity()));
    // The 2^69-th doubling's second copy starts at 3·2^69: its offset 5 is the third letter of the second leaf.
    let path = vec![Step::Child(1)];
    let mid = word.resolve(&cite(&word, (3 << 69) + 5, (3 << 69) + 6, path, 5)).unwrap();
    assert_eq!(mid.letters, g(&[3]));
    assert!(started.elapsed().as_secs() < 5, "resolution walks the structure, not the expansion");

    let mut other = word.reference(0, 3);
    other.digest = "e".repeat(64);
    assert_eq!(word.resolve(&other).unwrap_err().code, REPLAY_DEPENDENCY_MISSING);
    let mut model = word.reference(0, 3);
    model.identity.model = "model-2".into();
    assert_eq!(word.resolve(&model).unwrap_err().code, WORD_RANGE_INVALID);
    let mut alphabet = word.reference(0, 3);
    alphabet.alphabet = Alphabet::MenuLetters;
    assert_eq!(word.resolve(&alphabet).unwrap_err().code, WORD_RANGE_INVALID);
    let mut short = word.reference(0, 3);
    short.length -= 1;
    assert_eq!(word.resolve(&short).unwrap_err().code, WORD_RANGE_INVALID);

    // A part whose expansion the carrier does not hold.
    let held = Node::letters(&g(&[1, 2]));
    let root = Node::concat(vec![held, Node::missing("part-digest", 1 << 40)]).unwrap();
    let partial = Carrier::new(root, Alphabet::RetainedGenerators, identity(), &digest, None).unwrap();
    assert_eq!(partial.resolve(&partial.reference(0, 2)).unwrap().letters, g(&[1, 2]));
    let gone = partial.resolve(&partial.reference(1, 3)).unwrap_err();
    assert_eq!(gone.code, REPLAY_DEPENDENCY_MISSING);
    assert!(gone.detail.contains("part-digest"));
}

/// A menu word keeps its alphabet; a letter of another alphabet is refused when the carrier is built.
#[test]
fn ct_ev2_06_menu_letters_keep_their_alphabet() {
    let letters = [Letter::Menu { cap: 1, item: 2 }, Letter::Menu { cap: 3, item: 0 }];
    let inverse: Arc<[u32]> = Arc::from(&[1, 0, 3, 2][..]);
    let word = Carrier::literal(&letters, Alphabet::MenuLetters, identity(), Some(inverse.clone())).unwrap();
    let resolved = word.resolve(&word.reference(1, 2)).unwrap();
    assert_eq!((resolved.alphabet, resolved.letters.as_slice()), (Alphabet::MenuLetters, &letters[1..]));
    let inverted = Node::inverse(Node::letters(&letters));
    let back = Carrier::new(inverted, Alphabet::MenuLetters, identity(), "f", Some(inverse)).unwrap();
    assert_eq!(
        back.resolve(&back.reference(0, 2)).unwrap().letters,
        [Letter::Menu { cap: 3, item: 1 }, Letter::Menu { cap: 1, item: 3 }]
    );
    let mixed = [Letter::Menu { cap: 1, item: 2 }, Letter::Generator(5)];
    assert!(Carrier::literal(&mixed, Alphabet::MenuLetters, identity(), None).is_err());
}
