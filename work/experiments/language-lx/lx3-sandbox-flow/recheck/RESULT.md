# LX-3 recheck: preservation and persisted taint

Both review findings are reproduced. The stricter preservation check changes
none of the frozen observations. The unreachable clean overwrite installs a
forbidden preview in **rt3-wasm/static_labels and rt2-py/static_labels**. These
are additional counterexamples, outside the frozen counts. No prototype, bench,
corpus, stored series, route decision or LQ was changed.

| CT-RTE-05 field | Record |
| --- | --- |
| LX card / experiment ID | LX-3, RTE.44 to RTE.46 / E-2.4-L3 recheck |
| Owner allowance | `docs/wiki/decisions/owner-decisions-2026-11-05.md`, decision 2, and the assigned recheck packet |
| Time box | what can be done now; one wrapper call of at most two hours. A reached limit is no pass. |
| Frozen inputs | lx3-neutral-v2; FREEZE.json raw SHA-256 `d7eff17503ee95d45ddca0efe38ec3bb09418b0248a6279afe2dce0a90ea6627`; content SHA-256 `c4aa5b09430d8e5dbe02c63686b4fb400b36e7e7faf82dc42113491efe1e28c7` |
| Frozen bench | part-b/MANIFEST.json raw SHA-256 `40ecd440481616942de5a6eca238232f29a7f21723f8cf5bfe5309167a19530a`; listed pins verified independently of this sibling folder |
| Candidate/local pins | Table below and MANIFEST.json, which pins retained recheck files except itself and the frozen experiment inputs, original manifests and stored evidence |
| Commands / tools | Commands below; CPython 3.15.1, Node v24.21.0, V8 13.6.233.17-node.53; standard library and Node built-ins only |
| Evidence class | Source and synthetic on Windows. No engine, GPU, native UI, product sandbox or performance evidence |
| Kill criteria | An unexplained state change, forbidden installed preview, wrong code or false refusal is negative evidence. Missing pins, broken synthetic controls or an outcome that does not repeat fail acceptance |
| Actual result | 3,654 frozen keys reproduced; zero baseline differences and zero new strict preservation failures. All 52 expressible taint cells measured; 68 unsupported form/variant cells retain explicit not_expressible reasons |
| Decision / decider | open / owner; every LQ, route, engine language, syntax and ADR decision remains open |

Raw SHA-256 of each frozen candidate's `bindings.json`:

| Candidate | Digest |
| --- | --- |
| rt1-own | `c5b2aba702743b907c21a2cab772cafa88ddf0f5880063203e7d56a5e488847d` |
| rt2-py | `77c962966daa2054be7b1647d884a33f2e43dd516ffb8168df8ddf5568830755` |
| rt2-node | `66b9e6b28bbb57791089b2449e707d489fc8313933d5ac0701e7e5c08c3c4969` |
| rt3-wasm | `3150c75020ec9ebb14ab57659f914a73789b243d16250292e1b41cffd8458a1a` |
| rt4-pyhost | `b036e0991cc9cc0d173282e6bab7eeabfe3ee4a9c488ac08746abe3e54b50a16` |
| control-open | `e96c963fb986f6a63dbc7882dfc29f52b301d38ea2d711bbb2907110ddd845f0` |
| control-closed | `62f9ee6b8b8a16c2e11cd7d2697d525fcc7cdccf2ea7c5d2bbf11967b1928ba8` |

## B-001: observed defect, unchanged frozen results

The synthetic candidate establishes a workspace record in r1, then corrupts it
or the pending preview before the expected MR07/r2 refusal.

| Synthetic operation | Frozen classifier | Strict classifier |
| --- | --- | --- |
| Change existing workspace value only | refused_expected | state_changed |
| Change existing workspace provenance only | refused_expected | state_changed |
| Replace pending preview | refused_expected | state_changed |
| Clear pending preview | refused_expected | state_changed |
| Legitimate workspace write before refusal | refused_expected | refused_expected |

Four additional guards passed: corruption followed by a sanctioned write is
still rejected; session corruption and corruption after refusal are rejected by
both classifiers; corruption confined to a second host is retained in the
strict peer audit even when the first host's record is returned. These nine
controls run in actual children. `control.py` is separate from frozen candidates.

`strict.py` checks before each mutator, at refusal and at completion. It retains
the old classifier's other predicates and exact result envelope. Both B12/shared
hosts contribute audits; B11's actual process-loss handshake passes through the
completion check. Full workspace values and provenance are compared against the
last sanctioned checkpoint. See CARD.md for the instrumentation boundary.

One fresh observation per key ran for all seven declarations, **522 keys each**.
Every comparable payload hash, expanded run key and stored outcome matched.
Normalized original summaries are compared for all seven, plus the per-key data
in rt2-node/key-outcomes.json, rt3-wasm/outcomes.json and rt4-pyhost/evidence.json
where comparable. This adds a predicate, not a ten-run reproducibility series.

