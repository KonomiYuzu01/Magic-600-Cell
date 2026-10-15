# Card-1 certificates and the lattice-segment reference restorer (items 2 and 3)

Date: 13 October 2026. Evidence kind: exact source and fixture computation on the retained model (`assets/model.npz`, `assets/primitives.npz`, `assets/seed_atlas.json`, `assets/execution_trees.json.gz`) and the exact jumbling simulator, headless on Linux. Nothing here is Windows, DirectX, input or performance evidence.

## Summary

- **Item 2 is complete.** Explicit words for all 35 certificates P_o and all 11 certificates F_o of the accepted card-1 proof were constructed and pass an independent checker. With the accepted card-1 theorem (certificates and F1 imply G = 𝒦), the retained group equals its invariant bound, **without hypothesis C**. Its order is |G| = ∏_o N_o! |H_o|^{N_o} / (2 |H_o^ab|), a number of 598,061 decimal digits (about 4.99 × 10^598060); the index of G in the full wreath product is 2^43 · 5^2.
- **Item 3 (lattice segment): the restorer is built and verified; its library is not yet complete.** The restorer reads only the current configuration. It computes ν, cancels ν with a fixed library of I_a correction macros, and then solves the retained part with the cascade. An independent checker confirms every result.
  - **Restored (8 inputs):** random elements of 𝒦, retained scrambles, and scrambles with up to 20 planted macros.
  - **Exact failure certificates (3 inputs):** the three reachable deep tables E3, I1 and I2. Their ν lies outside the library span.
  - **Library:** 13 independent macros, 5 to 332 twists long.
    - Eleven of them, w9 among them, are mirrored half-turns of 5 or 9 twists.
    - One is a 228-twist macro that changes only a frame class (O11).
  - **Answers to card 5:** questions 3.2, 3.3 and 3.4 are all answered **No** (section 3.5). Question 3.1, completeness of the library, stays open.

## 1. Conventions

Words are signed primitive ids ±1..±1200, applied left to right (chronological). A permutation maps source slot to destination slot. A labelled state is an array L with L[slot] = home slot of the sticker now in that slot; a move (src, dst) acts by L'[dst] = L[src]. The conjugate u⁻¹ x u carries the support of x forward along u, as in card 1.

**Coherent coordinates.** For each orbit o the execution tree of `assets/execution_trees.json.gz` has buffers A_o, B_o, a third position C_o with reference fibre f_c (an ordered list of the k_o slots of C_o), and a node for every pair (position, frame). The fibre of A_o and B_o is the atlas's f_a and f_b. For every other position X the fibre is the image of f_c under the tree path to the first node at X. This is the retained theory's construction (Lemma 7: a legal transporter from the reference position to every other position). By rigidity (check R1 below), every legal word acts on each piece by an element of K+; hence the return map of any legal word at C_o lies in the image H_o of Stab_K+(C_o) on the stickers of C_o, and G ⊆ H_o ≀ S_{N_o} in these coordinates.

## 2. Item 2: the certificates

### 2.1 Construction

**P_o.** The atlas seed of each orbit is a legal word whose action on orbit o is a three-cycle of pieces with c³ = 1 on the fibres (frame-preserving in these coordinates). For 12 orbits (O01, O04, O05, O07, O18, O19, O20, O27, O30, O31, O32, O33) the seed moves nothing else. For the other 23 the seed also moves other orbits ("collateral"). The collateral graph is acyclic, so the orbits can be ordered so that each seed's collateral lies on orbits already done:

`1, 4, 5, 7, 18, 19, 20, 27, 30, 31, 32, 33, 3, 12, 14, 16, 24, 26, 28, 34, 10, 11, 13, 25, 29, 9, 23, 8, 21, 22, 2, 15, 17, 0, 6`.

For each orbit in this order, the collateral of its seed on every earlier orbit q is an element of ker ν_q (F1), and it is undone exactly by the solver of section 3.2 run on orbit q with q's pure stars, pairs and singles. The product seed · cleanup is pure on o; conjugated by the atlas relocation it becomes c_o, the frame-preserving cycle (A_o B_o C_o). The setup u_X for every X ∉ {A_o, B_o, C_o} is the tree path to the first node at X. Tree paths never use a grip in the signature of A_o or B_o, so they fix every sticker of both.

