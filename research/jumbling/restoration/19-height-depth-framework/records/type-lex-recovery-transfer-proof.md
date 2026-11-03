# A verified longer type-height descent after the literal trap

Status: exact producer construction and fresh full forward/inverse replay PASS. Independent commutation/typed-rank review is a separate receipt. No global selector or restoration theorem is claimed.

1. **Goal and admitted inputs.** Reuse the already checked recovery word from old `f3fc88372815b7297a6d62baf806a49265c3f0f439276e6b86844ef96c61cadd` after its independently verified first letter `(282,13)`. The recovery is

   `W=(59,7),(9,14),(543,10),(598,15),(371,2),(196,20),(262,21),(196,14)`.

   The literal `(262,21)` is the exact same-cap product of the previously checked `(262,14),(262,6)`. The new full replay executes item21 directly and checks its actual legality. No new word search is involved.

2. **Matching source and finite evidence.** Repository snapshot `a2ebb4cccd6b60d7527c001ef3b2cbc56b179249`; existing Windows Python/NumPy and exact simulator, `-I -B`, one numeric thread. The old source tree, complete recovery371 cache, independent first13 endpoint, and prior all-piece word certificates are pinned in `type-lex-recovery-transfer-results.json`. The first13 endpoint digest is `1aab57964a39e5af7e49fe49c1ea2b7d1fac02b77b8482ba6f32aae353a6e5f1`. Main/source and all earlier evidence remain unchanged.

3. **Exact transfer proof.** Stage14 `PROOFS.md`, Lemmas L1 and L10, supplies global noninteraction. For each of the seven distinct cap IDs in W, the exact value

   `Delta(282,c)=R_body^2*(NN+n282·nc)-2*kappa^2`

   is strictly negative; all Q(sqrt5) triples are saved. This is a global geometric condition, not a test only at the source. A letter at c moves only pieces wholly inside c and keeps them inside c. Such pieces remain OUT282. Conversely the pieces moved by `(282,13)` stay OUTc. Therefore either action changes neither the other cut's labels nor its full inside support. Their legal statuses are preserved and their piecewise actions commute. Induction over W proves that W is legal after `(282,13)`, that `(282,13)` is legal after W, and that the two orders give identical exact labelled-piece poses. Interactions among W's own caps are allowed.

   This also transports any sum of per-piece pose functions, including each typed q component: `Delta_q(W after28213)=Delta_q(W at oldsource)`. It is an application of existing locality facts, not a new unrestricted commutation claim.

4. **Constructed join and fresh validation.** Let S be the 3,106 original IN282 pieces. The prior recovery leaves every S pose exactly unchanged; the first13 move leaves every outside-S pose unchanged. The joined endpoint is formed by selecting the first13 poses on S and the recovery poses outside S. All177120 matrix identities are checked. Its table is `type-lex-recovery-transfer-joined-table.json`, SHA256 `722471dba8c67cd1a5296e5508acc60bcb85f245ebebc8315632dc41bd637859`, digest

   `a2f9f3524adff7336518fb0e6dc668515b05c01453187b5b5135b8c32260a4b6`.

   Separately, the new simulator run executes all eight W letters from the independent first13 state, then all eight inverse letters. Every letter is freshly legal, every support is disjoint from S, and every moved and unchanged piece satisfies its chronological exact matrix contract. The forward endpoint matches the entire constructed table, and the inverse restores the entire starting first13 table. This is sixteen fresh `State.apply` letters, not merely a hash comparison.

   The inverse is `(196,14),(262,21),(196,50),(371,5),(598,44),(543,10),(9,14),(59,6)`.

5. **Fixed potential and measured collateral.** Use the same fixed L=8 weights `w_t=product_{s>t}(1+8*N_s)` throughout. The initial `(282,13)` step strictly decreases this rank, and the W endpoint strictly decreases it again. No potential is switched between stages. W's first changed typed q component is type11, down by1. Piece114169 falls from height4 to height2. The initial and final low-priority coordinates are

   | Boundary | q11 | q12 | q13 | q14 | q15 | q16 |
   |---|---:|---:|---:|---:|---:|---:|
   | Old source | 2 | 6 | 2 | 1 | 3 | 6 |
   | After28213 | 1 | 3 | 1 | 0 | 1 | 7 |
   | After transferred W | 0 | 0 | 0 | 0 | 0 | 6 |

   All earlier types0..10 are zero throughout these boundaries. The final state therefore has **every piece of types0..15 at height at most2**, not necessarily on the lattice. The first nonzero q type is16.

   Complete prefix metrics, including typed I and q vectors, are recorded from actual canonical poses and exact current cut labels:

   | W prefix | I | Phi | Uniform R1 | Off count | Max height |
   |---|---:|---:|---:|---:|---:|
   | Start | 92 | 1020 | 602 | 103388 | 4 |
   | 1 | 92 | 1020 | 602 | 103388 | 4 |
   | 2 | 92 | 1020 | 602 | 102145 | 4 |
   | 3 | 92 | 1020 | 602 | 102145 | 4 |
   | 4 | 92 | 1020 | 602 | 101611 | 4 |
   | 5 | 92 | 1020 | 602 | 101611 | 4 |
   | 6 | 128 | 1370 | 813 | 102377 | 4 |
   | 7 | 128 | 1486 | 871 | 103438 | 4 |
   | 8 | 92 | 1136 | 660 | 102667 | 4 |

   The final state has568 high pieces. Total Phi and uniform R1 rise, and W has nonmonotone intermediate load; the progress claim is exactly the fixed typed rank at macro boundaries. No intermediate type elimination was assumed. The measured maximum height over all W prefixes is4, and maximum Phi is1486.

6. **Complete continuation cache and provenance.** The direct forward-path cache updates only actual moved physical rows, including lattice and off pieces. Retained moves use exact normal/class transport, and jumble moves recompute all80 interacting columns. No S row is reclassified during W because no W letter moves it. All177120×600 final labels, heights and branches also match the independently assembled row splice: take the certified recovery371 cache and replace its S rows with the certified first13 cache's S rows.

   Full cache: `type-lex-recovery-transfer-full-cache.npz`, SHA256 `5da8ca2083769e6766c7772b9b817c06797d152d0da859fb8f488e0bbd75c24c`.

   Transfer arithmetic/join receipt: `type-lex-recovery-transfer-results.json`, SHA256 `db2a8561b31068d35a4394fa8b9360508a48c2abd0b10eab9dd40bf1164b5398`;6.265625 CPU seconds, zero moves. Fresh replay receipt: `type-lex-recovery-transfer-full-replay.json`, SHA256 `59a324f7f10bf87fdd554355b9509812ab184c6f6a392d4061e8bf5e3bf23552`;19.5625 CPU seconds and16 moves. Each receipt binds its complete source/command/input hashes. Both independently authorized CPU budgets were respected. No process is active.

7. **Scope and open obligations.** This supplies a concrete two-macro chain of lengths1 and8 from a known reachable state, with a common fixed progress measure, full legality, all collateral, measured height peak and an exact next-state cache. It passes beyond the verified failure of any second strict literal step after the first orientation family. It does not contradict that L=1 obstruction: the new step is a longer macro. It does not prove further availability at type16, a fixed bounded-length selector on all reachable states, preservation of a general admitted class, or completion of the remaining height-at-most2 tail. No successor census or automatic iteration was run in this packet.
