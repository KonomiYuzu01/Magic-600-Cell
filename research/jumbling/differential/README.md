# Jumbling differential harness

Status: EN2 slice 1 supplies the NumPy-free harness core; slice 2 adds the J1
adapter and replays; slice 3 adds named-cap straddler sets and the cost leads
in [`leads/`](leads/README.md). Reference values follow in a later packet.
The store prototype is a separate packet under `candidate/`. Evidence here is
source, synthetic engines and fixture replays with the J1 reference engine;
it establishes no candidate-engine, Windows/DirectX or performance result.

The harness runs the same records through a reference engine and a candidate
engine. It compares the solved start (step 0), then each result in chronological
order. The first difference ends the run; neither engine receives a later step.
The input index is one less than the reported step, and is `None` for step 0.

## Engine observations

An adapter implements `Engine.name`, `start(menu)`, `apply(step)` and
`survey(grips)`. `start` resets to solved under the supplied menu record or no
menu, and returns a `StepResult` with status `start`. Each `apply` returns a
`StepResult` with status `admissible`, `blocked`, `invalid` or `uncertain`.
Every adapter uses these definitions:

- `digest` is the canonical state SHA-256 from state-contract section 2 and
  J1's `State.digest`. For every non-identity pose, form the key
  `json.dumps(matrix_json(pose))` using Python's default separators, and encode
  the sorted ids of pieces in that pose as little-endian **int32** bytes. Sort
  the `(key, id_bytes)` pairs; hash each key's UTF-8 bytes followed by its id
  bytes, pair by pair. Solved hashes the empty byte string.
- `lattice` is SHA-256 of one byte per piece in piece-id order: 1 when that
  piece's pose lies in K⁺, 0 otherwise.
- `moved` counts the pieces moved by this step, and is 0 for a refused step.
- `moved_set` is SHA-256 of the sorted moved-piece ids as little-endian
  **uint32** bytes. `moved_set_hash(ids)` implements it (duplicates are treated
  as one set member); `EMPTY_MOVED_SET` hashes the empty byte string.

All hashes are 64-character lowercase hexadecimal strings. A survey returns
exactly the requested integer grip ids, each mapped to one letter: `a`
(admissible), `b` (blocked) or `u` (uncertain). Adapters extract these letters
from their engine's data. A survey never reads or compares a chosen straddle witness,
certificates or points; returning certificate objects as survey values is a
contract error. Straddler sets are a separate, optional comparison (section
"Named-cap straddler sets").

`compare(reference, candidate, steps, survey=None, grips=None)` checks fields in
the order `status`, `moved`, `moved_set`, `digest`, `lattice`, then surveyed
`grips` in ascending id order. Contracts are validated before field comparison.
Every refused step must have zero moved pieces, the empty moved-set hash, and
the same digest and lattice hash as that engine's previous result. A candidate
contract violation reports `contract`; a refusal violation reports
`refusal_changed_state`. Either violation by the reference raises
`ReferenceContractError`, since that engine cannot then serve as an oracle.
Exceptions raised by adapter methods themselves propagate to the caller.

Survey selection is `None`, `'all'` (including step 0), or a set of step
indices. Grips default to all 600; supplied ids are sorted and deduplicated.
`Report.steps_compared` counts observations including step 0 and any mismatching
observation. `surveys_compared` counts surveyed observations, including a failing
survey, rather than individual letters. A step-field mismatch stops before its
survey. `Report.as_dict()` produces plain JSON-serialisable data, including the
engine names and first mismatch with its detail.

## Replay inputs and menu order

`load_steps(path_or_document, menu=None)` returns a `Steps` containing a tuple
of `steps`, the `menu`, `model_identity`, `menu_identity`, `contract_revision`
and `menu_order_digest` when available. Missing metadata remains `None`.
Journals use `{'kind': 'record', 'record': record}`; W-J fixtures supply both
their journal and their menu. A solved letter word needs a menu argument and
uses `{'kind': 'letter', 'grip': grip, 'item': item}`. Grips must be integer ids
in 0..599, and letter indices must be within the menu; booleans are rejected.
Unknown formats and malformed shapes raise `HarnessInputError`.
Menu and replay mappings are copied for each adapter, so mutating one adapter's
input cannot change the other adapter's input or the loaded records.

`menu_order.menu_identity` hashes the sorted compact JSON keys of the exact base
matrices, as J1 does. `order_digest` hashes the same keys in item order. Labels
are not hashed, and exact integer triples are not normalised. `check_menu`
checks the record's own identity and any supplied expected identity and order.
It raises `MenuIdentityError` or `MenuOrderError` for a mismatch, and
`MenuRecordError` for malformed 4 × 4 matrices of integer `[a, b, d]` triples
with positive denominators. `inverse_index` finds each matrix's transpose and
raises `MenuRecordError` when an inverse is absent. These structural checks do
not prove that the matrices are rotations or A4-invariant; engine adapters
remain responsible for mechanical menu validity.

