# Independent review of entry-profile transfer to the recorded tail

**PASS. No findings within the packet.** All 29 partially filled source orbits have independently verified global Jordan witnesses. The exact subset-state bound and the stated application of existing Stage18 Lemma TR and Proposition RP follow. This is an alternative quantitative construction on the specified D1-entry off-geometry/profile family; it is not general coverage of C2.

The domain consists of valid labelled states with the reconstructed D1 entry's exact off-piece identities and exact poses, and the same complete source SAFE-group hole profile. The reference entry digest is `7e128820541abc82e45c683db92f38413acba6e282c00f3185585e507b4e6818`. It has 1193 off pieces and 1193 holes, all off poses at left height 2 and right height 0. Arbitrary validity is not replaced by a promise of reachability. Reachability is needed only for the sticker-solving corollary below.

The source reference is `a2ebb4cccd6b60d7527c001ef3b2cbc56b179249`. Frozen input hashes matched before and after the computation:

| Input | SHA256 |
| --- | --- |
| `entry-profile-transfer-audit.json` | `f1a370f7151ad03f079a50f980f4b5ce7e28b330f7ca7e53ad405a63bfac1239` |
| `entry-profile-transfer-audit.py` | `9e419e1a86e9bf3839b5291d45a5f7591cf234402ff2f4b04516130e49d8c93d` |
| `entry-profile-transfer-command.ps1.txt` | `a65c0bd2f796431af8f4cc532f797c77a25b6b6809740ab2c47edb937fe315a9` |
| `entry-profile-core-audit.json` | `a519bb562d2ab47f12bf925cf021b0cf8981089a7975d933ec7f14d8d5f31e65` |
| `entry-profile-input/entry-reconstructed-table.json` | `ba22b6603ab1d602acbb87f8188a72acda4af13c99f23a17bbc6f3372eb81837` |
| `entry-profile-input/entry-core-arrays.npz` | `9434603bb4a23ecf30f90c08aa09616bab3ccc58d975e1c36fc9a01e06daee3a` |

The independent run is [entry-profile-transfer-independent-check.py](<archive>/entry-profile-transfer-independent-check.py), SHA256 `1f2c7cbe4b68ede25173224050e14d134a2e143d9ac8bc7d81f4376265b8e41e`. Its [production command](<archive>/entry-profile-transfer-independent-command.ps1.txt) has SHA256 `d636872cb1b85a854259f5561cf0f5fe77ce061d29120be0726778272083bb24`. The [computation receipt](<archive>/entry-profile-transfer-independent-check.json) has SHA256 `ab087d68990ab8192f33b5073d4d56b06f5df6c297733404ec5dee7475e57300`.

CPython 3.14.7 / NumPy 2.3.5 completed the independent computation in **11.734375 CPU seconds / 11.8733201 wall seconds**, with zero full simulator letters. No producer `BF.jordan`, `BF.check_jordan`, `BF.perm`, or `BF.orbits` call was used by the independent checker.

- 715800 off-piece/cut classifications were rebuilt from pulled-back exact normals and home vertices; 2386 scalar kernel checks agreed. This reproduced all 549 SAFE caps.
- The two model generators at every SAFE cap were checked to generate that cap's complete 12-element retained subgroup. A separate DSU over 3400506 generator edges reconstructed all 1272 source-group orbits, including their canonical least-chamber numbering. The entire stored orbit array, sizes and hole profile matched.
- Exactly 29 orbits are partially filled: orbit 6 and orbits 8 through 35. Every one has a certificate, with no missing or small exceptional orbit.
- The 29 witnesses use 27 distinct words, totalling 4080 literal permutation letters. Each complete source-to-destination permutation on all 177120 chambers was rebuilt by chronological scattering. The first 400-letter word was also rebuilt by direct full-array forward composition.
- Every complete global cycle decomposition passed the prime, size, unique-divisible-cycle and named-orbit tests. The checker also records an isolating exponent and verifies its nonzero residue modulo the prime.

All global permutations and the reconstructed orbit/profile/hole arrays are preserved in [entry-profile-transfer-independent-permutations.npz](<archive>/entry-profile-transfer-independent-permutations.npz), SHA256 `58847bede9a770f939153711dc3e1d68bbe7383f291abdf807a7d51809e4b5d9`. The computation receipt binds each witness to its full permutation digest and global cycle histogram.

