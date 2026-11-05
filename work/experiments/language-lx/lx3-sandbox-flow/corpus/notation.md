# LX-3 neutral corpus v1

This JSON is an **analysis notation, never a proposed language syntax or product
IR**. There is no parser, interpreter, renderer or candidate here. A later packet
must render each program mechanically into its candidate and pin that rendering,
its dependency closure and execution contract before running it. Do not replace a
case with a convenient host-language example. Every decision remains **open** for
the owner; this corpus answers no LQ and selects no BS option.

The standard-library-only `check_corpus.py` checks the closed object schemas,
typed literals, instruction shapes, names, module signatures, registry codes,
clause references, coverage, key recipes and hashes. It does not execute programs,
infer their safety, or establish that the expected results were observed.

## Program objects

Each case has one `program` with exactly `inputs`, `modules` and `runs`. Inputs
map names to `{type, value, origin}`. The host, never the program, supplies origin:
`manual` means a frozen solver input, `fixture` means public fixed data, and
`unknown` means missing/unreadable provenance. No program can mint provenance.
Modules map names to `{parameters, result, body}`; parameter names are ordered by
Unicode scalar order for positional arguments. A run has `id`, `profile`, `author`,
`boundary_class`, `invocation` and `body`. The host supplies these fields.

A value expression is either `{ref: name}` or `{type: T, value: literal}`.
There is no embedded source code. Types are `Int`, `Bool`, `Text`, `Word`, `Target`,
`Ints`, `Bools`, `Words`, `Targets`, `Matrix`, `Record`, `Rows`, `State`, `Receipt`
and `Bytes`. Integers have arbitrary precision; booleans are not integers. A word
is an ordered array of fake menu letters. `Rows` is a finite array of structured
data, including rational display references. `Record` is a JSON object. A state
is the four fake label entries. Bytes in a `retain` instruction are logical sizes,
not a request to create a file or allocate host memory in this packet.

Instructions are objects with precisely the fields listed below. `as` binds a
name of `type`; rebinding preserves its type. All arguments evaluate in array
order. A branch joins dependencies even when it selects constants. Loop iteration
uses the supplied order. Block-local names do not escape a loop; existing bindings
may be updated. A name assigned on both branches is available after the branch.

| op | Fields after `op` | Meaning |
| --- | --- | --- |
| `kernel` | `as, type, row, args` | Read/call a row in `fixture.json`; see `kernel-stub.md` |
| `calc` | `as, type, fn, args` | An exact operation in the table below |
| `if` | `test, then, else` | Evaluate a Bool, execute one instruction array |
| `for` | `item, type, items, body` | Bind each item of a fixed ordered list and execute the body |
| `call` | `as, type, module, args` | Call the named module; preserve effects and dependencies |
| `workspace_get` | `as, type, key` | Load the named case-local record, including its host provenance |
| `workspace_put` | `key, value` | Invoked workspace write; preserve provenance, including parts |
| `preview` | `as, type, word, target` | Request a preview with host-built provenance; `type` is Receipt |
| `pick` | `as, type, options, event` | Consume the input named by `event` as a manual zero-based index |
| `stop` | `reason` | End the run immediately; reason is `requested` |
| `return` | `value` | Return from this module or run |
| `emit` | `value` | Append an exact value to bounded output |
| `publish` | `exact, coordinates` | Emit a display plan with separately compared approximate fields |
| `retain` | `as, type, bytes` | Retain a logical byte array for the memory probes |
| `tick` | none | One charged continuation with no other action |
| `ambient` | `as, type, resource, logical_resource` | Attempt a planted ungranted access; never a grant |
| `dynamic` | `source` | Attempt dynamic evaluation of Text; subset checker must refuse |

These operations are stimuli, not promises that a product ships any corresponding
entry. In particular, arithmetic, ranking and elimination in a negative case do
not create an approved base tier or a trusted declassifier.

| fn | Exact meaning |
| --- | --- |
| `identity` | Preserve value and all provenance |
| `index`, `field` | Zero-based list item or named record field; join source and index dependencies |
| `eq`, `gt` | Exact equality or integer greater-than; no approximate input |
| `argmax_first`, `argmin_first` | Index of extremum; ties use the earliest supplied position |
| `rank_desc_indices` | Sort indexes by decreasing integer value, then increasing original index |
| `append`, `singleton` | Append one item without reordering; construct a one-item list |
| `sum`, `count_equal`, `length`, `add` | Left-to-right exact sum, exact count, literal length, integer addition |
| `mod_dot` | Left-to-right sum of pairwise products, reduced to [0,p) |
| `linear_solve_mod` | Gaussian elimination over p=2 or 5; scan columns left to right, first nonzero pivot row, normalize pivot, eliminate rows in increasing order; free variables zero. The frozen identity matrices have unique solutions. Charge each scalar operation. |
| `combine_corrections` | In row order concatenate correction word i exactly coefficient i times; no reduction of literal letters |
| `concat`, `inverse` | Literal chronological concatenation; reverse and replace by fixture inverse letters |
| `zip` | Pair corresponding entries; equal-length operands in this corpus |
| `indexed_points`, `point` | Rows [index,value] in input order; one row [x,y] |

