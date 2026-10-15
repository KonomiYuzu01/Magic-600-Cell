# 05. The seed table and the conflict graph (14 October 2026)

**The question.** Can the pieces that are off the lattice be put back one at a time, from the current state only,
with words taken from a table? The table holds *seeds*: words that, from the solved state, leave exactly one piece
off the lattice and one hole. Moved by an element of K⁺ to the position of a matching off piece, the inverse of a
seed puts that piece back.

The main document is [PROOFS-3E.md](PROOFS-3E.md): theorems, proofs, the table, the measured coverage and the list
of certificates.

Labels:
- **proved**: the author's proof, with no independent review;
- **certificate**: a finite exact computation, with its file and script;
- **float evidence**: floating point, without an exact check of the negative cases;
- **conditional**: the condition is named;
- **open**.

## Results

| Item | Status |
| --- | --- |
| Restoration theorems: Lemma R (relative replay), Theorem C (one seed), Lemma S (SAFE preparation), Theorem C′ (one piece at a time along an acyclic conflict graph), Theorem K (k-seeds) | **proved**. The preparation that Theorem C′ needs is checked per instance |
| Lemma G and Corollary G: a glued block leaves all at once, so no single seed restores a piece of it | **proved** |
| Seed table: 252 seeds in 76 of the 500 position classes, every seed replayed from solved | **certificate**, partial. Every class that occurs in X, E1, E2, I1 and I2 (34 classes) has a seed |
| Coverage on frozen reachable inputs | **certificate**: E2, I1 and I2 restored; E1 loses its 3-block and keeps 87; X, e2run24 and stop62 stop at glued blocks |
| E3 reachable from solved | **certificate**: one continuous replay of 39,392 admissible twists |
| Block lemma: in every reachable depth-one state some block can be restored as a whole | **open** |

### Position classes

A piece is *depth one* when one twist of the frame γ (the first jumble twist at grip 0) separates its pose from
the lattice. Its region is then fixed by a slot s, its *position*. The positions fall into 500 classes under the
symmetries that keep γ's frame: 488 classes of 360 positions and 12 of 120 (CERT-STRUCT). One seed covers exactly
one class. K⁺ permutes the five grip classes of 120 grips transitively, so a seed made at one grip class serves all
five. Every off piece in every input and sample of this stage is depth one.

### From single pieces to blocks

- **Theorem C′.** When the conflict graph of the off pieces is acyclic, they can be restored one at a time in its
  order. Each step leaves every other off piece at its exact pose and every lattice piece on the lattice.
- **Lemma G and Corollary G.** A *glued block* is a set of at least two off pieces such that every unblocked grip
  whose cap holds one of them holds all of them; pieces with the same movers are an example. Any word that keeps
  the other off pieces in place moves the whole block at its first move. So no single seed applies to a piece of
  the block, and no choice of seed, footprint or buffer vacancy breaks its conflict cycle. The earlier gap "a
  conflict cycle can always be broken" is false for single seeds.
- **Block lemma (open)** and **Proposition B (proved).** The gap is restated: in every reachable state whose off
  pieces are all depth one, some block can be restored as a whole. If that holds, every such state can be brought
  to the lattice. Stages 06 to 09 work on this question.

### Coverage (section 4 of PROOFS-3E.md)

| Input | Off pieces | Result |
| --- | ---: | --- |
| E3 | 0 | already on the lattice |
| E2, I1, I2 | 3 each | restored with 3-seeds, after a short preparation word. I2 is restored without its own inverse word |
| E1 | 90 | the 3-block restored; the 87-block is glued and no 87-seed exists |
| X | 107 | none: the blocks of 87, 17 and 3 pieces are all glued, and the 3-seeds meet the 17-block |
| e2run24 | 690 | none: 13 glued blocks of 3 to 111 pieces, no free piece |
| stop62 | 860 | none: 15 glued blocks of 3 to 111 pieces, no free piece |
| I_a@40 fixture | 94,147 | classified only: all depth one, 484 classes, 76 of them seeded |

Every run that restored pieces was replayed again by the independent verifier from its input table (CERT-VERIFY).

### The reachable history (section 5 of PROOFS-3E.md)

One continuous replay from the solved state checks every twist and the digest at the end of each segment. It
defines the deep samples that stages 06 to 09 use:

| Segment | Twists | Off at the end | State |
| --- | ---: | ---: | --- |
| I_a@40 fixture, first 40 records | 40 | 94,147 | the deep scramble |
| seams stop | 62 | 860 | **stop62** |
| E2 run | 24 | 690 | **e2run24** |
| the owner's word, first 36,563 twists | 36,563 | 107 | |
| u, the X reachability word | 302 | 107 | X |
| D17, D87, D3 (stage 01) | 2,401 | 0 | E1, E2, then E3 |

