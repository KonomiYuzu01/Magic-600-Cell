# Stage 14: the blocking structure of deep states (24 October 2026)

Status: **research, first round.** The owner changed the method of the height 4 to height 2 research on 24 October
2026 ([decision](../../../../docs/wiki/decisions/owner-decisions-2026-10-24-theorem-program.md)): each obstacle is
stated as a theorem with hypotheses that can be checked at a state, a construction with an explicit bound, and exact
certificates; computation tests hypotheses and checks certificates. This stage is the first problem: which grips are
blocked at deep states, by which pieces, and how a grip that a lowering needs can be freed without raising heights.
The problem card for the joint attack is [card 6](../../theory/cards/card-6-blocking-structure.md).

Labels: **proved** (with the proof here), **computed** (exact, by the script named), **lead**, **open**. Everything
is headless Linux research code on exact geometry; nothing here is Windows, DirectX, input or performance evidence.

## 1. Terms

Menu I_a; a piece's height h is the left tree distance of its pose (even; right distance 0), its branch (outer class)
the class whose centre is the first step of the geodesic from v₀ ([card 5](../../theory/cards/card-5-depth-and-lattice-group.md),
Lemma 5.1). Every grip c has a class CLS(c) (120 grips per class).

- A grip is **clean** when no piece straddles its cut (its twists are admissible), **blocked** otherwise; S_d is the set
  of straddlers of grip d, and I_c the set of pieces inside cap c.
- The **label** ℓ(A, d) ∈ {in, out, straddle} of piece A at grip d says where its posed region lies relative to H_d;
  the **holders** of a set of pieces are the grips whose caps hold every piece of the set whole.
- **Depth** Φ = Σ_A max(0, h(A) − 2). Φ = 0 is depth one.
- The **foreign load** F_x of grip x is the number of pieces inside cap x of height at least 2 whose branch is not
  CLS(x). A clean grip with F_x = 0 is **pure**.
- A **unit** is a set of pieces with one pose. A **high unit** has height at least 4. Its **own holders** are its
  holders of its own branch's class; only an own holder can lower it (card 5, Lemma 5.2).
- The **obstruction** of a grip x is S_x together with its foreign pieces.

## 2. Lemmas

**Lemma L (label calculus; proved).** Let c be a clean grip, k ∈ A4_c, and X′ = (c, k)X. Then:
1. a piece outside cap c keeps every label;
2. a piece inside cap c has ℓ_{X′}(A, d) = ℓ_X(A, k⁻¹d) for every grip d, where k⁻¹d is the grip with pole k⁻¹n̂_d
   (this is the retained conjugation of R22D-01);
3. heights do not change, and the branch of a moved off piece becomes σ_k(branch), where σ_k is the permutation of
   the five classes with CLS(k·e) = σ_k(CLS(e)).

*Proof.* (1) The piece does not move. (2) Its region R becomes kR, and n_d · ky = (k⁻¹n_d) · y = n_{k⁻¹d} · y because
k ∈ K⁺ permutes the poles; so kR lies inside, outside or across H_d exactly as R does for H_{k⁻¹d}. (3) The left
factor a of k is a unit icosian, which fixes v₀, so d(v₀, at) = d(v₀, t), and the first step of the geodesic to at is
a applied to the first step towards t. The menus satisfy Λ_{k·e} = kΛ_e k⁻¹ (for k = [a, b], n̂_{k·e} = a n̂_e b⁻¹), so
the left factor group of class CLS(k·e) is a L_{CLS(e)} a⁻¹, and the neighbour of v₀ it fixes is a·w_{CLS(e)}; hence
the class centre of the moved piece's branch becomes the centre of class σ_k(branch). ∎

So the dynamics of retained twists at clean grips is a permutation action on the label table, exact without geometry.
`retained_search.py` replays every word it reports on the exact simulator and rejects the word unless the replay
confirms the label calculus: every letter admissible, the target grip holding the unit at its height, and its straddler
count and foreign load as predicted. The ten words it reported at the seed 7304 stop and the three of the deeper search
pass (section 4).

**Corollary L1 (proved).** Under retained twists at clean grips:
1. every piece keeps its straddle number (how many cuts it straddles) and its number of holders;
2. the straddle incidence Σ_d |S_d| does not change; a retained twist only moves blocks from cut to cut:
   |S_d| after (c, k) = |S_d| − |I_c ∩ S_d| + |I_c ∩ S_{k⁻¹d}|;
3. two pieces with equal holder sets keep equal holder sets, and pieces that move together keep equal (or different)
   branches.

