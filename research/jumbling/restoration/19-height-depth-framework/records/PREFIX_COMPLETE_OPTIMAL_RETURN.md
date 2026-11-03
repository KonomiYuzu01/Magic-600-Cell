# Optimal return including incomplete pump blocks

Archived mathematical record from 2 November 2026. See README.md for its current scope and the independent review record. Original-source digests below identify the captured research version; IMPORT-MANIFEST.json binds this sanitized edition.

Historical proof draft, 2026-11-02. This extends GUARDED_OPTIMAL_RETURN.md. Its source isolation, perturbation, tree-code, and verified 1824-letter suffix premises remain unchanged. This is an exact recognition theorem for a specified infinite language, not a general restoration algorithm.

Let A be the guarded boundary language recognized by that document. Let V(i,k) be the first k letters of certificate word W_i, with 0 <= k < |W_i|. Define

    A_prefix = { V(i,k)(Y) : Y in A, i in {0,...,6}, 0 <= k < |W_i| }.

All these prefixes are legal at every Y in A. The source perturbation bounds were checked for every prefix of every W_i uniformly over the input ball, independently of the box chosen by the deterministic family rule. The full block might leave the guard if it was not the selected block; legality and strict height growth of its prefixes do not require the output guard.

The source step lemma gives h_K(V(i,k)(Y)) = h_K(Y) + 2k, and every K piece still shares a pose. The outside-K configuration depends only on the two cumulative finite debris labels of V(i,k), because every Y in A has the same sigma background. The finite prefix catalogue has 1746 occurrences before deduplication.

## Recognition without a recorded input history

For an input state S:

1. Require the seven labelled K pieces to share an exact pose Q.
2. For each catalogue entry (i,k), compare every outside-K canonical pose matrix with its exact prefix-background template. A pair of inferred debris labels can index candidates, but cannot replace this full comparison.
3. Let P(i,k) be the chronological target matrix product of the prefix. Form the boundary candidate with K pose P(i,k)^-1 Q and exact sigma background.
4. Run the guarded boundary recognizer on that candidate. On acceptance, also check h_K(S) - h_K(candidate) = 2k.
5. Return the literal inverse prefix, then the boundary decoder word, then the fixed sigma-to-tau240 suffix. If no candidate accepts, reject.

The finite enumeration may have several accepting descriptions. Uniqueness of a prefix description is neither required nor claimed. Any accepted description gives the same optimal total length.

For soundness, a recognized boundary candidate Y belongs to A. The source uniform prefix result makes V(i,k) legal at Y. Its target matrix and complete outside-K template reproduce S exactly. Therefore the literal inverse prefix is legal from S back to Y, with full collateral effects. Appending the already proved boundary return is legal and finishes at the same fixed tau240 state.

For completeness, an input in A_prefix has at least its defining (i,k,Y) description. The corresponding template matches, and the boundary recognizer accepts Y. No original input history is consulted.

Termination follows from the finite catalogue and the terminating boundary recognizer. A conservative number of boundary-decoder iterations per catalogue entry is floor((H-3888)/132), where H is the input K height. This is an iteration bound, not a claim about bit complexity of exact arithmetic.

## Exact depth

For any accepted description, the output length is

    k + (H - 2k - 240)/2 = (H - 240)/2.

Every legal I_a letter changes an individual piece's left height by at most 2. Hence no competing path to the fixed tau240 state, or to any state where a chosen one of these K pieces has height at most 240, can be shorter. The result also holds for the minimum to any or all seven K pieces at height at most 240, with optional exact equality to the sigma outside-K background at the endpoint. An intermediate input can have a different outside-K background; equality to that input background is not asserted.

Thus the earlier optimal state-only decoder extends from guarded block boundaries to every literal intermediate state of every admitted guarded concatenation. It also recognizes the extra safe prefixes in A_prefix as defined above. It does not cover arbitrary same-height states, arbitrary states passing only the Euclidean guard, or general lattice landing.

## Verification boundary

The proof is a finite-union closure consequence of the independently checked boundary theorem and the published uniform prefix admissibility result. The completed prototype is recorded in evidence/prefix-complete-optimal-decoder.json: 1746 occurrences give 1348 distinct exact product/background entries and 298 background-label pairs. Two intermediate inputs passed recognition and 620 total forward/inverse full-state replay letters; outside corruption and a guarded non-sigma floor state were rejected. The root independently recomputed all prefix products and labels, all seven complete tree-code vertices, and all21 pair distances in evidence/prefix-catalogue-independent-verified.json. PREFIX_COMPLETE_OPTIMAL_RETURN_REVIEW.md reviews the theorem and the explicit sigma-background endpoint contract. These checks establish the stated finite obligations and samples; the infinite claim still uses the written induction and published geometric premises.