## J1 adapter and replays

`j1_adapter.J1Engine(ctx=None, name='J1')` wraps `sim.State`, using one shared
J1 context when supplied. `start(None)` selects free mode; a menu record is
rebuilt with `TwistMenu.from_record`. The adapter refuses dropped duplicates
or a changed round-trip order digest, so letters retain their recorded item
indices. J1's menu identity alone does not pin that order. A letter is item
`i` of `TwistMenu.for_cap(c)`, cached per cap: the exact transport is
F_c · item_i · F_cᵀ, with J1's retained frame F_c.

The adapter reports J1's outcome status and moved count, `State.digest()`, and
SHA-256 of `State.lattice_flags().astype(numpy.uint8).tobytes()` (one byte per
piece, in piece-id order). A small `State` subclass captures the certified
`inside` array after a successful `_commit(twist, inside, journal)`; this
private J1 signature is an explicit dependency. Its moved-set hash uses the
same inside set that J1 committed, without a second classification. Invalid
record reconstruction and J1 refusals return zero moved pieces, the empty
moved-set hash and the preceding digest/lattice hashes. `survey(grips)` keeps
only the status letters for exactly those grips. The getter-only `state`
property permits test inspection; callers must not mutate J1 through it.

`tests/test_jumbling_j1_replay.py` checks:

- J1 against J1 on 60 records of each W-J fixture, plus 30 I_a records with
  all 600 grips surveyed at steps 0 and 30.
- One complete I_a replay and S4/I_b replays through their middle stages,
  checking each recorded moved count and the recorded stage digests. I_a also
  checks the swept twist's before/after digests, moved count and moved-set
  hash, and the end survey's 600 letters. These references are in
  [`wj-I_a.json`](../fixtures/wj-I_a.json),
  [`wj-S4.json`](../fixtures/wj-S4.json) and
  [`wj-I_b.json`](../fixtures/wj-I_b.json).
- The E2, E3, random_0 and random_1 journals in free mode and the negative
  control with its own menu, all paired and checked for recorded moved
  counts. The negative control also reproduces its 25 distinct digests and
  12 cap-0 pose-entry heights in [`acceptance.json`](../sim/acceptance.json).
- Planted digest and moved-set changes, a skipped apply, and a refusal that
  changes the digest: each stops at the first bad step, with the expected
  field and no later apply calls. A reference refusal violation raises
  `ReferenceContractError`. Adapter checks cover an outside-menu record,
  failed record reconstruction, a swapped menu order and duplicate items.

`tests/test_jumbling_x1_witness.py` pins the I_a order digest before loading
[`H6030.json`](../restoration/10-x1-unbounded/witnesses/H6030.json), replays all
3,156 letters once and compares J1 against J1 over the first 600 letters.
After each of those 600 letters and at the end, `trees.pose_tree` checks every
non-identity pose: right height 0 and left height at most twice the letters
played. Results are cached by each pose's exact key. During this module only,
a bounded LRU cache wraps the original pure integer `trees._v2int` helper to
reuse valuations of large power-of-two denominators; `pose_tree` itself runs
unchanged. The final digest, height histogram and seven pieces at left height
6030 reproduce
[`programs/H6030.out`](../restoration/10-x1-unbounded/programs/H6030.out).
The piece-height convention agrees with `programs/replay.py`: the left
distance of the pose's quaternion factor, including height 0 for identity
and K⁺ poses. The test checks this formula on the final pose table.

`j1_state.py` exports plain data from J1 without the candidate store:
`state_record` (the pose table with exact matrices or K⁺ indices, the pose ids
as `u32le-base64` and the digest), `letters_from_records` (each record matched
to its menu item by the exact transported matrix) and `twists_from_letters`.
`tests/test_jumbling_store_j1.py` writes these through `candidate/store.py` and
checks that each document reads back unchanged, byte for byte:

- The start, middle and end stages of I_a and the start and middle stages of
  S4 and I_b, each with its canonical digest recomputed from the stored pose
  table. The fixture journals are stored first, and the read-back records are
  the ones replayed to the recorded stage digests.
- Crash recovery at each of the three simulated crash points returns a whole
  revision. A scramble journal stays apart from the solver journal. A label
  exit keeps its exact exit poses; its certificate content is synthetic, so
  only the certificate's shape and digest binding are tested.
- Pose components above 2⁶⁴ after the first 204 H6030 letters. The 5,629
  letters of correction 35 (`card5:MB2-5` in `library-merged.json.gz`) are
  matched from its records, stored, decoded and replayed to a measured J1
  digest on the lattice. A parts tree with an inverse and a same-as part
  expands to its literal letters.

Run with Python 3.11 or later and NumPy (the pinned version is 2.3.5):

```sh
python tests/test_jumbling_j1_replay.py
python tests/test_jumbling_x1_witness.py
python tests/test_jumbling_store_j1.py
python -m unittest -q tests/test_jumbling_j1_replay.py tests/test_jumbling_x1_witness.py tests/test_jumbling_store_j1.py tests/test_jumbling_store.py tests/test_jumbling_differential.py
```

