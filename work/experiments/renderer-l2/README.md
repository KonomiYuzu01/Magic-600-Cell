# Shared level 2 finalizer and runner

Both framework candidates use this finalizer and capture runner: `sa2` (Godot)
and `sd` (Qt). The binding interface is
[`HARNESS.md`](../renderer-l2-packets/HARNESS.md), with the L2-F packet's adapter
and Qt scaling amendments. Only `finalize_run.py` writes the final records.
The runner starts a prepared app directly, captures that process with PresentMon
when required, and passes its PID, creation time and exit code to the finalizer.
Before each launch it starts `file_guard.py`, which holds every identity file and
its folders until the finalizer has finished (L2-V-002, `PLAN-L2-V-002.md`).

## Owner commands, in contract order

Run from the repository root on Windows. Complete these steps separately for
each candidate. No implement call, H-06 run, other capture, build or competing
framework/media process may be active during a run. Keep the same prepared build
for validation, geometry, short checks and formal captures.

1. Prepare the candidate. These commands build on the owner's machine; they
   do not run here as part of fixture acceptance.

   ```powershell
   python work/experiments/renderer-sa2/prepare_l2.py
   python work/experiments/renderer-sd/prepare_l2.py
   ```

   Select one candidate and the corresponding directory printed by its prepare
   command. The runner reads that directory's `launch.json` and never builds.

   ```powershell
   $candidate = 'sa2'  # Use 'sd' for Qt.
   $build = 'work/sa2b/<stamp>'  # For Qt, use the printed work/sdb/<stamp> directory.
   $runner = 'work/experiments/renderer-l2/run_scene.ps1'
   ```

2. Validate W1 through W4, 20 seconds each. This needs neither PresentMon nor an
   administrator console. Require four passing `validation-record.json` files.

   ```powershell
   & $runner -Candidate $candidate -Build $build -Validation
   ```

3. Run geometry once. This also needs neither PresentMon nor administrator rights.
   Require a passing `geometry-record.json` with all geometry/count results.

   ```powershell
   & $runner -Candidate $candidate -Build $build -Geometry
   ```

4. In an **administrator PowerShell**, with an idle machine on mains power,
   capture a 30-second W3 short run. Require `short-check.json`. It establishes
   the presenting PID, one swap chain, complete displayed seconds and exactly
   one DLL entry per present step in the checked interval.

   ```powershell
   & $runner -Candidate $candidate -Build $build -Short -Scene w3
   ```

5. In the same administrator console, run the two negative acceptance checks.
   Each invocation deliberately stops with a finalizer refusal. Check
   `refusal.json`: the first must contain only `size-mismatch`, the second only
   `vram-missing`.

   ```powershell
   & $runner -Candidate $candidate -Build $build -Short -Scene w3 -DebugHalfTarget
   & $runner -Candidate $candidate -Build $build -Short -Scene w3 -NoVram
   ```

6. Capture W1, W2 and W4. State the actual overlay configuration, and declare
   frame generation and upscaling truthfully: the gate marks a run without
   them `conditions-missing`, and the runner refuses to start one. After each
   run, type `yes` only if you watched the entire framework window without any
   obstruction. These scenes are attribution evidence; mark them preliminary
   unless owner-attended.

   ```powershell
   & $runner -Candidate $candidate -Build $build -Scene w1 -Runs 1 -Overlays 'none running' -Declare @('frame_generation=false','upscaling=false')
   & $runner -Candidate $candidate -Build $build -Scene w2 -Runs 1 -Overlays 'none running' -Declare @('frame_generation=false','upscaling=false')
   & $runner -Candidate $candidate -Build $build -Scene w4 -Runs 1 -Overlays 'none running' -Declare @('frame_generation=false','upscaling=false')
   ```

7. The owner attends three cold W3 runs and confirms each one after it ends.
   Each run starts a fresh framework process; the runner pauses 20 seconds
   between runs, then invokes the unchanged renderer gate on the series.

   ```powershell
   & $runner -Candidate $candidate -Build $build -Scene w3 -Runs 3 -Overlays 'none running' -Declare @('frame_generation=false','upscaling=false')
   ```

Steps 4 through 7 require the pinned PresentMon 2.6.0 executable under
`$env:ProgramFiles/Intel/PresentMon/PresentMonConsoleApplication/`.
Runs are retained in `work/loop-memory/perf/renderer/<candidate>/<stamp>-<scene>-<i>`;
gate summaries use a separate `<stamp>-<scene>-summary` directory. The runner
prints the summary path. Preserve refusals and raw captures as private evidence.
It stops a series on finalizer exit 5, app exit 3, or an unexpected failed label
or geometry check. An intentional `-Inject` run may return label-failure exit 2;
its overlay declaration is `fault-injection run; not gate evidence`.

The expected adapter defaults to `NVIDIA GeForce RTX 4070 Laptop GPU` in every
finalizer mode. To verify a deliberately different adapter, supply
`-Adapter '<exact framework adapter name>'` on every relevant runner command.
The run record carries `l2.expected_adapter`; using an override is not evidence
for the owner's RTX 4070 selection gate. Qt mapping uses `item_width`,
`item_height` and `device_pixel_ratio`, with an absolute tolerance of 0.000001
physical pixel and `texture_stretch` equal to `none`.

