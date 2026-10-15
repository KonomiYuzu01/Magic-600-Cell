# 08. Units and clean steps: stop62 from 689 to 86 (15 October 2026, v3.1)

**The question.** Stage 07 showed that no single letter lowers the off count below 1,549 off pieces. Can words of
several letters, built from the current state, still lower it step by step, and where does that stop? This stage
also settles how multi-frame and frozen blocks are handled.

The main document is [PROOFS-U.md](PROOFS-U.md) (version 3.1). General restoration is not claimed here.

Labels: **proved** (the author's proof, no independent review), **certificate** (exact computation with a record
that `scripts/` replays or recomputes), **measured**, **conditional**, **open**.

## The two ideas

- **Unit.** A block whose pieces lie in one frame domain is one unit. A block whose pieces lie in m ≥ 2 domains
  gives m units, one per frame, each with the rest of its block as its *free set*.
- **Clean step.** A word applied at the state such that every piece of the unit ends on the lattice, every lattice
  piece stays on the lattice (possibly in another chamber), and every off piece outside the unit and the free set
  keeps its exact pose.
- **Lemma U (proved).** A clean step lowers the off count by at least the size of its unit, whatever the free set.
  So a run of clean steps always terminates, and the only question is whether a step exists.

The driver `scripts/gen/cbrun3.py` reads the state, lists the units, scores candidate words (D-type and C-type words
of stage 06, single letters, commutator refinements [Q, k]) and closes the first usable one as V·W·V⁻¹, with V from
the peeling certificate.

## Results

| Item | Status |
| --- | --- |
| stop62 from 689 to 245 off pieces in 8 clean steps by the driver, then 245 to 233 by a refinement [Q, k] | **certificate**: independent replay, collateral 0 at every step |
| Carrier-move steps 233 to 86 (twice) and 233 to 61 (Lemma CM) | **proved**; three steps **certificate** |
| Lemma D: Bad(W) in each piece type can be vacated by K⁺ letters only if it has no more chambers than there are holes of that type | **proved** |
| Theorem S: at 245 every word of the whole D-type and C-type family of the four remaining units has \|Bad\| ≥ 333 > 245 | **certificate**, with an independent recount of all 37,380 words |
| Theorem Z: at 233 every root and depth-one refinement of the three remaining carriers has a Bad chamber of a type that has no hole | **certificate** |
| Proposition GH: the holes of the rare types 7 and 9 are glued to the carriers and move with them | **proved**; certificate at 233 |
| Theorem D1: one jumble twist away from the lattice, each of the 12 coset letters at that grip is a clean step back | **proved**; chain `d1` |
| Multi-frame blocks need no separate construction (Lemma U) | **proved**; instance replayed (chain `mf`) |
| Frozen blocks become ordinary units once the pieces that straddle their grips are restored (Proposition F) | **proved**; instance replayed (chains `frozen`, `frozen2`) |
| The family form of the Block lemma (some family word always works) | **false** for roots and depth-one refinements at stop62@233; open for deeper ones |

### The stop62 chain

| Steps | Off pieces | How |
| --- | --- | --- |
| 1 to 8 (chain `s62`) | 689 → 245 | driver steps; words of 10 or 22 letters, closures of 580 to 1,432 twists |
| 9 (chain `s62z`) | 245 → 233 | a depth-one refinement [Q, k] of the 12-piece block, found by the computation behind Theorem Z (328 twists) |
| carrier move (chains `s62m`, `s62m2`, `s62m3`) | 233 → 86 or 61 | 3 to 5 K⁺ letters move a second carrier so that its glued holes become the rare Bad chambers of the first carrier's word; one step lands both (1,749 to 1,955 twists) |

At 86 one carrier is left. When this stage was written no word for it was known. Stage 09 restores it with a
*pure twist*, a commutator word that turns the carrier's cell alone (1,374 twists, 86 → 0). Steps 1 to 3 of stage 06,
the nine steps above, the carrier move `s62m` and that pure twist form the 14-step stop62 chain of
[stage 09](../09-general-method/README.md).

### What it shows

- **The word is not the bottleneck; the closure is.** In every state examined, every unit with a mover had words
  that put it on the lattice. The failed tries failed at the closure: Bad(W) must be emptied by K⁺ letters, which
  Lemma D bounds type by type.
- **Errata to stages 06 and 07** (section 7 of PROOFS-U.md):
  - E1: Proposition N of stage 07 is about single letters. Multi-letter clean steps can lower the off count below
    1,549, as every step here does.
  - E2: the CX family is not covered by Proposition N(a); it is measured only.
  - E3: the conditional claim of stage 06 at stop62@689 cannot be met with its word pools in any order. The
    progress past 689 came from the refinements [Q, k], and past 233 from a carrier move.

## Replay

Run from `scripts/` with `python3 -I` and `ulimit -v 7000000`. `$J` is `research/jumbling` (its code on main is
unchanged since commit `27a001c`, which the runs used) and `$OUT` an output directory. `scripts/echain/` holds the five
stage 01 modules. Decompress the `.gz` files first.

    # independent replays (3 s to 4 min per chain)
    for c in s62 s62z s62m s62m2 s62m3 mf frozen frozen2 d1; do
      python3 -I gen/verify_u.py $J echain ../chains/$c/chain.json $OUT/replay-$c.jsonl
    done
    # the driver
    python3 -I gen/cbrun3.py $J echain <start table> <name> <work dir> [--par 2] [--only <focus json>] [--seed <dir>]

Section 8 of PROOFS-U.md lists 22 commands: the driver runs, block classes, Theorem S, Theorem Z, Proposition GH,
the carrier-move batch and the depth-two runs.

## Files

| Path | What it is |
| --- | --- |
| `PROOFS-U.md` | Units, clean steps, Lemma D, refinements, the driver, the Block lemma, runs and certificates, errata, reproduction, files |
| `scripts/gen/` | The driver `cbrun3.py`, units `mkblocks2.py`, scores `score4.py`, searches `dfam2.py` and `comm3.py`, the closure `tclose2.py`, chain collection and the independent replay `verify_u.py` |
| `scripts/ep/` | Certificates and diagnostics: type swaps, Bad recounts, glued holes, Theorem Z, carrier moves, depth-two runs |
| `scripts/mf/`, `scripts/fz/` | Block classes; the frozen start state |
| `scripts/echain/` | Unchanged copies of five stage 01 modules |
| `chains/<name>/` | `chain.json`, the start table, per step the word with digests, the unit and the free set, and `replay.jsonl` |
| `runs/<name>/` | Driver logs |
| `certs/` | Theorem S (`stop62/family-245/`), Theorem Z (`stop62/z233/`), the carrier moves (`stop62/cm233/`), the `ep/` outputs, frozen and multi-frame block classes |

Not stored: the Chinese delivery notes and the checksum list. Data files of 256 KiB or more are stored gzipped;
`IMPORT-MANIFEST.json` lists them with the SHA-256 of the original bytes.

## Limits

- Theorem Z's independent recount covers a sample only (483 depth-one refinements at 245).
- The depth-two runs are partial. Section 5.3's account of why the last carrier fails is measured on the words
  examined, not proved for every depth.
- The carrier-move steps are outside the driver.
- The new proofs have no independent review.
- Not run: Windows, DirectX, input, long sessions and performance measurements.
