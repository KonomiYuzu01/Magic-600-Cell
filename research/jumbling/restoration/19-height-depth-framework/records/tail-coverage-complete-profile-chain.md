# Complete bounded-height tail for the table00097 profile class

Archived mathematical record from 2 November 2026. See README.md for its current scope and the independent review record. Original-source digests below identify the captured research version; IMPORT-MANIFEST.json binds this sanitized edition.

## Statement

Let E0 be the1082 labelled off pieces at their exact poses in D1 table00097 (digest cc41accae7cac3add5ab1c3b734b5ba7bee72c1008ec666caf27a1150fdb9938). Let h0 be that table's complete hole-count profile on G(E0), determined by the source SAFE alphabet and the full source chamber-orbit array in the frozen b07 packet.

Every valid full labelled state with exactly this E0 and h0 admits a ten-stage strict' chain landing all its off pieces. Every prefix of the complete literal word has left height at most2. Every stage fixes the exact matrices of all surviving off pieces and preserves lattice membership of its source lattice pieces at its endpoint. The chain is:

| Stage | Admitted core | Core letters | Bad chambers | Boundary off count |
|---|---|---:|---:|---:|
| 1 | native recorded00098, b07 | 48 | 198 | 1082 -> 971 |
| 2 | native recorded00099, b16 | 96 | 111 | 971 -> 860 |
| 3 | native recorded00100, b13 | 24 | 198 | 860 -> 759 |
| 4 | native recorded00101, b17 | 576 | 86 | 759 -> 673 |
| 5 | SP cap91,item14 | 1 core + inverse | whole cap | 673 -> 389 |
| 6 | SP cap353,item22 | 1 core + inverse | whole cap | 389 -> 241 |
| 7 | SP cap217,item15 | 1 core + inverse | whole cap | 241 -> 216 |
| 8 | SP cap416,item14 | 1 core + inverse | whole cap | 216 -> 185 |
| 9 | SP cap548,item12 | 1 core + inverse | whole cap | 185 -> 60 |
| 10 | SP cap570,item15 | 1 core + inverse | whole cap | 60 -> 0 |

Native stages have the form XW, with X a source SAFE preparation that vacates every Bad(W) chamber. SP stages have the existing three-segment form X W Vprime W^-1. All preparations and middle words are rebuilt for the actual hole set. Stored later literal words are not reused merely because profiles agree.

Writing C(n,k)=binom(n,k) and N=177120, one explicit uniform menu-letter upper bound is

    2*C(N,198) + C(N,111) + C(N,86) + 740
    + C(N,673) + C(N,389) + C(N,241) + C(N,216)
    + C(N,185) + C(N,60) + 6*C(N,3097).

The exact integer and its digit count are in the completion manifest. This is a finite constructive existence bound with infeasible worst-case searches, not a practical-length claim or admission under10^7. A mathematically specified finite search computes every needed word.

## Proof and measured obligations

The prepared native-core criterion is proved in tail-coverage-b07-profile-primitive.md. Each of the four words has a legal, hole-independent trajectory for every source off piece; it lands exactly its stated unit and returns all survivors exactly. Full actual replay checks these contracts. Its Bad set includes all prefix straddle deaths and terminal off ghosts. Every one of the177120 lattice ghosts was followed; there are no deaths for these words, and Bad consists of the listed terminal off ghosts. Every good ghost, including those at currently empty chambers, stays in its G(finaloff)-orbit. Each orbit needs at least as many source holes as its Bad demand, and independent alternating action on each required partial source orbit is certified by a global Jordan word checked on all177120chambers.

Thus the word preceded by SAFE Bad-vacating preparation is strict', orbit-clean and admitted for every valid source with the stated exact off data and profile. All source off trajectories and all lattice ghosts have height0or2 at every prefix; source SAFE preparation preserves height. The four native stages therefore stay in C2. Their endpoint profiles follow Lemma OC and are independent of the selected preparation or hidden lattice orientations. The exact profile at each native handoff agrees with the next recorded source profile.

