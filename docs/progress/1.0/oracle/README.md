# Differential oracle: specification

Status: **draft specification for stage 2.** It turns section 3 of the [stage 2 experiment protocol](../stage-2-experiment-protocol.md) into inputs, outputs and comparison rules. It is not a frozen format: the case and trace formats carry a `draft` marker until the 2.5 architecture freeze. It changes no 0.4 code; the reference is 0.4 `core.py` and `session.py`, imported read-only.

## 1. Purpose

When 1.0 rebuilds a component with a 0.4 counterpart (the move engine, macro expansion, star construction, protection checks), the oracle runs the same scripted input through the 0.4 reference and through the new component, and compares the complete labelled state after every step. The first mismatching step is the bug report. A mismatch is never "close enough".

The oracle is a test tool. It never runs during normal user startup, never opens a personal session and never writes outside a fresh temporary directory or its own output directory.

## 2. Terms

- **Labels:** the 259,800 int32 sticker labels of a `PuzzleState` (`core.py:255`).
- **State hash:** `core.state_hash(labels)`, the SHA-256 of the labels as little-endian int32 (`core.py:18`). V10 of the 1.0 architecture calls the same digest `labels_sha256`; the oracle uses one name, `state_hash`.
- **Net permutation of a step:** the pair (s, d) returned by `Model.net(recipe)` (`core.py:182`): source and destination slot arrays, sorted by source. Its digest is `net_sha256` = SHA-256 of `s` as little-endian int32 followed by `d` as little-endian int32. This is the `source_to_destination_sha256` field of a 0.4 certificate (`session.py:153`).
- **Expansion digest of a step:** `expansion_sha256` = SHA-256 of the concatenation of every primitive of `Model.expand(recipe)` (`core.py:193`), in order, each as a signed little-endian int32; nothing else is hashed (no count, no separator). `primitive_count` is the number of primitives as a JSON integer. Test vector: the recipe `[{"kind":"word","moves":[1,-1]}]` has `expansion_sha256` `b15348c8f462384c01e83b6d499c6faf3f96808f5aa07c6bab4b65b36b4445d4` and `primitive_count` 2. It fixes the chronological order of primitives inside macros.
- **Support:** `Model.support(s)` (`core.py:199`): per orbit, the moved stickers and pieces.
- **Conflicts:** the support rows whose orbit is protected (`session.py:155`).

## 3. Inputs: oracle cases

A case is one JSON file (`C600-ORACLE-CASE-draft`), written gzip-compressed:

```
{ format, case_id, description,
  model: { model_id, manifest_sha256 },
  generator: { script, script_sha256, core_sha256, session_sha256, python, numpy, seed },
  mode: "chain" | "independent",
  initial: { kind: "solved" | "word", moves: [...], state_hash },
  protected: [orbit, ...],
  steps: [ { index, recipe: [...],
             expect: { accepted, error, state_hash,
                       net_sha256, net_moved_stickers, net_moved_pieces,
                       expansion_sha256, primitive_count,
                       support: [...], conflicts: [...],
                       progress_sha256 } } ] }
```

- `chain`: each step starts from the state after the previous accepted step. `independent`: each step starts from `initial`.
- `recipe` uses the 0.4 recipe form exactly: `{"kind":"word","moves":[...]}` and `{"kind":"star","orbit":o,"node":n,"sign":±1}` steps.
- `accepted` is false when the reference rejects the step; `error` then holds the error class (`invalid-input` for a `ValueError` from `Model.normalize` or `Model.move`, `protected` for the protection refusal of `Session.commit`), and `state_hash` equals the state before the step. For a rejected step the net, expansion and support fields are null when the input itself was invalid.
- `progress_sha256` is the SHA-256 of the canonical JSON (`core.canonical`) of `PuzzleState.progress()`, so per-orbit residual counts are compared too.
- `protected` applies to every step of the case.

## 4. Outputs: candidate traces

A candidate under test writes one trace per case (`C600-ORACLE-TRACE-draft`): `{ format, case_id, model: { model_id, manifest_sha256 }, candidate: { name, build }, steps: [ { index, accepted, error, state_hash, net_sha256, net_moved_stickers, net_moved_pieces, expansion_sha256, primitive_count, support, conflicts, progress_sha256 } ] }`. A candidate may omit a field only when the case marks it optional for that component (for example a renderer-independent engine reports no `progress_sha256` if it has no progress view); omitted fields are reported, never treated as equal.

