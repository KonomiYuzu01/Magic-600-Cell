# Diagrams

`diagrams.py` draws the chapters' 30 figures, Appendix A's 11 abstract shapes and
computed or recorded trees, and the unembedded step tree listed in `trees.json`
(42 SVGs in total), using Python's standard library and committed evidence only.
It does not replay scrambles, search for steps or draw puzzle states. Colours are provisional.

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
Every computed data set is checked at `Evidence.figdata`: input digests in
`figdata.json` must match current files or an explicitly pinned presentation
revision before its observations can be drawn. `CAPTION_INPUT_REVISIONS` permits
only the exact chapter digests of the caption pass. For the old renderer input,
`OBSERVATION_HELPERS_SHA256` verifies the unchanged AST of the helpers used by
`figdata.py` (`height_examples` and its parsers, and `literal_assignment`, with
their input-reading helpers and constants). The class-cell chapter claims are
also checked against the saved observations. These preservation checks leave
the recorded computation inputs untouched and run no simulation. Any other
mismatch or missing input refuses the figure and names the data set and path;
this applies equally to chapters, concepts and appendices. The renderer tests use
explicit synthetic copies when committed observations have stale provenance;
passing them does not certify that the committed data has been recomputed.

The manifest records repository-relative verification inputs, their hashes, each
output's hash and any evidence gaps. SVG metadata records the production command,
Python version and production inputs. A figure whose drawing code is unchanged
keeps its pinned production renderer, even when its caption or chapter digest
changes. `ORIGINAL_RENDERER`, `UPDATED_FIGURES` with `CHAPTER_DATA_RENDERER`,
`CONCEPT_RENDERER` and `APPENDIX_RENDERER` retain the earlier production stages.
`REVISED_FIGURES` uses the current renderer and actual production inputs for the
changed transition diagram, routed tree arrows, configured-tree titles and the
two new chapter 7 figures. The manifest's `inputs` inventory always hashes the
current verification inputs; `production_renderer_sha256` identifies the drawing's
production renderer. Shared computed-data additions alone do not change unrelated
SVG bytes or their pinned production input hashes. For another
appendix tree from a runner record, add an `{name, record, step}` entry to `trees.json`
(`magic600-tutorial-trees/1`); steps use the runner's zero-based indexes.
Trees retain every recorded part, zero-length span and reference arrow.
An `appa-` entry gets its figure number from `A-macros.md`; an unembedded tree has
no figure number. Configured-tree titles call the zero-based row value an index.
Reference arrows cross the label columns in the gaps above their target rows,
then enter through the gutter beside the part boxes. A regression check tests
every folded tree's arrow segments against all text boxes using the renderer's
text-width estimate. For an older committed
word or program, `figdata.py` loads the original named tree helpers and saves its
`appa-` tree in `figdata.json`; these figures also display end-exclusive letter
spans. Lock, shift, F2, F3 and S·T have abstract diagrams, explicitly without a
played word or action region. `appa-inventory` checks all permitted runner and
census step records for those absent rules, screening reserved paths before reads.
Committing a new runner or census record makes it stale: regenerate it with
`figdata.py --only appa-inventory`, and `--check` then passes again.
The picture `appa-late-cycle-region-after` plays the top-level `word` of
`figdata.json`, so the picture manifest (`manifest.json`) records the digest of
that whole file, and any change to it, the inventory's included, makes that
picture's entry stale for the viewer's `make_figures.py --check`. Run
`make_figures.py` without `--check` afterwards; it has no partial mode and draws
every picture again. While `word` is unchanged the PNGs stay byte for byte the
same and only that entry changes.

Chapter 7's added figures read `records/ch07-examples.json` directly.
`07-z36-phase-blocks` shows `D_z36` in the recorded fixed phase order, both height
steps and digest prefixes, and the stuck unit count and distinct recorded sizes;
the record does not assign a size to each of its 54 units. `07-checker-rows`
compares `A_d0.check` with every `B_tampered` copy, retaining every step and final
failure name, the pass flag, verdict, short end digest and exit code.

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
the integrator captures them. `catalogue.py` lists `A-*.md` after the chapters.
A PNG is drawn only when its saved bytes match `manifest.json` and its current
figure or sheet entry matches the recorded recipe, including panels and labels; a
sheet also needs every panel drawn and its recorded panel hashes equal to theirs.
SVGs must match the output digests in `svg/manifest.json`. Other saved or missing
pictures are stale; `catalogue.py --check` fails for stale chapter or appendix
references even when the catalogue text is current. Both a normal catalogue run
and `--check` inspect every Markdown image embed in `0*.md` and `A-*.md`, including
inline images. They refuse an alt text without a leading `Figure <number>:`, a
target outside `figures/png/` or `figures/svg/` (with the matching extension), or a
missing file, naming the chapter, line and target without writing the catalogue.
`diagrams.py --check` shares this validation and requires every SVG embed to name
a drawable figure, whose saved output is checked against the regeneration.
Run `test_catalogue.py` for temporary-copy embed, corruption, changed-recipe and
unchanged checks. The viewer's
capture and batch `--check` modes also hash saved PNGs, including sheets, and
report deleted or changed files without altering the saved outputs.

Framing (the viewer's `fit`, approved by the owner on 22 October 2026): a local
picture of a word that turns one cap uses `fit: cap`, so its Before and After
share the camera of that cap and its four nearest cells; a global picture with
`ghost: none` uses `fit: core`; local pictures of words that turn several caps
keep `all`, because a cap camera would leave most of their moved pieces outside.
The panels of one sheet share their framing, and a picture whose cap camera
leaves more than a tenth of its moved pieces outside keeps `all`; for these two
reasons eight panels of six sheets keep `all`.
The manifest's `framing` gives the moved pieces and how many are in frame.
Thirty-five pictures leave some outside. Five of them appear in the chapters,
and the captions of those figures (1.4, 2.2, 2.3, 3.15 and 6.12) state the
count; the other thirty are step pictures that no chapter places, and their
counts are in the manifest only.

Each observation carries its full state digest and `path:function` source. Support
rows are `[piece_id, position_before, position_after]`; pose changes include frame
changes at fixed positions. The old census has no step-30 digests: those four
fingerprints are anchored by the committed scramble and 16-move digests and by
every recorded N/off count. At the H6 handoff, the chart recomputes the H4 target;
the log's H6 `u = 0` describes an absent H6 target and is retained with that scope.
Undefined targets have null τ/u and the reason `no target`.

The b07 certificate expansion remains an explicitly marked gap. These are
headless research diagrams, not Windows, DirectX or performance evidence.
