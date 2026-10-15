# 07. One family for every block class: heap peeling (14 October 2026)

**The question.** Stage 06 had a template for one block class only. Is there one family of steps that applies to
every class (one or several frames, two movers, one mover, frozen), and do its parameters always exist?

The main document is [PROOFS-GF.md](PROOFS-GF.md). General restoration (from every reachable state) is not claimed.

Labels: **proved** (the author's proof, no independent review), **certificate** (exact computation with a record
that `scripts/` replays), **float evidence**, **conditional**, **open**.

## Results

- **One family, no templates (Lemma P, proved).** A step is one jumble letter at one cap: a grip c, a frame domain d
  inside cap c, and one of the 12 letters of the coset that sends d to the lattice. The pieces of that frame inside
  the cap go onto the lattice, the lattice pieces inside the cap go off, and nothing outside the cap moves. The gain
  is the first count minus the second. Every quantity the searcher compares is K⁺-invariant, so transport by shape
  is automatic.
- **Along a history the parameters always exist (Theorem R, proved).** The inverse of the last twist is a family
  step. But a reverse step can lose pieces: the inverse of fixture record 36 has gain −2,413 (certificate).
- **From the state alone there is no monotone single letter in the low regime (Proposition N(b), proved).** At
  h ≤ 1,548 off pieces every jumble letter raises the off count to at least 3,097 − h. Witnesses: a state at 790
  off and stop62 at 689. Stage 08 corrects the reading (its erratum E1): this is about single letters; a word of
  several letters can still end lower.
- **The searcher (heap peeling).** It picks the grip by gain and the letter by *sibling support*, a rule read from
  the state. Along the fixture history it selects the exact inverse at all 17 steps where the letter matters
  (certificate, margin at least 186,978). Every chain it writes is correct (Theorem C, proved) and is checked by an
  independent replay.

### Measured (certificate: every chain replayed independently)

| Start state | Off pieces | Steps | Off at the end | Off count reduced |
| --- | ---: | ---: | ---: | ---: |
| I_a@40 fixture (first 40 records of the W-J walk) | 94,147 | 36 | 10,415 | 88.9% |
| r40s11 (40 random jumble twists, seed 11) | 87,599 | 34 | 9,033 | 89.7% |
| p60 (first 60 records) | 105,676 | 44 | 17,316 | 83.6% |
| p100 (first 100 records) | 118,286 | 43 | 46,389 | 60.8% |
| S2 (first 2 records) | 5,404 | 2 | 0 | 100%: every piece on the lattice |

On the fixture 84,947 of the 94,147 off pieces end on the lattice, and every block class gains: 73.8% to 93.4% of
each class's pieces. The template family of stage 06 reached 1,101. Multi-frame and frozen blocks need no separate
construction: a cap that holds several frames is one row of the search, and a frozen block becomes reachable when
the pieces that straddle its grips are restored. No class is complete, though.

### Failure modes (section 8 of PROOFS-GF.md)

- **F1, low regime (proved):** no monotone single letter at 1,548 off pieces or fewer.
- **F2, trap (measured on all four deep runs):** a group of one frame lies inside a cap that is mostly lattice, and
  the letter that restores it loses 127 to 1,323 pieces. The same caps recur across runs.
- **F3:** histories that are not monotone (proved by example).
- **F5, completeness (open):** a run ends on the lattice or at a recorded stall, and nothing says which.

Go for the family and the searcher as a correct partial restorer from any reachable state; no-go for general
restoration. The stall states are stored (`runs/*/end-table.json`) as test inputs. Stage 08 takes up the low
regime with multi-letter clean steps.

## Replay

Run from `scripts/` with `python3 -I` and `ulimit -v 7000000`. `$J` is `research/jumbling` (its code on main is
unchanged since commit `27a001c`, which the runs used) and `$OUT` an output directory. Decompress the `.gz` files
first. Run names: `fx` the fixture, `r40s11`, `p60`, `p100`, `t2` (S2 to the lattice), `g2` (S2 to Y) and `y`
(Y to Y′).

    # independent replay (6 to 10 s per chain after loading)
    for c in fx r40s11 p60 p100 t2 g2 y; do
      python3 -I hp/verify_hp.py $J echain ../runs/$c/chain.json $OUT/verify-$c.jsonl
    done
    # the searcher (6 to 10 minutes per deep state; the defaults are the options used)
    python3 -I hp/hp.py $J echain ../data/states/fixture40-table.json $OUT/fx
    # the letter rule along the fixture's reverse history (records 40 to 1)
    python3 -I hq/lettertest2.py $J echain ../data/states/fixture40-table.json 40 1 > $OUT/lettertest-fixture.jsonl
    # per-class coverage
    python3 -I hp/classcov.py $J echain ../data/states/fixture40-table.json ../data/blocks-fixture40.json \
        ../runs/fx $OUT/classcov-fx.json
    # start states: the first n W-J records, and a random scramble
    python3 -I hx/prefix.py $J echain 40 $OUT/fixture40-table.json
    python3 -I hx/scr.py $J echain 40 11 $OUT/r40s11-table.json $OUT/r40s11-hist.json

## Files

| Path | What it is |
| --- | --- |
| `PROOFS-GF.md` | Facts, lemmas and theorems, the searcher, measurements, answers, failure modes, certificates and checks not run |
| `scripts/hp/` | The searcher `hp.py`, `hpcore.py`; the independent replay `verify_hp.py`, which imports no search script; coverage and history comparison; `old/` holds the earlier rule versions |
| `scripts/hq/`, `scripts/hx/` | The letter-rule test; start states and history anatomy |
| `scripts/gf/` | The lattice-free-cap families S, SV and CX |
| `scripts/echain/` | Unchanged copies of five stage 01 modules |
| `data/states/` | Start tables: the fixture, the first 2, 60 and 100 W-J records, r40s11 and its history |
| `runs/<run>/` | Each chain: `chain.json`, per-step words and exact sets, `run.jsonl`, the end table; `<run>.err` is the run log |
| `certs/` | Independent replays, the letter-rule certificate, the history anatomy, class coverage, history comparison, the summary |

Not stored: the Chinese delivery notes and the checksum list. Data files of 256 KiB or more are stored gzipped;
`IMPORT-MANIFEST.json` lists them with the SHA-256 of the original bytes.

## Limits

- The earlier rule versions and the S/SV runs have only their own simulator checks, no independent replay.
- A float preselection (margin 1e-6) picks the contents that get an exact test. It cannot make a written chain
  wrong, but it may hide candidates (float evidence).
- Six start states only; no lattice phase here.
- Lemma P, Theorem R, Corollary R, Proposition N and Theorem C have no independent review.
- Not run: Windows, DirectX, input, long sessions and performance measurements.
