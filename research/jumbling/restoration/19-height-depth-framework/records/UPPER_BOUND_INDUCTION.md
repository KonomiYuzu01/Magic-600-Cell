# Constructive upper bounds, induction contracts, and tail closure

Archived mathematical record from 2 November 2026. See README.md for its current scope and the independent review record. Original-source digests below identify the captured research version; IMPORT-MANIFEST.json binds this sanitized edition.

Historical research note, pinned source a2ebb4cccd6b60d7527c001ef3b2cbc56b179249. The new finite-neighborhood proposition below is a derived candidate submitted for independent review. The bounded-height counting theorem and general computability equivalence are existing accepted source results, not new findings.

The domain for a general restoration statement is R, the legal-reachable component of the exactly solved configuration O under the full I_a menu. Reachability of every physically plausible configuration is not assumed. Words count literal menu letters on all 600 labelled grips; all 177,120 piece poses and collateral effects are retained.

## 1. A finite, state-only extension of the gap-68 construction

Let A be the prefix-complete recognized language of PREFIX_COMPLETE_OPTIMAL_RETURN.md. Its recognizer uses the 1,746 proper-prefix occurrences (1,348 distinct exact product/background entries), exact comparisons of all outside-K poses, and the guarded boundary decoder. For S in A, the common height H of the fixed seven labelled K pieces is at least 3,888. Its state-only decoder returns to exact sigma in (H-3888)/2 letters. The fixed 2,012-letter inverse witness from sigma to O therefore gives

    d(S,O) <= (H-3888)/2 + 2012 = H/2 + 68.

The 2,012 letters are the unchanged 1,929-letter pump suffix plus a verified 83-letter seed. This is a full exact-solved endpoint, not merely a lattice or sticker-label endpoint. The source's original family bound was H/2+141 [S1:7126,7144]. The same elementary neighborhood argument was described earlier in the private chat with gap 141; no immutable artifact pointer for that corollary was supplied. The increment here is the written gap-68 statement on the broader recognized prefix language, not a new neighborhood mechanism.

Fix one labelled p in K once and for all; do not change the label when pieces move or K splits. For an integer k>=0, define

    A[k] = { Z : a legal path of at most k letters joins Z to some S in A }.

**Proposition.** Membership in A[k] is decidable from the exact state table and k. Every accepted input has an explicitly constructed legal word to O of length at most

    h_p(Z)/2 + 68 + 2k.

This allows split-K inputs, changed outside backgrounds, and arbitrary use of all grips on the connecting path. It does not require Z itself to pass the pump guard or have a common K pose.

**Algorithm.**

1. Work in the exact pose universe G0^177120 described in section 2, or accept the promise that the input is reachable. Set h=h_p(Z).
2. If h+2k<3888, reject. Otherwise enumerate every legal word v of length t<=k from Z, using exact full-state transitions.
3. At each endpoint S, run the existing prefix-complete recognizer. A possible common target height must be an even integer in [max(3888,h-2k),h+2k]; with the actual path length t the interval tightens to [max(3888,h-2t),h+2t].
4. Compare all outside-K canonical matrices with the exact prefix template. Require all seven target poses to coincide, the exact prefix-product inverse, and an accepted guarded predecessor chain ending at exact sigma. Background-label pairs only index candidates; they do not replace the full comparison.
5. On acceptance, output v followed by that recognizer's return to sigma and the fixed 2,012-letter solved suffix. On exhausting the finite search, reject membership in A[k].

A safe branching bound is 600*60=36,000 menu letters, including identities. Thus at most sum(j=0..k) 36000^j word occurrences are tested. For every candidate the prefix catalogue is finite, and the boundary loop has at most max(0,floor((h+2k-3888)/132)) iterations. This is a finite exact algorithm, not a practical running-time claim or a bound on arithmetic bit complexity. Exact canonical state deduplication is optional.

**Proof.** Every legal letter changes the height of the fixed physical label p by at most two. If v has length t and joins Z to recognized S, then |H-h|<=2t. The emitted word has length

    t + H/2 + 68 <= h/2 + 68 + 2t <= h/2 + 68 + 2k.

Soundness uses legal replay of v and the already proved full-state soundness of the prefix decoder. Conversely, if Z belongs to A[k], reverse its connecting legal path from S. Menu inverses are legal and undo the exact moved set [S1:7162], so the enumeration from Z reaches S and accepts. No input history is read. Accepting also certifies Z is in R, because the accepted return can be reversed from O.

