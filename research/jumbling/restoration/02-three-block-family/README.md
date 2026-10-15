# 02. The three-block finishing family (12 October 2026)

The last three off pieces of stage 01 are B′ = {16245, 16249, 32088}. This stage turns the word that restored them
into a family:
- it gives the geometric mechanism of the iterated conjugation W = P_m F P_m⁻¹;
- it gives a certificate that reads only the current state;
- it shows that this one family covers about 9.3 × 10¹⁰ hole configurations.

The main document is [report/three-block-family.md](report/three-block-family.md).

Labels:
- **proved**: the integrator's proof, with no independent review;
- **computed**: exact, by the script named;
- **open**.

## Results

1. **Layer calculus. Theorem L, Lemma D and Theorem F are proved.**
   - **Template.** A template σ = (R; x_0, …, x_{m−1}; F) builds P_0 = R, P_{k+1} = P_k x_k P_k⁻¹ and W = P_m F P_m⁻¹.
   - **Conditions.** R0, S1 to S3 and F1 to F4 are finite exact tests on the word, the exact contents of B′ and chamber
     ghosts. None of them reads the occupancy.
   - **Exact updates.** Each level has an exact update of four objects:
     - the parked set;
     - the bubble T_k = Bad(P_k⁻¹);
     - the position L_k of B′;
     - the chamber map π_k.
   - **Lemma D** gives the true domain of the inverse segment P⁻¹. The off contents must be exactly P's parked
     contents, at the exact positions where P leaves them.
   - **Theorem F.** KV(W) = ∅, Bad(W) consists of exactly 3 chambers, and B′ lands on the lattice.
   - **Termination.** For the template σ* the bubble sizes are 3,097, 134, 80, 14 and 10. The bubbles are not
     nested, so termination comes from checking the explicit finite template level by level, not from a descent
     argument.
2. **The certificate V(X, σ)** (`scripts/layers3.py` checks it and `scripts/constr3.py` constructs the word).
   - **What it reads.** Only the current pose table, the fixed model and the template, before any new word runs.
     It reads no history.
   - **What it checks:**
     - B′ is one rigid block;
     - the template conditions and updates, compared level by level with direct ghosts;
     - the vacancy conditions N1 and N2, a Jordan certificate, a peeling certificate and a greedy emptying word V.
   - **Theorem V.** When the certificate holds, U = V·W·V⁻¹ is admissible at every prefix, puts the three pieces
     back and leaves zero collateral.
   - **Failure.** Every failure is reported exactly, naming the input, the template layer and letter, N1, N2 or the
     peeling.
3. **The general branch (Theorem G, proved; Corollary C).** Fix B′ at E2's exact pose h.
   - **Theorem G.** If the type-14 hole is not at chamber 84071, σ* with a V of at most 18 letters restores any
     occupancy, with |U| ≤ 99.
   - **Coverage.** 3,599 × C(7,200, 2) = 93,273,123,600 hole configurations, with the lattice pieces in any
     arrangement. Reachability of these states is not claimed.
   - **Corollary C.** K⁺ equivariance extends the family to all 7,200 poses k·h. Its premises are checked exactly,
     among them 72,000 of 72,000 conjugated template letters.
   - **Lemma T (type invariant, proved).** For every type t, the number of holes of type t equals the number of off
     pieces whose home chamber has type t.
4. **The frozen branch: the type-14 hole at 84071.**
   - **Ruled out, by computation.** No finishing letter works with σ* (Proposition O), and no SAFE-ALL preparation
     exists (Proposition N).
   - **Theorem Q (duality, proved).** The following five conditions are equivalent:
     - a frozen state is reachable;
     - a frozen state is restorable;
     - a word w exists with B′ landing on the lattice under h and Bad(w) = {84071, y1, y2};
     - every frozen state is restorable;
     - every frozen state is reachable.
   - **Open (O1).** Does such a word exist? A finite search found none, which is not a proof that none exists.

## New inputs, fixed before construction

| Input | Source | Result |
| --- | --- | --- |
| I1 | E2 plus 60 SAFE-ALL letters; the reachability replay passed | certificate issued; \|V\| = 13, \|U\| = 89; replay 89 of 89 prefixes, 3 restored, collateral 0 |
| I2 | E2 plus a 294-twist word; the reachability replay passed | certificate issued; \|V\| = 4, \|U\| = 71; replay 71 of 71 prefixes, 3 restored, collateral 0 |
| S3 | E2 with piece 47855 moved from 84071 to 89251; synthetic, reachability not proved | exact failure certificate: N2 is short by 1 in orbit 14 |

The ends of I1 and I2 have every piece on the lattice. Their lattice phase to the solved state is in
[stage 09](../09-general-method/README.md) (`earlier/x-e3-solved/`).

## Replay

Run from this directory. `$TH` is `research/jumbling/theory`. Decompress the `.gz` tables first.

    python -I scripts/constr3.py $TH helpers inputs/I1-table.json inputs/sigma-star.json runs/I1/construct
    python -I $TH/replay_check.py --table inputs/I1-table.json --words runs/I1/construct/parts/0-V.json \
        runs/I1/construct/parts/1-P.json runs/I1/construct/parts/2-F.json runs/I1/construct/parts/3-Pinv.json \
        runs/I1/construct/parts/4-Vinv.json --depth-every 1 --claims runs/I1/construct/claims.json \
        --export runs/I1/end-table.json --out runs/I1/replay.json
    python -I scripts/compare3.py $TH helpers inputs/I1-table.json runs/I1/end-table.json runs/I1/compare.json
    python -I $TH/replay_check.py --table inputs/E2-table.json --words inputs/I1-generation-word.json \
        --depth-every 1 --out runs/I1/generation-replay.json

For I2, replace I1 by I2. The equivariance and frozen-branch scripts give their usage in their headers.

## Files

| Path | What it is |
| --- | --- |
| `report/` | Theorems, proofs, limits and data tables |
| `scripts/` | The checker `layers3.py`, the constructor `constr3.py`, and the scripts for generation, comparison, the synthetic input, the frozen branch and the equivariance checks |
| `helpers/` | Four modules of stage 01, copied unchanged |
| `inputs/` | The E2 source table, σ*, I1 and I2 (pose tables, generation records and generation words), S3 |
| `cert/` | The template certificate on E2, type certificates, the frozen-branch analysis and the equivariance checks |
| `runs/` | Per input: the construction certificate, word parts, claims, replay report, comparison, end table and logs |

## Limits

- **Scope.** Only the family where B′ is rigid at one of the poses in K⁺h. Other poses of B′, other three-piece
  blocks, non-rigid triples and larger blocks are out of scope. A new pose first needs a template and a
  certificate, and the existence of a template is not proved.
- **The frozen branch.** Only the σ* prefix is ruled out; O1 is open.
- **General E.** No claim that general E is solved.
- **Not run.** Windows, DirectX, the native host regression and performance measurements. All runs are headless
  exact simulator runs on Linux.
