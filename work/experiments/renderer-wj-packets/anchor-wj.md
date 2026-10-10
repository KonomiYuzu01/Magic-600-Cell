# Implement: area-centroid shrink anchors in the W-J probe, references and lattice check

## 1. Goal and acceptance
- Goal: the W-J probe, its stdlib reference, its committed reference outputs and its CPU lattice check use the shrink anchors of `work/experiments/renderer-sb/SPEC.md` section 3 ("Shrink anchors" and step 1). The owner decided the rule on 10 October 2026 (`docs/wiki/decisions/owner-decisions-2026-10-10-anchor.md`). This closes E-2.4-0J acceptance item 2 at source level.
- Acceptance check: `check_lattice.py`, then `check_wj.py`, both with the engine Python; both must exit 0 (the contract's command runs them in that order).
- Also before finishing: the probe builds with `work/experiments/renderer-wj/build.cmd`, set `M600_WJ_SERIAL_NINJA=1` in the sandbox, into a disposable directory that you remove. Then `wj_probe.exe --selftest --scene wj --menu S4 --data <dir>` passes on data from `prepare_wj.py` in a fresh directory under `Path(tempfile.gettempdir())`, which you also remove.
- Out of scope: shaders, `gpu.cpp`, the trace and revision checks, the run scripts, `RESULT.md`, everything under `research/jumbling/` and `work/experiments/renderer-sb/`, and the GPU runs.

## 2. Actual problem and reproduction
- With `assets/mesh_centers.f32` as the anchor, `check_lattice.py` exits 1. For J1 generator 1, 4,125 of the 4,605 moved stickers have shrunk bounds differing from the destination slot's by up to 0.0144216457 world units. The raw meshes agree to 1.07e-7.
- With the SPEC anchors, the integrator's NumPy computation (10 October 2026) gives a maximum bound error of 7.73e-8, with no slot above 2e-6.
- `work/experiments/renderer-sb/reference_geometry.py` provides `sticker_anchors(offsets, vertices)`, the stdlib definition. Its result is an `array('f')` of 1,732 values. Its SHA-256 for the current assets is `0b6ead284d3f62719e3e6ec893d6c199d49698d246a59164bc362446e39faca6`, the SPEC pin.

## 3. Environment and versions
- Windows 11, the engine Python `tools/.venv/engine` (CPython 3.14.7, NumPy 2.3.5) first on `PATH`.
- MSVC x64, Windows SDK 10.0.26100.0 with DXC, CMake and Ninja, as in `README.md`.
- In the Codex sandbox, `tempfile.TemporaryDirectory()` and `mkdtemp()` fail with WinError 5. Create temporary directories as `Path(tempfile.gettempdir()) / <unique name>` with a plain `mkdir()`, as `check_wj.py` does.

## 4. Necessary source and evidence
- `work/experiments/renderer-sb/SPEC.md` section 3.
- `work/experiments/renderer-wj/`:
  - `probe.cpp` (`Assets::Assets`, the selftest) and `probe.h`;
  - `reference_wj.py` (`geometry`, `build_reference`, `main`) and the `ref_*` files;
  - `lattice_wj.py` (`controls`), `check_lattice.py`, `check_wj.py`, `prepare_wj.py`, `README.md` and `PROTOCOL.md`.
- Consumers that must keep working unchanged:
  - `gpu.cpp` uploads `Assets::centers` as the anchor buffer;
  - `geometry.hlsl` and `lattice.hlsl` read it;
  - the GPU lattice check compares shrink centres and bounds with a tolerance of 2e-6.

## 5. Attempts so far
- E-2.4-0J call `20261010T024836Z-a9b1fb0b` built this probe and its checks with `mesh_centers.f32` as the anchor. The lattice item failed, as designed, and the owner then chose the anchor rule.

## 6. Constraints and owned files
- Never change `assets/`, the model identity, labels, mechanical state, the fixtures, `research/jumbling/`, `work/experiments/renderer-sb/` or the shaders.
- Import `sticker_anchors` from `reference_geometry.py`; do not copy or change it. A parallel call changes other parts of `reference_geometry.py` (`build_reference`, `ASSET_NAMES`, a new `ANCHOR_SHA256` constant). Rely only on the names `reference_wj.py` already imports, plus `sticker_anchors`.
- `reference_wj.py`:
  - take the anchors from `sticker_anchors`, and remove `mesh_centers.f32` from `NAMES`, so the index `inputs` no longer list it;
  - check the anchors' f32 little-endian SHA-256 against the SPEC pin (a local constant), and refuse on a mismatch;
  - record `"anchors": {"rule": "area-weighted triangle centroid, SPEC.md section 3", "sha256": <digest>}` in each `ref_<menu>_index.json`;
  - regenerate all `ref_*` files with the documented commands. Sample files must stay byte-identical.
- `lattice_wj.py`:
  - the anchors are `np.array(sticker_anchors(...), dtype=np.float64).reshape(433, 4)`, not `mesh_centers.f32`;
  - add a pass threshold for the centre error: `max center error` at most 2e-6, like the bound error;
  - keep the failure message and the exit code of `check_lattice.py` for a real disagreement.
- `check_lattice.py`: update the docstring; it is now expected to pass.
- `probe.cpp` / `probe.h`:
  - as in the S-B probe, `Assets::Assets` fills `centers` (433 x 4 floats) from the SPEC rule with `double` arithmetic in the SPEC order and one `float` rounding, instead of reading `mesh_centers.f32`;
  - refuse a sticker without finite positive area;
  - keep the manifest digest loop;
  - the selftest checks the anchors' SHA-256 against the pin and prints `selftest: shrink anchors SHA-256: ok`;
  - no compiler flag changes.
- `README.md` and `PROTOCOL.md`: replace the failing-lattice text with the anchor rule and the new expected result (CPU lattice check passes; the GPU lattice check is not run). Keep every other statement.
- Leave no `__pycache__`, export, build directory or other generated file in the worktree, apart from the regenerated `ref_*` files.

```implement-contract
{"allowed_files": ["work/experiments/renderer-wj/probe.cpp", "work/experiments/renderer-wj/probe.h", "work/experiments/renderer-wj/reference_wj.py", "work/experiments/renderer-wj/lattice_wj.py", "work/experiments/renderer-wj/check_lattice.py", "work/experiments/renderer-wj/README.md", "work/experiments/renderer-wj/PROTOCOL.md", "work/experiments/renderer-wj/ref_*"], "acceptance_check": ["python", "-B", "-c", "import subprocess, sys; sys.exit(subprocess.call([sys.executable, '-B', 'work/experiments/renderer-wj/check_lattice.py']) or subprocess.call([sys.executable, '-B', 'work/experiments/renderer-wj/check_wj.py']))"], "stop_condition": "check_lattice.py passes for lattice-start and lattice-retained with bound and centre errors at most 2e-6; check_wj.py passes with the regenerated ref_* files matching byte for byte; the probe builds and its selftest passes including the anchor digest line; no generated files remain outside ref_*"}
```

## 7. Required return format
- A short report: the changed files, the `check_lattice.py` lines (maximum bound and centre errors for both controls), the `check_wj.py` summary, the anchor digest in the indexes, the build and selftest output with tool versions, and anything not done.