**F_o.** For each tree node n at position C_o with a non-reference frame x, the word g_n (its tree path) fixes A_o and B_o and returns C_o with frame x. The word Q_n = (c_o^{g_n}) c_o⁻¹ is pure on o and supported on B_o and C_o, with value x at B_o and its inverse partner at C_o. Then:

- C2 and C5: one Q_n has the exact form (s, s), respectively (r, r⁻¹).
- D5 (O06): a product of two Q's gives two distinct reflections (t, s). With r := t s (t first, then s), this is the card's (rs, s) with r of order 5, and s r s = r⁻¹ is checked.
- A5 (O34): products of Q's on two positions always carry conjugate values of equal order, so instead two single-position elements were built as commutators [Q, Q'] of pairs that share only C_o. One of them, transported to B_o by c_o⁻¹, gives f = (a at B_o, b at C_o) with a of order 2, b of order 3 and ⟨a, b⟩ = H_34. The checker builds the explicit isomorphism H_34 → A5 with a ↦ (1 2)(3 4) and b ↦ (1 3 5) and verifies that it is a bijective homomorphism on all 60 × 60 pairs.

All certificates are stored as one straight-line program (`certificates/certificates.json`, 4,788 nodes: `word`, `conj`, `prod`). Its expansion is an explicit finite word in the 1,200 generators. Evaluating the program by composing complete permutations is exactly evaluating that word, because the map from words to permutations is a homomorphism.

### 2.2 Independent checker (`checker/check_certs.py`)

The checker uses only NumPy and the repository assets and shares no code with the constructor. All checks are exact integer computations on the 259,800 slots.

| Check | Statement | Result |
|---|---|---|
| R1 | Each of the 1,200 primitives moves each piece of its cap rigidly by its pole permutation g: sticker (p, f) goes to (q, g f) with sig(q) = g(sig(p)); unlisted stickers of a cap piece are fixed by g in place; pieces outside the cap do not move | 0 failures |
| R2 | The 1,200 pole permutations lie in one group of order 7,200 | order 7,200 |
| H | For every orbit, Stab_K+(C_o) acting on the fibre of C_o has the order and element-order profile of the card's H_o (1, C2, C5, D5, A5) and stabiliser order 7200/N_o | 35/35 |
| P | c_o's complete permutation is exactly f_a → f_b → f_c → f_a and the identity on every other sticker; every other position has exactly one setup; each setup fixes all stickers of A_o and B_o and sends f_c onto the stickers of X, matching the recorded tree frame | 35/35 |
| F | f_o fixes every sticker outside two pieces, fixes both positions, and has exactly the card's frame values (with the explicit A5 isomorphism) | 11/11 |
| F1 | In the same coordinates, ν_o(generator) = 0 for all 1,200 primitives and all 35 orbits (sign and H^ab class) | 0 failures |

`checker/selftest.py` alters the certificate file four ways (one move dropped from a seed word, two setups swapped, a frame word replaced, a setup removed); the checker rejects each. The checker fails closed: any exception is reported as FAIL.

### 2.3 Results by orbit

