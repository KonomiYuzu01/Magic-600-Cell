# 06. A block-shape family of restoration words (14 October 2026)

**The question.** Stage 05 showed that glued blocks must leave the off-lattice set together. Can the words that
restored the 17- and 87-piece blocks of stage 01 become a family, parametrized by the block's own geometry and
carried by K⁺ between blocks of the same shape?

The main document is [PROOFS-BF.md](PROOFS-BF.md). Answer in short: yes for one template, with per-instance
hypotheses; not as a general theorem.

Labels: **proved** (the author's proof, no independent review), **certificate** (exact computation with its file
and script), **float evidence**, **conditional** (the condition is named), **open**.

## Results

| Item | Status |
| --- | --- |
| Lemma Q: the D-type word Q = A, B, X, B⁻¹, X⁻¹, A⁻¹, X, B, X⁻¹, B⁻¹ puts a block on the lattice when four exact cut tests Q1 to Q4 hold | **proved** |
| Lemma Q′: the C-type word [X, B] needs only one mover | **proved** |
| Lemma V: closure V·W·V⁻¹ with a SAFE vacating word V | **proved** |
| Lemma V′: closure with K⁺ letters that may carry off pieces | **proved**; not searched |
| Lemma TT: a word found for one block works, carried by K⁺, on every block of the same shape | **proved** |
| Theorem CB: blocks restored one at a time along an acyclic conflict digraph; Proposition M: a step never blocks a free grip, and blocks only split | **proved**, with the vacating word as a per-instance hypothesis |
| Block census: 174 blocks in 71 shapes over X, E1, E2, stop62, e2run24 and the fixture | **certificate** |
| Template search from the current state only; on X it reproduces D17's Bad sizes 198 and 87 | **certificate** |
| Runs on the two mid-depth samples and on the I_a@40 fixture | **certificate**, partial coverage |
| One-mover, several-frame and frozen blocks (78% of the fixture's off pieces) | **open**: no construction here |
| Block lemma and general restoration | **open** |

### Measured coverage

Every step below was replayed independently from its input table: every twist admissible, the target block on the
lattice, collateral 0, every other off piece at its exact pose (`certs/verify-*.jsonl`).

| State | Off pieces | Steps | Restored |
| --- | ---: | --- | --- |
| stop62 | 860 | D17 carried from X by K⁺ (838 twists with the closure), then two D-type words (84 and 824) | 171 pieces, 3 of 15 blocks; 689 left |
| e2run24 | 690 | D17 carried from X, then two D-type words | 110 pieces, 3 of 13 blocks; 580 left |
| I_a@40 fixture | 94,147 | 7 words carried or searched by shape, Bad(W) already vacant | 1,101 pieces (1.2%), 7 of 140 blocks |

The first three steps of the stop62 chain are the first three steps of the full stop62 restoration in
[stage 09](../09-general-method/README.md).

### Where the family stops

- **At 689 off on stop62 every block has a word**, and with these words the conflict digraph is acyclic with three
  levels (`certs/conflict-stop62-689.json`). The run stops at the closure: every try with |Bad(W)| ≥ 333 fails the
  necessary condition N2 (too few holes in some SAFE orbit), and the tries with |Bad(W)| = 198 found no V in 600 s.
  Stage 08 later showed that this hypothesis cannot be met with these word pools (its erratum E3).
- **At the fixture the template does not apply to most blocks.** 57% of the off pieces lie in one-mover blocks,
  which have only C-type words, and the three measured one-mover blocks have no clean one. Blocks whose poses span
  several frames and the frozen 19,720-piece block have no construction here. Only 13 of 600 grips are SAFE there.

Section 7 of PROOFS-BF.md gives the go/no-go: go for the family as a component, no date for general restoration.
Stage 07 replaces the template by one family that covers every block class.

## Replay

Run from `scripts/` with `python3 -I` and `ulimit -v 7000000`. `$J` is `research/jumbling` (its code on main is
unchanged since commit `27a001c`, which the runs used), `$OUT` an output directory. `scripts/echain/` holds the five
modules of stage 01 that the scripts import. Decompress the `.gz` files first.

    # independent replay of the four chains (about 50 s for each mid-depth chain, 15 s for the fixture)
    for c in stop62 e2run24 fixture fixture-115; do
      python3 -I verify_chain.py $J echain ../runs/chains/$c/chain.json $OUT/verify-$c.jsonl
    done
    # the template search for one block
    python3 -I dfam.py $J/theory echain ../data/states/stop62-table.json <block.json> $OUT/d \
        --max-q 1500 --good-q 300 --min-q 600 --park-q 3 --max-park 800 --no-pole
    # the closure V·W·V⁻¹ of one block
    python3 -I tclose.py $J/theory echain <pose table> <block.json> <W.json> $OUT/close
    # the conflict digraph of Theorem CB from the search summaries of one state
    python3 -I mkconflict.py $OUT/conflict.json ../runs/dfam/stop62-689-*.summary.json

`fxrun.py` (the fixture runs) also needs `classes.json` and `arrays.npz` of stage 05 (`../05-seed-table/data/`;
`e3lib.py` rebuilds `arrays.npz` when absent). The headers of `blockrun.py`, `fxrun.py` and `safecount.py` give their
usage; `run_dfam.sh`, `batch_dfam.sh` and `run_ts.sh` are the batch files of the runs.

## Files

| Path | What it is |
| --- | --- |
| `PROOFS-BF.md` | Theorems, census, template search, coverage, limits, go/no-go and the certificate list |
| `scripts/` | All scripts; `scripts/echain/` holds unchanged copies of five stage 01 modules |
| `data/states/` | Start tables: stop62, e2run24 and the I_a@40 fixture |
| `data/templates/` | The templates carried to the fixture: source tables, blocks and words |
| `data/` | The block census `census-all.json` and the fixture block list |
| `runs/chains/` | Every restoration chain: start table, per-step words with digests, targets, results, vacating certificates |
| `runs/` | Block-by-block logs `blockrun-*.jsonl`, fixture transport logs `fxrun*.jsonl`, search summaries `dfam/`, type-S summaries `typeS/` |
| `certs/` | Independent replays `verify-*.jsonl`, SAFE grip counts, the stop62 conflict digraph at 689, the fixture stop records |

Not stored: the Chinese delivery notes and the checksum list. Data files of 256 KiB or more are stored gzipped;
`IMPORT-MANIFEST.json` lists them with the SHA-256 of the original bytes.

## Limits

- The family is one template. Blocks with fewer than two movers, blocks in several frames and frozen blocks are
  outside it.
- The vacating word is a hypothesis per instance; Lemma V′ is proved but was not searched.
- The two mid-depth runs were stopped by hand after their last restoration.
- The new proofs have no independent review.
- Not run: Windows, DirectX, the native host regression and performance measurements.
