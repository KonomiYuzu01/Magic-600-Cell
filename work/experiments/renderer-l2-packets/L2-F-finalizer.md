# Packet L2-F: shared level 2 finalizer and runner (E-2.4-02 and E-2.4-03, level 2)

Run from the `claude/renderer-l2` checkout, with no H-06 run, gate capture or other implement call active, other than the parallel L2-G and L2-Q calls:
`python tools/agents/codex_review.py --kind implement --model gpt-6.1-sol --effort max --timeout 7200 --packet work/experiments/renderer-l2-packets/L2-F-finalizer.md`

Plan: `work/experiments/renderer-l2-packets/PLAN.md` (sections 3 and 4). Contract: `work/experiments/renderer-l2-packets/HARNESS.md`, which is binding. This packet implements its sections 7 to 10 for both candidates. L2-G (Godot) and L2-Q (Qt) write the apps in parallel, against the same contract. Where the contract leaves a choice, make it and name it in the final message.

## 1. Goal and acceptance
- Goal: one finalizer and one runner judge every level 2 run of both candidates (`sa2`, `sd`) by the same code:
  - `finalize_run.py` is the only writer of `run.json` and of the short-check, geometry, validation and refusal records (`HARNESS.md` sections 7 and 8);
  - `run_scene.ps1` is S-B's capture runner ported to framework apps (section 9).
- Acceptance check: `python work/experiments/renderer-l2/check_l2.py` exits 0 (section 10, L2-F part).
- Done when:
  - every mode, check, refusal code and output of section 7 is implemented;
  - the composite identity of section 8 is implemented;
  - `run_scene.ps1` implements section 9;
  - `check_l2.py` covers section 10 for L2-F;
  - `README.md` gives the owner's commands in the order of section 11.
- Non-goals:
  - the Godot and Qt apps and their prepare steps;
  - any change to `tools/perf/renderer_gate.py`, `tools/perf/b412_summary.py` or anything else under `tools/`;
  - running PresentMon, a framework or a GPU process in the sandbox;
  - publishing results.

## 2. Actual problem and reproduction
- S-B's probe wrote its own `run.json`, and its runner filled the swap chain and the operator's confirmation into it (`work/experiments/renderer-sb/probe/run_scene.ps1`).
- In level 2 the record has three sources:
  - the framework app's `harness.json`;
  - the DLL's `native.json`, `trace.jsonl` or `geometry.json`;
  - PresentMon's CSV.
- The gate (`tools/perf/renderer_gate.py`) trusts the declared backbuffer size and build identity, and accepts a run without `vram_peak_mb`. The plan's Astra check (20261003T213525Z-b263caa3) required each harness to prove these itself and to refuse to write a gate record otherwise: plan section 3 and `HARNESS.md` section 7.

## 3. Environment and versions
- Base: branch `claude/renderer-l2` (this worktree's HEAD).
- Python 3.14 (CPython, 64-bit), standard library only. Windows PowerShell 5.1 runs `run_scene.ps1`. Keep that file ASCII-only, because 5.1 reads a UTF-8 file without a BOM as ANSI.
- Owner's machine:
  - Windows 11 (build 26200);
  - RTX 4070 Laptop GPU;
  - PresentMon 2.6.0 at S-B's pinned path;
  - Godot 4.7.2 .NET and Qt 6.10.3 builds that the other packets prepare.
- Evidence kind in the sandbox: source/fixture only.

## 4. Necessary source and evidence
- Contract amendments made after the first call. They bind this call. This worktree's `HARNESS.md` predates them; the integrator commits them with the result:
  - New refusal `adapter`, in every mode: `environment.adapter` is not exactly the name given by the new finalizer option `--adapter <name>`, whose default is the gate's GPU, `NVIDIA GeForce RTX 4070 Laptop GPU`. `run.json` adds `l2.expected_adapter`. `run_scene.ps1` takes `-Adapter <name>` and passes it on as `--adapter`; without it the finalizer's default applies. The valid fixtures use the default name.
  - Qt's `scaling` keys are `device_pixel_ratio`, `item_width` and `item_height` (the item's size in logical pixels, as numbers) and `texture_stretch`. The `sd` `scaling` refusal: the displayed width or height in physical pixels differs from `item_width` or `item_height` times `device_pixel_ratio` by more than 0.000001 pixel, or `texture_stretch` is not `none`.