The neighborhood formula does not prove optimality or preservation of the input's outside-K background: the endpoint is O. A fixed k is not known to cover arbitrary reachable states. Since sigma belongs to A, the union over all k equals R, but increasing k without a bound merely gives another terminating search on the promised reachable domain. It supplies no new uniform computable bound in the input height.

**Verification boundary.** The catalogue and seed evidence hashes were checked; no new geometric neighborhood search was run. There are 104 synthetic arithmetic checks of the displayed inequality, not 104 realized puzzle states. The infinite proposition rests on the written proof and the existing decoder/admissibility premises [S2,S3,S4,S7].

## 2. What bounded height actually makes finite

The accepted source already gives the necessary finite-fiber theorem [S1:7155-7164]:

- A reachable pose is in G0: its left quaternion lies in the norm-one icosians with 2 inverted, and its right quaternion lies in the binary icosahedral group 2I.
- G0 preserves the reachable-pose universe. Right tree heights are zero and left heights are even.
- The left tree is 5-regular; the left group is transitive on its even vertices with stabilizer 2I. Each tree position has 7,200 exact pose representatives.

The number of even tree vertices through distance 2m is

    D_m = 1 + sum(j=1..m) 5*4^(2j-1)
        = (4*16^m-1)/3.

Hence at most

    P_m = 7200*D_m
    S_m = P_m^177120

exact full pose tables have all left heights at most 2m. This deliberately counts invalid/overlapping tables too, so it is an upper bound for the physical states. The source proves that the poses are finitely listable: products of at most m menu rotations, multiplied on the right by every K+ element, with exact duplicate removal. For example D_0=1, D_1=21, D_2=341; P_1=151200. The finite arithmetic audit reproduced the shell count for m=0..8.

The G0 restriction is essential. Merely asking for rational-Q(sqrt5) rotations with bounded left and right tree heights is insufficient: [S1:7163] explicitly gives infinitely many such rotations with both heights zero. A right-height-zero check alone is therefore not a certificate that the finite-fiber theorem applies.

**Conditional BFS bound.** Suppose a proved ceiling B=2m guarantees a legal path from Z to the specified goal with every prefix staying at all-piece height at most B. BFS inside that exact height-restricted graph returns a path of length at most S_m-1, because a shortest path repeats no full state. Exhaustion without a goal proves only failure inside that ceiling. It is not a global impossibility certificate unless the ceiling theorem applies.

In particular, a known computable ceiling E(h), valid for every reachable input of maximum initial height h, yields the explicit uniform upper bound S_floor(E(h)/2)-1 to exact O. One must require E(h)>=h and round ceilings consistently with even heights.

A bound on initial heights alone does not supply such a ceiling. There are finitely many reachable inputs of bounded height, and reversibility gives each some finite return; thus a finite maximum distance exists. But this maximum is not computed by the count S_m. The paths may leave the height slice, and one cannot stop an enumeration merely by assuming all reachable inputs in that slice have appeared. This distinction was already proved as T9-05 [S1:1539-1545].

Theorem D [S1:7166-7186] already relates a total computable clean-step bound, decidable membership in R, and a computable excursion ceiling. It provides neither the bound nor the decision procedure. It is a statement of the unresolved quantitative problem, not a new impossibility theorem.

## 3. The three obligations for a useful uniform induction

There is already a state-only terminating method M* on the promised reachable component: it fairly enumerates legal words until a free clean step is found and then invokes the certified lattice phase [S1:351-366]. The input history appears only in the existence proof. What remains to obtain a uniform constructive upper bound is quantitative coverage and composition, not rediscovery of reversibility.

