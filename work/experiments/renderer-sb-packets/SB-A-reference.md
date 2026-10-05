# Packet SB-A: S-B geometry reference and asset check (part of E-2.4-01)

## 1. Goal and acceptance
- Goal: write the stdlib-only Python reference of the S-B projection and its committed outputs, and `check_assets.py`, exactly as `work/experiments/renderer-sb/SPEC.md` sections 2, 3 and 7 define them. The D3D12 probe (a separate packet, written in parallel) is checked against these outputs, so the SPEC is the contract; do not change it.
- Acceptance check: `python work/experiments/renderer-sb/check_assets.py` exits 0 and prints one summary line per check.
- Non-goals: anything GPU, Direct3D or C++; changes to `SPEC.md`, `cameras.json` or `workload/`; NumPy (the acceptance check runs on a Python without it).

## 2. Actual problem and reproduction
- The 0.4 native projection is not in repository source, so the probe needs its own declared reference (plan section 1, "Geometry reference"). SPEC section 3 declares it as a port of `web/renderer.js:21-34`.
- Expected: deterministic reference files that the probe's geometry-check mode can compare against within the SPEC tolerance.

## 3. Environment and versions
- Base: branch `claude/renderer-sb` (this worktree's HEAD).
- Windows 11, CPython 3.14 (64-bit), stdlib only. Evidence kind: source/fixture and synthetic geometry.

## 4. Necessary source and evidence
- `work/experiments/renderer-sb/SPEC.md`: sections 2 (assets, column-major frames, slot numbering), 3 (pipeline steps 1-7, parameters), 4 (turn data, smoothstep, sign), 7 (cameras, poses, sample definition, output files, tolerance).
- `work/experiments/renderer-sb/cameras.json`: three cameras as rotate sequences.
- `work/experiments/renderer-sb/workload/turn.json`: `plane_u`, `plane_v`, `angle`, `moving_slots`, `move_src`/`move_dst`, `inverse_src`/`inverse_dst`, `labels.revision_even_sha256` / `revision_odd_sha256`.
- `web/renderer.js:8` (parameters), `:21-34` (vertex shader), `:120` (`rotate`).
- `tools/perf/check_renderer_assets.py`: digest and count check of the five workload assets (run it as a subprocess; do not copy it). `assets/mesh_centers.f32` is not in its list; verify it against `assets/manifest.json` in `reference_geometry.py` before use.
- `assets/manifest.json`: `files` maps `assets/<name>` to SHA-256 hex.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | `turn.json` matches the engine | exported with `core.Model` by the integrator | generator then inverse returns to solved labels | passed; a rotated slot centre lands near, not on, its destination (max 0.46): recorded, visual only |

## 6. Constraints and owned files
- Never change `assets/`, `core.py`, `web/`, `SPEC.md`, `cameras.json` or `workload/`.
- `reference_geometry.py`:
  - reads the assets with `array`/`struct`, verifies each digest against the manifest, and computes in float64;
  - writes, for each camera `c0..c2` and pose `start`, `mid`, `end`, the file `reference/<camera>_<pose>.f32` with `ndc_x`, `ndc_y`, `w` per sampled vertex as f32 LE (`struct.pack('<3f', ...)` of the float64 values);
  - writes `reference/sample.u32` and `reference/index.json` (`format` `magic600-sb-reference-v1`: parameters, aspect, sample count, moving-sample count, per-cell vertex count 30480 for each of 600 cells as one value plus the offsets check, the SHA-256 of every reference file and of every input file);
  - has `--out <dir>` (default `reference/`) and a function that returns the file bytes in memory, for `check_assets.py`.
- `check_assets.py`:
  1. runs `tools/perf/check_renderer_assets.py` with `sys.executable` as a subprocess and requires exit 0;
  2. checks `turn.json`: format, 4,605 moving slots, 4,600 src/dst pairs both ways, every src in the moving set, `plane_u`/`plane_v` orthonormal within 1e-9; rebuilds both label arrays from `move_src`/`move_dst` with `array('I')` and compares their SHA-256 with the two digests in `turn.json`; applies the inverse and requires the solved labels again;
  3. regenerates the reference bytes in memory and compares them byte for byte with the committed files and with the digests in `index.json`.
  - It creates no files or folders (the sandbox's temp ACL breaks `tempfile.mkdtemp`), uses no network, and finishes in under 60 s.
- Commit the generated `reference/` files with the scripts: they are the immutable reference outputs the probe is checked against. Expected size about 1 MB.
- Code style: match `tools/perf/check_renderer_assets.py` (short module docstring, plain functions, `main()` returning an exit code).

```implement-contract
{"allowed_files": ["work/experiments/renderer-sb/reference_geometry.py", "work/experiments/renderer-sb/check_assets.py", "work/experiments/renderer-sb/reference/*"], "acceptance_check": ["python", "work/experiments/renderer-sb/check_assets.py"], "stop_condition": "reference_geometry.py writes the SPEC section 7 outputs, the outputs are committed under reference/, and check_assets.py passes: asset digests and counts, turn.json consistency with its label digests, and a byte-for-byte match of the regenerated reference"}
```

## 7. Required return format
- Implementation: changes only in the assigned worktree; final message lists the changed files, the acceptance result (the summary lines), the sample counts and open points.
