# LX-3 shared bench: candidate interface v1

Experiment only; all decisions stay open. `reference.py` has policy/approval
enforcement off; its resource meter bounds recursion. Controls prove failures,
never route safety.

Supply `candidate.json`, `bindings.json`, `adapter.py`, and rendered/dependency
bytes; the controls demonstrate these formats. Declare all three forms crossed
with both variants in `candidate.json.cells`: `supported`, or `not_run` with a
reason. Every key remains present. Omission is a harness failure, never a pass.

`bindings.json` satisfies all six `keys.json.candidate_bindings_required` items:

| Required binding | Interface field |
| --- | --- |
| Build/source and executed dependency bytes | `source_files`: part-b-relative file to raw SHA-256; include adapter, declaration, rendered artifacts, full executed dependency closure and every shared harness module |
| Edition, validated execution contract, neutral charges | `edition`, `execution_contract`, `charge_mapping`; retain the renderer/checker and map each executed instruction/constituent to the shared meter before running |
| Exact kernel/checker identities | `kernel_implementation`, `checkers`; resolve their bytes in `source_files` |
| Grants, form, variant, provenance | `effective_grants` per profile; the cell; `provenance_contract` |
| Consumed records, parts, picks, prior output | `consumed_inputs` declares the host ledger contract; the host records actual value/block/parts and pick digests and the previous exact output hash |
| Neutral paths to executed bytes | `source_map[case_id][path]`: `{file,start,end,sha256}`; UTF-8 byte offsets, end exclusive; `rendered_file` identifies the principal artifact |

Render the unchanged notation mechanically; pin renderer, output and closure.
Map every branch, including unexecuted ones. `bindings.neutral_paths()` enumerates
`r1/body/0`, `r1/body/3/then/0`, `modules/name/body/0` (no loop-instance suffix).
Controls execute pinned `programs.json`, not a proposed language syntax.

After pin verification, **`execute(ctx)`** returns a host `Value`/`None` or raises
`meter.Stop`. The runner builds the result. The adapter uses:

- `ctx.program`, `ctx.run`, `ctx.case`: stimulus, invocation, host events. Each
  case has a fresh child; sequential runs share workspace/pending preview only.
- `ctx.bind_grants(subset)` before `ctx.admit()`: freeze the mechanism's effective
  subset of its declared grants. `ctx.executed(path, actual_bytes)` records byte
  approval evidence for the bytes actually executed.
- `ctx.check(operation, values, labels=..., read_state=..., **facts)` calls the
  shared policy with the candidate's computed knowledge and its fixed grants.
  Operations are kernel row names or neutral op names; facts are `resource`,
  `manual_target`, `fixed_entry`. Candidates never reimplement policy.
  `ctx.refuse(code, operation=..., layer=..., path=...)` records the actual gate.
- `ctx.meter`: `continuation(path,weight=1,loop_index=None)`, `kernel`, `bind`,
  `output`, `interrupt`; all charges precede effects. Steps precede kernel cost,
  then memory, then output. `ctx.kernel_call(row, Value_args, path)` precharges
  kernel calls, checks batch cancellation and validates cache reuse. Batch buffer
  growth is already retained; do not charge its final binding twice. Other
  bindings, including aliases/loop items, retain their full canonical value size.
- `ctx.host`: `input`, `make`, `metadata`, `workspace_get/put`, `pick`, `preview`,
  `preview_outcome`, `publish`, `trap`. `Value.data` and `Value.block` are immutable.
  Only the trusted adapter supplies mechanism knowledge to `make`; the executed
  program supplies no provenance fields. Pass all data/control dependencies.
  The host preserves stored blocks and builds source/author/profile/parts/pick
  records. `ctx.display` and `ctx.coverage` record the actual display and examined
  ordered set. See the method signatures and the non-enforcing reference for
  the complete operation wiring and pinned constituent tariff.

Solving previews replace the fake pending proposal; other profiles capture.
Separation receipts are fixed acknowledgements: gate `preview_outcome` through
policy. Traps only return markers and record calls. No commit operation exists.
Python adapters are trusted bench glue, not an OS sandbox. Publish/charge the
neutral exact plan; put display approximations only in `ctx.display`.

S01 attempts dynamic evaluation (no host `eval`). S02 changes the dependency in
`ctx.program` from return 1 to 2; `ctx.case["program"]` and its approval retain 1.
Apply that substitution to executed bytes, preserving the approval digest.
B11 kills the child at admission. B12 exercises six cache conditions, including
concurrent runs sharing an immutable value. B10 remains a corpus issue.

The binding digest expands frozen bindings with form, variant, grants and the
consumed-input ledger. Run key = SHA-256 of canonical
`{neutral_sha256,binding_sha256}`. `profile.json` pins the twelve exact fields,
excluded fields and display tolerances. Compare the full key and exact payload
(coverage/counters/stop included), plus reference and pairwise display precision.

From the repository root (CLI paths after flags are relative to `part-b/`):

```text
python work/experiments/language-lx/lx3-sandbox-flow/part-b/run_matrix.py --candidate candidates/control-open --out build/open --runs 1
python work/experiments/language-lx/lx3-sandbox-flow/part-b/run_matrix.py --candidate candidates/CANDIDATE --out build/Windows --runs 10 --leg Windows
python work/experiments/language-lx/lx3-sandbox-flow/part-b/run_matrix.py --candidate candidates/CANDIDATE --out build/Linux --runs 10 --leg headless_Linux
```

Keep both folders on the comparison host, then run from `part-b/`:
`python -B -m harness.report --inputs build/Windows build/Linux --out build/comparison.json`.
Outputs are `results.jsonl`, `run.json`, `summary.json`; a crash/hang produces an
explicit failure for every affected key. Default child timeout is 10 seconds
(`--timeout`). Large raw outputs belong under `build/`, not the retained patch.
No false-refusal threshold is imposed; every refusal and code stays in the report.
