Changed only the six allowed files under `research/jumbling/viewer/`:

- `export_tutorial.py`: verified replay, render arrays, provenance and exact moved-piece IDs.
- `full.html`, `full.js`: canvas-only screenshot mode with emphasis and local/global views.
- `capture_tutorial.mjs`: offline capture, manifests and PNG hash checks.
- `test_export_tutorial.py`, `README.md`: regression coverage and usage instructions.

Acceptance passed: **12 tests**, including both Node syntax checks.

Capture was attempted but blocked: localhost listening returned `EPERM`, and Chromium could not launch. Browser capture and `check_full.mjs` validation remain for the integrator.