For the experiment meter, each scalar arithmetic/comparison and each visited
collection element is a separate charged continuation in the stated algorithm.
An integer operation's weight is max(1, ceil(maximum operand magnitude bit length
/ 64)); no host narrowing is allowed. A Bool comparison costs one; a Text
comparison visits Unicode scalars in order through the first difference or end.
Scalar equality on structured values visits their canonical ordered components.
Length is constant-time metadata and has no extra constituent charge. Identity,
field/index access and singleton have no extra constituent charge. Collection
construction/append/zip/concatenation/inversion visits each output element once;
sum/count/dot/extremum additionally charge their arithmetic/comparisons in order.
Descending ranking uses insertion sort by the stated pair of keys, preserving
the exact comparison order. Elimination follows the stated pivot order, with a
linear scan of residues 1 through p-1 to find an inverse. Combining corrections
visits each coefficient and emitted letter. These are finite fixture tariffs,
not an LP1 contract decision; a candidate must map them before claiming a key.

`identity`, primitive field/index access and literal iteration retain the join of
their operands, not an invented fresh origin. `concat` and `inverse` are the
enumerated transparent named-word operations. All other word/target-producing
calculations add `computed`. Nothing removes `state`, `research` or `unknown`
except an enabled fixed declassifier with its frozen preconditions. Both possible
branch dependencies are retained statically; the dynamic form additionally knows
the executed path. No stop-sensitive repair is assumed.

## Runs, outcomes and comparison

Each case starts from a fresh copy of the fake fixture. Runs inside a case are
sequential, each separately invoked by the host. They share that case's workspace
and pending preview; they do not share execution counters. This is necessary for
laundering and accumulation. A program cannot invoke the next run. Captured picks
are fixed inputs, never automatically synthesized clicks.

Every `preview` calls the adapter described in `kernel-stub.md`; the program
cannot set its provenance or destination. Tool-authoring and research requests
are captures. Solving requests are subject to the interim policy. A successful
request changes only the fake pending preview. There is no commit operation.

An allowed case's `expected.value` is its returned value. `emitted`, `display`,
`previews` and `workspace_writes` are separately asserted observations; in the
canonical exact payload, ordered output/provenance events live in `traces` and
returned literal words in `words`. Display coordinates are compared separately.
Preview observations use `{word,target,destination}`, never a launch-local ID.
Actual traces must also carry the host provenance described in `kernel-stub.md`.
Absent observations are empty arrays or null, not unexamined wildcards.
For a display the adapter attaches `source_digest` (the canonical neutral program
digest) and `input_digest` (the canonical fixture session digest); expected plans
freeze both. The program supplies neither host attachment. A later candidate
also pins the mapping to its actual executed source bytes; these neutral bindings
do not replace that approval.

Must-refuse cases require their exact code and preserved session fields. At the
denied operation, the previous pending preview and existing record provenance
stay unchanged. Earlier permitted workspace writes are not retroactively undone.
For stop/emission cases, leaving a prohibited selected preview pending is failure,
even if no puzzle state was committed. For multi-run cases, report every run and
the terminal gated use; a harmless preparation run is not itself a refusal.

Predictions under `forms` are textual expectations, not exemptions or evidence.
In particular, ordinary labels miss the unfixed stop/emission and accumulation
channels. An admission denial can satisfy the interim policy but does not prove
the inner label mechanism ran: record the actual refusing layer. No case is
removed, excused, weakened or relabelled after observation. False refusals use the
entire frozen 35-tool denominator, with each actual code listed and no threshold.

The active registry has no dedicated usable flow code while ERR.24 is pending.
The nearest existing policy refusal is `LG3:capability.not_granted` (BND.83);
`policy.json` records the gap. Reserved spellings are not activated. A later owner
decision changing the expected code needs a new corpus version before a new run.

## Keys and freezing

`keys.json` pins all input files, source documents and each neutral run-input key.
It is explicitly not a complete executable DET.1 ReproKey: no candidate exists.
The later renderer must additionally freeze source/dependency bytes, edition,
execution contract and charge mapping, implementation and checker identities,
effective grants, form/variant, and each consumed workspace/provenance/pick digest.
No missing binding may be filled with a current default. Actual dependency bytes
are retained or resolvable; these analysis digests alone cannot authorize execution.

`FREEZE.json` hashes every file in this directory. Ordinary entries hash raw bytes.
Self-reference has a stated normalization: its own entry is set to null, the
manifest is serialized by `profile.json`'s compact canonical JSON, and those bytes
are hashed. This is a normalized self hash, not an impossible raw self hash.
The content digest hashes the canonical filename-to-digest map excluding the
manifest. A normal acceptance invocation recomputes the manifest in memory and
compares every byte. It creates the initial manifest only when none exists; it
never overwrites a mismatching freeze. Removing a freeze to hide a change is not
a valid version transition. Any later change, including report or checker edits,
requires a new corpus version and retained prior evidence before any candidate run.

Only the initial preparation command writes `keys.json` and only before a freeze:

```text
python work/experiments/language-lx/lx3-sandbox-flow/corpus/check_corpus.py --prepare-keys
python work/experiments/language-lx/lx3-sandbox-flow/corpus/check_corpus.py
```
