# Diagrams

`diagrams.py` draws the chapters' 26 abstract figures and every step tree listed in
`trees.json`, using Python's standard library and committed evidence only. It does
not replay scrambles, search for steps or draw puzzle states. Colours are provisional.

From the repository root:

```sh
python3 -I -B research/jumbling/tutorial/figures/diagrams.py
python3 -I -B research/jumbling/tutorial/figures/diagrams.py --check
python3 -I -B research/jumbling/tutorial/figures/test_diagrams.py
python3 -I -B research/jumbling/tutorial/figures/figdata.py
python3 -I -B research/jumbling/tutorial/figures/figdata.py --check
python3 -I -B research/jumbling/tutorial/figures/test_figdata.py
```

The default output is `figures/svg/`. Use `--out /tmp/tutorial-svg` for temporary
output, or `--only ch06-orbit6-tree tree-7106-19` to select figures. Partial redraws
preserve other manifest entries; `--check --only NAME` checks that selection.
`--check` regenerates into a temporary directory, compares the input and output
SHA-256 values and the actual saved SVGs, names differences and exits 1 without
changing the saved files. Tests use temporary directories exclusively.

The manifest records repository-relative verification inputs, their hashes, each
output's hash and any evidence gaps. SVG metadata records the production command,
Python version and production inputs. Figures unaffected by the computed data keep
their original SVG bytes and original production renderer digest; the manifest's
`inputs` inventory records the renderer used for the current verification, and
`production_renderer_sha256` identifies the SVG's original renderer. For another
appendix tree, add an `{name, record, step}` entry to `trees.json`
(`magic600-tutorial-trees/1`); steps use the runner's zero-based indexes.
Trees retain every recorded part, zero-length span and reference arrow.

`figdata.py` replays only the named development records and calls their original
count, target and pose functions to produce `figdata.json`
(`magic600-tutorial-figdata/1`). It checks recorded state digests and observations;
a mismatch fails before writing. The 3,313-letter seed 7302 replay makes a full
recomputation take several minutes. Its state is shared by the height, cap-64 and
prune computations within one invocation. No simulation cache is saved or read.
`--only FIGURE` limits computation and preserves other data sets; `--check`
recomputes the selection and compares every value and input digest without writing.
Data generation does not redraw SVGs; run `diagrams.py` afterwards.

Each observation carries its full state digest and `path:function` source. Support
rows are `[piece_id, position_before, position_after]`; pose changes include frame
changes at fixed positions. The old census has no step-30 digests: those four
fingerprints are anchored by the committed scramble and 16-move digests and by
every recorded N/off count. At the H6 handoff, the chart recomputes the H4 target;
the log's H6 `u = 0` describes an absent H6 target and is retained with that scope.
Undefined targets have null τ/u and the reason `no target`.

The b07 certificate expansion remains an explicitly marked gap. These are
headless research diagrams, not Windows, DirectX or performance evidence.
