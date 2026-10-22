Changed four files in `research/jumbling/viewer/`:

- `full.js`: opaque emphasis plus 0.12-alpha ghosts protected by stencil; added `emph=off`, cap orientation and revised framing.
- `capture_tutorial.mjs`: validates, passes through and records the resolved cap.
- `test_export_tutorial.py`: added orientation and capture-plumbing checks.
- `README.md`: documented the parameters.

Acceptance passed: **15 tests**, including both Node syntax checks and orientation calculations for all 600 caps.

**+w is nearest the perspective eye**, because projection divides by `1.18 - w`. Stereo uses the opposite, −w pole.

Open: integrator captures and `check_full.mjs` remain unrun. Global stereo needs visual verification around the existing antipodal projection singularity.