The witnesses establish the strong premise needed for transfer, rather than merely large projected groups. For an orbit A of size n, the checked word has a prime p with `n/2 < p <= n-3` and exactly one globally p-divisible cycle, of length p inside A. Raising the word to the lcm of all other cycle lengths isolates this p-cycle and fixes every chamber outside it. The orbit action is transitive. A nontrivial block system is incompatible with a p-cycle larger than half the orbit: it has fewer than p blocks, each smaller than p. Thus the action is primitive; Jordan's theorem gives Alt(A). The normal closure of the isolated cycle provides Alt(A) supported on A alone. Consequently the product of these alternating groups over the 29 partial orbits lies in the actual source SAFE group. There is no unproved independence assumption between different orbit projections.

Let `n_A=|A|` and `h_A` be the checked full source profile, and define

`M = product_A binom(n_A,h_A)`.

Fully empty and fully occupied orbits contribute a factor 1. The global alternating subgroups act transitively on all hole subsets with this profile. For any state in the stated valid family, the actual recorded entry hole mask is therefore reachable by a SAFE word Y. Its edges are actual retained I_a menu letters, each costing one literal twist. The hole-mask graph has at most M vertices, so a shortest path has no repeated vertex and

`|Y| <= M-1`.

This argument takes place in the quotient by hole masks, not in the much larger labelled-pose state space. It is sound because SAFE legality and chamber action depend only on the fixed off geometry, and the later recorded path depends only on the reduced state. Removing a repeated-mask segment may change hidden lattice labels or orientations, but neither the following SAFE edges nor applicability of the recorded tail requires those hidden data to match.

The exact independently computed `M-1` matches the candidate's 2833-digit integer. The looser bound `binom(177120,1193)-1` also matches exactly and has 3106 digits. All integers and the all-lattice/sticker totals are retained in the computation receipt; no seed word, power, or generator-expansion cost is silently treated as one letter.

The source theorem being reused is [Stage18 PROOFS.md](<repo>/research/jumbling/restoration/18-tail-completion/PROOFS.md): Proposition RP at line 251 and Lemma TR at line 294. The file SHA256 is `59bb0a8b1accbd15699b89721bf45ba3399ddda2875e125ca1702344f32aa928`. RP supplies the recorded 21-step strict-prime completion of length 1483042. After Y, the off poses and complete hole mask are exactly RP's required reduced entry state, so TR/COV-06 applies to the whole recorded sequence. The all-lattice bound is therefore

`M-1 + 1483042 = M+1483041`.

Alternatively it is at most `binom(177120,1193)+1483041`. This review reuses RP as an accepted source-certified word; it does not replay or newly certify all 1483042 literal prefixes. In particular, the separate recent rule-stage3 literal receipt does not discharge RP's other replay obligations. The newer eleven-stage height-2 construction remains a distinct alternative.

For a reachable member of this entry-profile family, the legal constructed tail preserves reachability. The accepted reachable all-lattice sticker theorem may then be applied to its actual labelled endpoint, adding 20892696 letters. Thus the sticker-solving bound is

`M-1 + 1483042 + 20892696 = M+22375737`.

This uses a state-specific lattice correction/retained schedule, not D1's fixed recorded final word. The latter would be invalid at an arbitrary changed labelled endpoint, as source Proposition B at line 302 explains. The inherited lattice-cost review has SHA256 `42f5e19afe1887074fbd3d9c53797eefe160fee3b265a346b0cb9d778320d450`; the separate correction-library height review has SHA256 `e6db2d841b6eda9c77bea402ae78d91fead6d3601e45592628289f294dc2f382`. Their source group, invariant, cascade and correction premises are reused here, rather than recomputed.

No height-2 claim is made for RP. SAFE Y preserves the initial height ceiling 2. Using only the existing I_a per-letter height change bound of at most 2 and RP's length gives the conservative whole-construction ceiling

`2 + 2*1483042 = 2966086`.

The appended lattice phase has its separately accepted ceiling 2, so it does not enlarge this conservative ceiling. The more favorable height-2 bound belongs to the independent eleven-stage alternative, not to this transfer-to-RP argument.

There are no scoped findings, counterexamples, or unresolved checks. The conclusion is a constructive finite upper bound on this precise valid off-geometry/profile family; a sticker-solving corollary additionally requires reachability. It proves neither universal C2 coverage, practical search complexity, complete expanded tail replay, nor exact canonical full-pose restoration. Candidate/source/main files stayed read-only; no network, installs, uploads, paid API, or follow-up research was used.