The "E2 run" is the 24-twist run of step E2 of the joint attack (`theory/e2_frag.py`, record
`theory/results/joint-attack-I_a-40.json`). Its end state e2run24, the "E2 end" of the theory pages (690 off
pieces), is not the state E2 of stage 01 (3 off pieces). The owner's word starts there.

The total is 39,392 twists. So X, E1, E2 and E3 are reachable from solved by one replayed word, and the statements
that assumed it (for example ν(E3) in card 5) hold without that condition.

## Replay

Run from `scripts/` with `python3 -I` and `ulimit -v 7000000`. `$J` is `research/jumbling`; its code on main is
unchanged since commit `27a001c`, which the runs used. `$OUT` is an output directory. Decompress the `.gz` files
first. `data/arrays.npz` is not stored: `e3lib.py` rebuilds it at the path given when it is absent.

The frozen input tables are stored in stages 01 and 02. Their SHA-256 values match `certs/inputs.json`:

| Input | Table |
| --- | --- |
| X | `../01-closure-x-to-e3/x-17-block/input/X-table.json.gz` |
| E1 | `../01-closure-x-to-e3/e1-87-block/input/E1-table.json.gz` |
| E2 | `../01-closure-x-to-e3/e2-b-prime/input/E2-table.json.gz` |
| E3 | `../01-closure-x-to-e3/e2-b-prime/D3/E3-table.json.gz` |
| I1, I2 | `../02-three-block-family/inputs/I1-table.json.gz`, `I2-table.json.gz` |

    # independent check (about 15 minutes): 252 seeds, 3 k-seeds and four restoration runs
    python3 -I verify_3e.py $J ../data/classes.json ../data/arrays.npz ../data/seeds-lib.json ../data/kseeds.json \
        $OUT/v.json ../runs/run-E2.json <E2 table> ../runs/run-I1.json <I1 table> \
        ../runs/run-I2.json <I2 table> ../runs/run-E1.json <E1 table>
    # the pipeline on one input (about half a minute)
    python3 -I e3pipe.py $J ../data/classes.json ../data/arrays.npz ../data/seeds-lib.json <X table> \
        $OUT/run-X.json --exec --name X --kseeds ../data/kseeds.json
    # cap sets and blocks
    python3 -I capsets.py $J ../data/classes.json ../data/arrays.npz <X table> $OUT/capsets-X.json
    python3 -I blocks.py $OUT/capsets-X.json $OUT/blocks-X.json ../data/kseeds.json 16245,32088,16249
    # structure and seed table
    python3 -I structure.py $J ../data/classes.json 0 $OUT/structure.json
    python3 -I seed_table.py $J ../data/classes.json ../data/arrays.npz ../data/seeds-lib.json \
        ../certs/base-reach3.json $OUT/table.json ../runs/run-*.json

The history replay `replay_e3_history.py` also needs the owner's 41,457-twist construction. That word is not in
this repository; `certs/inputs.json` records its SHA-256. `classify_all.sh` and `run_deep.sh` are the batch files
of the runs, with paths as variables.

## Files

| Path | What it is |
| --- | --- |
| `PROOFS-3E.md` | Theorems, proofs, the seed table, coverage, the history replay and the certificate list |
| `scripts/` | All scripts; usage is in each file header |
| `data/` | Grip classes `classes.json`, the seed library `seeds-lib.json` (252 seeds), the k-seeds, the deep sample tables `deep/` (stop62, e2run24 and the fixture) |
| `certs/` | Structure, harvest reach, seed table, cap sets, blocks, chain blocks, the history replay, the independent check, input digests |
| `runs/` | One pipeline record per input (`run-*.json`, with every preparation and restoration word) and its log |

Not stored: the Chinese delivery notes and the checksum list. Data files of 256 KiB or more are stored gzipped;
`IMPORT-MANIFEST.json` lists them with the SHA-256 of the original bytes.

## Limits

- Only 76 of the 500 classes have seeds, and the harvest reaches 353 classes; the fixture uses 131 classes outside
  them (float evidence).
- Only depth-one pieces are treated. Every observed off piece is depth one, which is not a proof.
- The conflict and block analysis of the 94,147-piece fixture was not run.
- Lemmas R, S, H, G and Theorems C, C′ and K have no independent review.
- Not run: Windows, DirectX, the native host regression and performance measurements.
