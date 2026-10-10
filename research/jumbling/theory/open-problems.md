# Jumbling: open problems

One list of every open problem of the jumbling theory, gathered from the [theory draft](theory-draft.md), [restoration](restoration.md) section 5 and the owner's [deferred-problems decision](../../../docs/wiki/decisions/owner-decisions-2026-10-10-jumbling-deferred.md). The source documents keep the full statements and proofs; this page keeps status, latest evidence and the next step. Scripts and result files are indexed in [README.md](README.md).

Status: **in progress** (worked on now), **open** (not judged out of reach, lower priority), **deferred** (judged out of reach for now by owner decision of 10 October 2026), **closed**.

Last updated: 10 October 2026.

## Restoration (top priority)

| ID | Problem | Source | Status | Latest evidence | Next step |
| --- | --- | --- | --- | --- | --- |
| R1 | Do the phase U rules reach the lattice from deep configurations (40 records and more), and in how many steps? | restoration section 5, open 1 | **in progress** | No 40- or 60-record run has reached the lattice. I_a@40 stops at N = 1,192 (`results/seams-I_a-40-stop.json`). See "R1 in detail" below. | Joint attack on the I_a@40 stop: rules that open a cap holding off-grip seams failed in two- and three-twist words. |
| R2 | A proved bound on the length of the shortest N-lowering word. | restoration section 5, open 1 | deferred | Along true scramble paths the shortest lowering prefix reaches 36 (an upper bound) under S4₀ at 640 records (`results/paths-S4-640.json`). | Not needed for completeness (Theorem U). |
| R3 | A restoration rule that reads the configuration and never needs a trial twist. | restoration section 3 | deferred | A visible reading of the twist order is fallible (the invisible order, restoration section 3). | Reopens if a visible test is proved exact. |

### R1 in detail

What is settled:
- Theorem U (proved): every reachable configuration off the lattice has an N-lowering word. Restoration is always possible in principle; the question is a short, findable word.
- Up to 20 twists the rules reach the lattice on every tested input: 51 fixture and random runs and 80 lens words (restoration section 5).
- On 40 and 60 records the repair word passes every earlier stopping point, but the descent slows as N falls (restoration section 3, "Result on deep inputs").

The I_a@40 stop (N = 1,192, grip N = 1,057, 15 misaligned caps):
- No single twist lowers any N_c.
- Each rule of section 3 was run on its own from this configuration. Reopen, conjugate repair, lock and shift each found no lowering step (shift: 33 minutes, cloud timing of research code).
- 135 of the misaligned cells lie on rotated cut hyperplanes that are no grip's cut (off-grip seams). They form four groups, by domain, of 59, 37, 25 and 14 cells.
- The pieces of each group lie in the common part of about six caps, and about half of those caps have a closed cut. For example, the group of 59 lies in caps 30, 34 and 145 (misaligned) and 31, 73 and 89 (closed).
- By M2 a twist changes N only on its own cut. An off-grip seam therefore has to be carried onto a grip's cut before any twist can close it.
- Every repair rule of section 3 opens with a misaligned cap. None opens a closed cap that holds an off-grip seam.

- Of the caps that hold off-grip seams, several are blocked (no admissible twist), among them 31, 73, 96, 152, 153, 295 and 302. Others are free and aligned, among them 64, 89, 183, 283, 290, 301 and 319.

Experiment (scratch script, not kept; exact N): the words (c, x)(d, y) and (c, x)(d, y)(c, z), where c is any of the 20 caps that hold off-grip seams (aligned caps allowed), x any admissible twist of c, d a misaligned cap meeting c, y the twist of d that lowers N_d most and z the twist of c that lowers N_c most. Result: no word lowers N. After almost every opening (c, x), no neighbouring cap has a twist that lowers its own N_d (one candidate in all 20 caps, and it did not lower N).

So two- and three-twist words around these caps are not enough. Next step: a joint attack on this stop (two independent analyses of the same packet, then an experiment decides).

## Lattice phase

| ID | Problem | Source | Status | Latest evidence | Next step |
| --- | --- | --- | --- | --- | --- |
| L1 | Statement (a): the retained group G contains every labelled-sticker configuration with the Lemma 8 invariants. | restoration section 4 | open | The retained theory proves Lemma 8 as an upper bound, and alternating actions on 9 of 35 orbits (Lemma 9). | A separate task for the retained puzzle. |
| L2 | Statement (b): every lattice configuration reachable through jumbling has the Lemma 8 invariants. | restoration section 4 | open | Proved for conjugated third-turns (Proposition C). Every end configuration of phase U so far has v in v(G). | Check the end configurations of deep runs once they reach the lattice. |
| L3 | Do the lattice states reached through jumbling (theory draft item 4, including the lens configurations) lie in G·solved? | theory draft item 4 | open | No known invariant separates the lens words from G (`results/lens-invariants.json`). | Needs a retained witness word; follows from L1 and L2. |

## Structure of the reachable set

| ID | Problem | Source | Status | Latest evidence | Next step |
| --- | --- | --- | --- | --- | --- |
| X1 | Finiteness of R(S4₀), R(I_a), R(I_b), and a multi-cap criterion. | theory draft item 3 | deferred | Tree heights stay small: largest (2, 4) and (3, 1) under S4₀ after 640 twists. | Reopens with a mechanism that bounds the heights. |
| X2 | A repeatable admissible word that raises tree heights, which would prove a menu infinite. | deferred-problems record, "still pursued" | open | None found yet. | When spare compute allows. |
| X3 | The number of reachable configurations and the worst-case distance. | theory draft item 7 | deferred | Not known even for the retained puzzle. | Reopens with a method for the retained puzzle. |
| X4 | A bound on the return distance in terms of a visible defect. | theory draft item 5 | deferred | The visible procedure asked for there is now restoration phase U (R1). | Same as R2. |
| X5 | Block preservation through jumble twists, and per-phase certificates. | theory draft item 6 | open | Not started. | After R1. |

## Closed

| Problem | Closed by |
| --- | --- |
| A procedure that reads only the visible configuration (theory draft item 5) | Restoration phase U; complete in practice up to 20 twists (R1 covers deeper inputs). |
| The S4@60 stop at N = 7,472 (rules of `restore.py`) and at 6,294 (face match and cover order added) | Conjugate repair passes both (restoration section 3). |
| Why deep descents leave the reverse path | The invisible order (lead, restoration section 3): two twists whose order no visible test reads. |
