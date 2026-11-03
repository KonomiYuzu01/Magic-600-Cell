# State-only regional-profile descent through the verified height-four frontier

Archived mathematical record from 2 November 2026. See README.md for its current scope and the independent review record. Original-source digests below identify the captured research version; IMPORT-MANIFEST.json binds this sanitized edition.

## 1. Model, source and claim

Use snapshot a2ebb4cccd6b60d7527c001ef3b2cbc56b179249, the full 177120 physical pieces and 259800 labelled sticker slots, exact Q(sqrt(5)) matrices and chronological literal I_a menu letters. The cost metric is not the retained-generator metric. Let lambda_p be the complete home region of physical piece p, K+ the finite orientation-preserving lattice symmetry group, h_p the existing tree height, and type(p) its fixed registry type. No physical labels are changed by the procedure.

For each valid exact reference table Y, define Gamma(Y) by certificates consisting of a type-preserving bijection pi on physical pieces and t_p in K+ with

    t_p(lambda_p) = lambda_pi(p),
    Z_p = Y_pi(p) t_p.

This is the definition and word-transfer setting of REGIONAL_HIGH_RESTORATION.md (SHA256 40d90a463aca52b6917f600be8a82787838bdc962628c2482280dbfc6386bf46). The present result specializes that previously reviewed lemma to a finite, nontrivial descent segment. It does not introduce a new general symmetry theorem.

THEOREM. The finite indexed reference catalogue below defines two exactly decidable domains of current states. In either fixed mode, an admitted state at a macro boundary has a history-free legal continuation to its declared exit. The admission certificate is preserved at each boundary, every physical piece's collateral trajectory transfers, and one fixed integer progress rank strictly decreases at each selected block. From the initial profile the literal costs are 5 or 9, and every literal prefix has maximum height 4. From every admitted post-A profile, the exact distance to eliminating q11 is 3 literal letters. The exits still have height 4 and q16=6; they are research frontiers, not solved or all-lattice states.

## 2. Frozen reference catalogue

Let Y0 be the previously checked reachable table with digest

    f3fc88372815b7297a6d62baf806a49265c3f0f439276e6b86844ef96c61cadd.

Its file irregularity-descent-source-end-table.json has SHA256 1c724b3fa0b46c210e8d245aeb586703f76e2c97a5db0ecedaf34200ef84bf13. Define chronological blocks

    A_j = (282,j)
    D   = (59,7),(9,14),(543,10),(598,15)
    C   = (371,2),(196,20),(262,21)
    E   = (196,14),

where j runs through [13,34,35,36,37,38,19,17,39,40,41,42]. The representative A is A_13. The 12 matrices are the independently checked left-retained coset of A. Their regional arrangements may differ; they are not identified as equal full states.

The indexed catalogue consists of Y0 and, for each j, the six exact endpoints Y_Aj, Y_AjC, Y_AjCE, Y_AjD, Y_AjDC, Y_AjDCE. Thus there are 73 indexed references, without a claim that all references or Gamma fibres are distinct. They are defined by exact finite legal words; no stored history is required from an input Z.

Existing independent receipts establish legality and full effects for j=13 and all 12 first-A variants. Every cap of D,C,E is globally noninteracting with cap282. Every cap of D is globally noninteracting with every cap of C,E. This is disjointness of piecewise puzzle supports, not unrestricted matrix commutation. Therefore the suffix supports and legality are unchanged by the first-A variant, and the exact coset covariance transfers the recorded metrics to all 12 variants. A retained world symmetry preserves lattice membership and height on the affected support. No fresh 73-table replay is claimed.

Write q_t = sum over type-t pieces of max(h_p-2,0)/2, and let n be the number of off-lattice pieces. The boundary values, identical for all j within a row, are:

| Phase | q11,q12,q13,q14,q15,q16 | Phi | n | H |
|---|---|---:|---:|---:|
| Y0 | 2,6,2,1,3,6 | 536 | 103857 | 4 |
| A | 1,3,1,0,1,7 | 1020 | 103388 | 4 |
| A,C | 0,0,0,0,0,7 | 1486 | 105215 | 4 |
| A,C,E | 0,0,0,0,0,6 | 1136 | 104444 | 4 |
| A,D | 1,3,1,0,1,7 | 1020 | 101611 | 4 |
| A,D,C | 0,0,0,0,0,7 | 1486 | 103438 | 4 |
| A,D,C,E | 0,0,0,0,0,6 | 1136 | 102667 | 4 |