So retained twists alone never lower the straddle incidence. In the measured states it is between 849 (the D1 bulk
stop, with 1,193 pieces off) and 144,682, and at least 98,166 at every state of the 3000-twist lines (section 4).
`retained_search.py` checks point 2 at its target grips: after each replayed word the measured straddlers equal the
label calculus's prediction.

**Lemma Φ (one jumble; proved from card 5, Lemma 5.2).** Let x be a clean grip of class Q and h a jumble item of
Λ_x. The twist (x, h) changes heights as follows. Every lattice piece in the cap goes to height 2, so Φ does not
change. Every piece of another branch at height at least 2 rises by 2. A piece of branch Q falls by 2 if the second
vertex of its geodesic is h⁻¹v₀; otherwise its height does not change. Hence

  ΔΦ(x, h) = 2·(F_x − D_x(h)),

where D_x(h) counts the pieces in the cap of branch Q, height at least 4 and second vertex h⁻¹v₀. A pure grip's
twists never raise Φ, and a lowering twist at a pure grip lowers it.

**Lemma C (co-holding; proved).** A twist at grip c moves a piece exactly when c is one of its holders. So pieces with
equal holder sets are moved by the same twists. By Corollary L1 they keep equal holder sets under retained twists, so
only a jumble can separate them. If a high unit G has the same holders as a piece p of another branch at height at
least 2, then p is in the foreign load of every own holder of G, and a lowering of G after retained twists only
raises p in the same twist (the two stay co-held, and their branches stay different).

**Lemma T (the last jumble; proved).** If X = (c, h)Y with h a jumble item and CLS(c) = Q, then c is clean at X
(Proposition 2.2 of the theory draft). The pieces in its cap are:
- pieces that were lattice pieces, now in a frame of Q;
- pieces of branch Q that h did not lower;
- pieces of another branch that h raised, now of branch Q;
- pieces that h lowered from height 2 to 0;
- pieces that h lowered from height h′ ≥ 4 to h′ − 2 ≥ 2. Their new branch differs from Q: the image of the old
  third vertex is a neighbour of v₀ other than w_Q.

So c is pure unless the last twist lowered a high piece. Undoing it with (c, h⁻¹) changes Φ by exactly minus what
the twist changed.

## 3. Tools (`scripts/`)

| Script | What it computes |
| --- | --- |
| `blockmap.py` | Exact label table of every piece at every grip (cached in a `.npz` file, about 1.2 MB), then per grip: class, straddlers by height and branch, clean, pure, foreign load; the matrix M[c, d] of pieces inside c that straddle d; the exact effect of every retained twist at every clean grip on every straddler set (Corollary L1); clean covers of blocked grips; for every high unit its holders and own holders with their state. Checks: no lattice piece straddles; no height-2 piece straddles a cut of its own class (card 3, Theorem B); the table reproduces its digest. |
| `jamcheck.py` | For the own holders of every high unit (or named grips): the obstruction in units (pose groups), each unit's cell and its clean holders (pure own class, impure own class, other class) and blocked holders, and the foreign load. With `--moves`, every non-identity menu twist at the clean holders of the obstruction units is applied exactly and undone, and the change of Φ, the largest height, the off count, the straddlers and foreign load of the target and the units that land are recorded. |
| `retained_search.py` | Lemma L as a search: a beam search over words of retained twists at clean grips for an own holder of a high unit that is clean and pure, with cost straddlers + foreign load; every reported word is replayed exactly. |
| `coheld.py` | Lemma C: for every high unit, the foreign pieces with the same holder set (co-held) and those whose holder set contains the unit's. |

The label table takes 2 to 4 CPU minutes per deep state (one process); the rest runs in seconds to minutes. Commands
are in section 6.

## 4. Measured facts (exact)

Inputs are the runner's pose tables in `13-tutorial-runner/runs/dev/` (`results/sources.json` lists each table, its
SHA-256 and its digest): the D0 and D1 scrambles of 20 and 40 twists and the D1 bulk stop; for seed 7303 the
3000-twist scramble, the state after its 18,900 s bulk, the states after the first and third steps of the height
rule `r22d75`, and the end of its height phase; for seeds 7305 and 7304 the census stop, for 7305 also the state after
21,600 s of bulk from it, and the end of the height phase; and the ends of the height phases of seeds 7306, 7307 and
7302. The six height ends are the inputs of the HD2P/2 campaign. Summaries: `results/states.json`.