| Orbit | N | H | seed collateral orbits | cleanup factors | c_o length | setups | max setup depth | F_o length |
|---|---:|---|---:|---:|---:|---:|---:|---:|
| O00 | 1200 | C2 | 32 | 984 | 243,763,786 | 1197 | 20 | 487,527,582 |
| O01 | 3600 | 1 | 0 | 0 | 2,212 | 3597 | 12 | — |
| O02 | 2400 | 1 | 19 | 189 | 9,981,150 | 2397 | 10 | — |
| O03 | 3600 | C2 | 2 | 8 | 5,642 | 3597 | 10 | 11,290 |
| O04 | 7200 | 1 | 0 | 0 | 562 | 7197 | 8 | — |
| O05 | 7200 | 1 | 0 | 0 | 562 | 7197 | 8 | — |
| O06 | 720 | D5 | 27 | 928 | 176,017,672 | 717 | 9 | 704,070,698 |
| O07 | 7200 | 1 | 0 | 0 | 298 | 7197 | 8 | — |
| O08 | 3600 | C2 | 15 | 96 | 1,527,208 | 3597 | 7 | 3,054,422 |
| O09 | 7200 | 1 | 7 | 28 | 149,890 | 7197 | 7 | — |
| O10 | 7200 | 1 | 5 | 20 | 18,712 | 7197 | 7 | — |
| O11 | 3600 | C2 | 4 | 20 | 9,202 | 3597 | 6 | 18,410 |
| O12 | 7200 | 1 | 1 | 4 | 1,030 | 7197 | 6 | — |
| O13 | 7200 | 1 | 2 | 8 | 3,288 | 7197 | 6 | — |
| O14 | 7200 | 1 | 2 | 10 | 3,108 | 7197 | 6 | — |
| O15 | 2400 | 1 | 22 | 540 | 6,417,246 | 2397 | 6 | — |
| O16 | 7200 | 1 | 1 | 2 | 620 | 7197 | 6 | — |
| O17 | 1440 | C5 | 19 | 585 | 4,180,480 | 1437 | 6 | 8,360,964 |
| O18 | 7200 | 1 | 0 | 0 | 1,546 | 7197 | 6 | — |
| O19 | 7200 | 1 | 0 | 0 | 152 | 7197 | 6 | — |
| O20 | 7200 | 1 | 0 | 0 | 152 | 7197 | 6 | — |
| O21 | 7200 | 1 | 5 | 16 | 28,920 | 7197 | 6 | — |
| O22 | 3600 | C2 | 9 | 68 | 133,602 | 3597 | 6 | 267,210 |
| O23 | 7200 | 1 | 4 | 10 | 3,758 | 7197 | 6 | — |
| O24 | 7200 | 1 | 1 | 2 | 272 | 7197 | 6 | — |
| O25 | 3600 | C2 | 3 | 8 | 1,252 | 3597 | 5 | 2,508 |
| O26 | 7200 | 1 | 1 | 2 | 266 | 7197 | 6 | — |
| O27 | 7200 | 1 | 0 | 0 | 104 | 7197 | 6 | — |
| O28 | 1440 | C5 | 3 | 30 | 7,428 | 1437 | 5 | 14,860 |
| O29 | 2400 | 1 | 6 | 42 | 17,028 | 2397 | 5 | — |
| O30 | 7200 | 1 | 0 | 0 | 104 | 7197 | 5 | — |
| O31 | 7200 | 1 | 0 | 0 | 152 | 7197 | 6 | — |
| O32 | 3600 | C2 | 0 | 0 | 106 | 3597 | 6 | 216 |
| O33 | 2400 | 1 | 0 | 0 | 56 | 2397 | 5 | — |
| O34 | 120 | A5 | 1 | 28 | 13,992 | 117 | 5 | 363,828 |

Lengths are expanded word lengths in primitives. The longest certificates (O00, O06) are machine-scale because the cascade cleanup nests stars of long seeds; this does not affect validity. Shorter pure seeds (for example by isolating commutators) were not attempted.

### 2.4 Consequence

The accepted card-1 proof (Theorem 8) states: under F1, P_o on every orbit and F_o on every nontrivial-frame orbit imply G = 𝒦. All 46 certificates hold, and F1 was rechecked in the same coordinates. Hence **G = 𝒦 = ∏_o ker ν_o**, unconditionally. Hypothesis C is no longer needed: its nine orbits are covered by explicit P_o certificates like every other orbit. In particular every lattice configuration with ν = 0 is a retained state, and the restorer below can solve any of them.

## 3. Item 3: the lattice-segment reference restorer

### 3.1 Algorithm

Input: one configuration Y, as an exact pose table (or, for synthetic retained inputs, as labelled stickers). Nothing else is read: no journal, no generation word, no record of earlier restoration steps.

