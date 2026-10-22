# Diagrams

`diagrams.py` draws the chapters' 28 figures, Appendix A's abstract shapes and
computed trees, and every step tree listed in `trees.json`, using Python's standard
library and committed evidence only. It does
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
appendix tree from a runner record, add an `{name, record, step}` entry to `trees.json`
(`magic600-tutorial-trees/1`); steps use the runner's zero-based indexes.
Trees retain every recorded part, zero-length span and reference arrow.
An `appa-` entry gets its figure number from `A-macros.md`. For an older committed
word or program, `figdata.py` loads the original named tree helpers and saves its
`appa-` tree in `figdata.json`; these figures also display end-exclusive letter
spans. Lock, shift, F2, F3 and S·T have abstract diagrams, explicitly without a
played word or action region. `appa-inventory` checks all permitted runner and
census step records for those absent rules, screening reserved paths before reads.

`figdata.py` replays only the named development records and calls their original
count, target and pose functions to produce `figdata.json`
(`magic600-tutorial-figdata/1`). It checks recorded state digests and observations;
a mismatch fails before writing. The 3,313-letter seed 7302 replay makes a full
recomputation take several minutes. Its state is shared by the height, cap-64 and
prune computations within one invocation. No simulation cache is saved or read.
`--only FIGURE` limits computation and preserves other data sets; `--check`
recomputes the selection and compares every value and input digest without writing.
Data generation does not redraw SVGs; run `diagrams.py` afterwards.

The concept selections `ch04-point-model` and `05-class-cells-71` rebuild the
committed R22D-16 stop and the 71-off pose table, respectively. Named research
classes compute the cap patch, chronological point path, contents and owners;
the census ownership block stops before its search. Input hashes and rebuilt
state digests are checked before observations are saved. Both figures use SVG
embeds. Figure 4.3 follows section 4.10: W = C⁶, and Bad(W) comes from
`PointModel.bad`. It also shows V⁻¹(Bad(W)), the home chambers that the middle
word V W V⁻¹ takes off, measured by exact admissible replay on all home chambers
and checked against the SAFE preimage. Its table counts the chambers of each set
by home cell, including partial home-cell fibres. V belongs to another branch and
has no Q point action; the point path and identity test apply to C⁶.
Chapter 5 reports any disagreement with its example's counts in the figure.
Recompute these alone with `figdata.py --only ch04-point-model 05-class-cells-71`.

The appendix selections are `appa-inventory`, `appa-u2-tree`, `appa-h4-pair-tree`,
`appa-pure-tree`, `appa-correction-tree` and `appa-late-cycle-tree`. The first two
height trees read committed experiments, without replaying the 3,313-letter
height prefix or running a search. The pure replay is the accepted b17 word
(823 → 737 off), rather than chapter 5's rejected b05 candidate. The late data
checks the whole first orbit's word/tree hashes, then replays only factors 0 and 1.
The top-level `word` of `figdata.json` is their literal 810-letter expansion,
provided for the viewer's existing JSON-word reader; `--check` verifies it too.
Partial updates preserve that tape. Existing chapter SVG bytes and their original
production input hashes stay unchanged; the diagram manifest separately records
the current shared JSON and renderer hashes used to verify them.
Appendix A cites the word by the sha256 of its compact JSON, which other data
sets in the shared file do not change; the diagram tests check that digest.

`figures.json` adds `appa-` Before/After recipe sheets with full digests, identical
exact-pose support on both sides, and the established shrink values. Other entries
cite the chapter's existing example figures. Run the viewer's `check_recipes.py`
on the appendix selection before capture. No PNGs are generated by these scripts;
the integrator captures them. `catalogue.py` lists `A-*.md` after the chapters and
marks uncaptured recipe pictures as missing without a reason placeholder.

Each observation carries its full state digest and `path:function` source. Support
rows are `[piece_id, position_before, position_after]`; pose changes include frame
changes at fixed positions. The old census has no step-30 digests: those four
fingerprints are anchored by the committed scramble and 16-move digests and by
every recorded N/off count. At the H6 handoff, the chart recomputes the H4 target;
the log's H6 `u = 0` describes an absent H6 target and is retained with that scope.
Undefined targets have null τ/u and the reason `no target`.

The b07 certificate expansion remains an explicitly marked gap. These are
headless research diagrams, not Windows, DirectX or performance evidence.
