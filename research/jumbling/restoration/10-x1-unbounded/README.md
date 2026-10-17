# 10. X1: the reachable set of I_a is infinite (17 October 2026)

Open problem X1 ([open problems](../../theory/open-problems.md)) asked whether R(I_a), the set of configurations
reachable from the solved state under the twist menu I_a, is finite. Since R(I_a) is finite exactly when the left
heights of reachable poses are bounded ([card 5](../../theory/cards/card-5-depth-and-lattice-group.md), Lemma 5.4),
the question was whether those heights are bounded.

**Answer: no.** An explicit family of words F_0, F_1, F_2, … is admissible from the solved state, and F_t takes seven
pieces to left height at least 3888 + 132t. So the left heights are unbounded and R(I_a) is infinite.

The main document is [REPORT.md](REPORT.md): the theorem, the family, three lemmas with their proofs, the exact
certificate and how it was found. It was written by an unattended run (provenance below) and checked independently
by the integrator before it entered the repository.

Labels: **proved** (a written proof, whose finite premises are exact computations), **certificate** (an exact
computation with a replayable record).

## Results

| Result | Status |
| --- | --- |
| For every t ≥ 0 the word F_t is admissible from the solved state and takes the seven pieces K = (118527, 126380, 126382, 126384, 126385, 149644, 149650) to left height 3888 + 2(\|F_t\| − 2085) ≥ 3888 + 132t (REPORT.md section 1). So R(I_a) is infinite | **proved**: tree, isolation, step, perturbation and covering lemmas, and an exact certificate (`programs/CERT.py`) |
| Height 6030 by a word of 3,156 letters (`witnesses/H6030.json`); F_3 (2,895 letters, height 5508) and F_40 (13,177 letters, height 26072) | **certificate**: every letter replayed on the exact simulator |
| Along the family at most 20 grips are out at once: only grips 11 and 80 move after letter 156 | **proved** (REPORT.md section 2.3) |
| A uniform bound on the restoration length, or on the length of one clean step, does not exist | **proved**: such a bound exists if and only if R(I_a) is finite ([METHOD.md](../09-general-method/METHOD.md) section 5, T15-04) |
| Earlier records of the run: heights 20, 22, 34 and 38 by words of 150 to 160 letters, beyond the earlier record of 14 | **certificate** |

## How it works

- **The state X.** After the first 156 letters of `witnesses/H38.json` the seven pieces K lie at left height 30, and
  the caps of grips 11 and 80 both hold them whole. The two grips are 36 degrees apart and lie in different branches
  of the tree (3 and 2). Both cuts are clean, and no other piece lies whole in both caps.
- **Isolation.** The other pieces in cap 11 (2,505) and in cap 80 (2,702) are the debris. For 26 items at grip 11 and
  25 at grip 80 (the Good sets), turning a cap's debris by the item puts it wholly outside the other cap. While the
  product of the letters at each grip stays in its Good set, the debris only turns inside its own cap and never blocks
  the other cut. Then a letter is admissible exactly when K does not straddle its cut, and the debris takes at most
  26 × 25 states.
- **The pump.** Each jumble at 11 or 80 that keeps the products in Good and keeps K whole in the other cap raises K by
  2, into the branch of its grip (tree lemma). 1,929 such letters lead to the centre σ, at height 3888.
- **The ball certificate.** Seven words W_0 to W_6 (66 to 302 letters) each return both products to their values at σ
  and move K's pose by a left multiplication L(λ_i) close to the identity. For every unit quaternion u with
  |u − 1| ≤ 1/200, one of the words keeps every hypothesis of the step from the pose L(u)Q* (perturbation lemma), and
  113 boxes choose a word that brings λu back into the same ball (covering lemma). An induction on t gives F_t.
- **Not periodic.** Which word comes next depends on u_t. This agrees with the senior review: a repeated word cannot
  raise heights without bound (T9-02, T16-03).

## The integrator's check (17 October 2026)

The integrator checked the result in a cloud session, apart from the run, before importing it:
- **Reading.** Sections 1 and 2 of REPORT.md were checked line by line against the simulator's code
  (`sim/state.py`: `apply`, `_commit`, `classify`). A letter is admissible exactly when it is a menu item, no piece
  straddles its cut and no piece is uncertain, and it moves exactly the pieces certified inside its cap. No gap was
  found in the five lemmas or in the induction.
- **The run's programs, re-run.** Every program in `programs/` was re-run from the repository root: all PASS.
  Their outputs agree with the run's own apart from run times, and `programs/FAMILY.py` rewrites `witnesses/F3.json`
  and `witnesses/F40.json` byte for byte. The outputs stored in `programs/` are these re-runs.