Without NumPy each new test module is skipped with a NumPy message. Under a
shared unittest command each of these modules runs its tests in one isolated
worker (`tests/isolated_numpy.py`), so `tests/test_jumbling_differential.py`
keeps its proof that NumPy and J1 never load in its process. Each module shares
one context and disables bytecode writes before local imports. The replay
tests write no output; the store tests write only in `tempfile` directories
outside the repository, and an audit hook fails any other write.
Set `JUMBLING_FULL_REPLAY=1` to compare both engines on all three complete
fixtures (including end surveys against their recorded letters), all five
journals and all H6030 letters, to check the H6030 bound after every letter,
to extend the S4/I_b single replays through their end-stage and sweep
references, and to store the S4/I_b end stages and replay correction 35's
original records. CI uses the default bounded run.

J1 against J1 exercises the harness, adapter and recorded references. It
does not establish agreement with a candidate engine. Headless timings of
these research replays are not product performance evidence.

## Named-cap straddler sets

A blocked cap's early exit names one straddling piece; the straddler-set rule
of ENG-0J item 2 needs all of them. `straddlers.straddler_set(state, cap,
budget=None)` follows J1's pose groups and anchor supersets but keeps every
straddler, each with J1's exact certificate (two points of the posed piece on
opposite sides of the cut). It returns `blocked`, the sorted `pieces`, the
`certificates`, a status, the counts and the evaluation counts. A finite
exact-evaluation budget may leave pieces uncertain: they count only as
`uncertain_pieces` and make the status `uncertain`, never clear and never
straddlers. `check_certificate(state, cap, piece, certificate)` parses the two
points anew and checks exactly that both lie in the piece's posed region and
on opposite sides of the cut; it trusts no cached h value, vertex index or
scan order. Private J1 dependencies: `State._kpose`, `_groups`, `_anchors`,
`_pose_matrix`, `_eval` and `_straddle_certificate`.

`compare(..., straddlers=None, caps=None, certify=None)` adds this comparison
after the surveys. `straddlers` selects steps as `survey` does, and `caps`
(required with it) names the caps. Each engine's optional `straddlers(caps)`
returns, per requested cap, `blocked` (equal to `bool(pieces)`), `pieces` (a
sorted list of unique piece ids) and `certificates` keyed by exactly those
pieces; an uncertain set is a contract error. A bad reference mapping raises
`ReferenceContractError`; a bad candidate mapping, or a missing method when
straddler steps are selected, reports `contract`. The blocked flags and piece
sets are compared cap by cap (field `straddlers`). Certificate contents are
never compared between engines: when the caller supplies `certify`, bound to
the reference engine's current state, each candidate certificate is checked
with it (field `certificate`). `Report.straddlers_compared` counts the
straddler observations, including a failing one. `J1Engine` implements
`straddlers` and `certify` with `straddlers.py`.

`tests/test_jumbling_straddlers.py` checks the full sets at four caps of the
`wj-I_a` state after 60 records against J1's per-piece path over every off
piece and against the survey, the budget and filtered cases, certificates that
must fail (moved, swapped, renamed, malformed), J1 against J1 through the
harness with alternative certificate points, planted set, flag, certificate
and contract faults stopping at the first bad step, and the X2 and
classification-cost smoke runs. Synthetic engines in
`tests/test_jumbling_differential.py` check the comparison's contract without
NumPy.

## Import independence

`independence.scan_source` and `scan_paths` inspect Python syntax without
executing scanned code. Violations name their path and line.

- `HARNESS_POLICY` allows standard-library modules, `numpy`, J1's `sim` and
  `sim.*`, `exact`, `differential` and its existing top-level harness modules,
  and relative imports of those modules. It forbids `differential.candidate`,
  including relative imports of it, and other modules.
- `CANDIDATE_POLICY` allows standard-library modules, `numpy` and modules
  inside `differential.candidate`, including relative imports that stay there.
  It forbids J1, `exact`, the harness modules and other modules.

Both policies reject dynamic-code names `__import__`, `importlib`, `runpy`,
`exec`, `eval` and `compile`, and references to `sys.path`. Straightforward
import aliases of those access routes are checked too. This AST check is an
independence check, not a security sandbox. Relative imports use the source
file's package path; for `scan_source` without a package path, they use the
policy's package (`differential` or `differential.candidate`). `harness_files()`
lists the top-level Python modules; `candidate_files()` lists Python files
recursively under `candidate/`, or an empty list when it does not exist.

## Test

From the repository root, with Python 3.11 or later and no installed NumPy:

```sh
python tests/test_jumbling_differential.py
```

The test disables bytecode writes before local imports. Test-generated files
live only in `tempfile` temporary directories outside the repository. No J1 or
candidate engine is imported, and no fixture is mechanically replayed here.
