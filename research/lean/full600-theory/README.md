# Full 600-cell base theory in Lean

A Lean 4 and Mathlib project for the base theory in [`research/ARCHON-HANDOFF-THEORY.md`](../../ARCHON-HANDOFF-THEORY.md). It holds Milestone 1, the permutation core, with every proof complete. Milestones 2 to 6 are not started.

## What is proved

All 16 Milestone 1 theorems in `ArchonFull600Theory/Basic.lean` are proved. The file has no `sorry`, no `admit`, no `native_decide` and no new axioms.

| Handoff | Lean theorem | Content |
| --- | --- | --- |
| S1 | `scopeSameRestriction` | Two permutations of six labels agree on a three-label target orbit but differ as full actions. |
| S2 | `scopeVisibleDisjoint` | Two permutations of seven labels have disjoint visible supports but do not commute. |
| S3 | `disjointFullSupportsCommute` | Permutations with disjoint full supports commute. |
| L14 | `transportNonfixedMap` | The nonfixed map of `S⁻¹ W S` is the transport of the nonfixed map of `W` under `S`. |
| L9 | `supportedAlternatingActions` | For a permutation group `G` with nine pairwise-disjoint preserved orbits of sizes 3600, 2400 and seven times 7200, each with a pure-controller certificate, there is an injective homomorphism from the product of the nine alternating groups into `G` that acts as each factor on its orbit and fixes every other slot. |
| L10 | `balancedColorLowerBound` | With 600 colors and balanced solved copies (6, 4 and 12 per color), the balanced-color count `B` bounds the supported colorings and the reachable colorings from below. |
| L11 | `wordBallCount` | The ball of radius `r` in the word metric of a `q`-letter alphabet has at most `(q^(r+1) - 1)/(q - 1)` colorings. |
| L11 | `wordDistanceLowerBound` | For the 1800-letter alphabet of the 600 caps, some reachable coloring has word distance at least 46648. |
| L11 | `totalVariationLowerBound` | Any law supported on colorings reached by words of exactly 1000 letters has total variation distance greater than `1 - 10^(-148595)` from the uniform law. |
| N5 | `dihedralFiveCommutatorCard`, `alternatingFivePerfect` | `[D₅, D₅]` is the rotation subgroup with 5 elements; `[A₅, A₅] = A₅` with 60 elements. |
| N6 | `threeCycleIdentityChecks` | `t_Y⁻¹ t_X = (A Y X)` and `(A x y)(A z x) = (x y z)` in chronological order, on every domain of size 4 to 9. |
| N7 | `balancedColorCountGtPower` | `10^151851 < B`. |
| N8 | `radius46647Comparison`, `radius46648CrossCheck` | `1800^46648 - 1 < 1799 B` and `1799 B ≤ 1800^46649 - 1`. |
| N9 | `thousandLetterSupportComparison` | `1800^1000 · 10^148595 < B`. |

`#print axioms` on each of the 16 theorems lists only `propext`, `Classical.choice` and `Quot.sound`. N7 to N9 depend on `propext` alone.

### How the proofs work

- N7 to N9 are exact natural-number comparisons on numbers of about 150,000 digits. They are proved by `decide +kernel`, which the kernel checks with its arbitrary-precision arithmetic. `native_decide` is not used, so no compiler trust enters.
- L9 builds the homomorphism with `MonoidHom.noncommPiCoprod` from the nine permutations extended by the identity. Each orbit's three-cycles are produced from the certificate's controllers and buffers, and they generate the alternating group by `Equiv.Perm.closure_three_cycles_eq_alternating`.
- L10 counts each orbit's balanced recolorings by orbit-stabilizer for the alternating group (the stabilizer of a coloring with at least one repeated color contains an odd permutation), and multiplies the nine counts through L9.
- L11 compares the word ball at radius 46647 with `B` through N8, and bounds the support of the 1000-letter law by `1800^1000` through an injection into length-1000 vectors and N9.

## Changes to the round-19 statements

No theorem's proposition was changed. Each statement was checked against the blueprint chapter and the handoff; see the pull request that added the proofs.

- The starter definition `hello` and its blueprint section were removed.
- Eleven Mathlib modules were added to the imports the proofs need.
- `wordBallCount` gained an `omit` line that drops four instance arguments the statement never used, as the linter reported. The theorem is now slightly more general.
- Linter options (`set_option linter.… false in`) were added where the round-19 scaffold includes hypotheses that a proof does not use, for example `hPreserved` in L10 and `hRealizes` in the total-variation bound, and where `threeCycleIdentityChecks` holds for every `n`, so its bounds `4 ≤ n ≤ 9` go unused.
- In the blueprint chapter, each proved result's proof is marked `\leanok`.

## Build

The toolchain is pinned in `lean-toolchain` (Lean v4.36.0), and `lakefile.toml` requires Mathlib at `v4.36.0`.

```
lake exe cache get
lake build
```

With Mathlib's build cache in place, `lake build` rebuilds only this library; `Basic.lean` takes under a minute. The build passes with no errors and no linter warnings. Lake still notes, once for each import, that the imported Mathlib modules are designed for the module system; this file is not a module.

## How it was produced

- Statements: Archon 0.3.3, an automated Lean formalization loop run outside this repository, in iteration 19 of its `autoformalize` stage, with OpenAI GPT-6.2 Sol at effort `max` through the Codex CLI. The agent logs and Archon's state directory are not included.
- Proofs: a Claude Code session (Claude Opus 5.5), without Codex calls.