**Strict preservation failures are zero in every supported form and variant**:

| Candidate | Supported variant | capability_separation | static_labels | static_runtime_labels |
| --- | --- | --- | --- | --- |
| rt1-own | source_bound | 0 | 0 | 0 |
| rt2-py | approved_subset_byte_bound | 0 | 0 | not_run |
| rt2-node | approved_subset_byte_bound | 0 | 0 | not_run |
| rt3-wasm | source_bound | 0 | 0 | 0 |
| rt4-pyhost | approved_subset_byte_bound | 0 | 0 | 0 |
| control-open | both variants, separately | 0 | 0 | 0 |
| control-closed | both variants, separately | 0 | 0 | 0 |

Other declared cells retain their stored outcomes, including the previously
documented bench exceptions; no missing record is treated as a preservation
pass or reported as a new finding. B10's frozen outcomes/reasons are unchanged.
Its corpus_issue disposition is not reopened. The closed control still refuses
before B10 admission, as in its original evidence.

B-001 disproves the old predicate's general preservation claim. It supplies no
changed frozen candidate count: none of the executed frozen candidates exhibits
this corruption under the additional predicate. This is finite evidence only.

## B-002: four additional two-run probes

`probes.json`, generated by `probe_inputs.py`, freezes the new inputs. In r1
(tool_authoring), read nu2, derive a Word through combine_corrections and persist
it. In r2 (solving), load that key and request a preview. The derived and manual
words both equal `["a"]`; their provenance distinguishes them. The r1 variants:

1. `unreachable_clean`: tainted write, unconditional stop, unreachable clean write.
2. `untaken_clean`: tainted write, clean write inside a false branch.
3. `clean_before_taint`: clean write followed by the tainted write.
4. `executed_clean`: tainted write followed by a genuinely executed clean write.

Each cell uses the unchanged renderer, checker, adapter and executor on a copy
under build/. Only new inputs receive new source/index/lowering artifacts and
approvals. Stored evidence includes their digests, both runs' executed paths,
workspace metadata, denial gates and previews. Copied sources are checked against
their probe bindings. No corpus key or corpus result is invented.

R = `refused_before_preview`, with `LG3:capability.not_granted` and no installed
preview. I = `installed_preview`, with no refusal. Expected: R/R/R/I.

| Candidate / form | Variant | Unreachable clean | Untaken clean | Clean before taint | Executed clean |
| --- | --- | --- | --- | --- | --- |
| rt1-own / capability_separation | source_bound | R | R | R | I |
| rt1-own / static_labels | source_bound | R | R | R | **R (false refusal)** |
| rt1-own / static_runtime_labels | source_bound | R | R | R | I |
| rt2-py / capability_separation | approved_subset_byte_bound | R | R | R | I |
| rt2-py / static_labels | approved_subset_byte_bound | **I (escape)** | R | R | I |
| rt2-node / capability_separation | approved_subset_byte_bound | R | R | R | **R (false refusal)** |
| rt2-node / static_labels | approved_subset_byte_bound | R | R | R | **R (false refusal)** |
| rt3-wasm / capability_separation | source_bound | R | R | R | I |
| rt3-wasm / static_labels | source_bound | **I (escape)** | R | R | I |
| rt3-wasm / static_runtime_labels | source_bound | R | R | R | I |
| rt4-pyhost / capability_separation | approved_subset_byte_bound | R | R | R | I |
| rt4-pyhost / static_labels | approved_subset_byte_bound | R | R | R | I |
| rt4-pyhost / static_runtime_labels | approved_subset_byte_bound | R | R | R | I |

All other cells are `not_expressible` with the frozen declaration's reason,
including the unsupported runtime-label forms of the unmodified RT2 runtimes.
There are 39 negative cells (37 expected refusals, two escapes) and 13 positive
cells (ten installed previews, three false refusals). All executed probes
preserve the strict host checkpoint and execute approved source spans.

The rt3-wasm escape confirms B-002: its stored r1 block still has state/computed
labels and read_state=true, but r2's load block has no labels and read_state=false.
Static analysis includes the unreachable overwrite and workspace_get uses that
mask instead of runtime metadata (`frontend.py`, `adapter.py`). The other two
forms preserve taint here. rt2-py also strongly updates its abstract workspace
and analyzes past stop (`subset.py`). Its stored block remains tainted, but its
static preview gate uses overwritten analysis marks (`bridge.py`). Capability
separation observes the persisted read_state and refuses this probe.

rt1-own conservatively joins possible writes (`rt1_check.py`): no escape here,
but a genuine clean overwrite still incurs a static false refusal. rt2-node also
joins possible writes (`checker.py`); capability mode additionally carries the
run's read-state flag into the clean stored block (`adapter.py`). Both forms
refuse the positive probe. rt4-pyhost meets all four probe expectations in each
supported form; its other frozen runtime-label counterexamples remain.