1. **Lattice test.** Every piece must have a pose in K+. Otherwise the restorer stops with "not a lattice configuration" (that input belongs to an earlier segment).
2. **Coordinates and ν.** The labelled stickers of Y are read in the coherent coordinates of section 1, and ν(Y) = (ν_o(Y))_o is computed: 35 position signs, 8 frame classes in C2 (the seven C2 orbits and the D5 orbit O06, whose H^ab is C2) and 2 frame classes in C5 (O17, O28). As a vector: 43 coordinates over F2 and 2 over F5.
3. **Correction.** The correction library is a fixed list of I_a words M_1, …, M_r, each admissible from solved and ending on the lattice, each stored with its ν. The restorer solves Σ n_i ν(M_i) = −ν(Y) by elimination over F2 and over F5 (combined by the Chinese remainder theorem; each multiplicity is reduced modulo the order of ν(M_i), and a negative multiplicity uses the inverse word). It applies the macros in the exact simulator. If no solution exists, it stops with a **failure certificate**: ν(Y), the library rank, and the residual left after elimination.
4. **Retained solve.** After the correction ν = 0, so the labelled state lies in 𝒦 = G (section 2.4). The cascade solves the 35 orbits in the reverse of the construction order of section 2.1 (`6, 0, 17, 15, 2, 22, 21, 8, 23, 9, 29, 25, 13, 11, 10, 34, 28, 26, 24, 16, 14, 12, 3, 33, 32, 31, 30, 27, 20, 19, 18, 7, 5, 4, 1`) with the raw relocated atlas seeds, whose collateral lies only on orbits solved later. Each orbit is solved by the buffer method: positions by the three-cycles (A X Y) = t_X⁻¹ t_Y with t_X = u_X⁻¹ c u_X, then frames by the transported pairs Q_n, with a final commutator single at C. The output is a straight-line program over the 1,200 retained generators.

The correction words are I_a words; the retained solve uses only retained twists. Both are applied to Y itself.

### 3.2 Correctness

**Theorem R.** For every configuration Y the restorer returns exactly one of:
(a) a word w (corrections, then the retained program) with Y·w = solved;
(b) "not a lattice configuration";
(c) a failure certificate showing that ν(Y) is not in the span of ν(M_1), …, ν(M_r).
If the library spans 𝒜_J := ν(G_J), then (c) never happens for a reachable lattice configuration.

*Proof.* Each M_i was replayed from solved: every twist admissible and in the I_a menu, the end on the lattice. By card 2, Lemma 2 (the decorated shape decides admissibility; card 5, fact S), M_i is admissible from every reachable lattice configuration and acts on all of them by one fixed relabelling ρ(M_i), so ν(Y·M_i) = ν(Y) + ν(M_i) (ν is a homomorphism, F1). After the correction ν = 0, so the relabelling lies in 𝒦, which equals G by section 2.4. The cascade is an explicit retained word; the restorer asserts the identity on all 259,800 stickers after it, and the checker of section 3.3 confirms it independently. If Y is reachable and on the lattice, Y = solved · u for a lattice-returning word u, so ν(Y) = ν(ρ(u)) ∈ 𝒜_J; when the library spans 𝒜_J, step 3 has a solution. □

So the restorer is complete exactly when its library spans 𝒜_J. That is card 5, question 3.1, which is open; section 3.5 gives what is known.

### 3.3 Implementation and independent check

- `scripts/restore_lattice.py` is the restorer. It builds the cascade kits once (from the raw seeds and the atlas relocations), checks each library macro by replaying it from solved and recomputing its ν with its own code, and writes a summary, the solutions as one straight-line program, and the start labels of each input.
- `checker/verify_restore.py` shares no code with the restorer. For each restored run it rebuilds the start configuration from the input specification (exact simulator for tables and words, stored labels for synthetic inputs), replays every recorded correction twist by twist (each in the I_a menu, each admissible, each macro ending on the lattice, the result matching the recorded digest), evaluates the solution program with its own evaluator and applies it: the result must be the identity. For each failure certificate on a pose table it recomputes ν with the separate coordinate code of `nucoords.py`, compares it with the restorer's ν, replays every library macro from solved, and checks by rank over F2 and F5 that ν(Y) is outside the library's span.

### 3.4 Results

Library `restore/library-final.json` (13 macros, section 3.5). Restorer output: `restore/restore-summary.json`; independent check: `restore/verify-report.json` (pass: 8/8 restored inputs return to the identity with matching start and length, 3/3 failure certificates confirmed; 130 s).

