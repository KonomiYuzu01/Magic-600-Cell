# Diagrams

`diagrams.py` draws the chapters' 26 abstract figures and every step tree listed in
`trees.json`, using Python's standard library and committed evidence only. It does
not replay scrambles, search for steps or draw puzzle states. Colours are provisional.

From the repository root:

```sh
python3 -I -B research/jumbling/tutorial/figures/diagrams.py
python3 -I -B research/jumbling/tutorial/figures/diagrams.py --check
python3 -I -B research/jumbling/tutorial/figures/test_diagrams.py
```

The default output is `figures/svg/`. Use `--out /tmp/tutorial-svg` for temporary
output, or `--only ch06-orbit6-tree tree-7106-19` to select figures. Partial redraws
preserve other manifest entries; `--check --only NAME` checks that selection.
`--check` regenerates into a temporary directory, compares the input and output
SHA-256 values and the actual saved SVGs, names differences and exits 1 without
changing the saved files. Tests use temporary directories exclusively.

The manifest records repository-relative inputs, their hashes, each output's hash
and any evidence gaps. SVG metadata also records the production command and Python
version. For another appendix tree, add an `{name, record, step}` entry to
`trees.json` (`magic600-tutorial-trees/1`); steps use the runner's zero-based indexes.
Trees retain every recorded part, zero-length span and reference arrow.

Visible data notes identify the unsupported internal counts at cap 64, individual
prune-matrix assignments, b07's absent certificate expansion, the final neighbour
count of the three-letter repair, and the full support of the twisted-cycle parts.
The height chart leaves unrecorded or undefined `u` values as gaps. No missing
observation is replaced with an estimate. These are headless research diagrams,
not Windows, DirectX or performance evidence.