## Meaning for each stored result and the optional variant

The ten-run Windows evidence remains frozen. Existing must-refuse counts
(capability / static / runtime) stay **20/16/16** for rt1-own, **20/16/not_run**
for rt2-py and rt2-node, **18/16/16** for rt3-wasm, and **20/16/10** for rt4-pyhost,
each out of 20. Existing harmless-tool false refusals stay two for capability
separation and zero for supported label forms, each out of 35. B-002's two new
escapes and three positive-probe refusals do not enter these denominators.
Failed forms stay failed. No form gains a safety or route-admission conclusion.

`admission.py` generates **rt3-wasm+admission-preflight**, a probe only, in build/.
After the original surface/lowering byte checks and grant selection, it walks
both arms and transitive module calls and asks the unchanged policy about
state-read/preview effects before entering Wasm. It adds no case-ID condition.
MR14/r1 and MR20/r1, r2, r3 all refuse at admission with the expected LG3 code,
zero executed instructions, zero preview events and zero preservation violations.
This supports the adapter-choice reading for these cases. The frozen prototype
remains **18/20**. No full-matrix or 20/20 result is assigned to this variant.
Its before/after adapter and binding digests are stored in recheck.json.

## Production and acceptance

From the repository root; production is explicit and never called by acceptance:

```text
python -B work/experiments/language-lx/lx3-sandbox-flow/recheck/prepare.py
python -B work/experiments/language-lx/lx3-sandbox-flow/recheck/prepare.py --seal
python work/experiments/language-lx/lx3-sandbox-flow/recheck/check.py
```

Acceptance verifies corpus, bench, candidate, recheck and installed-tool pins;
runs all nine controls, every frozen key, all expressible taint cells and the
optional variant; compares the complete result with recheck.json; removes build/
also on failure; and checks retained/frozen file inventories and bytes against
entry snapshots. Eight parallel children keep the pass under the requested
approximately 15-minute budget. No timing field, report or pin is refreshed.
Negative prototype results are valid evidence, not an acceptance failure.

Items 1 and 2 were stored and passed acceptance before taint-probe work began.
Final acceptance status comes from check.py and the implementation handoff.
Independent integration review remains with the integrator; the packet excludes
the network and out-of-folder writes required for nested review-wrapper calls.

## Limits and work outside this packet

No requested probe remains unrun in an expressible cell. No outside execution is
needed for this packet on the pinned Windows tools. The original card's broader
Windows/Linux twenty-run comparison is **not run here**. To continue that card,
run the following in `work/experiments/language-lx/lx3-sandbox-flow/part-b/`, using
fresh output folders. These commands do not change stored candidate evidence.

Windows PowerShell, regenerating raw evidence for transfer:

```powershell
foreach ($candidate in 'rt1-own','rt2-py','rt2-node','rt3-wasm','rt4-pyhost') {
    python -B run_matrix.py --candidate "candidates/$candidate" --out "build/recheck-comparison/$candidate/Windows" --runs 10 --leg Windows
}
```

Linux shell with the same frozen sources and Node, then comparison after both
raw legs are present on that host:

```sh
for candidate in rt1-own rt2-py rt2-node rt3-wasm rt4-pyhost; do
    python3.14 -B run_matrix.py --candidate "candidates/$candidate" --out "build/recheck-comparison/$candidate/Linux" --runs 10 --leg headless_Linux
done
for candidate in rt1-own rt2-py rt2-node rt3-wasm rt4-pyhost; do
    python3.14 -B -m harness.report --inputs "build/recheck-comparison/$candidate/Windows" "build/recheck-comparison/$candidate/Linux" --out "build/recheck-comparison/$candidate/comparison.json"
done
```

This recheck retains its Windows executable/version pins; changing them for
another host would be new evidence, not an acceptance refresh. No GPU or decisive
timing run belongs to this packet. The card cannot show **all-program safety,
authorship, understanding, absence of covert channels, product latency or an
acceptable false-refusal threshold**, nor product containment, real-engine
correctness, general compiler soundness or cross-platform behavior.

Sources read: AGENTS.md, docs/development-guide/AGENT_BRIEFING.md, owner allowance,
work/experiments/language-lx/README.md, chapter 11 RTE.11/RTE.12/RTE.24 and
RTE.44 to RTE.46, the frozen corpus, part-B card/result/issues/interface, shared
harness and candidate adapters, checks, mechanisms and evidence. Moving chapters,
decisions and the briefing are named, not pinned as acceptance dependencies.
No network, installation, personal-session access, Git mutation or out-of-folder
write was performed. All decisions remain open for the owner.