| Obligation | Required statement | Present boundary |
|---|---|---|
| Bounded coverage | For every reachable state in the claimed nonterminal domain, choose and construct an exactly legal progress word with a proved computable cost bound. Its endpoint must make progress in the declared full-state rank. | Good sets, one clean lens, or a few branch-compatible items do not prove controller viability. GC is finite conditional on acceptance; Proposition V requires actual reachability of an accepting controller state [S5:279-301,355-365]. |
| Inductive contract | The chosen word must preserve the domain/resources needed at the next state, with explicit endpoint collateral, height/excursion bounds and well-founded progress. Costs of every setup, landing and debris return count. | A return letter can raise the target again [S5:329-347]. A decreasing Phi proves termination of committed macros, not that a positive-Phi stop has another macro [S5:371-382]. For free off-count steps, heights can grow and require a recurrence; R21-01 states the correct distinction [S1:7118]. |
| Tail and final-goal closure | Prove a bounded construction for every reachable low-height residue produced by the induction, and then for the requested solved endpoint. A plan must remain valid after earlier steps or be reconstructed with a bound. | Cover is not inductive; whole-cap-only rules have a concrete obstruction at reachable inputs. Complete static plans suffice on their plan class, but do not cover every tail [S6:112-124,304-323]. The lattice theorem solves sticker labels and permits hidden exact-piece rotations; it does not by itself give the unknown exact-pose lattice reachability table [S1:331-336,7164]. |

A precise conditional composition illustrates the required interface. Set

    r(Z) = sum_p max(h_p(Z)-2,0)/2.

Suppose every reachable state with r=j>0 has a constructed legal macro of length at most L(j), ending at a state with strictly smaller r. Suppose every reachable r=0 state with n>0 off-lattice pieces has a constructed step of length at most T(n), ending again at r=0 with strictly fewer off pieces. Finally suppose a constructed lattice restorer for the chosen goal has cost at most C.

Then

    sum(j=1..r(Z)) L(j) + sum(n=1..177120) T(n) + C

is a valid upper bound. A skipped rank only removes nonnegative terms. If a rule is proved only on a proper class J, membership of the initial state and preservation of J are additional parts of its contract. In the tail, off-count decrease alone is insufficient if other pieces become height 4 or higher: that would leave r=0 and invalidate this two-phase induction.

This formula is a conditional proof template, not an established numerical bound: no globally covering L or T is supplied here. For the exact-pose solved goal, C also needs an exact-pose base case. For the sticker-solved goal, the accepted reachable-lattice restorer is the relevant existing base theorem. A proved global excursion ceiling would provide a different route through the conditional BFS bound above.

## 4. Evidence map and attribution

Line references below are to the exact captured bytes. The source excerpts, full SHA256 values, input receipt hashes and arithmetic checks are preserved in evidence/upper-bound-induction-evidence.json (SHA256 8ea8dea9d6917b44f03bbd6cae607f7072c151540e5e3d68e62d9b4121d914b2). Reproduction: upper-bound-induction-command.ps1.txt. New computation: 0.03125 CPU seconds, zero full-state moves, zero new geometry tests.

| Ref | File and relevant lines | SHA256 |
|---|---|---|
| S1 | source/research/jumbling/restoration/09-general-method/SENIOR-REVIEW.md:109-115,331-366,1539-1545,7118-7144,7152-7187 | 25d00279f65a74eebfab77b6c69acf0d7687d4f0ff4a9bee6029807bee841812 |
| S2 | GUARDED_OPTIMAL_RETURN.md:17-23,64-90,101-120 | 7196bad98121a02d3edd9c7ab69783f1d1a7587a70d61a99cc568952b5f50147 |
| S3 | PREFIX_COMPLETE_OPTIMAL_RETURN.md:5-43 | 7c2aa21578fa869e1f2440580d17923786ee3bf99ba79c3acb61589028a573f3 |
| S4 | RESTORATION_DEPTH_PROFILE.md:5-18 | 61dd12f3e8c9517ce8d6b12a9f322c4555af16786fb9090d1a40885bab270165 |
| S5 | source/research/jumbling/restoration/15-lowering-machine/PROOFS.md:258-301,329-382 | 319b9136e63524f5dafb19d76701331a1a3f037fa95e353401481b6633f7b152 |
| S6 | source/research/jumbling/restoration/18-tail-completion/README.md:112-124,284-323 | f976dc53d3dd238ed54c9bea754a3004194acbb5b464402f7e49cd921c129c27 |
| S7 | source/research/jumbling/restoration/10-x1-unbounded/REPORT.md:96-142,162-187 | e2128c7be5abb4b9a190efc3ff7923d61826673988ce4ff0adeb9752cb0eca78 |

The mathematical progress claimed in this packet is the explicit gap-68 finite-neighborhood recognizer contract and proof, plus a source-grounded separation of the three remaining induction obligations. The general feature-dependent upper bound and universal tail closure remain open. No source or main-worktree file was changed.
