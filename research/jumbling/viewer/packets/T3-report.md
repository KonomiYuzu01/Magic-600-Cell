Changed four files in `research/jumbling/viewer/`:

- `full.js`: added `ghost=near|all|none`, 0.08 ghost alpha, and hemisphere framing for global stereo. Emphasis stays opaque; local framing stays unchanged.
- `capture_tutorial.mjs`: validates, passes through and records ghost mode.
- `test_export_tutorial.py`: added parameter, manifest and topology checks.
- `README.md`: documented the options and framing.

θ = **45°**, between the farthest touching cell centre (**44.4775°**) and the nearest non-touching centre (**49.1176°**). The computation test confirms every cap plus exactly its 56 touching neighbours lies within θ.

Acceptance passed: **16 tests**, including both `node --check` runs.

Open: browser captures and `check_full.mjs` remain for the integrator.