- `work/experiments/renderer-l2-packets/HARNESS.md`: binding. Read all of it; the app side (sections 2 to 6) defines what the finalizer reads.
- S-B, read-only:
  - `work/experiments/renderer-sb/probe/run_scene.ps1`: the runner to port;
  - `work/experiments/renderer-sb/probe/src/probe.cpp`, `runJson`: S-B's `run.json` fields;
  - `work/experiments/renderer-sb/probe/src/gpu.cpp`, `environment()`: the environment keys.
- The DLL's outputs, read-only:
  - `work/experiments/renderer-sa2/native/include/sa2_interop.h` (the ABI 2 section);
  - `work/experiments/renderer-sa2/native/README.md`;
  - `work/experiments/renderer-sa2/native/src/scene_record.cpp`: the exact `native.json` keys, including `turn_ms` (W3) and `label_check` (W3, W4);
  - `work/experiments/renderer-sa2/native/src/scene.cpp`: the `geometry.json` writer, with S-B's format `magic600-sb-geometry-check-v1`.
- The gate, read-only and imported: `tools/perf/renderer_gate.py` (`validate_run`, `condition_reasons`, `summarize_private`) and `tools/perf/b412_summary.py`.

## 5. Attempts so far
- Call 20261004T055817Z-915d5d9b was invalid through no fault of its own: implement runs from another checkout created `codex/*` branches while it ran, which the wrapper refuses. Its output is not used. This call starts fresh, with the amendments of section 4.

## 6. Constraints and owned files
Owned: everything under `work/experiments/renderer-l2/`:
- `finalize_run.py`
- `run_scene.ps1`
- `check_l2.py`
- `README.md`
- fixture helpers, if you need them

Change nothing else. In particular, `HARNESS.md`, `PLAN.md`, everything under `work/experiments/renderer-sa2/`, `work/experiments/renderer-sd/` and `work/experiments/renderer-sb/`, and everything under `tools/` stay byte-identical.

Rules:
- `check_l2.py`:
  - set `sys.dont_write_bytecode = True` before importing;
  - build fixtures only under `work/experiments/renderer-l2/check-<pid>/` and remove that folder on every exit, failure included;
  - never use `%TEMP%`, start a framework, PresentMon, PowerShell scripts with side effects or a GPU process;
  - it may run `powershell.exe -NoProfile -Command` with the language parser only, to parse `run_scene.ps1`, and skip that step with a printed note when PowerShell is unavailable.
- `finalize_run.py`:
  - never write outside the run directory;
  - never overwrite;
  - write each output with exclusive create (`open(..., "x")`) after building it completely in memory;
  - write JSON as UTF-8 with LF and `allow_nan=False`.
- PresentMon CSV parsing:
  - follow `renderer_gate.py` (`utf-8-sig`, `csv.DictReader(strict=True)`);
  - the integer QPC column is `QPCTime`;
  - check the complete last row by its final byte (LF), as S-B's runner does.
- Timing checks use the QPC clock of `native.json` (`markers`, `qpc_frequency`) and PresentMon's `QPCTime`, never wall time.
- `run_scene.ps1` keeps every guard of S-B's runner (section 9). It never edits a record; it only calls the finalizer.
- Line endings: LF for `.py` and `.md`, CRLF for `.ps1`.

```implement-contract
{"allowed_files": ["work/experiments/renderer-l2/*"], "acceptance_check": ["python", "work/experiments/renderer-l2/check_l2.py"], "stop_condition": "finalize_run.py implements every mode, check, refusal code and output of HARNESS.md sections 7 and 8; run_scene.ps1 implements section 9 with every guard of S-B's runner; check_l2.py passes and covers section 10 for L2-F (a valid synthetic W3 run that renderer_gate accepts and judges met over three runs, one fixture per refusal code with exactly that reason, including adapter, each finalizer mode with the app mode it expects, the geometry fixture with only harness.json and geometry.json, the condition-sample cases, the short-mode half-target and no-VRAM cases, the label-fail exit 2, the identity acceptance, output-exists, geometry and validation records, and the static runner checks); README.md gives the owner's commands"}
```

## 7. Required return format
- Implementation: changes only in the assigned worktree. The final message lists:
  - the changed files;
  - the acceptance result, with the number of fixtures per refusal code;
  - what was not verified in the sandbox (every PresentMon, framework and GPU step);
  - each choice the contract left open, and the choice made;
  - every place where S-B's runner or the gate's contract forced a deviation from `HARNESS.md`, and why;
  - open points.