| input | ν coordinates ≠ 0 | moved pieces | corrections | result | retained length | s |
|---|---:|---:|---|---|---:|---:|
| `uniform:1` | 0 | 176,499 | — | restored | 229,763,734 | 12.7 |
| `uniform:2` | 0 | 176,492 | — | restored | 230,484,810 | 2.6 |
| `word:3:300` | 0 | 169,401 | — | restored | 204,360,248 | 5.8 |
| `w9word:4:300` | 2 | 170,561 | M0 | restored | 206,596,100 | 8.6 |
| `w9word:5:0` | 2 | 1,634 | M0 | restored | 0 | 1.3 |
| `mix:6:200:6` | 14 | 164,541 | M5, M7, M9, M10 | restored | 194,793,196 | 10.2 |
| `mix:7:300:10` | 5 | 172,435 | M1, M4, M6, M10 | restored | 210,808,310 | 38.7 |
| `mix:8:100:20` | 19 | 156,735 | M2, M4, M5, M6, M7, M9 | restored | 185,296,322 | 42.1 |
| table: I1 end | 17 | 175,175 | — | failure certificate | — | — |
| table: I2 end | 16 | 175,399 | — | failure certificate | — | — |
| table: E3 | 16 | 175,041 | — | failure certificate | — | — |

- `uniform:<seed>`: a uniformly random element of 𝒦 given as labelled stickers (nearly all of the 176,520 moving pieces displaced). It needs no correction; it tests the cascade on G = 𝒦.
- `word:<seed>:<n>`: n random retained twists in the exact simulator. `w9word`: the same followed by w9; `w9word:5:0` is w9 alone.
- `mix:<seed>:<n>:<k>`: n random retained twists with k random library macros or their inverses inserted at random places (6, 10 and 20 planted). These exercise the correction step with every kind of macro; the restorer is not told which macros were planted and picks its own combination from ν alone.
- Tables: the end tables of I1 and I2 (three-block family package) and the E3 table (closure chain package). All three are reachable lattice configurations. Their failure certificates are exact and confirmed by the checker (section 3.5).
- The retained length is the expanded length of the cascade word in primitives; corrections add 5 to 332 twists per macro. The time is one run each, on Linux.

### 3.5 The correction library and the lattice correction group 𝒜_J

**Library.** Every macro is admissible from solved and ends on the lattice; its ν was computed from the end state and recomputed by the restorer and by the checker.

| id | family | twists | ν |
|---:|---|---:|---|
| M0 | w9 (card 2) | 9 | signs O08, O25 |
| M1 | mirrored half-turn, one-twist setup | 5 | signs O08, O11, O22, O25, O34 |
| M2 | mirrored half-turn, one-twist setup | 5 | sign O34 |
| M3 | mirrored half-turn, two-twist setup | 9 | signs O13, O15, O16, O19, O20, O22, O23, O26, O27, O28, O31, O32, O33, O34 |
| M4 | mirrored half-turn, two-twist setup | 9 | signs O08, O09, O11, O14, O16–O21, O23–O31 |
| M5 | mirrored half-turn, two-twist setup | 9 | signs O10, O12, O14, O17, O18, O21, O22, O23, O25, O26, O27, O28, O31, O33 |
| M6 | mirrored half-turn, two-twist setup | 9 | signs O10, O12, O14, O15, O17, O19, O23, O24, O29, O30, O32, O33, O34 |
| M7 | mirrored half-turn, two-twist setup | 9 | signs O07, O08, O10, O15, O17–O28, O31, O33, O34 |
| M8 | mirrored half-turn, two-twist setup | 9 | signs O08, O16, O18, O19, O20, O22, O23, O26, O27, O28, O31, O32, O33, O34 |
| M9 | mirrored half-turn, two-twist setup | 9 | signs O20, O21, O22, O23, O24, O29, O30, O32, O33 |
| M10 | mirrored half-turn, two-twist setup | 9 | signs O12, O15, O16, O18, O20, O22, O23, O26, O27, O28, O31, O32, O33, O34 |
| M11 | difference of two E2 endings (W3 → I1) | 228 | frame class 1 on O11 |
| M12 | difference of two E2 endings (W3 → A5) | 332 | sign O12; frame class 1 on O11 |