Types 0 through 10 have q=0 at these boundaries. This is a property of these references and their Gamma fibres, not a universal theorem about low registry types. All 36 coordinates, not only the displayed six, are used in the rank and the evidence.

## 3. Exact finite recognition and legal language equivalence

For a given reference Y and input Z, construct a finite bipartite graph on two copies of the physical-piece set. Put an edge p to q precisely when types agree and

    t = inverse(Y_q) Z_p is in K+, and t(lambda_p)=lambda_q.

All tests are exact with the fixed finite K+ and complete region data. A perfect matching gives pi and t_p. Conversely every Gamma certificate supplies such a matching. Trying the finite reference catalogue therefore decides the stated domain and returns a certificate, without a scramble history. This is an existence/termination algorithm, not a performance claim or a completed practical recognizer.

For a matched pair, Z_p(lambda_p)=Y_pi(p)(lambda_pi(p)). Thus the sets of occupied regions are identical and validity transfers from Y. For any cut, corresponding regions have the same IN/OUT/STR classification. Its literal move is legal at Z if and only if it is legal at Y. A moved pair receives the same exact left multiplier; an unmoved pair stays fixed. Consequently

    Z_p(w) = Y_pi(p)(w) t_p

The same pi and gauges give a certificate for Gamma(Yw) at each next boundary. Nothing here requires Z to be reachable from solved. Synthetic Gamma inputs may be unreachable; no reachability conclusion or lattice sticker theorem is invoked for them.

## 4. State-only block controller, closure and costs

Fix one mode before starting. At any boundary, recognize the input against that mode's catalogue, choose the first matching j in the displayed order if several exist, and emit the next block. At Y0 choose j=13. At an exit return FRONTIER_REACHED. Outside the finite union return OUTSIDE_CERTIFIED_DOMAIN, which is not an unreachability decision.

| Mode | Admitted phases in order | Blocks | Remaining literal costs |
|---|---|---|---|
| Compact | Y0, A, A,C, A,C,E | A,C,E | 5,4,1,0 |
| Cleanup | Y0, A, A,D, A,D,C, A,D,C,E | A,D,C,E | 9,8,4,1,0 |

The boundary q,n values separate phases within either mode, so an input cannot simultaneously be an exit and a nonterminal phase. Same-phase overlaps between different j do not cause an action conflict: the next block is the same. The language-transfer identity proves the next move is legal and the endpoint remains admitted, with the same certificate. Induction over the finite remaining phases proves termination at the stated exit and the stated costs. These are exact lengths of this selected continuation, hence upper bounds on reaching the exit set, not optimal restoration distances.

The domain is closed under these selected macro transitions. Literal internal prefixes remain valid and obey the height ceiling but are not asserted to belong to the macro-boundary catalogue. A runtime interrupted inside a block would require a saved literal index or a larger prefix catalogue; neither is a new implemented feature here.

Let N_t be the fixed model type counts and set

    w_t = product over s>t of (1+3 N_s),
    R3 = sum_t w_t q_t,
    S = (177120+1) R3 + n.

The independently proved bounded-difference rank theorem applies to A,C,E, whose lengths are 1,3,1 and whose first changed q coordinates are respectively 11,11,16, each decreasing by 1. Hence every compact block strictly decreases the SAME R3. The cleanup block D has length 4 but preserves the entire q vector and decreases n by 1777. Therefore every cleanup block strictly decreases the SAME S. There is no application of a length-3 theorem to D and no change of weights between phases. Lower-priority q coordinates and intermediate values may rise.

All literal reference prefixes have H=4; transfer proves the same for all admitted inputs. At either exit every piece of types 0..15 has height at most 2, while type16 retains q16=6. The cleanup exit has 1777 fewer off-lattice pieces than the compact exit. The endpoints are different full states and cannot be substituted in subsequent legality tests.

## 5. Exact depth and what was actually generalized

