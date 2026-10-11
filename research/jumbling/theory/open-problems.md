# Jumbling: open problems

One list of every open problem of the jumbling theory, gathered from the [theory draft](theory-draft.md), [restoration](restoration.md) section 5 and the owner's [deferred-problems decision](../../../docs/wiki/decisions/owner-decisions-2026-10-10-jumbling-deferred.md). The source documents keep the full statements and proofs; this page keeps status, latest evidence and the next step. Scripts and result files are indexed in [README.md](README.md).

Status: **in progress** (worked on now), **open** (not judged out of reach, lower priority), **deferred** (judged out of reach for now by owner decision of 10 October 2026), **closed**.

Last updated: 11 October 2026.

## Restoration (top priority)

| ID | Problem | Source | Status | Latest evidence | Next step |
| --- | --- | --- | --- | --- | --- |
| R1 | Do the phase U rules reach the lattice from deep configurations (40 records and more), and in how many steps? | restoration section 5, open 1 | **in progress** | No 40- or 60-record run has reached the lattice. I_a@40 stops at N = 1,192 (`results/seams-I_a-40-stop.json`). The joint attack explained the stop (fragments) and found the first lowering words that need no record (lens commutators); E2 is descending from it. See "R1 in detail" below. | E2 (no history, lens commutators, potential (off-lattice count, N)) to its end; if it stalls, nested commutators. A proof that a lowering step of this kind exists at every configuration off the lattice would close R1. |
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

This excludes only that family. Like the conjugate repair, it tries only the middle twist that lowers N_d most, so words whose middle twist leaves N_d equal or raises it were not tried.

Floor (joint attack; checked against the counts of every misaligned cap at this stop): N_c ≥ |In_c − Out_c|, where In_c and Out_c count the cut cells of c covered inside and outside, and twists of c alone keep both counts. At this stop eight caps are at their floor (17, 34, 50, 162, 187, 237, 315, 390), so no twist of those caps alone can lower N_c; another cap must first change their counts.

### Joint attack on the I_a@40 stop (10 and 11 October 2026)

Two sealed analyses of the same packet (the senior reviewer, and the solver), then a cross-examination of the solver's claims by the senior reviewer, then experiments. Call records and dispositions are private (`work/reviews/`); this section keeps the results. "Diagnosis" marks a result that reads the scramble journal; no rule reads it.

What the stop is (cross-examination verified the first three items):
- **Fragments.** 860 pieces are off the lattice, in 5 domains of 424, 63, 197, 164 and 12 pieces, all at tree height (2, 0); the other 176,260 are on the lattice. N = 1,192 = 535 cells on the surface of the lattice domain (holes) + 657 cells on the surface of the fragments. No cap holds a whole off-lattice domain that makes up most of its inside (a carrier); every fragment is enclosed by the lattice domain.
- **The wall.** At this stop every jumble twist raises its own N_c by at least 1,148 (cap 30), and by 1,322 to 1,566 for a cap whose inside is all lattice; a retained twist changes N_c by 0 to +222. So the first jumble twist of any word costs about N itself.
- **Isolation** (`isolate.py`, exact; `results/isolate-I_a-40-stop.txt`). Only the fragment of 164 pieces is exactly the common part of two caps (30 and 145). For the other four, the intersection of every cap that holds the fragment whole still contains 165 to 798 other pieces.
- **Why every rule stops.** Each rule accepts only words with a lowering sub-step (conj, lock, shift, reopen), and no single twist lowers any N_c. Whether a later sub-step could lower N after the wall was left open by the cross-examination.
- **How it arose** (diagnosis). Of the first 31 solver twists, 29 undo one scramble record each, one cancels two and one leaves a retained residue; at that point (S31, N = 8,036) the reverse of the 8 remaining records reaches the lattice, its first twist raising N by 948. The rules instead made repair words that split carriers into fragments. A lowering word from the stop exists (Theorem U): the reverse of the solver's 27 non-identity repair twists and then the 8 undos, 35 twists, with N up to 8,984 on the way and never below 1,192 before the end.