## 5. Comparisons, in order

For each step, in index order:
1. `accepted` and `error`;
2. `state_hash` (full labelled state, exact);
3. `net_sha256`, `net_moved_stickers`, `net_moved_pieces` (complete permutation, including all collateral);
4. `expansion_sha256`, `primitive_count` (chronological primitive order);
5. `support` and `conflicts` (protection constraints), as sets of rows;
6. `progress_sha256`.

The comparator stops at the first step with any difference and prints the case, the step index, the recipe and every differing field. Before any step, the comparator checks that the trace's `model.model_id` and `model.manifest_sha256` are present and equal to the case's. Exit code 0: all steps equal; 1: a step mismatch; 2: an unreadable or malformed file, a missing or different model identity, or a missing step.

## 6. Case set

| ID | Mode | Content | What it catches |
|---|---|---|---|
| O01 | independent | every generator 1..1200 and its inverse from solved | single-move tables |
| O02 | chain | 500 generators drawn with a fixed seed | composition, state carry-over |
| O03 | chain | every orbit 0..34: stars at the first, middle and last frame node, both signs, after a seeded scramble | seed certification, relocation, setup transport |
| O04 | chain | mixed recipes of word and star steps (2 to 6 parts) | chronological order inside a recipe |
| O05 | chain | each O04 recipe followed by its inverse (`core.invrecipe`) | inverse construction; the state hash returns |
| O06 | chain | protected orbits set; steps whose support touches them, and steps that do not. Includes: a star with nonempty collateral where only one collateral orbit is protected (refused, state unchanged); and a word `[g, -g]` whose generator moves a protected orbit in between (accepted: protection concerns the net support at the end of the operation) | protection refusal on collateral as well as target, net-not-intermediate semantics, unchanged state after refusal, conflict sets |
| O07 | independent, from a seeded scramble | invalid inputs: move 0, 1201, -1201, a non-integer, an empty recipe, a bad star orbit, sign, node; and compound recipes whose invalid part follows a valid prefix: the word `[1, 0]`, and a valid word step followed by a star with an invalid node | whole-recipe validation before any effect: `invalid-input`, unchanged `state_hash` and `progress_sha256`, null effect fields |

Sizes are kept so that one full generation runs in minutes on the owner's machine; seeds and counts are fixed in the generator and recorded in each case.

## 7. Reference generator

- `tools/oracle/make_cases.py --out <dir>` builds the case files with the 0.4 reference. It imports `core` and `session` read-only and never modifies them.
- Steps that need protection run through `session.Session` in a fresh temporary directory created by the generator and removed afterwards (`preview`, then `commit` or the refusal). The generator accepts no data-directory argument and refuses an output directory that contains a `session.sqlite3`.
- Output is deterministic: two runs with the same source produce byte-identical case files (fixed gzip mtime, sorted keys, no timestamps).
- `tools/oracle/compare.py <case> <trace>` implements section 5.
- `tools/oracle/replay_trace.py <case>` is a second, independent reference path for self-tests: it applies each primitive of `Model.expand` one by one with `Model.move` (no `Model.net`, no star cache) and writes a trace. It must equal the case on every step.

## 8. Tests

`tests/test_oracle.py`, fresh temporary directories only:
- the generator is deterministic for a small seeded subset of every case kind;
- `replay_trace.py` matches every generated step (independent paths agree);
- the comparator reports the first mismatching step when one `state_hash`, one `net_sha256`, one conflict row or one `accepted` value of a trace is changed, and exits 2 when the trace's `model_id` or `manifest_sha256` differs or is missing, and for a missing step;
- the expansion digest test vector of section 2 is reproduced;
- a deliberately streaming candidate (one that applies a valid prefix before rejecting the invalid part) fails O07;
- a refused protected step leaves `state_hash` equal to the previous state;
- the generator refuses an output directory that contains `session.sqlite3`.

## 9. Environment

NumPy is required by `core.py`. The tests run wherever the engine environment exists (the owner's Windows machine: `tools/.venv/engine`). A cloud session without NumPy runs none of them and says so.

## 10. Out of scope

- The V10 frontend fixture (`C600-COMPARE-FIXTURE-v1`: workspace, `pick` and `resolve` probes). This oracle covers the engine layer; picking is a renderer check (renderer plan section 4).
- Any change to `core.py`, `session.py` or other critical files.
- Performance.