Optional capture controls are `-TraceMs` (default 192000), `-PrerollMs` (4000),
`-TurnMs` (190, W3 only), further `-Declare` conditions and `-Inject` for W3/W4.
Runs without `-Inject` must declare `frame_generation` and `upscaling` as `true`
or `false`. State declarations truthfully. Short runs always use
30000 ms; validation always uses 20000 ms. Geometry and short each run once;
validation runs each scene once. `-Runs` controls ordinary captures only.
`-NoVram` and `-DebugHalfTarget` require `-Short`.

## Records and identity

The runner calls the finalizer while the run's guard is still holding the files:

```powershell
python -B work/experiments/renderer-l2/finalize_run.py '<run directory>' --candidate sa2 --mode run --launched-pid 4242 --launched-created <FILETIME> --app-exit 0 --overlays '<operator confirmation>'
```

Use the actual launched PID, creation time and exit code, and a confirmation
obtained after the run. A run whose guard has released can no longer be
finalized: the binding checks need the live guard and its held files, so a
direct call after the run gives `identity`. Modes `run`, `short` and `validation` expect app mode `run`; mode
`geometry` expects app mode `geometry` and only `geometry.json` from the DLL.
Exit 0 means a passing record, 2 a recorded label/geometry failure, 5 a refusal
and 1 a usage or output-I/O error. Existing finalizer outputs give exit 5 with
`output-exists` on stderr and no writes. Records use exclusive create, UTF-8,
LF and finite JSON numbers. Input files are preserved.

The composite identity covers every required framework part, the DLL and its
four shader blobs, plus canonical JSON of the flat framework configuration.
Each digest is the guard's, checked through the finalizer's own handle on the
same file (`HARNESS.md` section 7); the encoding is unchanged.
It hashes sorted `name=sha256` lines, each ending in LF. Paths and run options
do not enter the identity; changing only the DLL bytes changes it. Geometry,
validation, short and gate captures of unchanged parts/settings share it.

Choices left by the contract:

- Geometry retains the full DLL summary under `geometry`, including per-cell
  counts and the DLL's own identity, beside the composite `build`.
- PresentMon diagnostic tallies are sorted maps from mode/interval/tearing
  value to row count. They count only the selected PID in the trace interval.
- `checks` lists every refusal predicate with `pass` when a record is written;
  each predicate includes the contract's mode restriction. Validation-only
  predicates therefore impose no restriction on ordinary captures.
- Short records also retain `label_check` when present, so exit 2 keeps the
  failed check available for inspection.
- Geometry loads W1. Child environment overrides use `ProcessStartInfo` and
  never alter the runner's environment. Short mode performs one capture.

Compatibility details inherited from S-B and the gate:

- A present step is `(previous QPC, next QPC]`, as
  `renderer_gate.trace_step_reasons` implements it. Checked seconds use the
  contract's half-open intervals and only `Dropped=0` counts as displayed.
- S-B derives `changed` only when mains plus battery accounts for every
  sample; incomplete power observations derive `unknown`, even if both kinds
  occurred. This follows `gpu.cpp:environment()` rather than interpreting
  "both occurred" in the contract as sufficient when samples are unknown.
  Consistent non-mains records reach the gate and report `conditions-not-met`.
- Session queries and administrator checks apply to capture modes. Geometry
  and validation retain the mutex/process guards but do not invoke PresentMon
  or logman, preserving their promised operation without administrator rights.
- The runner's helper waits stay bounded to 30 seconds, and the capture's
  post-stop exit wait retains S-B's separate 60-second bound. It checks the
  CSV's final LF before finalization. It delegates chain selection, interval
  checks and record writing instead of editing S-B's record placeholders.

## Sandbox acceptance and remaining evidence

```powershell
python work/experiments/renderer-l2/check_l2.py
```

This uses standard-library Python fixtures in a unique folder under the system
temp directory, each held by a real `file_guard.py`, and removes them on
success or failure. It checks every refusal code,
both candidates and every mode, condition consistency, debug negative checks,
failed labels/geometry, exclusive outputs and composite identity. Three
synthetic 192-second W3 runs per candidate give the gate's `met` verdict.
It statically checks the runner and uses only the PowerShell language parser
when available; no runner script is executed.

```powershell
python -B work/experiments/renderer-l2/check_guard.py
python -B work/experiments/renderer-l2/guard_experiments.py --build work/sdb/<stamp>
```

`check_guard.py` tests the guard's protection, alias refusals and release.
`guard_experiments.py` runs the synthetic experiments of `PLAN-L2-V-002.md`
section 7, item 1, against a Qt build of `renderer-sd/build.cmd` (no window,
GPU or PresentMon); `RESULT.md` records them.

These are source/fixture results. Preparing real builds, framework startup,
foreground/window/DPI behavior, GPU validation, geometry readback, process
VRAM sampling, PresentMon/ETW capture and real gate performance remain unverified
in the sandbox. Complete the owner sequence above before making those claims.