Experiments:
- **Floor and complete ABA.** Every word (c, x)(d, y)(c or d, z) on the cap pairs (315, 154) and (162, 237), with x and y any non-identity twist and z the best final twist: 3,481 admissible prefixes per pair, none lowers N; the floor rules out every prefix.
- **E1: return carriers first** (`e1_carrier.py`) (a single twist that puts a whole carrier back on the lattice, taken even if N rises; words of retained twists only are rejected). From S31: N 8,036 → 504 in 17 twists, then stops with 335 pieces off the lattice, in fragments again. From scratch on S4@40 (`e1_carrier.py`): still running, N from 46,394 to 5,840 and the off-lattice count from 80,455 to 3,968 after 3.7 hours.
- **Lens commutators.** The word (d, a)(c, q)(d, a⁻¹)(c, q⁻¹), where cap d holds a fragment F whole, a·γ_F is in K⁺ and F lies in the common part of c and d. At this stop (30, 44)(145, 39)(30, 15)(145, 33) lowers N from 1,192 to 1,140 and the off-lattice count from 860 to 789: the first lowering word found at this stop that does not come from the journal. Seven words that are exact on the lens lower the off-lattice count.
- **What the commutator does, piece by piece** (`comm_anatomy.py`, exact; `results/comm-anatomy-I_a-40-stop.txt`). Both twists of the word above are jumble twists. A piece caught by a twist and by its inverse comes back unchanged: 2,401 pieces (caught by the twists of 30 only) and 2,702 (of 145 only). The fragment of 164 pieces is caught by the first, third and fourth twists, so its net change is the last twist alone, (145, 33), which puts it on the lattice. 93 lattice pieces are caught by the first three, net change a⁻¹qa, and leave the lattice. So a commutator changes only the pieces whose membership in R_c or R_d changes between a twist and its inverse; it is clean when every such piece belongs to a fragment that the net change returns. Here the net is 164 − 93 = 71 pieces returned.
- **E2: no history** (`e2_frag.py`, running). A solver given only the stop configuration, with no record of its own, the potential P = (off-lattice count, N) in lexicographic order and the lens commutator as an extra rule; every accepted step must lower P, so the run cannot cycle. Partial result after 2.9 hours: P from (860, 1,192) to (690, 1,080) in 7 accepted steps (3 commutators, which lower the off-lattice count by 71, 94 and 5, and 4 conjugate repairs, which lower N). Every step so far was found without any record. The off-lattice count falls only through commutators and the last one returned 5 pieces, so whether the run reaches the lattice is open. Step log: `results/joint-attack-I_a-40.json`.

Part B of the solver's proposal (reopen the solver's own repair words until a carrier exists, then return it) was not adopted as the method. It depends on the solver's history, so it cannot restore a configuration handed over without one. Its stopping rule can return to an earlier state and loop. At this stop it stops after 13 reversals, not 27.

Owner decision (10 October 2026, [record](../../../docs/wiki/decisions/owner-decisions-2026-10-10-jumbling-complete.md)): a guarantee limited to shallow scrambles (for example 20 twists) is not acceptable; R1 closes only with a method that restores every reachable configuration.

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

Results of 11 October 2026 (`deep_scramble.py`, `results/deep-scramble-I_a.json`):
- **Lattice-prefix rule** (proved; checked exactly). Let G be a lattice configuration and L a twist sequence admissible from the solved state, reaching W. Then L is admissible from G, and every piece p ends in the pose W[s(p)] · G[p], where s(p) is the piece whose home place p occupies in G. Proof: K+ maps retained chambers onto retained chambers, so in G every posed piece region is the home region of a piece of the same orbit and G has the posed regions of the solved state; whether a twist is admissible, and which pieces it moves, depends only on the posed regions; induction along L. Check: 60 random retained twists, then 60 random admissible twists (half of them jumble twists), 0 exact mismatches over all pieces. Consequence: a retained scramble placed before a jumble scramble changes only which piece sits where. The off-lattice count, the domains and N are those of the jumble scramble alone.
- **Inscribed 600-cells.** The 600 grip poles split into five inscribed 600-cells of 120 poles each (neighbours 36 degrees apart). The caps of one of them hold 97.02% of the pieces (171,840 of 177,120). From the solved state, no twist of a pole blocks any of its 12 neighbours in the same set (I_a: every pole of one set, every non-identity element). One jumble twist per pole of a set, each pole taking the element that leaves the most pieces off the lattice, gives 90.8% of the pieces off the lattice after one round (120 twists) and 93.5% after three (360 twists), with no twist blocked. Random admissible scrambles stay at about two thirds (I_a, 63 to 69% from 700 to 1,400 twists), and a greedy choice over random grips reached 76% after 300 twists (scratch runs, not kept). From the solved state each jumble twist blocks 25 grips, all 16 nearest among them (retained twists block none), so random twists leave lattice islands that no free grip reaches.
- Use: the 93.5% sample is the deep test input for the observation views ([jumbling-observation](../../../docs/progress/1.0/jumbling-observation.md), section 9) and a restoration input far deeper than the W-J fixtures. By the lattice-prefix rule it can be placed after any retained scramble to mix the colours as well.

## Closed

| Problem | Closed by |
| --- | --- |
| A procedure that reads only the visible configuration (theory draft item 5) | Restoration phase U; complete in practice up to 20 twists (R1 covers deeper inputs). |
| The S4@60 stop at N = 7,472 (rules of `restore.py`) and at 6,294 (face match and cover order added) | Conjugate repair passes both (restoration section 3). |
| Why deep descents leave the reverse path | The invisible order (lead, restoration section 3): two twists whose order no visible test reads. |