| State | Digest | Off | n2 | n4 | n6 | Clean grips | Pure | Straddle pairs (height 2 / ≥ 4) | Median straddlers of a blocked grip | High units | … with a clean own holder | … with a clean pure own holder |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| D0 7106 scramble (20 twists) | `9ad04d45…` | 51,828 | 51,828 | 0 | 0 | 255 | 255 | 58,140 (58,140 / 0) | 160 | 0 | 0 | 0 |
| D1 7101 scramble (40 twists) | `be08cfaa…` | 90,441 | 90,441 | 0 | 0 | 85 | 85 | 106,919 (106,919 / 0) | 200 | 0 | 0 | 0 |
| D1 7101 bulk stop | `7e128820…` | 1,193 | 1,193 | 0 | 0 | 578 | 565 | 849 (849 / 0) | 34.5 | 0 | 0 | 0 |
| 7303 scramble (3000 twists) | `ee1a56a8…` | 116,111 | 114,470 | 1,552 | 89 | 26 | 23 | 144,682 (140,437 / 4,245) | 248 | 42 | 14 | 9 |
| 7303 after 18,900 s of bulk | `516f427d…` | 96,817 | 95,559 | 1,169 | 89 | 25 | 17 | 105,673 (102,546 / 3,127) | 168 | 28 | 10 | 7 |
| 7303 after height step 1 | `6d72a29b…` | 98,096 | 96,838 | 1,171 | 87 | 25 | 15 | 107,756 (104,632 / 3,124) | 175 | 28 | 9 | 6 |
| 7303 after height step 3 | `65d33e72…` | 100,900 | 99,884 | 929 | 87 | 26 | 13 | 111,419 (108,924 / 2,495) | 185 | 22 | 3 | 0 |
| 7303 D2 height end | `d01dc91e…` | 100,650 | 99,634 | 929 | 87 | 26 | 13 | 112,321 (109,826 / 2,495) | 188.5 | 22 | 3 | 0 |
| 7305 census stop | `273eab69…` | 105,437 | 103,140 | 2,297 | 0 | 24 | 16 | 117,206 (111,880 / 5,326) | 198 | 58 | 27 | 17 |
| 7305 after 21,600 s of bulk | `736ea9e7…` | 101,937 | 99,640 | 2,297 | 0 | 25 | 18 | 112,162 (106,976 / 5,186) | 191 | 58 | 21 | 20 |
| 7305 D2 height end | `cc5ef7ec…` | 109,872 | 108,453 | 1,419 | 0 | 22 | 12 | 126,117 (122,909 / 3,208) | 214 | 42 | 6 | 0 |
| 7304 census stop | `7e2304b2…` | 95,569 | 94,384 | 1,185 | 0 | 41 | 34 | 98,166 (95,286 / 2,880) | 159 | 19 | 8 | 8 |
| 7304 D2 height end | `ac127dc3…` | 96,471 | 95,674 | 797 | 0 | 41 | 29 | 103,763 (102,003 / 1,760) | 169 | 14 | 3 | 0 |
| 7306 D2 height end | `ac4be82c…` | 110,315 | 108,360 | 1,955 | 0 | 26 | 8 | 128,899 (124,638 / 4,261) | 208.5 | 34 | 6 | 0 |
| 7307 D2 height end | `895b45de…` | 100,569 | 98,984 | 1,585 | 0 | 38 | 21 | 111,078 (107,147 / 3,931) | 185.5 | 28 | 5 | 0 |
| 7302 D2 height end | `2430f4cc…` | 103,484 | 102,750 | 734 | 0 | 37 | 28 | 110,769 (109,318 / 1,451) | 189 | 21 | 6 | 0 |

Checks held at every state: no lattice piece straddles a cut, and no height-2 piece straddles a cut of its own
branch's class. The blocked count at `ac127dc3…` (559) equals the HD2P/2 campaign's Bk there, and the best retained
twist found by `blockmap.py` is at grip 23, the grip of HD2P/2's first PEEL step.

**What the table shows.**
- Deep states are jammed: 22 to 41 of 600 grips are clean along the 3000-twist lines. Almost all blocking comes from
  height-2 pieces, which lie in other classes' frames (95.4% to 98.7% of the straddle pairs); the rest comes from
  high pieces.
- The height rule uses up the lowerings. At the 3000-twist scramble, the census stops and after the bulk, 6 to 20
  high units have a clean pure own holder; after the first steps of the height rule, and at every height end, none.
- After its bulk, the depth-40 line is almost free: 578 clean grips and no piece above height 2. At depth 3000 the
  bulk changes the picture little in 5 to 6 hours.

**Anatomy at the 7304 stop** (`results/s7304-jam.json`, `results/s7304-coheld.json`). The 14 high units have 13 own
holders between them, one or two per unit. One of them is clean (540, foreign load 111, all of branch 1). The others
have:
- 26 to 299 straddlers in 2 to 14 units, mostly cell contents of other classes' frames of 78 to 197 pieces, or parts
  of large frame domains of 993 to 2,762 pieces;
