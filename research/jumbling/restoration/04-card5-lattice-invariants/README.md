# 04. Card 5 for the menu I_a: the lattice correction group and heights (14 October 2026, second edition)

When every piece is back on the lattice, the configuration lies in a retained-puzzle state space. A jumbled history
can still leave invariants that ordinary twists cannot change. [Card 5](../../theory/cards/) asks what those
invariants can be (question 3), and whether the jumbled poses stay bounded (question 1).

The main document is [PROOFS.md](PROOFS.md), which gives the full statements, proofs, certificate definitions and
recomputation commands. This edition supersedes the first edition and the interim package of the same day.

Labels:
- **proved**;
- **proved under H**, where H is the card's tree model T3 to T7;
- **certificate**: a finite exact computation with its script and output;
- **open**.

## Results

| Item | Status |
| --- | --- |
| 3.1 the lattice correction group A_J | **answered: A_J = C2^34 × C5^2 exactly.** Lower bound: Theorem LB with certificate CERT-LB-V2 (40 generator words). Upper bound: Theorem A and Theorem B5 with certificate POT5 |
| 3.2 sign 8 = sign 25? | **no**: the 9-twist word M8 has sign 8 odd and sign 25 even (Theorem 3.2) |
| 3.3 frames zero? | **no**: f8, f11, f22, f25, f32 and the C5 frames of orbits 17 and 28 are nonzero on A_J |
| 3.4 are conjugated half-turns enough? | **no**: their ν values generate exactly ⟨b4, b7, b8, …, b34⟩ = C2^29. The upper bound holds under H |
| 1.1 is R(I_a) finite? | **answered on 17 October 2026: no** ([stage 10](../10-x1-unbounded/README.md)). At this stage it was **open, with proved partial bounds.** Proposition HB, with certificate REFX: model orbits 0, 1, 2, 3, 6 never pass left height 2; orbits 4, 5, 7, 8 never pass height 4; orbit 9 never passes height 6. Height 6 is reachable (H6), and so is height 8 (senior review round 15, T15-01: seven pieces of model orbits 19 to 24). Orbits 10 to 34 have no proved bound |
| 1.2 Safe | answered under H in the handoff |
| 1.3 normal form | proved for heights 2 and 4; open from height 6 |

Five of the seven sub-questions are answered.

### Theorem B5: b5 = 0 on every reachable lattice configuration

The proof uses a potential π. π maps each possible region of an orbit-5 piece to an orbit-5 slot, so that two
conditions hold:
- π is the identity on lattice regions;
- every legal twist acts on the π-images of all orbit-5 pieces by an even permutation of slots.

The sign of that permutation is then invariant, and on the lattice it is b5. The proof has three parts:
- **Lemma R.** One piece's refined state changes only by rules R1 to R5. For orbit 5 the transition system closes
  exactly: 144 states, 49 classes, heights 0, 2 and 4 only.
- **Lemma T.** At each grip, the orbit-5 slots of the cap are matched K⁺-equivariantly to the 12 grips of the 36°
  family F5(d).
- **Lemma P.** 32,550 exact checks pass.

The same construction fails for orbit 4 in all 1,728 pairings, which agrees with the known odd b4 word.

## Structure facts

- **TREEFACTS, a certificate.**
  - Every grip normal moves the tree vertex v0 one step, in 5 branches of 120.
  - All 36,000 menu elements have right factors that are unit-icosian multiples.
  - A pose's K⁺ class is the coset of its left factor, and left height is a function of the class.
- **AL, Lemma L and Lemma FN, proved.**
  - A piece of height at least 4 has no three-dimensional face on a fixed plane.
  - Face neighbours have tree distance at most 2.
  - Pieces of height at least 4 move rigidly with their face neighbours, so high blocks rise and fall together.
- **Negative evidence, superseded.** The searches below found nothing above height 6:
  - ADJ-H6, FLT-H6, TWOSTEP, CLEAR78, PUMP, SINGLE, RWALK and INNERX;
  - COVER6X, the complete one-twist extensions along the fixture history.

  Height 8 is reachable all the same: four admissible letters after the fixture records 1 to 723 take seven pieces
  of model orbits 19 to 24 there (T15-01, `../09-general-method/SENIOR-REVIEW.md` section 20). TWOSTEP had raised
  those seven pieces to height 6 through grips 593 and 501 and stopped after two letters; a third letter, at 593,
  reaches height 8.

## Replay

`J` is `research/jumbling` and `<checkout>` is the repository root. Decompress the `.gz` files first.

    # lower bound: the 40 generator words (about 12 minutes)
    python -I scripts/verify_words.py $J <checkout> scripts words/generators.jsonl lb.json
    # Theorem B5, certificate POT5 (about 10 seconds)
    python -I scripts/potcert5.py $J pot5.json
    # Proposition HB, certificate REFX (about 10 seconds); tree facts TREEFACTS (about 2.5 minutes)
    python -I scripts/refexact.py $J 1,2,3,4,5,6,7,8,9,10 40 refx.json
    # every cap decision of REFX recomputed exactly, certificate REFX-EXACT (about 30 seconds)
    python -I scripts/refx_exact.py $J refx-exact.json
    python -I scripts/treefacts.py $J data/t-T723x433-12.json treefacts.json

Section 6 of PROOFS.md lists every certificate and its command. `data/arrays.npz` is not stored: `e3lib.py` of
stage 05 rebuilds it when absent.

## Files

| Path | What it is |
| --- | --- |
| `PROOFS.md` | Statements, proofs and certificate definitions |
| `words/` | The 40 generator words of CERT-LB-V2, single-hole words, frame-loop words and a 6-twist b4 word |
| `certs/` | Certificate outputs, named as in PROOFS.md section 6 (`b5/`, `q1/`, `f8-c517/`) |
| `scripts/` | Generation and verification scripts; usage is in each file header |
| `data/` | The H6 deep-state table, the grip classes, the item-2 correction library, harvest targets and a random sample |

## Limits

- The new proofs have no independent mathematical review: V′, Lemma U, Proposition U′, Theorem A, Lemmas R, T and
  P, Theorem B5, Proposition HB, AL, Lemma L and FN.
- 1.1 and 1.3 stay open. Bounds are proved for the inner 10 model orbits only.
- Loop searches, face graphs, point location, pumps, clearing and random walks are floating-point heuristics or
  diagnostics. They are evidence only; only exactly replayed letters are certificates.
- Not run: Windows, DirectX, the native host and performance checks.
