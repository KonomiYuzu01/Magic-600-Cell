# LX-3 review notes

The integrator's notes on the frozen LX-3 records. `part-b/` and its candidates stay as they are, because their manifests
pin every report. This file is outside those pins. It changes no stored count, and it decides nothing. LX-3's decision stays
open and the owner decides it, with LQ9 and LQ21.

## Where these come from

One Sol review read the bench and the five candidates at `12435328` in three read-only shards. Shard A found nothing.
Shard B reported B-001 and B-002. Shard C reported C-001. The integrator adopted all three. `recheck/` then reproduced
B-001 and B-002 with new controls and probes. `linux-comparison/` holds the twenty-run comparisons.

## C-001: rt2-node's report misstates its crash count

`part-b/candidates/rt2-node/RESULT.md` lines 247, 301, 355 and 369 say 0/0 crashes. Its `summary.json` and
`key-outcomes.json` record ten `B12/r1/shared` crashes in each of its four unsupported cells, 40 in all. The stored data is
right and the prose is wrong. Read the report with this correction.

| Candidate | Crashes in `B12/r1/shared`, all in cells declared `not_run` |
| --- | --- |
| rt1-own | 30 |
| rt2-py | 40 |
| rt2-node | 40 (its report says 0) |
| rt3-wasm | 30 |
| rt4-pyhost | 30 |

## The B12 crash is a bench defect, left unfixed

Every crash above has one cause, in the bench. `harness/matrix.py:65` returns the `not_run` record before line 90 attaches
`shared_value_identity`, and line 102 reads that field for both shared workers, which raises `KeyError`. The adapter is
never entered. No candidate is at fault, and no supported cell is affected.

A fix would change the frozen matrix file (SHA-256
`2046930338b40a8aded4a2286a6992ea8edd8a9d8a7b3818801701f32f2642fa`), so it would change every candidate's binding identity,
and it would touch only cells that do not run. The integrator leaves it. A later LX-3 run on a new bench should fix it.

## B-001: what "no state change" can bear

`harness/classify.py:60-62` compared a field only when `case["unchanged"]` named it literally, and the corpus describes those
fields in words. `harness/host.py:159-162` took the refusal's snapshot at the same instant as the value it was compared
with. So the frozen classifier could not see a changed workspace value, a changed provenance block, or a replaced or cleared
pending preview. `part-b/RESULT.md:25`, which says such changes fail an observation, claims more than the code checked.

`recheck/strict.py` adds a stricter predicate without touching the frozen one. Nine synthetic controls run in real children:
the four corruptions above read `refused_expected` under the frozen classifier and `state_changed` under the strict one, and
a legitimate write stays expected. On all seven declarations, 522 keys each, 3,654 frozen keys reproduced with zero baseline
differences and zero strict preservation failures in every supported form and variant.

So B-001 was a real defect in the check, and it changed no frozen count. This is finite evidence for these prototypes and
inputs. It proves no general preservation property.

## B-002: label forms can lose a taint through the workspace

Four new two-run probes write a word derived from state into the workspace, then load it and request a preview in a solving
run. The expected results are refused, refused, refused, installed.

- **Escapes.** With a clean write that can never run after an unconditional stop, rt3-wasm and rt2-py install a forbidden
  preview under static labels. rt3-wasm replaces its static mask at each `workspace_put` and keeps analysing past the stop
  (`frontend.py`, `adapter.py`). rt2-py updates its abstract workspace the same way past the stop (`subset.py`), and its
  preview gate reads the overwritten marks (`bridge.py`).
- **False refusals.** On the positive probe, rt1-own under static labels, and rt2-node under capability separation and
  static labels, refuse a clean word. rt1-own and rt2-node join possible writes, and rt2-node's capability mode also carries
  the run's read-state flag into the clean block.
- **The count.** 52 expressible cells: 39 negative (37 expected refusals, two escapes) and 13 positive (ten installed
  previews, three false refusals). 68 unsupported cells stay `not_expressible`. rt4-pyhost meets all four expectations in
  each supported form.

These probes are new inputs, outside the frozen corpus. Their results do not enter the 20 or the 35 of the stored counts.
Those stay 20/16/16 for rt1-own, 20/16/`not_run` for rt2-py and rt2-node, 18/16/16 for rt3-wasm and 20/16/10 for rt4-pyhost,
with two false refusals for capability separation and none for the supported label forms. Failed forms stay failed. The
integrator's reply to B-002 moves from `needs_verification` to `adopt`.

## The rt3-wasm admission variant

rt3-wasm's capability separation misses MR14 and MR20. `recheck/admission.py` builds a variant in `build/` that walks a
solving run's closure and asks the unchanged policy about its effects before Wasm starts. MR14 and MR20 then refuse at
admission with the expected code, with no executed instruction and no preview. That supports the reading that the two
misses come from the adapter's choice of when to check.

The variant ran on those two cases only. No allowed program went through it, so its false-refusal count is unknown. The
frozen prototype stays at 18 of 20.

## Windows and Linux comparison

`linux-comparison/` holds the comparison of a ten-run Windows leg with a ten-run headless Linux leg, for rt1-own, rt2-py and
rt4-pyhost: 10,440 rows each, six cells each, `reproducibility_counterexamples` empty in all six cells of all three records.
Windows ran CPython 3.15.1 and WSL ran CPython 3.14.4. rt2-node and rt3-wasm have no Linux leg, because WSL has no Node.
These records sit outside `part-b/` so that no manifest changes. They show that hashes repeated across these two legs for
these inputs, and they say nothing about performance or a product runtime.