At the fourth endpoint the off geometry is exactly the673off pieces of recorded table00101. The six prescribed whole-cap tests were recomputed using that off geometry and the virtual profiles. Every cap is unstraddled and lands all its off content. Every fixed-target count is zero, every source capacity inequality holds, every integral flow has full demand, and every required source and middle orbit has a checked global Jordan witness. Complete chosen prepared hole sets and integral flows are saved. Each middle SAFE alphabet is included in its endpoint SAFE alphabet. The propagated final profile is identically zero, and the final off set is empty. These are precisely Stage18 P and SP hypotheses, so SP supplies all six stages at the actual endpoints.

For each such SP stage, source and middle retained segments preserve height, W lands its height2off content while taking only lattice pieces to height2, and W^-1 restores those displaced pieces. Thus these stages also stay in C2. The boundary rank |off| decreases strictly at all ten stages. At the end every physical piece has a K+ pose. No assertion is made that the labelled stickers are solved or every exact pose is identity.

For a native stage with b Bad chambers, inverse target-image BFS gives a preparation of length <=C(N,b)-1, followed by its fixed core. Summing four cores gives the first line of the bound: their lengths total744, hence the constant740. For each SP stage with n source holes, source-hole BFS gives preparation length <=C(N,n)-1; the existing inverse target-image BFS on the3097chamber cap gives middle length <=C(N,3097)-1. Adding W and W^-1 gives C(N,n)+C(N,3097). Summing the six displayed source counts proves the total. All graph edges are actual retained I_a menu letters, so no generator expansion is omitted.

## Why this is an actual coverage increment

The independently checked residue theorem excludes completion of table00097 by ordinary whole-cap SP and by the proposed orbit-clean double-SAFE subset-cap catalogue. It even excludes a first single-cap COV-12 residue landing after an arbitrary orbit-clean strict' prefix preserving the residue poses. Nevertheless the certified48letter b07 multi-letter core, with SAFE preparation when needed, lands111residue pieces and supplies the first stage above. The passing ten-stage profile plan therefore contains the whole table00097 profile class, which ordinary SP cannot complete. This is strict inclusion of these admitted tail classes, not universal coverage of C2.

The recorded path and its literal cores were already known. COV-10 preparation, Lemma OC, SP and target-image bounds are also existing source results. The additional checked fact is that these specific four native words admit uniform profile-level preparation and orbit-clean handoffs, allowing a complete mixed static profile plan instead of dependence on one recorded sequence of literal hole sets. The one-letter source SAFE counterexample(194,1) followed by the unprepared b07 word, ending1144off, proves why the preparation and handoff argument cannot be omitted.

For an input promised reachable from solved, all actual constructed endpoints remain reachable because every letter is legal. This permits a subsequent lattice sticker-restoration theorem with the corresponding reachable-endpoint hypothesis. It does not establish a uniform sticker-restoration cost here, and does not address the exact-pose LRH gap.

## Frozen evidence and limits

- First native stage: tail-coverage-b07-resource-audit.json, tail-coverage-b07-profile-guard.json, tail-coverage-b07-prepared-profile.json, tail-coverage-b07-height-and-counterexample.json, their commands and exact arrays/tables under tail-coverage-input/.
- Next three native stages: tail-coverage-short-core-chain.json and command. All696forward letters and696inverse letters passed; inverse restored all177120matrices. [timing metadata omitted] All three global good-ghost crossing counts are zero, all preparation certificates pass and height maxima are2.
- Six SP stages: tail-coverage-six-stage-static-suffix.json and command. [timing metadata omitted]; zero physical twists, since this is an exact finite existence/admission certificate. Saved integral flows, chosen J sets, complete source/middle/global witness checks and full chamber orbit arrays support every P test.
- Input tables, word blobs and unit lists are pinned and Git-blob verified. The completion manifest binds all source and incremental evidence by SHA256 and checks native-to-SP profile joins.

No full literal suffix was generated, no literal gate3 replay is claimed, and no practical general solver or global upper-bound theorem follows. The result is a complete, quantitative, bounded-height tail construction on an explicit family that the prior restricted static plan cannot cover.
