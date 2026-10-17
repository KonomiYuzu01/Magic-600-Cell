# 09. A state-only general method, 15 October 2026

The method reads only the current pose table and restores a jumbled configuration in two phases:
1. clean steps take the pieces that are off the lattice back onto it;
2. a lattice phase solves the remaining lattice configuration.

[METHOD.md](METHOD.md) is the main document:
- the standard of "restored" (owner decision, 15 October 2026);
- the method;
- the status of every part it rests on;
- what the new standard removes from the earlier programme;
- the gaps;
- the results;
- reproduction commands;
- the checks that were not run.

[SENIOR-REVIEW.md](SENIOR-REVIEW.md) records the Codex senior review of 15 October 2026: gap M1 (existence, the
s61c obstruction and certificate, bounds), Theorem C (the class-cell pure twist, at most 720 letters) and the other
cell kinds, the review of the clean-step and lattice-phase proofs with their corrections, and a milestone audit of
the method as a whole: Theorem M\* (with an exhaustive fallback the method is complete), the toolkit theorem with
its bounds, and a bound of 1,324,686,034 twists for the lattice phase. Round 5 narrows the coverage question of
gap M1; round 6 gives gap M2 lower bounds and a construction of at most 356,565,166 twists, and records the
integrator's searches for short pure three-cycles (33 of 35 orbits). Round 7 shows that a seed of the lattice
cascade needs no fibre alignment and replaces 31 of the 35 seeds by shorter relocated elements: the cascade then
restores the two synthetic inputs in 4.5 × 10^7 twists instead of 2.3 × 10^8, and needs at most 71,516,526 twists
for every input, plus at most 25,852 for the corrections. Round 8, a closing audit, corrected seven statements of
these pages and the summaries, and showed that the round 7 choice of seeds is optimal over the recorded candidates.
Round 9 asked whether the reachable set is finite (which decides a uniform bound for gap M1): still open, with a
uniform period for repeated words, an exact one-letter check at the deep state H6, and a correction to stage 04.
Round 10 performs each three-cycle of the lattice cascade with one application of its seed, conjugated by a short
word: a construction then needs at most 50,560,184 twists for every input (not implemented). Setting the frames
during the position loop instead fails as proposed. Round 11 sets the frames inside the position loop with two
returns, uses the fewest three-cycles and clears the remaining frames two at a time: a construction then needs at
most 20,866,844 twists for every input, about 1.33 times the fewest twists that any cascade
solving one orbit at a time with conjugates of these seeds spends on them; implemented on 16 October, it restores
the two synthetic inputs in 1.58 × 10^7 twists (METHOD.md section 6.4). Round 12, a closing audit, stated the
lattice-phase results as one proposition, qualified that comparison, corrected nine other statements of these pages
and the summaries, and added the code of the round 9 checks. Rounds 13 to 15 took up finiteness again: the bound 6
on left heights is equivalent to a statement about straddled caps (P6), an island raised by a grip that was not out
stays trapped until that grip returns under a finite condition, and round 15 found four admissible letters after
the fixture records 1 to 723 that reach height 8, so P6 is false. Round 16 found no height 10 near that word and
closed two routes to a proof or a pump. Round 17 reached height 14 with an exact word of 152 letters: before each new
raise, a letter at the grip that made the last raise turns the pieces so that a free cap of another branch holds
them whole. At height 14 the raiser traps them until it returns. Round 18 showed that this trap is decided by four
orientations and not by height, that at most 120 grips are out at once, and, with two lemmas of the integrator not
yet reviewed, that the relay stops at 14 because the pieces lie deep in the raiser's cap and each return of the
raiser acts as one retained letter at it. Round 19 proved both lemmas and showed that traps cannot carry the height:
heights without bound need raises at grips that are already out, or fresh raises where the trap condition fails,
again and again. A lemma of the integrator, not yet reviewed, explains what such raises need: a jumble maps exactly
the cuts of its own branch onto cuts, so the pieces it moves off the lattice block the cuts of the other branches
that run through them, and the raise to 12 used pieces that the previous raiser had returned to the lattice.
Round 20 proved that lemma and found that it explains the lining only where pieces lie: jumbled caps leave empty
space inside the 600-cell, and a clean cut can cross it next to the high pieces. Frames are decided by height, so a
clean cut is lined only by lattice pieces and pieces of height 2 in its branch, and the number of raises at grips
that are already out is not bounded by itself: one such raise and its inverse can be repeated without end. Whether
left heights are bounded, and so whether the reachable set is finite, stayed open after round 20, where the
senior-review series on this question stopped at the owner's instruction. On 17 October 2026 an unattended run
settled it: left heights are unbounded and the reachable set is infinite ([stage 10](../10-x1-unbounded/README.md)).

The driver implements M\* (`scripts/gm/mstar.py`, `scripts/gm/fallback.py`): when its heuristic steps find
nothing, an exhaustive search over the menu words takes the step. The runs of 15 October used the earlier driver
v4, which had no fallback.