(O16–O21 means O16 to O21 inclusive.) The 13 vectors are independent over F2; the span has 2^13 elements and no F5 part.

**Where the macros come from.**

- *Mirrored half-turn conjugates* (card 2, Lemmas 15 and 16; `scripts/search_mirror.py`). w = (v v^a)(16, a)(v v^a)⁻¹, with a a retained half-turn of A4_16, v one or two jumble twists near grip 16, and v^a the image of v under the global rotation a. w9 is itself of this form. One-twist setups: all 6,912 candidates for one half-turn were replayed; 6,624 end on the lattice, with 3 independent ν. The other two half-turns of A4_16 are conjugate to it under the third-turns of A4_16, which fix grip 16, so they give the same ν values. Two-twist setups, random (seeds 8 and 9): 15,477 and 12,918 candidates, of which 10,401 and 8,648 end on the lattice; ranks 10 and 11. All mirrored macros together, w9 included, have F2 rank 11. As card 2 Lemma 16 predicts, every mirrored macro has zero frame part.
- *Differences of two endings of one start* (`scripts/diff_macro.py`). If E·a and E·b are both lattice configurations, then D = a⁻¹ b starts on the lattice and ends on it, so by card 2, Lemma 2 it is admissible from solved and is a fixed correction macro. E need not be on the lattice or reachable. The start E is the E2 table of the closure chain, which has three pieces off the lattice (the block B′: one piece of O11 and two of O12). Fourteen endings were used: W3 and the S-type word of the closure package; I1 and I2 of the three-block package; and ten new endings A1–A5 and B1–B5, made with that package's generator (`gen3.py`, SAFE-ALL and setup modes) and constructor (`constr3.py`). Relative to W3 the differences have ν ∈ {0, frame O11, frame O11 + sign O12}. These are exactly the ways B′ can land: the frame of its O11 piece and the order of its two O12 pieces. D(I1 → I2), 514 twists, also has ν = frame O11.
- *Searches that found nothing new.* The card 5 C3 probe (139 conjugated half-turns after w9's setup) gave only ν(w9). Lens words and random chains (`search_macros.py`) gave only 0, ν(w9), M1 and ν(M1)+ν(w9). The shape-collision search (`search_collide.py`) is a direct sample of card 2, Lemma 3's edge returns: it follows random walks at grip 16 and three grips 35° to 50° away, and closes a word whenever two walks reach one decorated shape. It explored 6,341 shapes and closed 7,601 returns, all with ν = 0.

**What this says about 𝒜_J (card 5, question 3).**

- *Lower bound.* 𝒜_J contains the span of the library (order 2^13) and ν(E3), because E3 is reachable through the recorded closure chain. Together they give a subgroup of order 2^14 · 5. The ambient group 𝒜 has order 2^43 · 5^2. Position signs on O00 to O06 are zero on every macro and every reachable lattice configuration examined; this is not proved.
- *Q3.2: is the sign of O08 always equal to the sign of O25?* **No.** M8, nine twists, is odd on O08 and even on O25. The reachable E3 also separates them: O25 odd, O08 even.
- *Q3.3: is every frame component of ν zero on G_J?* **No.** M11, 228 twists, changes only the frame class of O11. The reachable E3 has nonzero C2 frame classes on O22, O25 and O32, and C5 class 2 on O17.
- *Q3.4: do conjugated half-turns generate 𝒜_J?* **No.** By card 2, Lemma 8 (accepted), every conjugated half-turn u(d,a)u⁻¹ has zero C5 coordinates. Conjugated third-turns have ν = 0 (card 2, Proposition C). So the subgroup they generate has zero C5 part, but ν(E3) has C5 class 2 on O17 and E3 is reachable.
- *Q3.1: determine 𝒜_J.* Still open. ν(E3), and so ν(I1) and ν(I2), lies outside the library span. The residual after elimination is the same for all three: signs on O14, O21, O22, O27, O28 and O30; C2 frames on O22, O25 and O32; C5 on O17. No short family found so far reaches those frame parts:
  - mirrored setups have zero frame part (Lemma 16);
  - conjugated half-turns have zero C5 part (Lemma 8);
  - the E2 ending differences only move the frame of B′'s single O11 piece.
  The only word known to realise ν(E3) is E3's own recorded history. That runs from the I_a@40 fixture through the owner's 36,563-twist prefix and the X → E1 → E2 → E3 chain, tens of thousands of twists. It was not assembled into a macro here.
- *Upper bound.* None is known beyond 𝒜. Card 5, question 3.1 asks for one.

### 3.6 Limits and checks not run

- **Library completeness is open** (card 5, question 3.1). The restorer is complete only for configurations whose ν lies in the span of its library, and it certifies every other input exactly. E3, I1 and I2 are reachable and lie outside the span (section 3.5); the only word known to realise their ν is their recorded history of tens of thousands of twists, spread over several packages, which was not assembled into a macro here.
- The cascade words are machine-scale (about 1.9 × 10^8 to 2.3 × 10^8 primitives). This is a reference restorer and a correctness oracle, not a human method; the human method is the orbit-first block building of the solving workflow.
- Only the lattice segment is covered. The depth-one segment waits for the 3E freeze; the depth-reduction segment waits for card 5, question 2.
- Not run: Windows, DirectX, native-host regressions, any performance measurement. All results are exact Linux headless computations on the retained model and the exact jumbling simulator (synthetic geometry), each timed once.


## 4. Reproduction

Run from the package root with the repository at `$ROOT` (main checkout; only `assets/` is read) and the jumbling research tree at `$J` (`research/jumbling` on branch `claude/jumbling-e3`, commit `27a001c`). Use Python 3 with NumPy, run as `python3 -I`.

```
# item 2
python3 -I scripts/construct.py scripts $ROOT <out dir>        # writes <out dir>/certificates.json
python3 -I checker/check_certs.py $ROOT certificates/certificates.json certificates/check-report.json
python3 -I checker/selftest.py $ROOT certificates/certificates.json <work dir>

# item 3: library (searches, then merge), restorer, checker
python3 -I scripts/search_mirror.py scripts $ROOT $J <out> 2400 8 -16          # two-twist setups; '16' alone also runs the one-twist stage
python3 -I scripts/diff_macro.py scripts $ROOT $J <out.json> W3=<word>... I1=<word>+<word>...
python3 -I scripts/build_library.py scripts restore/library-final.json <search libraries...>
python3 -I scripts/restore_lattice.py scripts $ROOT $J restore/library-final.json <out dir> uniform:1 uniform:2 word:3:300 w9word:4:300 w9word:5:0 mix:6:200:6 mix:7:300:10 mix:8:100:20 table:<I1 end> table:<I2 end> table:<E3>
python3 -I checker/verify_restore.py $ROOT $J restore/library-final.json <out dir> <report.json>
```

The exact argument lists of every run are in `restore/commands.txt`. The pose tables E3, I1 end and I2 end come from the closure-chain package (`D3/E3-table.json`) and the three-block package (`runs/I1|I2/end-table.json`). `scripts/run_endings.sh` regenerates the ten endings A1–A5 and B1–B5 with that package's `gen3.py` and `constr3.py`.

## 5. Package contents

| path | content |
|---|---|
| `certificates/` | item 2: the certificate program, checker report, construction log, seed survey, selftest log |
| `restore/library-final.json` | the 13 correction macros (exact twist records and ν) |
| `restore/restore-summary.json`, `restore/verify-report.json`, `restore/*.log` | restorer summary and independent check |
| `restore/solutions.json` | the retained solutions as one straight-line program |
| `restore/start-labels-*.npy` | labelled start state of each synthetic input (the checker rebuilds and compares them) |
| `restore/commands.txt` | exact argument lists of every run |
| `restore/searches/` | search logs and libraries of every family, the ending difference macros, and the shape-collision statistics |
| `scripts/` | constructor, coordinates, restorer, searches, difference macros, library merge, ending generation (`run_endings.sh`) |
| `checker/` | certificate checker, its selftest, restoration checker |

`SHA256SUMS` lists every file of the package. Absolute paths in the logs are replaced by `$ROOT`, `$P28`, `$P30` and `<work>`.