- foreign loads of 78 to 1,552 pieces.

Unit by unit, the obstruction of an own holder is of three kinds. Some of it is held by a clean pure grip of its own
class: at 522 all of it, 127 pieces of branches 1 and 3 held by the clean pure grips 451 (class 1) and 344 (class 3);
at 64 all 885 foreign pieces and 95 of the 136 straddlers. Some is held only by clean grips of other classes or by
impure ones. And some is held by no clean grip at all: at 2 and 183 the whole obstruction, at 596 almost all of it.
Some straddlers are high units of the own branch that straddle the own holder of another unit (1007 at 64; 1247 and
1378 at 596). No high unit has a co-held foreign piece (Lemma C); unit 2304 has 25 foreign pieces whose holder sets
contain its own.

**Retained preparations** (Lemma L; `results/s7304-retained.json`, `results/s7304-retained-deep.json`). The beam
search (depth 6, beam 6) lowers the cost straddlers + foreign load of the best own holder for 10 of the 14 units, from
111 to 559 to between 12 and 322, with words of 1 or 2 retained twists, and finds no improvement for the other 4.
Every word was replayed exactly, and the replayed straddler count and foreign load equal the label calculus's
(`results/s7304-retained-check.json`, `results/s7304-retained-deep-check.json`: `--check` of both outputs). For
unit 1557 the best own holder becomes pure, with 12 straddlers left; for units 1132, 1582 and 2559 at 522, 2
straddlers and 17 foreign pieces are left. A deeper search (depth 14, beam 30) finds nothing better for three units.
No unit reached a clean pure own holder.

**Single letters at the obstruction's holders** (`jamcheck.py --moves`; `results/s7304-moves-522.json`). At 522 all
118 non-identity letters at 451 and 344 leave Φ unchanged (both grips are pure). The best leaves 6 straddlers and 27
foreign pieces (a retained letter at 451). The letters that land obstruction units take the lattice pieces of their
caps into frames, and some of those re-enter cap 522: (451, 43) lands three foreign units and leaves 4 straddlers and
36 foreign pieces, and the letters that land the 87-piece unit 1910 leave at least 28 straddlers.

**HD2P/2** (pull request 131: literal words of at most two letters, SINGLE, CL2 and PEEL; Bk the global blocked count).
It ended STUCK at all six D2 height ends. The independent audits passed at 7302, 7303, 7304, 7306 and 7307; the audit
at 7305 was still running when this page was written.

## 5. Reading

- The jam is not a lack of moves in general (there are always clean grips: the last twisted grip is one, Lemma T). It
  is the state of the grips that a lowering needs. The high units still left are buried under layers of other classes'
  frames, in their own holders' caps (foreign load) and across their cuts (straddlers).
- Retained twists cannot remove those layers: they only move blocks and frames between cuts and caps (Corollary L1),
  and the measured preparations stop above zero.
- A layer of height-2 frames is invisible to Φ. Removing it needs jumbles of that layer's class. They are Φ-neutral
  at pure grips of that class (Lemma Φ), but a landing takes the lattice pieces of its cap into frames, and those can
  re-enter the cap that is being freed.

These are leads for the card's questions, not results.

## 6. Reproduction

From the repository root, with one thread per process (`OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1`),
`S=research/jumbling/restoration/14-blocking-structure/scripts`, `T` a table from `results/sources.json` and `D` a
directory outside the repository:

    python3 -I -B $S/blockmap.py $T $D/state.json --labels $D/state.npz --name <name>
    python3 -I -B $S/jamcheck.py $T $D/state.npz $D/jam.json
    python3 -I -B $S/jamcheck.py $T $D/state.npz $D/moves-522.json --caps 522 --moves
    python3 -I -B $S/retained_search.py $T $D/state.npz $D/retained.json --depth 6 --beam 6 --seconds 150
    python3 -I -B $S/retained_search.py $T $D/state.npz $D/retained-deep.json --depth 14 --beam 30 --seconds 600 --groups 1557,1132,2234
    python3 -I -B $S/retained_search.py $T $D/state.npz $D/retained-check.json --check $D/retained.json
    python3 -I -B $S/coheld.py $T $D/state.npz $D/coheld.json

The first command writes the label cache that the others read. `results/states.json` holds the `summary` and `high`
blocks of the first command's output for each state, and `results/s7304-end-caps.json` its per-grip rows at the 7304
stop; the other files in `results/` are the outputs of the other commands at the 7304 stop
(`d2-s7304/component/tables/00005.json`). Label caches stay outside the repository.