## Contents

| Path | What it is |
| --- | --- |
| `scripts/gm/` | The driver `gdrive.py` with its outer loop `mstar.py` and exhaustive fallback `fallback.py` (Theorem M\*), its step scripts, the tests `test_mstar.py` and `test_fallback.py`, `gen_scramble.py` and `omega_check.py` (Bad(W) against the cell, Theorem C) |
| `scripts/echain/` | Modules the step scripts share (stage 01) |
| `scripts/lattice/` | The lattice restorer `restore_lattice_late.py` and the independent checker `verify_late.py` |
| `scripts/run/` | The run commands used on 15 October; paths are neutral tokens |
| `data/library-merged.json.gz` | The 36-macro correction library of the lattice phase (rank 34 over F2 and 2 over F5) |
| `earlier/stop62-860-to-0/` | stop62, 860 off pieces, to 0 in 14 clean steps, then to solved. Contents: the chain (start table and step words), the independent replay, the lattice restore and verify reports with the solution record, and `step014-pure-twist-omega.json` (Bad(W) = Ω_X at s86a) |
| `earlier/x-e3-solved/` | Lattice phase and end-to-end check for E3 (the end of the X chain of stage 01) and for I1 and I2 (stage 02) |
| `runs/stop62-v4/` | The state-only v4 run on stop62: the chain (start table and step words), the driver log with private paths replaced by `<working tree>`, and the independent replay (`verify_u.jsonl`, `all_ok`, final off 269) |
| `runs/e2run24-v4/` | The state-only v4 run on e2run24, all 9 steps to its stop at 194 off pieces: the chain (start table and step words), the driver log with private paths replaced by `<working tree>`, and the independent replay of every step (`verify_u.jsonl`, `all_ok`) |
| `scripts/short-cycles/` | The exact K⁺ action (`kplus.py`), the single-overlap search for short pure elements (`short_cycles.py`), the two-level search for the orbits it leaves (`short_cycles2.py`) and the independent checker of their words (`check_cycles.py`); `SENIOR-REVIEW.md` section 11 |
| `data/short-cycles.json.gz` | The shortest verified element per orbit and kind that `short_cycles.py` found, with its word |
| `data/short-cycles-2.json.gz` | The same for `short_cycles2.py` on the orbits O00, O02, O06, O15, O17, O28 and O29 |
| `scripts/realign/` | Round 7: `realign.py` relocates the shorter candidates onto the cascade's buffers, certifies them on all slots and computes the bound; `restore_realigned.py` runs the stage 03 cascade with them and writes records that the stage 03 checker replays; `SENIOR-REVIEW.md` section 12 |
| `data/realign.json.gz` | The certified replacement per orbit (the source of the candidate, the lengths of the candidate, the relocation and the final word, the final word, the flags and the collateral orbits) and the bound, as `realign.py` wrote it; it holds no asset digests, relocation words or buffer labels |
| `scripts/schedule11/` | The schedule of round 11 (`SENIOR-REVIEW.md` section 16): `sched11.py` builds the construction of T11-04 and T11-05 orbit by orbit, `restore_schedule11.py` runs it on the committed synthetic inputs and writes records that the stage 03 checker replays, `check_schedule11.py` is the acceptance check written before the implementation and strengthened after its review, `mutants_check11.py` shows that it rejects the false passes the review found, `test_sched11.py` tests the gadgets |
| `runs/schedule11/` | The round 11 run on the committed synthetic inputs: the acceptance check's output, the mutation test's output, the run summary, the stage 03 checker's report, and the commands with the SHA-256 of the files not stored |
| `runs/realigned-cascade/` | The round 7 run on the committed synthetic inputs: logs, the summaries of the run and of the control run with no seed replaced, the stage 03 checker's report, and the commands with the SHA-256 of the files not stored |
| `IMPORT-MANIFEST.json` | The files stored gzipped, with the SHA-256 of their uncompressed bytes |

## Paths in commands

- `<research/jumbling>` is `research/jumbling` of this repository.
- `<repo>` is the repository root. The lattice scripts read it and never write to it.
- `<echain>` is `scripts/echain`.
- `<library>` is `data/library-merged.json` after decompression.

Decompress every `.gz` file before use, for example with `gunzip -k`. The runs used Python 3.13 with NumPy 2.5 on
Linux, with `ulimit -v 7000000` per process.

## Evidence

All of it is exact computation on synthetic geometry in headless Linux runs. None of it is evidence for Windows,
DirectX, input, long sessions or performance. A Codex senior review on 15 October 2026 checked the new proofs,
corrected several statements, proved Lemma C as Theorem C and proved Theorem M\* and the conditions for replacing a
seed of the lattice cascade ([SENIOR-REVIEW.md](SENIOR-REVIEW.md)); no human has reviewed them.
