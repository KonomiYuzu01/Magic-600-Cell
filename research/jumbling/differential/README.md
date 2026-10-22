# Jumbling differential harness

Status: this is the NumPy-free first part of EN2. The J1 adapter, fixture
replays, reference values and cost leads follow in later packets. The store
prototype is a separate packet under `candidate/`. Current evidence is source,
fixture parsing and synthetic engines; it establishes no engine replay or
Windows/DirectX or performance result.

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
from their engine's data. The harness never reads or compares a chosen straddle
witness, certificates or points; returning certificate objects as survey values
is a contract error.

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