- **Own checkers.** Three programs in `check/` were written from the statements, not from the run's code:
  - `replay_witness.py` replays a witness from the solved state with the simulator. It takes every letter as written:
    a letter that is not two integers, with grip 0 to 599 and item 0 to 59, is rejected before the replay. After
    every letter it checks that the seven pieces share one pose with right height 0, and it records their left
    height. It checks the final height and digest. It ran on the eight records from H20 to H6030.
  - `certificate_check.py` checks every premise of the proof that the certificate supplies:
    - the tree lemma's premises for all 120 items at grips 11 and 80: each left factor fixes w_x, jumbles move v0 to
      distance 2 and retained items fix it;
    - X: clean cuts, the debris sets, and the Good sets from its own vertex margins (the same lists as `CERT.py`);
    - the step hypotheses at each of the 1,929 pump letters;
    - for each of the seven words: (*) at every state, P_i = L(λ_i), λ_0 > 0 and |λ_i| = 1;
    - the cover of the ball, by a uniform grid of 512 cells, and the box inequality, with its own square-root bound.
  - `family_check.py` checks that all 120 items are orthogonal and fix their grip normals. It builds F_40 with its own
    Hamilton product and box lookup and gets the run's word choices and word. It replays F_40 with the simulator and
    checks three things:
    - after X, every letter moves exactly the debris of its cap and K;
    - every 100 letters and at all 41 member boundaries, each of the 177,120 pieces is at the pose that property Z(t)
      of the proof predicts, K included (its X-pose turned by the product of all letters after X);
    - at every boundary, K is at L(u_t)Q*, at the predicted height.
- **Codex review (17 October 2026).** Two shards reviewed the import. The mathematics shard found no gap. The code
  and pages shard found three defects, all fixed: `replay_witness.py` converted letters with `int()`, so a
  fractional letter was truncated and still verified; `family_check.py` left K out of its checks every 100 letters;
  and the root README still called X1 open. Probes confirmed both checker defects before the fix and their
  rejection after it. The outputs in `check/` are re-runs of the fixed checkers.
- **Not done.** No human has reviewed the proof.

## Replay

From the repository root, with Python 3 and NumPy, `S=research/jumbling/restoration/10-x1-unbounded` (cloud times):

    python3 -I -B $S/programs/CERT.py                       # the certificate, sections A1 to D (about 5 minutes)
    python3 -I -B $S/programs/FAMILY.py 40                  # builds and replays F_0 to F_40 (about 25 minutes)
    python3 -I -B $S/programs/PUMP.py $S/witnesses/H6030.json 156
    python3 -I -B $S/programs/replay.py $S/witnesses/H6030.json 3150
    python3 -I -B $S/check/certificate_check.py $S/witnesses/CERT.json $S/witnesses/H6030.json   # about 2 minutes
    python3 -I -B $S/check/family_check.py $S/witnesses/CERT.json $S/witnesses/H6030.json 40 $S/witnesses/F40.json
    python3 -I -B $S/check/replay_witness.py $S/witnesses/H1230.json

`FAMILY.py N` writes `witnesses/F<N>.json`. Every program reads `research/jumbling` (the simulator `sim/` and the
theory modules) without writing to it. The simulator and theory code on main are unchanged since commit `f281b08`,
which the run used.

## Files

| Path | What it is |
| --- | --- |
| `REPORT.md` | The run's report: theorem, proofs, computations, how the certificate was found, earlier records |
| `witnesses/` | `CERT.json` (η, the seven words, the 113 boxes), `H6030.json` and the other records, `F3.json` and `F40.json` |
| `programs/` | The run's exact programs and their outputs re-run here: `CERT.py`, `FAMILY.py`, `PUMP.py`, `replay.py` (the generic witness checker) and `H20.py` |
| `check/` | The integrator's checkers and their outputs |
| `IMPORT-MANIFEST.json` | The SHA-256 of every imported file as the run wrote it, and which files had their paths rewritten |

## Provenance

- **The run.** One unattended Claude Code run in headless mode, on the owner's machine and on the owner's API credit,
  under the [owner decision of 16 October 2026](../../../../docs/wiki/decisions/owner-decisions-2026-10-16-x1-api-budget.md).
  It read an export of commit `f281b08` and worked from 23:45 UTC on 16 October to 05:02 UTC on 17 October in three
  sessions:
  - The first session ended when the machine ran out of memory.
  - The second recovered and found heights 20 to 34.
  - The third began after the owner told the run to stop searching and work on the mechanism and the proof. It found
    the pump, the certificate and the proof.

  Two referee subagents with fresh context checked the mathematics and audited `CERT.py` and `FAMILY.py` before the run
  stopped (REPORT.md section 6).
- **Cost.** The run's own estimate was USD 71.99. On 17 October the Anthropic Console showed USD 73.74 spent in
  the month to date, a figure that includes the probe that tested the setup before the run. The budget was USD 200.
  The owner disabled the API key after the run.
- **Import.** Only the proof's files were imported, with paths rewritten (`IMPORT-MANIFEST.json`); the source
  archive's SHA-256 is in the manifest. The run's working directory (search programs, scratch states) and its log
  stay in that archive. Six lines of the log have estimated timestamps that the run itself marked as wrong.

## Limits

- The theorem is about the menu I_a. R(S4₀) and R(I_b) are no longer studied (owner decision of 13 October 2026).
- No human has reviewed the proof. The proof, the referees and the integrator's checkers rest on the repository's
  simulator as the definition of admissibility.
- Floating point was used only to find the words and boxes; every claim rests on exact arithmetic over Q(√5).
- Everything here is exact computation on synthetic geometry in headless Linux runs. Times are cloud timings, not
  performance evidence. Not run: Windows, DirectX, the native host regression, input, long sessions and
  performance measurements.