The independent shortened-witness review proves the exact q11-elimination distance at all 12 post-A references is 3. It freshly identifies the sole type11 high piece, verifies all five holders blocked, and proves the full straddler sets of its two lowering holders have no common whole holder. Thus no first legal letter moves it, and no one-letter preparation permits a lowering second letter. The word C supplies the legal three-letter upper bound. Retained moves and arbitrary other first cuts are included in that scoped lower bound.

Because legal-word languages and the entire q vector agree prefix by prefix under Gamma, a word eliminates q11 at Z if and only if it does so at the corresponding Y_Aj. Hence the same exact distance 3 holds throughout their fibres. This is a direct equality-of-languages argument; it does not assert that the five-letter word is shortest or determine full restoration depth.

The milestone therefore upgrades a finite list of witnesses into an explicitly recognizable family of current labelled configurations with closed macro transitions, fixed progress ranks, full collateral and exact local depth. The family is finite in this fixed model: there are finitely many reference matrices, finite K+, and finitely many type-preserving bijections. It is not an unbounded-height extension, all height-four states, or an arbitrary reachable-state theorem.

## 6. Evidence and unresolved obligations

Reused independent proof/evidence boundaries:

- Regional transfer: REGIONAL_HIGH_RESTORATION.md SHA256 40d90a463aca52b6917f600be8a82787838bdc962628c2482280dbfc6386bf46; independent review SHA256 fd704b6654bab0e9d8a34406d50a88ec923e84d41962e235aa7c3dbef643f1be.
- Fixed weights: type-height-weight-proof.md SHA256 38900979b087f8d620843c5f73464a327dd373268bf78d9ae5dfc5186a4be2da; independent review SHA256 397700905baa165350f3ce1def4ec4774ff394ff7bc6440180d4739d404f5dd1.
- Nine-letter transfer/full effects: type-lex-recovery-review-note.md SHA256 3570973b0bda276025d59d9afc3ca9d37c7998fc5fb1673f1cd4cdc13efc70c7; receipt SHA256 46dacb5202f170496c9d323c1eaee3045630f0fb8804f3e88bb09e3323bd436d.
- Short witness, common ranks, disjoint support and all-12 exact depth: type-lex-trimmed-independent-review.md SHA256 845d6a4b8b775b2785a916773efaba18335962d5baf9a6bc97e5736113cde1e1; receipt SHA256 6aa749702a36441ca1dda4306d7f9da3ff50582d3709e1b2b165e80ccd5dc5fe.
- Short endpoint table SHA256 3ecd27648d1cf2f81240e418dbef55355a3ec6146c2dd6b75f78d6c8bea7d7eb; long endpoint table SHA256 722471dba8c67cd1a5296e5508acc60bcb85f245ebebc8315632dc41bd637859.

A separate bounded synthetic Gamma certificate/replay checks one nontrivial physical-piece cycle and the nine-letter transfer, then its complete inverse. Its result and two independent review shards must be recorded separately before this candidate is reported as independently checked. A sample matching certificate is not a full implementation of finite perfect-matching recognition; no fresh short-path replay is claimed for that synthetic input.

Open: existence of a legal next descent beyond the type16 exit with maintained earlier priorities; a uniform length or proved height-dependent length contract; coverage/entrance for arbitrary reachable states; all reachable height-at-most-2 tails; eventual entry to the already proved reachable-lattice sticker phase. The fixed-rank and controlled-cost upper bounds remain conditional on those hypotheses. Old fair-search termination is not a new quantitative bound. The actual exits do not satisfy the lattice phase's entrance premise.

Review packet scope: (1) accept only this finite family theorem and its stated depth/cost scope; (2) the problem is transferring finite witness chains into recognized current-state domains; (3) source/model/runtime are pinned above and in reused receipts; (4) inspect only this document and the listed private evidence; (5) prior regional and concrete word proofs are reused, not rerun; (6) main and source stay read-only, reviewer outputs use separate profile-family-review-* files, no search or product changes; (7) return PASS or precise findings with path/line/digest and explicit checked/unverified scope, then stop. Numerical replay has its separate bounded packet. This is research verification, not an architectural or release decision.
