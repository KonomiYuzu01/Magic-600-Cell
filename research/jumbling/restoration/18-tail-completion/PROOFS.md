# Stage 18: proofs of the joint attack on card 9 (26 October 2026)

The results of the joint attack on [card 9](../../theory/cards/card-9-tail-completion.md), with proofs. The process,
the computed facts and the answers to the card's questions are in
[README section 3](README.md#3-joint-attack-26-october-2026). Each result below was proved in one of the two sealed
analyses or in the cross-examination, and its proof was checked line by line by the other analyst and by the
integrator. The source of each is given in brackets: Astra, Claude Opus (the second analyst), or both; "scope by" names
the analyst whose objection fixed the statement as given here. PB is stage 12's
[`PROOFS-B.md`](../12-low-height/PROOFS-B.md); Theorem A and Corollary A1 are those of
[`buffer-transport.md`](../../theory/buffer-transport.md), sections 2 and 3.

Notation (card 9 and PB). Z is a valid state of 𝒞₂ (every off piece at height 2), Ω its 177,120 chambers, H = H(Z) its
hole set (as many holes as off pieces) and off(Z) its off pieces with their poses. For a set E of off pieces at fixed
poses, G(E) is the group of the letters at grips whose caps no piece of E holds or straddles (the SAFE-ALL letters for
E); G(Z) = G(off(Z)), and Fix(G) is the union of the one-chamber orbits of G. A whole-cap instance (c, i) at Z has C,
the 3,097 chambers of cap c; U, the off pieces in c; g, the element of item i; D ⊆ C, the landing chambers of U; and
E = off(Z) ∖ U. A preparation X is a SAFE-ALL word at Z with H·X = J; the inside mask is empty when J ∩ C = ∅. With
W = (c, i), the middle is Z·X·W; with the inside mask empty its off pieces are E and the displaced lattice content of
all of C, so its SAFE-ALL group M = M(E, c, i) depends on (E, c, i) only. V′ is a SAFE-ALL word at the middle and σ its
chamber permutation. The four-segment form of a step is X·W·V′·W⁻¹·V′⁻¹ (card 9, T6), the three-segment form
X·W·V′·W⁻¹ (COV-12). The profile of a hole set on a group G ≥ G(Z) is A ↦ |H ∩ A| on the G-orbits A; its digest is the
first 16 hex digits of the SHA-256 of the JSON text of the sorted rows [least chamber, orbit size, holes] with
holes > 0 (so `4f53cda18c2baa0c` is the empty profile). A **global Jordan witness** for an orbit Δ of n chambers is a
word w of the group whose permutation of all of Ω, recomputed by `buffer.check_jordan`, has n ≥ 8, a prime p with
n/2 < p ≤ n − 3, exactly one cycle of length divisible by p, and that cycle inside Δ.

**The static test P(E, h; c, i)** (the second analyst's `prep_test`), for a set E of off pieces at fixed poses, a
profile h on the G(E)-orbits and an instance (c, i):
- (a) c is eligible for E: unstraddled, its off pieces nonempty and of one frame domain, i a landing item (COV-12);
- (b) fixed = 0: D ∩ Fix(M) = ∅;
- (c) src_short = 0: h(A) ≤ |A ∖ C| for every G(E)-orbit A;
- (d) the integer max flow from the source orbits A (supply h(A)) through capacities |A ∩ Δ ∖ C| to the M-orbits Δ
  (demand |D ∩ Δ|) equals |D|;
- (e) a global Jordan witness (`buffer.jordan` with seed 29, each witness checked again by `check_jordan` on all of Ω)
  for every partially filled source orbit (0 < h(A) < |A|) and every M-orbit that meets D.

## 1. Whole-cap steps and their profiles

**Lemma PI (profile identity) [Claude Opus; three-segment endpoints with chosen holes: both].** Let S be a whole-cap
step at Z for (c, i) with any SAFE preparation X (any J ∈ G(Z)·H, any inside mask), in either form, ending at Z′ with
holes H′. Then for every orbit A′ of G(Z′):

  |H′ ∩ A′| = |H ∩ A′| − |D ∩ A′|.

So the profile of Z′ is fixed by (H, D, E) and does not depend on X, on V′, on the form or on which holes V′ leaves
outside C. A chamber that is a one-chamber orbit of G(Z′) and occupied at Z is occupied at Z′. The same holds for a
joint step (Lemma J) with C and D the unions of the caps' sets.

*Proof.* (1) The SAFE-ALL alphabet depends only on the off pieces and their poses (lattice pieces never straddle). The
off pieces of Z′ are E at Z's poses, and E ⊆ off(Z) and E ⊆ off(Z·X·W); removing off pieces makes no SAFE cap unsafe.
So G(Z) and the middle group lie in G(Z′) (the argument of R22C3-01), and X and σ map every G(Z′)-orbit onto itself.
(2) X ∈ G(Z), so |J ∩ A′| = |H ∩ A′|. (3) W rotates the content of the cap: every lattice piece on C ∖ J leaves the
lattice (g ∉ K⁺), U lands on D, so the middle holes are H1 = (J ∖ C) ⊔ (C ∖ D) (COV-12). (4) V′ vacates C
(σ⁻¹(C) ⊆ H1), and W⁻¹ returns the displaced pieces to C ∖ J and moves no lattice piece (C is vacant, E is outside the
cap and does not straddle it, U was moved out of C by V′). So the three-segment form ends with
H′ = (σ(H1) ∖ C) ∪ (C ∩ J), and the four-segment form, which applies σ⁻¹ once more, with
H′ = (H1 ∖ σ⁻¹(C)) ∪ σ⁻¹(C ∩ J). (5) σ preserves A′, so in both forms |H′ ∩ A′| = |H1 ∩ A′| − |C ∩ A′| + |C ∩ J ∩ A′|
= |J ∩ A′| − |D ∩ A′| = |H ∩ A′| − |D ∩ A′|. (6) For a one-chamber orbit {q}, |H′ ∩ {q}| ≤ |H ∩ {q}|. ∎

**Lemma PK (prescribed endpoint holes) [Astra].** Let the inside mask be empty. The middle holes are
Hmid = (C ∖ D) ∪ J. For every K ⊆ Ω ∖ C with |K ∩ Δ| = |J ∩ Δ| − |D ∩ Δ| on every M-orbit Δ, a SAFE-ALL word V′ at the
middle with Hmid·V′ = C ∪ K makes X·W·V′·W⁻¹ strict′; the step lands exactly U and leaves exactly K as holes. Its
length is |X| + 2 + |V′|.

*Proof.* V′ fixes E and every displaced piece (SAFE-ALL letters keep the off pieces) and clears C. W⁻¹ is admissible
(the off geometry at the middle after V′ is that after W), returns the displaced pieces to C, fixes E and moves no
lattice piece in C. With the inside mask empty the displaced content is all of C, so no hole is left inside C, and the
holes are exactly K. ∎

**Corollary CH (a changed step needs new later words) [Astra; computed by the integrator].** Dropping V′⁻¹ from a step
changes its endpoint holes, from (H1 ∖ σ⁻¹(C)) ∪ σ⁻¹(C ∩ J) to K. Since the holes and the off poses decide every later
outcome (COV-06), a stored later word is valid only at the hole set it was built for. Counterexample: T6's step 1 in
the three-segment form ends at `170be703…`; T6's stored step 2 played there lands its unit and keeps the other 104 off
poses but takes one lattice piece off (129 → 105, strict′ false, end `ed53fbb5…`; README section 3.4). By Lemma PI the
static test is unaffected (T6's remaining stages pass P at `170be703…` with the virtual numbers), so a chain of
three-segment steps is built in one of three ways: rebuild each step at the state actually reached (Theorem SP(a); the
rule's way), prescribe K by Lemma PK from a layered flow (Theorem SP(b)), or connect with a SAFE word Y (Lemma TR). The
last step of a chain can always be shortened.

**Lemma OC (orbit-clean steps) [Claude Opus; scope by Astra].** A strict′ step Z → Z′ whose final chamber map φ on Z's
lattice pieces keeps every occupied chamber x in its G(Z′)-orbit satisfies |H′ ∩ A′| = |H ∩ A′| − |D ∩ A′| for every
G(Z′)-orbit A′, with D the final chambers of the landed pieces. *Proof.* R22C3-01's inventory identity
h′ = h + |L ∩ A′| − |φ(L) ∩ A′| − |D ∩ A′|, with |φ(L) ∩ A′| = |L ∩ A′|. ∎ Whole-cap steps are orbit-clean (Lemma PI). A
mixed catalogue of cores has the static theory of section 2 only if every admitted core is orbit-clean; a fixed inside
mask alone does not prove it, and examples do not establish it for a catalogue (scope by Astra).

## 2. The static plan theorem

**Theorem SP (static plans) [both: the per-stage test and its proof Claude Opus's, the layered flow Astra's; the
certificate scope settled in the cross-examination].** Let π = (c_1, i_1), …, (c_m, i_m) be a sequence of whole-cap
instances from Z, each landing the whole off content U_t of its cap: E_0 = off(Z), E_t = E_{t−1} ∖ U_t, E_m = ∅, each
c_t eligible for E_{t−1} with landing item i_t and landing chambers D_t. Define the virtual profiles h_0(A) = |H ∩ A| on
the G(E_0)-orbits and

  h_t(A′) = Σ_{A ⊆ A′} h_{t−1}(A) − |D_t ∩ A′|  on the G(E_t)-orbits.

Suppose P(E_{t−1}, h_{t−1}; c_t, i_t) passes for every t.

(a) *Realization.* There are SAFE-ALL words X_t and V′_t such that the three-segment steps X_t·W_t·V′_t·W_t⁻¹, each
built at the state actually reached, form a strict′ chain Z = Z_0 → … → Z_m that lands every off piece; Z_t has the
off set E_t at Z's poses and the profile h_t, for every admissible choice of J_t, K_t, X_t and V′_t and in either form.

(b) *Equivalence.* The flow parts of P (fixed = 0, src_short = 0, flow = |D_t|) hold at every stage exactly when
Astra's layered integral flow exists: at each layer, arcs from the source orbits A to the middle orbits Δ of capacity
|A ∩ Δ ∖ C|, arcs from Δ to the next layer's source orbits A′ of capacity |Δ ∩ A′ ∖ C|, a consumption arc from Δ to
the sink of capacity |D ∩ Δ|, supply |H ∩ A| at the first layer and no final outlet, of value |H|.

(c) *Certificates.* Witnesses on the partially filled source orbits and the middle demand orbits suffice. A middle
orbit that meets C but not D needs no action: its chambers in C are holes at the middle, so it is left as it is.
Astra's predicate as first written also asked for every partially filled middle orbit; that is needed only when a
construction moves a nondemand orbit (atlas seeds that permute later orbits, or a prescribed K that differs from J on
such an orbit). Conversely it allowed an exact action certificate for a small or exceptional orbit, which P's Jordan
screen does not accept.

*Proof of (a)*, by induction on t. Z_{t−1} has off set E_{t−1} at Z's poses (strict′ keeps every other off pose) and,
by Lemma PI, profile h_{t−1} on G(E_{t−1}) = G(Z_{t−1}). P depends only on (E, h), the fixed model and the witness
search, and `buffer.jordan` is deterministic (`random.Random(seed + stage)`, no clock), so P passes at the actual
Z_{t−1}. The flow gives J with J ∩ C = ∅ and |J ∩ A| = h(A). A checked witness makes w^M (M the lcm of the other cycle
lengths) a single p-cycle inside the orbit that fixes every other chamber; its conjugates generate Alt of the orbit
acting on that orbit alone (Theorem A), so the product of these alternating groups over the certified orbits lies in
the group, and a SAFE X with H·X = J exists (Alt is transitive on the subsets of each size); Lemma L builds one. At the
middle, |Hmid ∩ Δ| − |C ∩ Δ| = |J ∩ Δ| − |D ∩ Δ| ≥ 0 on every demand orbit by the flow, and D ∩ Fix(M) = ∅. Sealed
three-cycles on the demand orbits (seeds that fix every chamber outside their orbit, R22B-08) move the landed pieces
out of C into holes of J and fix every other orbit pointwise, where the chambers in C are already holes. So V′ vacates
C, and COV-12 gives a strict′ step. For the profile: σ lies in M ≤ G(E_t), since every grip SAFE at the middle is SAFE
at E_t, and X ∈ G(E_{t−1}) ≤ G(E_t); for every G(E_t)-orbit A′, |K ∩ A′| = |Hmid ∩ A′| − |C ∩ A′| =
|J ∩ A′| − |D ∩ A′| = h_t(A′), whatever K is (the choice of K only places holes inside the middle orbits). The
four-segment form is Lemma PI. ∎

*Proof of (b).* M_t ≤ G(E_t) and G(E_{t−1}) ≤ G(E_t), so every middle orbit Δ lies in exactly one G(E_t)-orbit A′, and
an arc Δ → A′ has capacity |Δ ∖ C|, which an inflow minus a consumption never exceeds. By conservation the inflow to
a next-layer source A′ is Σ_{A⊆A′} h_{t−1}(A) − |D_t ∩ A′| = h_t(A′) whatever the flow chooses, so the layer supplies
are the virtual profiles. From P to the layered flow: take P's flow at layer t; the excess h(A) − f(A) fits into the
residual capacity |A ∖ C| − f(A) because src_short = 0, and is routed through the middle orbits to the next layer,
where it arrives as h_t; at the end h_m = 0, so no outlet is needed. Conversely, the arcs out of A carry h(A) within
|A ∖ C|, so src_short = 0, and the consumed part of each layer is a flow of value |D_t|. A one-chamber middle orbit
{x} with x ∈ D has inflow capacity 0, so neither flow consumes there (fixed = 0). ∎

*Proof of (c).* Shown in (a): the demand-orbit actions are supported on their own orbits. Separate alternating
projections would not be enough, since they can be diagonal (Astra's example: A5 acting diagonally on two five-point
sets cannot prescribe their hole subsets independently); the global witnesses exclude this, because each gives an
alternating group acting on its orbit alone (both). ∎

**Lemma JC (joint resource contract) [Claude Opus].** Let I_1, …, I_m be instances at Z with pairwise chamber-disjoint
caps, each passing P at Z, with middle groups M_k at Z. If one integer flow from the source orbits A (supply |H ∩ A|)
through capacities |A ∩ Δ ∖ C_k| to the disjoint union of the demand columns (k, Δ) (demands |D_k ∩ Δ|) saturates
Σ_k |D_k|, then in every order, at every stage, the next instance satisfies P(b) to P(d) at its virtual state; P(e)
must be computed there (or obtained by Lemma HL).

*Proof.* Let P be the landed instances and h_P(A′) = |H ∩ A′| − Σ_{j∈P} |D_j ∩ A′| on the orbits of G_P = G(E_P). Each
landed M_j and G(Z) lie in G_P. A flow edge (A, j, Δ) with positive flow has A ∩ Δ ≠ ∅, so A and Δ lie in one G_P-orbit
A′, and the landed instances' share of A′ is exactly their landings there. Aggregating the flow of an unlanded k over
the G_P-orbits and the orbits of its later middle group (which contains M_k) respects the capacities, meets the demands
and uses at most h_P(A′) from A′. P(b) holds because the later middle group contains M_k, and P(c) because
h_P(A′) ≤ |A′ ∖ C_k|. ∎

**Lemma HL (half-size lift) [Claude Opus].** Let G′ be transitive on an orbit Δ′ with |Δ′| ≥ 5 and contain Alt(Δ)
acting on Δ alone, for some Δ ⊆ Δ′ with |Δ| ≥ 3 and |Δ| > |Δ′|/2. Then G′ contains Alt(Δ′) acting on Δ′ alone.
*Proof.* A nontrivial block B meeting Δ cannot contain Δ (it would exceed half of Δ′); if it meets Δ′ ∖ Δ at z, an
element of Alt(Δ) carrying a point of B ∩ Δ to a point of Δ ∖ B fixes z and so maps B to itself, a contradiction;
otherwise B is a nontrivial block of Alt(Δ), which is primitive. So G′ is primitive on Δ′ and contains a three-cycle;
by Jordan's theorem it contains Alt(Δ′), and the normal closure of a three-cycle acting alone acts on Δ′ alone. ∎ It
complements R22C3-01, which needs p > |Δ′|/2 for an old witness to serve a merged orbit.

## 3. The rule

**Rule WC-PLAN [the enumeration Astra's, the per-stage test, the gate and the persistence proof Claude Opus's, the
admission Astra's].** Stated as chapter 5 states its rules:
- *Reads:* only the current pose table (the off pieces with their poses, and the hole set) and the fixed model
  (`assets/manifest.json`: chambers, caps, menu items); units only name rows of the record.
- *Gate:* the residue fixpoint R*: start with all units and remove every unit that some cap holding it could still land
  (the cap is eligible for its content in the current residue R, and some item has a group G*_R(c, i) that fixes no
  landing chamber; G*_R as in Theorem N); repeat until nothing changes. If R* ≠ ∅, no complete whole-cap plan exists
  (Theorem N's proof) and the rule gives no step.
- *Candidates and order:* plans, sequences of (cap, item) whose stages are eligible at their virtual states and cover
  the off set, by number of stages first (iterative deepening) and then lexicographically in (cap, item) stage by
  stage; a prefix is extended only if the remaining off set can still be covered by as many eligible caps as stages
  remain (coverability depends on the off set only). At most 400 tests of P, memoized per virtual state and instance.
- *Test:* P at every stage's virtual state; the plan is the first in the order whose every stage passes.
- *Word and tree:* only the plan's first stage is played, built at the current state: J from the flow (current holes
  kept where the flow allows, ties to the least chamber, J ∩ C = ∅), X a SAFE-ALL word with H·X = J, W = (c, i), V′ a
  SAFE-ALL word at the middle that vacates C, W⁻¹; the tree is `wholecap [c, i]` with the spans X, W, V′ and W⁻¹.
- *Admission:* the exact length |X| + 2 + |V′| before expansion is at most 10⁷; before the first step is released,
  either uniform bounds for the complete static suffix or a complete chain built and checked on cloned states; a
  released step must pass the three gates of README section 3.3 (exact length and expanded word, composed gate,
  twist-by-twist replay), land exactly its cap's content with no collateral and every other off pose kept, and end
  with the predicted profile. A mismatch is an error, not permission to reuse old words. Replanning is allowed only to
  another complete admitted suffix from the state reached.
- *Contract:* strict′, and the end profile digest equal to h_1 (Lemma PI).

**Theorem R (persistence) [Claude Opus; with Astra's admission].** If WC-PLAN finds the plan π = (s_1, …, s_L) at Z
and plays s_1, then at the state Z′ reached it finds (s_2, …, s_L), with a subset of the tests made at Z. Hence from a
state where it finds a plan whose suffix is admitted, it lands every off piece in L ≤ (number of units) strict′ steps
and never sticks.

*Proof.* After s_1 the virtual state equals the actual Z′ in (E, h) (Lemma PI); eligibility, coverability and P
depend only on (E, h). A plan of fewer than L − 1 stages at Z′ would, after s_1, be a plan of fewer than L stages at Z,
against the depth at which π was found. The plans of L − 1 stages at Z′ that precede (s_2, …, s_L) were tested at Z as
continuations of s_1 and failed, under the same memoized keys. So the budget used at Z suffices along the run. ∎

Without the admission, a structurally passing first stage whose later builder exceeds 10⁷ would not be an executable
step, although its virtual suffix still passes P (Astra). So Theorem R is a conditional no-stick theorem for the plan
class; the bounded execution of a particular plan is an experiment (README section 3.2 decides it at the stop).

## 4. The entry

**Theorem N (whole caps alone stick at D1's entry) [Claude Opus; reproduced by Astra; scope by Astra].** Let R be the
residue {60011 (b07, 111 pieces), 106716 (b12, 41), 106783 (b13, 101), 107733 (b14, 78), 112734 (b15, 25)}, 356 pieces
at their poses at D1's entry `7e128820…`, which are also their poses at E2's 737 state `2e3b8f99…` and at D1's table
00097. From any of these three states, no finite sequence of whole-cap steps that land the whole off content of their
caps lands any piece of R: any instance, any SAFE preparation (any J, any inside mask), either form, any order, and the
joint steps of Lemma J.

*Proof.* Take a branch Z_0, Z_1, … of such steps and its first step that lands a piece of R, from Z_t. For s ≤ t,
off(Z_s) ⊇ R at R's poses, so G(Z_s) ≤ G(R) and Fix(G(R)) ⊆ Fix(G(Z_s)).
1. *Only four caps can land R.* A whole-cap step lands exactly the off pieces in its cap. The pieces of R lie in caps
   148, 203, 217, 224, 242, 252, 270, 286, 324, 353 and 371; caps 242, 252, 270 and 371 are straddled by pieces of R,
   and 203, 224 and 286 have no landing item for R's content. These facts depend only on R's poses, so the landing
   step uses c ∈ {148, 217, 324, 353}.
2. *Mandatory chambers stay occupied.* F_c = C ∩ Fix(G(R)) ∖ H(base) has 26, 42, 39 and 48 chambers for these caps.
   Each is a one-chamber orbit of every G(Z_s), so by Lemma PI, step by step, it is occupied at every Z_s with s ≤ t,
   and at Z_t·X for every SAFE X.
3. *An optimistic middle group.* At the landing step's middle the pieces of R outside c are still off at their poses
   and F_c's occupants are displaced to g(F_c), so the middle group lies in G*(c, i), the group of the caps met neither
   by R ∖ c nor by g(F_c). A joint step needs pairwise chamber-disjoint caps, and the four caps pairwise share 395
   chambers, so exactly one of its caps holds pieces of R and the same bound holds for the joint middle.
4. *The certificate.* G(R) has 588 SAFE grips, 774 orbits and 739 one-chamber orbits. For each of the four caps and
   each of its 12 items, G*(c, i) fixes at least 5, 5, 22 and 7 landing chambers of c's content of R (computed by
   Claude Opus at all three states with the same minima, and by Astra at the entry and at 737).
5. An occupied chamber of the target C then lies in Fix of the middle group, so N1 fails at the middle and no V′ vacates
   C (R22B-29), whatever the preparation. ∎

*Scope (Astra).* The catalogue is the all-content-landing whole-cap steps. Theorem N says nothing about COV-12's
broader cores that move surviving off pieces for a while, about other COV-15 candidates, or about mixed plans: D1's
recorded runner prefix followed by whole caps completes from the entry (Proposition RP).

**Corollary N1 (Question 2.2) [Claude Opus; scope by Astra].** At D1's entry, a recorded reachable state of 𝒞₂, the
glued edge unit 112734 has no passing whole-cap instance at its cell's caps 217 and 324 under any preparation, on any
whole-cap branch while R is off. COV-14's isolated triangle stays a counterexample over valid states whose
reachability is open; Theorem N concerns a reachable state but a narrower, explicitly enumerated catalogue, and does
not widen COV-14's scope.

**Corollary N2 (Cover is not inductive) [Claude Opus; Astra agrees].** For the whole-cap catalogue, E2's 737 state has
five passing caps (COV-33), and five passing whole-cap steps from it ((26,14), (91,14), (513,13), (416,14), (570,15),
a branch of Claude Opus's plan search) reach a state with exactly R off, where every item of every eligible cap is
excluded (COV-33's exclusion over all preparations; Theorem N). For the runner's catalogue, D1's entry has a step
and the stop, reached by 17 of its steps, has none. So COV-15's conditional theorem applies along a run only if Cover
holds at every state reached (the proof is an induction over the states visited); Cover at the entry alone is not
enough. A checkable inductive replacement is a complete static plan (Theorem SP with Theorem R).

**Proposition RP (the recorded-path family) [Astra].** The reduced states (off identities, exact off poses, complete
hole mask) of D1's 17 recorded tail steps, followed by T6's first three steps and its last step without V′⁻¹, form a
certified completion from the entry: 21 strict′ steps, 1,483,042 letters, the longest 449,078. By COV-06 each word
applies to every valid labelled table in the same reduced state. This is a family along one record, not a rule.

## 5. Lengths

**Lemma L (the length of a whole-cap step) [both; scope by Astra].** Let the orbits have checked seeds of length s,
word-valued Schreier tables of greatest expanded path lengths e (P-tables) and f (Q-tables), and c = s + 2e + 2f.
- With *sealed* seeds (each acts as a three-cycle on its orbit and as the identity elsewhere; R22B-08 builds one), a
  preparation moving m_A holes in each source orbit A and a middle word landing |D ∩ Δ| pieces in each demand orbit Δ
  satisfy |X| ≤ 2 Σ_A m_A c_A and |V′| ≤ 2 Σ_Δ |D ∩ Δ| c_Δ; the three-segment step has at most |X| + 2 + |V′| letters,
  the four-segment one |X| + 2 + 2|V′|.
- With *atlas* seeds that fix only the earlier orbits (R22B-30), the multiplier is the target count |T ∩ Δ|, or the
  mismatch when the orbit is reached; the initial mismatch counts are unsound, since an earlier seed may permute a
  later orbit (Astra). The anchor-transfer cost s + 8e + 4f + 2 (R22B-31) belongs to this variant.
- R22B-05 gives tables with e ≤ (d − 2)(2d − 1) and f ≤ (d − 3)(2d − 3)(2d − 1) on an orbit of d chambers, so the
  per-operation cost is O(s + d³) *given the seed length s*. R22B-08's seed needs, besides the Jordan witness, a SAFE
  word of small support (2 ≤ r, 2r − 1 ≤ d), and its bound 4q + 4[ℓM + 2(d − p)][C(d, r) − 1] contains M and C(d, r):
  a Jordan witness alone gives no polynomial or short bound (R22B-09's construction bound has 539 digits; it is not a
  lower bound on the shortest word). R22B-07 gives a deterministic finite fallback, not a short one (scope by Astra).

*Proof.* R22B-01, step 3: to replace an excess hole z by a missing target t in one orbit, choose a third point r and
orient the three-cycle (z r t) or (z t r) by whether r is a hole; each three-cycle is U·C·U⁻¹ with C a seed and U a
table word, or a product of two such, so it costs at most 2(s + 2e + 2f). A sealed seed changes only its own orbit and
only the status of z and t there, so exactly m_A (or |D ∩ Δ|) operations suffice and no other orbit is disturbed; the
flow supplies the hole z, and fixed = 0 excludes one-chamber targets. ∎

**Proposition T6-4 (a bound for T6's step 4) [Claude Opus; reproduced by Astra].** E2's b06+b05 middle
(`b06-14-z1-table.json.gz`) and the empty-mask middle of (91, 14) at T6's 71-state have the same 562 SAFE grips (equal
as sets, rechecked by Astra) and the same target, cap 91. R22B-31's row for that middle (|V′| ≤ 3,489,744, with
k_i = |T ∩ Δ_i|, independent of the holes) therefore applies, and T6's step 4 has a word of at most
21,592 + 2 + 2·3,489,744 = 7,001,082 letters, or 3,511,338 in the three-segment form. No bound below 10⁷ is
established for a stage that has not been built.

**Lemma TS (the trailing inverse) [Claude Opus; chain scope by Astra and the integrator].** (a) If a SAFE word V
vacates Bad(W) at Z, V·W is strict′ (|V| + |W| letters). (b) Under R22C-T1′'s hypotheses W·V′·W⁻¹ is strict′
(2|W| + |V′| letters). (c) X·W·V′·W⁻¹ is strict′ (COV-12). *Proof.* (a) After V, Bad(W) is vacant; W takes no lattice
piece off, lands U and keeps the other off poses. (b) V′ fixes every off piece, so every letter of W⁻¹ is admissible;
Bad(W⁻¹) is vacant, so W⁻¹ takes no lattice piece off and returns the displaced pieces; U was moved out of Bad(W⁻¹) by
V′. ∎ These are single steps: in a chain, every shortened step but the last changes the next step's input
(Corollary CH).

**Lemma TR (transfer by a SAFE word) [Claude Opus; scope by Astra].** If Z and Z_ref have the same off pieces at the
same poses and the same profile on G = G(Z) = G(Z_ref), and every partially filled orbit has a global Jordan witness,
then a SAFE word Y with H(Z)·Y = H(Z_ref) exists (of at most 2 Σ_A m_A c_A letters with sealed seeds, m_A the holes
that differ in A), and for every strict′ step or literal step sequence S at Z_ref, Y·S at Z is strict′ and ends with
the same off poses and hole set (COV-06). It transfers the hole geometry, not the labelled identity of the end.
*Proof.* The product of the Alt(A) acts on each orbit alone and is transitive on hole sets with the same counts per
orbit; then COV-06. ∎

**Proposition B (the lattice phase after a changed end) [Astra; Claude Opus agrees].** A fixed legal word acts
bijectively on labelled states, so two different end tables cannot both be sent to the solved labelled state by the
same word: a changed end needs its own lattice phase or a checked bridge to an end whose lattice phase is verified. The
bridge V′₄⁻¹ (424,994 letters) maps the shortened step 4's end `4d51ea67…` to T6's end `c75c8dde…` (Astra, by
composition; its twist-by-twist replay was stopped at the freeze, README section 3.3), so the shortened chain followed
by that bridge and the recorded lattice phase restores D1, at the cost of the saving.

## 6. Joint steps

**Lemma J (a joint step on separated caps) [Claude Opus; the separation conditions by Astra].** Let (c_k, i_k),
k = 1, …, m, be instances at Z whose caps are eligible and pairwise chamber-disjoint, whose off-piece sets are disjoint,
and such that no displaced region of cap c_k under i_k meets another cap c_l, inside it or straddling it. Then
W = (c_1, i_1) ⋯ (c_m, i_m) is admissible letter by letter, lands U = ∪U_k on D = ∪D_k and displaces exactly the
lattice content of ∪C_k, and X·W·V′·W⁻¹ with V′ vacating ∪C_k at the joint middle is strict′. The joint test is P with
C = ∪C_k and D = ∪D_k. *Proof.* A whole-cap item maps its cap's region to itself, so displaced content stays in its own
cap (computed: 3,097 of 3,097 regions for each instance used); by the separation hypothesis each later letter sees its
cap unstraddled and holding exactly U_k, and W⁻¹ in reverse order likewise; COV-12's argument applies to the union. ∎

**Proposition JT (the joint triple at the stop) [Claude Opus; reproduced by Astra].** At the stop the instances
(91, 14), (217, 15) and (560, 13) have disjoint off sets of 71, 54 and 33 pieces, separated caps, and pass the joint
test: flow 158, a joint middle of 490 SAFE grips, 28 source and 28 middle witnesses. By Lemma J one strict′ step
X·(91,14)(217,15)(560,13)·V′·(560,13)⁻¹(217,15)⁻¹(91,14)⁻¹ lands all 158 pieces. Its word was not built, its length
below 10⁷ is conjectured, and no saving over three stages is established (Astra): both use six core letters, the joint
preparation must move at least 82 holes out of the three caps (25 + 7 + 50), and its middle alphabet (490 SAFE grips
against 541 to 562) is smaller. The three stages of README section 3.2, with
cap 548 third, moved 50 holes in all.

## 7. Obstructions and criteria

**Lemma FO (fixed-chamber obstruction) [Astra; Claude Opus's form for blocked caps].** (a) If every cap that contains a
chamber x is blocked at a state (it holds an off piece or is straddled), x is fixed by the SAFE-ALL group there: only
letters at SAFE caps move chambers, and only chambers of their own cap. (b) Source orbits that hold no hole stay fully
occupied under every preparation. Displace just those mandatory chambers of the cap, keep every other off piece, and
take the group of the remaining SAFE caps: it contains every actual prepared middle group, so if it fixes an occupied
landing chamber at every landing item, every preparation fails. A failed empty-mask flow, a failed witness search or a
builder timeout is not such a certificate.

**Proposition CB (a bracket for glued units) [both].** For a glued unit at Z the candidate steps are finite: the
whole-cap instances at caps of its cell that hold it whole (unstraddled, one domain, a landing item) and the closures
and T1′ steps with its cell-cycle core. Sufficient: some candidate passes P (Theorem SP builds the step), or a closure
satisfies N1 and N2 with witnesses on its target orbits (Corollary A1). Necessary for a whole-cap step: Lemma FO(b)'s
optimistic group fixes no landing chamber and each of its orbits can receive enough holes. Exact: only COV-10's
exhaustive test. At the stop the bracket decides every candidate; at the entry its necessary side excludes 112734's
caps (Corollary N1).

**Proposition CL (no closure lands a unit at the stop) [Claude Opus; scope by Astra].** At the stop no SAFE V, with or
without a preparation, makes V·W·V⁻¹ or V·W strict for the pure or transport cores of card 9's T1. *Proof.* V ∈ G(Z)
keeps the hole count of every orbit and fixes Fix(G(Z)) pointwise, so Bad(W) ⊆ holes(Z·V) needs N1 and N2 at Z, and a
SAFE preparation only lengthens V. The 86 home chambers of the cells {12, 91} and {217, 324} (all occupied) contain 6
one-chamber orbits each (computed), so N1 fails for 9013 and 112734; for 60085 and 81223 T1 records N2 deficits of 20
and 37. ∎ This covers closures only; prepared T1′ and partner steps were not examined.

The shadow explanation of T1's fixed chambers at the cell-cycle middles (the cell's content stays off with its
partner, so the caps through the shadowed chambers stay blocked) is an explanation, not a recomputed certificate
(Astra); its finite check (construct the two middles and enumerate the SAFE grips through their fixed targets) was not
run.
