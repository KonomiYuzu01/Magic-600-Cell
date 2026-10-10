# Packet: J2 viewer shows a certified J1 sequence (implementation)

Run as an implementation with the default reviewer model, effort `max` and speed tier `fast`, in the worktree the wrapper creates. Change only the allowed files.

## 1. Goal and acceptance
- Goal: mid-stage row J2 of `docs/progress/1.0/jumbling-plan.md` section 2: "shows certified J1 sequences, with blocked grips and certificates". Today the viewer shows only the witness sequence of `witness.py`. Add a second scene whose every pose, twist, grip status and certificate comes from the J1 simulator (`research/jumbling/sim/`), and let the viewer show either scene.
- **Source.** Add `sim_s4_source()` to `export_scene.py`, returning the same dictionary as `witness_source()`. It uses the exact S4 menu (`sim.TwistMenu.s4`) and this deterministic script from solved:
  1. (0, q), with q the quarter-turn with Cayley parameter ω = (1, 0, 0) in the cap frame of pole 0;
  2. (d, a), with a the first third-turn of A4_d, where d is the grip nearest to pole 0 (largest n₀ · n_d, ties to the smaller index) that q realigns (q⁻¹ n_d is a pole) and whose cap meets the cap of pole 0;
  3. an attempted twist at the first blocked grip of the resulting state, with a retained element. J1 must reject it, the state must stay unchanged, and J1's straddle certificate is recorded;
  4. (d, a⁻¹), then (0, q⁻¹). The final state must equal solved by digest.
- **Data in the scene.**
  - The surveys of every exact state come from `State.survey()`, with J1's certificates.
  - The scene records J1's journal document, which carries the model, menu and contract identities.
  - J1's straddle certificates are mapped onto the fields the viewer already reads (vertex below and above, signed h, exact points); the extra J1 fields are kept.
  - The pieces in the scene are the union of the inside sets of all applied twists. Their regions come from the exporter's existing exact builder. For every exported piece, its vertex set must equal J1's `regions.vertices(p)` exactly (as a set). Any difference stops the export.
- **Files.** The new scene is written as `scene-s4.json` and `scene-s4.bin`, next to the existing `scene.json` and `scene.bin`, which stay as they are. `python research/jumbling/viewer/export_scene.py` without arguments keeps writing the witness scene.
- **Viewer.**
  - A scene selector shows "Witness E2–E4" or "J1: S4 sequence".
  - Every existing deep link (such as `#s1-g1-p7`) keeps opening the witness scene. Links to the new scene use a scene token, for example `#s4-s1-g1-p7`. The token must stay a bare `#` anchor of letters, digits and hyphens.
  - The digest check, the lattice and moving rules, and the Global and Local certificate markers apply to both scenes unchanged.
  - The README describes the second scene, its source, its checks and its links.
- **Headless check.** `check_viewer.mjs` runs its scene assertions for both scenes:
  - model counts;
  - grip counts per exact state equal to the scene's surveys;
  - signed certificate rows;
  - certificate markers keeping the sign of h;
  - the moving-piece text;
  - the digest check;
  - deep links.

  The integrator runs it outside the sandbox.
- **Acceptance.**
  - The acceptance check below exits 0 inside the sandbox. It writes `scene-s4.json` and `scene-s4.bin`, and every check in the exported header is true.
  - `node --check research/jumbling/viewer/viewer.js` passes.
  - The integrator then runs `node research/jumbling/viewer/check_viewer.mjs --vendor <dir>`, which must pass for both scenes.
- Out of scope:
  - look, colour and motion;
  - `research/jumbling/sim/` (read only);
  - `witness.py` and the existing witness scene files;
  - performance.

## 2. Actual problem and reproduction
- `export_scene.py` has one source, `witness_source()`. Its docstring already names the J1 simulator as the intended replacement, returning the same dictionary.
- The viewer loads `scene.json` only.

## 3. Environment and versions
- Branch `claude/jumbling-1-0`, committed head. Linux, Python 3.11 or later, NumPy, Node 22. Three.js 0.160.0 is loaded through the page's import map. Playwright with headless Chromium may not run inside the sandbox; the integrator runs it.

## 4. Necessary source and evidence
- `research/jumbling/viewer/export_scene.py`: `witness_source`, `export`, `main`.
- `research/jumbling/viewer/viewer.js`: `loadScene`, `readHash`, `updatePanels`, the certificate markers.
- `research/jumbling/viewer/check_viewer.mjs` and `research/jumbling/viewer/README.md`.
- `research/jumbling/sim/`:
  - `State`: `apply`, `survey`, `journal_json`, `digest`;
  - `TwistMenu.s4`, `cayley`, `a4_element`;
  - `regions.vertices`;
  - `README.md`.
- `research/jumbling/state-contract.md` section 3: the certificate definitions.

## 5. Attempts so far
| # | Step | Result |
|---|---|---|
| 1 | J2 viewer on the witness scene, with review fixes (`3487b02`) | 43/43 headless assertions; senior verification passed |
| 2 | J1 extended to the amended contract (`38652f0`) | 31 acceptance flags pass |

## 6. Constraints and owned files
- Change only the allowed files below. Do not commit; the wrapper collects the patch. The exporter decides no legality in floating point.

```implement-contract
{"allowed_files": ["research/jumbling/viewer/export_scene.py", "research/jumbling/viewer/viewer.js", "research/jumbling/viewer/index.html", "research/jumbling/viewer/check_viewer.mjs", "research/jumbling/viewer/README.md", "research/jumbling/viewer/scene-s4.json", "research/jumbling/viewer/scene-s4.bin"], "acceptance_check": ["python", "research/jumbling/viewer/export_scene.py", "--source", "sim-s4"], "stop_condition": "the S4 scene is exported from J1 with all header checks true, the viewer shows both scenes, and the acceptance check passes"}
```

## 7. Required return format
- Changes only in the assigned worktree. The final message lists:
  - the changed files;
  - the script as run (the grips, twists and blocked grip chosen);
  - the header checks;
  - the acceptance result;
  - open points.
