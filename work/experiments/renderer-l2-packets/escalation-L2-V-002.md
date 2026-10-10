# Escalation packet: L2-V-002, binding the level 2 build identity to the loaded build

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: an independent diagnosis and a design ruling for finding L2-V-002. The owner sent the finding to Codex Astra on 4 October 2026 as an escalation. The level 2 candidate had no review rounds left, and the owner merged it under an owner exception while this finding was open. Claude implements the ruling afterwards, on branch `claude/renderer-l2-followup`.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`. `summary` gives:
  - the ruling: which rule binds each part to the build the process ran;
  - the threat model it covers;
  - the acceptance tests the fix needs;
  - what the existing level 2 records may claim.

  `findings` lists:
  - each defect you confirm in the current code;
  - each defect in Claude's proposal in section 5.

  Each finding carries a counterexample and an experiment. The verdict is `findings` if a change is needed; otherwise `pass` with reasons.
- Questions:
  1. Diagnosis: is L2-V-002 real, and what is its full scope? Which parts of the composite identity (HARNESS section 8) are hashed only after the run, with nothing that ties them to what the process loaded? The DLL and its shaders are tied: the DLL hashes its own file and the shader files at scene load (`renderer-sa2/native/src/scene_record.cpp` 70 to 91), and the finalizer requires equality (`finalize_run.py` 260 to 274). The open parts look like these:
     - `godot:exe`, `godot:assembly`, `godot:project.godot`, `godot:Main.tscn`;
     - `qt:exe` and every `qt:<module>`.
  2. Threat model: Claude proposes accidental replacement only. That covers a rebuild, an export, a `prepare_l2.py` run, an editor save or a file sync replacing a part:
     - while a run is in progress;
     - between the framework's load and the app's checks;
     - between the app's exit and the finalizer.

     An adversary who controls the machine is out of scope. Is that enough for the renderer selection gate's rule "claims only for the measured build identity"? If not, what more is needed?
  3. Design: is the proposal in section 5 sound, complete and minimal? Check these points in particular:
     - Is the module version ID (MVID) of a Godot.NET.Sdk 4.7.2 build a content fingerprint? Is the build deterministic by default under that SDK, and does the MVID change whenever the IL changes?
     - Is comparing `GetMappedFileNameW` with the path a reliable "this is the mapped file" test? Consider `\\?\` prefixes, case, 8.3 names, `subst`, junctions, symbolic links and network paths. Is there a simpler sound test, such as comparing file IDs?
     - Does a handle held with `FILE_SHARE_READ` alone stop rename, delete and write of a mapped image and of a plain file on NTFS, from the app's check until process exit?
     - Is holding handles needed at all, or is a start digest plus the finalizer's equality check enough for the accidental threat model?
     - Qt can load modules after the start check: plugins such as `platforms/qwindows.dll`, or modules loaded and unloaded during the run. The Qt app enumerates modules only when it writes `harness.json` (`renderer-sd/app/src/l2.cpp` 407 to 432). What rule covers late and transient modules?
  4. Existing evidence: the owner's attended level 2 runs of 4 October were finalized under the current rule. Their identities: Qt build `64f08369...` from `a52dd1c`, and Godot build `709a24eb...` from `2e1868e`. Both builds came from offline `prepare_l2.py` runs at committed heads, with `matches_head` true. `prepare_l2.py` records part digests at preparation time (`renderer-sa2/prepare_l2.py` 80 to 86), and `--skip-build` refuses changed digests. Nothing was rebuilt during the owner's runs. What may the sanitized gate summary claim for these identities, and in what words?
  5. Tests: which fixtures in `check_l2.py`, app checks in the two `check_project.py` files, and owner-machine experiments must the fix add? Each must fail on the current code.
- Out of scope:
  - every other part of level 2;
  - the separate blind-seconds candidate in the working tree (`check_blind_seconds`, its two fixtures and the `HARNESS.md` table row 220), which is under its own review;
  - performance claims;
  - findings below `major`.

## 2. Actual problem and reproduction
L2-V-002 (`major`, adopted) came from review `20261004T085033Z-8b356e3e`, the second and last scoped verification of the level 2 candidate.
- Finding as reported: `ProjectAssemblyPath` (`renderer-sa2/project/Level2.cs` 140 to 152) checks only that the file's full assembly name equals the running assembly's. The finalizer hashes the current file after the run (`finalize_run.py` 275 to 278).
- A full assembly name holds name and version metadata, not a content fingerprint.
- Godot 4.7.2 loads the project assembly from a stream and disposes the stream (`FRAMEWORK-FACTS.md` G9).
- Counterexample: Godot loads build A of `SA2Smoke.dll`. Before `ProjectAssemblyPath` runs, the file is replaced by build B with the same full name. The name check passes, the process runs A, and the finalizer records SHA-256(B) as part of the identity.
- Status:
  - An in-memory probe confirmed that the finalizer accepts replaced bytes and records their new hash.
  - The full Godot replacement sequence was not reproduced.
  - Claude's reading of the code suggests the same gap for the executables, the project files and the Qt modules. Question 1 asks you to confirm or reject that.

## 3. Environment and versions
- Branch `claude/renderer-l2-followup`, from `main` at `7057cb8`. Level 2 came in through PR #57 into `claude/renderer-sb`, then PR #54 into `main`.
- Godot 4.7.2 .NET (`4.7.2.stable.mono.official`), project `Godot.NET.Sdk/4.7.2`, `net8.0`, `EnableDynamicLoading` true. The csproj sets no `Deterministic` property.
- Qt 6.10.3 (`msvc2022_64`).
- Python 3.14, Windows 11, NTFS. The owner's machine has an RTX 4070 Laptop GPU.
- Runs are attended by the owner, started by `run_scene.ps1`, and finalized immediately after the app exits.

## 4. Necessary source and evidence
- `work/experiments/renderer-l2/finalize_run.py`:
  - `file_digest` (241);
  - `build_identity` (246 to 287);
  - the identity check and its refusal reason `identity`.
- `work/experiments/renderer-l2-packets/HARNESS.md`:
  - section 6, `files` (167 to 170);
  - section 8, the composite build identity (241 to 249);
  - section 9, the runner.
- `work/experiments/renderer-l2-packets/FRAMEWORK-FACTS.md` G9.
- `work/experiments/renderer-sa2/project/Level2.cs`:
  - 90 to 95, the `files` record;
  - 140 to 152, `ProjectAssemblyPath`.
- `work/experiments/renderer-sd/app/src/l2.cpp` 407 to 437 (`files()`, called from `write()`).
- `work/experiments/renderer-sa2/native/src/scene_record.cpp` 70 to 91 (the DLL's load-time identity).
- `work/experiments/renderer-sa2/prepare_l2.py` 80 to 104 (preparation-time part digests).
- `work/experiments/renderer-l2/check_l2.py`:
  - `identity_cases`;
  - the `identity-*` refusals.
- Godot loader at tag `4.7.2-stable`: `modules/mono/glue/GodotSharp/GodotPlugins/PluginLoadContext.cs`.
- Checks that pass on the current code:
  - `python -B work/experiments/renderer-l2/check_l2.py` (172 cases);
  - `python work/experiments/renderer-sa2/check_project.py`;
  - `python work/experiments/renderer-sd/check_project.py`.

## 5. Attempts so far
No fix has been written. Claude's proposal:
1. The app records a start digest for every part, before the trace. It reads each file once, computes SHA-256 and writes `files` as `{name: {"path": ..., "sha256": ...}}`. It binds each part to the loaded module as follows:
   - Godot assembly: the MVID of the loaded module (`typeof(Smoke).Assembly.ManifestModule.ModuleVersionId`) must equal the MVID read from the same bytes with `System.Reflection.Metadata` (`MetadataReader.GetGuid(GetModuleDefinition().Mvid)`). This replaces the full-name check.
   - Executables and loaded DLLs (`godot.exe`; the Qt exe and its modules): the path's device form must equal `GetMappedFileNameW(module base)`. The app then holds a `FILE_SHARE_READ` handle until it exits.
   - Project files read at startup (`project.godot`, `Main.tscn`): these have no loaded-object fingerprint. The app records their start digest and holds a `FILE_SHARE_READ` handle.
   - Any mismatch, or any part that cannot be opened, fails the run with a new harness reason `build-binding`, before device work.
2. The finalizer recomputes each digest after the run and requires it to equal the app's start digest. Otherwise it refuses with `identity`. The identity text stays `<name>=<sha256>`, so an unchanged build keeps its identity.
3. The Qt app also enumerates modules at start. It fails the run if a module from the executable's directory appears or disappears between start and write.

Alternatives Claude considered:
- (a) Start digest plus finalizer equality, without the MVID and mapping checks. This leaves the window between load and the app's check open.
- (b) A runner digest before launch. It brackets the load from outside, but needs a new runner output and does not catch a replacement that is later restored.
- (c) Hashing what the process loaded from memory. The assembly stream is disposed, and a mapped PE image differs from its file (relocations, section layout).

## 6. Constraints and owned files
- The future fix may touch:
  - `finalize_run.py`, `check_l2.py`, `HARNESS.md`, `FRAMEWORK-FACTS.md`;
  - `renderer-sa2/project/Level2.cs` (and `Level2Windows.cs`) and `renderer-sa2/check_project.py`;
  - `renderer-sd/app/src/l2.cpp` and `renderer-sd/check_project.py`.
- The fix must not touch S-B's runner or `tools/perf/renderer_gate.py`, and must not change the meaning of `build_identity` for an unchanged build.
- Synthetic labels only. Raw PresentMon output, traces, logs and machine diagnostics are never committed. Public text carries no paths and no user name.
- Not a critical path. The owner chose this escalation (`--model gpt-6-astra --effort ultra`, standard tier).

## 7. Required return format
One JSON object matching `schemas/review-result.schema.json`:
- `verdict`: `findings` or `pass`;
- `summary`: the ruling of section 1;
- `findings`, each with:
  - ID;
  - severity;
  - evidence as `path:line` with the file digest;
  - counterexample;
  - suggested experiment;
  - verification status.

State which claims you checked in source or by experiment and which are inference.
