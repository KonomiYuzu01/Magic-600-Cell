# ADR-D02: the Jumble and research surfaces

Status: **Proposed** (draft of 26 November 2026). Not accepted; nothing in it is frozen. Its framework items and slot
names are proposed for UI/UX freeze round 2, with K2 ([design and art path](../../../../wiki/decisions/owner-decisions-2026-11-22-design-art-path.md)).
Its acceptance is Track J (section 14). The owner's A4 picks of 26 November 2026 and the owner's direction that open
research is ADR-D02's core are recorded in [the decision page](../../../../wiki/decisions/owner-decisions-2026-11-26-adr-d02-a4.md).

Rests on:
- ADR-D01, the design system, revisions 3a to 6, accepted and frozen at A7 ([README](README.md)). ADR-D02 is hosted in
  ADR-D01's frame and edits none of its text. Revision 6 closed the host rules for these surfaces (J-3, J-4, J-6, J-7,
  J-8, J-9, N-1); ADR-D02 adds no host rule to them.
- DT-J1's outcome and the owner's answers of 17 and 22 October 2026, including the 17 October repair-list exception
  (`docs/progress/1.0/design-table/topics/jumble-solver-aids.md`, "Outcome" onward). None of it is reopened.
- The human-solve boundary (`docs/development-guide/AGENT_BRIEFING.md`, section 2).
- The Jumble interfaces and observation pages (`docs/progress/1.0/jumbling-interfaces.md`,
  `docs/progress/1.0/jumbling-observation.md`) and the handoff pages in `docs/progress/1.0/jumbling-handoff/`.
- LP4 as landed: the editor services, the palette, the key map, recording, the 1.0 log file and the glossary
  (`docs/progress/1.0/glossary/terms.json`). LP5 as landed: the research archive, the checkers, claim bindings and the
  research panel's data interface (`docs/progress/1.0/language-lp5-plan.md`, section 6).
- The command table (`docs/progress/1.0/command-table.json`). ADR-D02 edits no row; section 15 lists its row requests.

How it was made: the plan in `docs/progress/1.0/adr-d02-plan.md`. A1 listed 285 rows against what is settled; A2 gave
each open row precedents from ADR-D01's tool analyses; A3 ran three rounds of the four design-table roles as
subscription subagents, with no paid API, and was supplemented after LP4 and LP5 landed; A4 measured the data three
picks needed and put twelve picks to the owner. Where the owner took a recommended option, this record states it
without its alternatives; the alternatives stay in the private round records.

How to read the tags. Every item ends with one tag, as in ADR-D01:
- **(framework)** A host rule for these surfaces. Scripts, settings and themes cannot change it. Proposed for freeze
  round 2.
- **(default)** The shipped default, held as data in the default profile (ADR-D01.1). The researcher or solver may
  change it, and a `.6cs` program may change it through ADR-D01.2's routes.
- **(tuned)** A value set with the owner watching (for ADR-D02, in the product shell), stored as token data. This record fixes the slot, not
  the value. Values come in freeze round 3.
- **(open)** Left to a later step (section 16).

An item marked **[data: X]** is provisional until X exists; until then the greybox shows the item's unavailable state
or its fallback.

## Context

Jumbling is an independent puzzle with its own model identity, beside 600-cell-Full, and a research line that is
frozen and handed over as data (owner, 9 and 27 October 2026). The owner set the line's goal on 27 October 2026: a
puzzle to be solved, with an open methodology for restoring any scramble of depth 3,000. The program's research module
must let a researcher do that research inside the product and produce results that the checkers confirm.

ADR-D01 hosts the solving surface. Its revision 6 already fixed how tool results, releases, previews and research
origin behave. What is left for ADR-D02 is the layout, look, words and motion of the Jumble aids and the research
panel, their slot names, and the benchmark that accepts them.

The owner's direction of 25 November 2026 governs every choice below: open research is ADR-D02's core, and inside the
frame the researcher's freedom is as high as possible.

## Facts every item rests on

- At a blocked cap, N_c and the floor are undefined. The four cap classes (blocked, closed, at floor, above floor)
  are exclusive and exhaust the caps, but only for certified, completed rim rows; a cap the survey left uncertain, or
  whose rim row is pending, is pending. Total N is not the sum of the caps' N_c.
- Total N is pending until `seam_facts(all)` completes, and incomplete after a cut-off. Revision 6's J-3 keeps
  undefined apart from pending.
- The cap-centred chart is not one-to-one when unclipped. No raw view certifies that there are no holes; N = 0 does.
- A repair row cannot carry one ordinal per piece: where the cap's region has the lattice shape, at least 2,702 pieces
  move in both step 1 and step 3.
- A held seat key carries 12 twists.
- Shaders compile offline (`docs/wiki/decisions/renderer-freeze.md`). Every encoding and knob below is constant data or
  a precompiled variant.
- Measured at A4 on the research model (not renderer or performance evidence):
  - The contact graph of the off-lattice alignment domains needs 4 colour classes at W-J record 300 and at both deep
    exports, and 4 is exact (each has 4 pairwise touching domains); 1 class suffices at the I_a@40 stop
    (`stop40_reference()` in `research/jumbling/differential/references/obs_refs.py`: the W-J I_a fixture at 40
    records, then its 62 recorded restorer twists). Contacts on off-grip cuts count.
  - Drawn whole, the seam faces of a deep export are 0.59 to 0.66 million triangles (28 to 32 MB as float32
    triangles). Between 32% and 39% of the seam-face volume is exposed; no face is covered whole.
- Two read-only measurements of A3 (M-RING, the ring sheet's neighbours; M-FOLD, the chart's folding) are redone in
  the repository before an item rests on them.

## Decision

### 0. Open research

- **ADR-D02.1** Every operation of the research module is a command row. A researcher can call each one from their
  own `.6cs` scripts, the palette and the key map, in any order. The module has no fixed path, no forced sequence and
  no step that waits for another unless its input does. (framework)
- **ADR-D02.2** Every arrangement in this record is a default. Views and panels are placed freely (ADR-D01's free
  views); a layout the researcher saves is theirs. (framework)
- **ADR-D02.3** Researchers name their own parts and records. A name is data shown wherever the part or record shows;
  it never changes a command row's identity or a record's digest. (framework)
- **ADR-D02.4** A research run chooses its seeds and depths freely within a declared budget. The budget is declared
  before the run starts, the run records it, and a run that reaches it stops as cut off (ADR-D01.61's budget-stopped
  state), never as a verdict. (framework)
- **ADR-D02.5** The frame bounds the freedom. In a solving session, research output never becomes a preview (revision
  6, J-8); built-in tools never choose, rank or apply a solving word; the repair list follows the 17 October exception
  with DT-J1's O11 (b) ΔN column and nothing more (ADR-D02.32, ADR-D02.33). Outside a solving session, research output can be viewed, copied, saved and run. Where two
  ways of building an item differ only in the researcher's freedom, the freer one is built. (framework)

### 1. Hosting in the Solve layout

- **ADR-D02.6** The Solve layout hosts Jumble with no new layout and no new dock:
  - a host-drawn Jumble cell in the top bar;
  - a Jumble mark on every view border, differing in shape from `mark.practice`;
  - the totals on Global's border (ADR-D02.22);
  - on Local's border, the named-cap cell, then the kept twist checks, Cap covers and the repair slot, expanding one
    at a time; a closed list keeps a one-line header with its count;
  - with N > 0 the strip and the rail show `numeral.undefined` as a whole, switched on the exact lattice flag and never
    on a pending N; at N = 0 the ν band stands beside the rail (ADR-D02.28).
  (framework for the switch on the exact lattice flag; default for the arrangement)
- **ADR-D02.7** Slot names: `slot.tool.jumble.{survey, straddlers, covers, release_check, repair, nu,
  corrections_check, checkpoint}`. They take ADR-D01.61's seven slot states as revision 6 amends them. Display words
  never rename a slot or a command row. (framework)

### 2. Global in Jumble

- **ADR-D02.8** Until the render contract revision R5 admits a chart, Global keeps 600-cell-Full's projection chain,
  and deep states are read through the tint modes, the focus layers and the cap map. (default)
- **ADR-D02.9** Two chart presets, both offered, the researcher switching between them at any time from the palette:
  - **Ring sheet**, `view.preset.global.ring_sheet`: the 20 Hopf rings as 20 rows of 30 rigid cell charts; cyclic
    columns with a wrap mark; the caption "only row neighbours are face neighbours". Each posed sticker's chart is fixed
    once per revision on the CPU: the nearest cell's chart, ties to the lowest cell id.
  - **Hopf rings**, `view.preset.global.hopf_rings`: 20 circles of 30 cells around a dodecahedron's vertices, in three
    dimensions, turning with the camera.
  The ring sheet is the default chart: the palette's chart row applies it unless the researcher has picked the Hopf
  rings. Each preset needs R5's admission and its own gate row before it ships. [data: R5, a gate row per
  preset] (framework for the two presets and their names; default for which is first)
- **ADR-D02.10** Under a chart preset Global draws no sweep. At a new revision, moved pieces appear at their
  nearest-cell charts with the moved set outlined for one fixed time (the half-turn rule of ADR-D02.37); Local and the
  preview view carry the motion. With a chart in Global and the slice in Local, a live twist of a cap other than the
  named one is swept by no view; the projection chain of ADR-D02.8 keeps that reading. (framework)

### 3. Entry

- **ADR-D02.11** An entry slot `view.preset.jumble_entry`, keyed to the mode. Jumble on applies it; Jumble off reverts
  what it set that the researcher has not changed since; the slot follows the adopted revision's mode, so undo, redo,
  a branch switch and a reopen agree. (framework)
- **ADR-D02.12** The entry slot's value is chrome only. It opens the cap map in Solve's bottom dock and arms the
  contact-class mode (ADR-D02.21; it draws nothing until the first jumble twist) and the never-hidden set
  (ADR-D02.26). It sets no
  threshold and no filter, and never changes a projection, a layout or a shrink. Local shows its empty state: "Name a
  cap: click a numbered cap on the cap map, or F1 › jumble.cap.name <id>" (the owner's bug report of 26 November
  2026: the earlier words did not say where to click). Readouts show true zeros. (framework for "chrome only"; default
  for the value) [data: the product core's per-piece style and line paths for whatever the contact-class mode and the
  never-hidden set draw on pieces; until then the legend shows them armed]

### 4. The cap map

- **ADR-D02.13** One canvas-drawn shell control of all 600 caps in the ring-sheet arrangement, in Solve's bottom dock,
  collapsed by default. A researcher may also place it as a free pane beside the pair. [data: M-RING redone in the
  repository, mapping each cap id to its cell of the ring sheet] (default)
- **ADR-D02.14** Each cap shows its class by pattern (ADR-D02.27): blocked, closed, at floor or above floor, for
  certified and completed rim rows only; otherwise ADR-D01's pending state. N_c shows as a tabular numeral at
  admissible caps only, never at a blocked cap. The legend reads "at floor (N_c = F_c > 0)". (framework)
- **ADR-D02.15** The threshold is a set of predicates written as formulas, `N_c ≥ k`, `N_c ≤ k` and `N_c = F_c`, with
  members, non-members and blocked caps counted apart, drawn as the Query set outline. There is no default threshold
  and no sort (DT-J1's O1 (b)). While the dock is closed, the threshold field and its count stay on Global's border.
  (framework)
- **ADR-D02.16** Naming a cap shows its 80 meeting caps in a static form that is neither an outline nor a dim. No job
  starts before a cap is named. A cap's cell names it; hovering only inspects. (framework)

### 5. The cut slice and the raw view in Local

- **ADR-D02.17** The cut slice is an overlay in Local: the outside layer filled, the inside layer as outlines at a
  smaller shrink; a held key splits the layers into two linked copies with the same rotation, scale and shell; a
  radius-shell control about the cap's pole. The turn preview is a ghost of the inside layer carrying
  `pattern.preview`, and the twist is named on release only. At a blocked cap there is no ghost and no cut-cell
  state, only the straddler sections with their point pairs. [data: R5; a second drawn view for the held split]
  (framework)
- **ADR-D02.18** The raw view is one cap-centred stereographic mode of Local, `view.mode.raw`:
  - Its clip is a half-space x·p̂ ≥ h parallel to the cut's hyperplane, set by the researcher. The default
    h = (121/125)·inradius is exactly the named cap's cut; h = 0 is 90°. The clip goes no further than 90°.
  - Within 90° the cell size varies about 2.3×, at the cut about 1.06×.
  - A static tick marks where the 80 meeting poles fall inside the clip; there is no detent.
  - One key adds the antipode chart side by side: the pole chart after −I, a proper rotation, sharing the clip and the
    handedness (a determinant sign checked at build), labelled "antipode", never "mirrored". At h = 0 whole stickers go
    to one ball by anchor, half-open, so each sticker is drawn once.
  - Every twist of the named cap is a rigid rotation about the chart's centre and keeps the clip, so Local sweeps it
    truly.
  - Caption: "No raw view certifies no holes; N = 0 does."
  [data: R5, the stereographic stage, J10 for holes, RPI.36 for the clip] (framework for the chart, the clip rule and
  the caption; default for side by side and the default h)

### 6. Targets and selection

- **ADR-D02.19** A cap's cell names the cap; a sticker selects its piece; a grip chord grips the named cap;
  certificate-point glyphs are pickable in the slice (ADR-D02.26). Each has a keyboard route. Tier 2 shows facts in a
  fixed order per kind; Tier 1 shows the kind, the id and two to four numerals. (framework)

### 7. Encodings

- **ADR-D02.20** One per-pose tint channel with exclusive modes: none, contact class, height and off-lattice. Modes are
  never stacked. (framework)
- **ADR-D02.21** **Contact classes.** An alignment domain's colour is its contact class, from a proper colouring of
  the domain contact graph, contacts on off-grip cuts included:
  - touching domains always differ in class; the lattice domain is neutral;
  - the class is drawn on the outline channel, so the fill keeps the sticker label;
  - one deterministic, sticky rule keyed to the domain ID of the interfaces' JI.53 (the coset g K+ of a pose), saved
    with the view state, so a domain keeps its class across commit, undo, redo, a branch switch and a reopen;
  - six classes; when a state needs more, an overflow state that never reuses a class across a contact and states
    its count;
  - inspecting a domain shows its exact id.
  It replaces TH-02's "one colour per domain" (owner, 26 November 2026). Its companion is a domain focus layer: one
  domain at full colour, the others dimmed, with a count. Slots: `tint.jumble.contact_class.{1, 2, 3, 4, 5, 6}`,
  `tint.jumble.contact_class.overflow`. (framework for the rule and the slots; tuned for the colours)
- **ADR-D02.22** **Totals.** Global's border shows N with its on-grip and off-grip parts (pending until
  `seam_facts(all)` covers it, incomplete after a cut-off), the alignment domains, the off-lattice pieces, n2, n4, n6
  and "above 6" with its largest height (undefined at a count of 0). Local's border shows the named cap's In, Out, A,
  N_c, floor and u. Tabular numerals in fixed slots, one neutral change mark, the unit word "seams". No gauge and no
  "lower is better" colour (DT-J1's A1 and O2 (a)). (framework)
- **ADR-D02.23** **Height ramp.** One neutral-hue lightness ramp over the pieces' heights: steps 2, 4 and 6, and one
  overflow step above 6 carrying the largest height's numeral. Off by default. A deeper step is a proved fact (height h
  needs at least h/2 more jumble twists), not a verdict. Its companion is a focus layer "height at least k" with its
  count. Slots: `tint.jumble.height.{2, 4, 6, above}`. (framework for the ramp's order and slots; tuned for the hue)
- **ADR-D02.24** **Off-lattice mode.** A neutral-dark outline on off-lattice pieces, fills untouched, read from the
  exact lattice flag, never from heights. (framework)
- **ADR-D02.25** **Exposed faces.** By default a plain neutral matte, its lightness outside the sticker band, carrying
  no status pattern. The researcher can turn on a contact-class tint of the exposed faces with the contact-class mode;
  it adds no geometry, only the per-pose class index. Until the exposed-face geometry exists, nothing is drawn in the
  hole, and the view border and Tier 1 say "exposed faces unavailable"
  (`input.exposed_geometry_unavailable`). Slots: `material.exposed_face`, `material.exposed_face.tinted`,
  `state.exposed_face.unavailable`. [data: the exposed-face geometry, R5 for the tint] (framework for the slots and the
  unavailable state; default for matte; tuned for the values)
- **ADR-D02.26** **Overlays** are lines only:
  - the named cap's cut as its trace on the surface (constant line data per cap);
  - certificate points as below/above glyph pairs;
  - hidden pieces that matter as outlines with an exact count;
  - the never-hidden set of `jumbling-observation.md`, rule 5, always on.
  The cut trace and the hidden-piece outlines show while a cap is named. Certificate-point glyphs lie outside the
  look-cost envelope: they get their own gate row (at least 1,654 points for one blocked grip at S1) and the owner's
  acceptance, with the PHS.52 look-cost revision (owner, 26 November 2026). Until both exist, the slice shows the
  points' count in their place. (framework)
- **ADR-D02.27** **Status patterns.** One pattern per status:
  - cap classes: blocked as a dark solid ground with a glyph that is not a slash; closed plain; at floor a corner
    bracket; above floor the numeral only;
  - the threshold set as the Query set outline;
  - cut cells: aligned plain, inside-only "/", outside-only "\". The two slash hatches belong to the slice's one-sided
    states only.
  Slots: `pattern.cap.{blocked, closed, at_floor, above_floor}`, `pattern.cut.{inside_only, outside_only}`.
  (framework)
- **ADR-D02.28** **The ν band.** At N = 0, a band beside the rail, aligned with its 35 cells: one 0/1 mark per cell;
  the 8 frame-group marks on their orbits' cells; the C5 values of O17 and O28 as five-position dials, never a bar; the
  nine always-zero coordinates as fixed dots. The ν marks use a shape outside the ring family. Off the lattice the band
  shows `numeral.undefined`. (framework for the band and its alignment; default for the dial)
- **ADR-D02.29** **Signatures.** A lattice-to-lattice word shows its static 35-slot signature and ν row, shipped as
  build data checked from solved. A word that leaves the lattice shows `numeral.undefined` in its slots, with its exact
  length and alphabet. A Jumble-on log tab shows residual ticks when saved at N = 0, and otherwise the undefined band
  with N. (framework)
- **ADR-D02.30** Highlight-and-dim of the named blocked cap's straddlers is a built-in focus layer, never a default.
  It is turned on with ADR-D01.46's existing focus-cycling key; once on, it stays on across named caps and dims only at
  a blocked cap; it never dims the cap map; dimmed label colours stay distinct from every undimmed label under G4.
  (framework)

### 8. Words, the repair list and checks

- **ADR-D02.31** **Word flows.** The commit bar shows a word's exact expanded length with its alphabet; then check,
  preview with "part j of p, twist k of n", and commit whole. A stored word with named spans shows as a folded tree in
  word order with expanded lengths. Playback steps by part, and twist by twist inside an opened part; above X3's
  bound it shows numerals only. A word that crosses to the base puzzle shows "On the lattice, ν = 0: labels pass to
  600-cell-Full", with reachability shown. [data: X3] (framework)
- **ADR-D02.32** **The repair list.** A host slot under the named-cap cell:
  - columns `# | x d | y z | ΔN`, in the 17 October exception's order (menu order of x, then d), with no pair marked
    as best; ΔN is a numeral in a fixed column, never a bar, colour or weight (DT-J1's O11 (b));
  - no row is focused when the list opens; arrow keys focus a row;
  - digits 1 to 9 copy rows 1 to 9, in the repair slot's focused-list context only; later rows by focus and Enter;
  - a copied row is a sealed span: removed whole, never edited; typing the same twists makes an ordinary word;
  - bar words: "Repair k of m" (ADR-D02.34), "Whole repair only";
  - empty: "Cap 150, rev r: no repair lowers N (k of k pairs tried)", with "repair" defined by the pinned rule on its
    term card; cut off: "Not found within budget (j of k tried)";
  - a completed result shows its coverage (revision 6, J-7): the empty line names the cap and revision, and a second
    line, always shown when the list is empty and on expand otherwise, names the pin, the menu and the tie rule it ran
    under. A result without coverage is an error.
  (framework)
- **ADR-D02.33** The solver picks a row; the program never picks one, never orders the rows by merit and never stages
  a row by itself. Copying a row stages it through `jumble.repair.stage` (requested as CR-15, section 15) as a sealed,
  whole-only fresh preview that writes the repair-pair record. (framework)
- **ADR-D02.34** **A row's moving sets.** Focusing a row draws one step's moving set at a time, on the outline
  channel, labelled "Repair k of m": m = 2 for a two-twist row, whose identity z is data and never a step, and m = 3
  otherwise. Step k's set is the inside set at its prefix state, after steps 1 to k−1. The steps use the preview view's
  step key or `view.preview.scrub`; no new chord. (framework)
- **ADR-D02.35** **Kept twist checks.** Under the named-cap cell, a list in release order:
  - each completed check is one row with its try number at the named cap and revision (1, 2, 3 and so on, from the
    host's release record, superseded releases counted, restarting at each commit); the session release number shows
    on hover;
  - columns `try | twist | N_c | floor | ΔN | Δn_h | Δu`, each naming its cap (N and floor of the twisted cap, Δu of the
    named one);
  - each run of consecutive superseded releases is folded into one row in its place: "not checked:" with those
    twists' glyphs and no ΔN;
  - the word check is a collapsed row of exact end-state numerals with its "lifted" field.
  (framework)
- **ADR-D02.36** The folded row of ADR-D02.35 follows the owner's reading of revision 6's J-7 and N-1 (owner,
  26 November 2026). It is written when the newer release is made, from the solver's own release record; it carries
  no field of any cancelled job; cancelling a job that is no longer the latest request changes nothing on screen
  (N-1), and cancelling the latest job returns its slot to the previous state (J-7). Test: on wj-I_a at a named cap,
  release a seat's twelve twists faster than the checks complete; then
  - the completed rows appear in increasing release order;
  - each folded row appears in the frame of the release that superseded its twists;
  - each folded row shows only "not checked:" and the twists' glyphs: no N_c, floor, ΔN, Δn_h or Δu, and no pending
    mark;
  - the screen is identical before and after every cancel of a job that is no longer the latest request.
  (framework)
- **ADR-D02.37** **Motion.** Version 2 of the event-to-ID table adds:
  - `slice.turn`, silent;
  - `jumble.sweep` and `jumble.half_turn`, cue `cue.jumble_twist`;
  - `jumble.enter` and `jumble.leave`, cue `cue.jumble`, chrome only, no sticker moves;
  - `repair.step`, cue `cue.step`;
  - `cap.name`, cue `cue.cap`;
  - `slot.fill`, a reserved row that is off, with its cue `cue.check` reserved.
  A result appears in place with no motion and no cue. The slice settles without waiting for a result. A half-turn
  cuts to its end pose and outlines its moving set for one fixed time. A refused twist gets no sweep. Every motion has
  its row and turns off with reduced motion (revision 6, J-9). (framework)
- **ADR-D02.38** **Cap covers.** A slot under the named-cap cell: the alignment-domain line, then "covered by",
  "covers" and "undefined", each in cap-id order. Hovering an id inspects that cap; the cap map gets no mark. Absence
  never reads "no": answers at blocked meeting caps are undefined and listed. Its expanded form is a table of the 80
  meeting caps, both directions, each yes, no or undefined. (framework)
- **ADR-D02.39** **ν-corrections.** Library › Built-in › ν-corrections: 36 rows in id order, each with its id, exact
  length and static ν row. Shift+Enter previews one. Space marks rows into a set with signed multiplicities, and one
  chord checks the set: "ν after this set", "labels not predicted". (framework)
- **ADR-D02.40** **Result and refusal words.** Words come from the typed result onto revision 6's slot states: empty;
  ready, "none (k of k tried)", naming its coverage as revision 6's J-7 requires; budget-stopped, "Not found within budget (j of k)" or "Incomplete: <reason>"; error;
  a cancel returns the previous state; "Not run". Each word has one non-colour mark, outside the ring family. Refusals
  name the failing fact:
  - "Refused at preview: uses a research run's output."
  - "Jumble off needs N = 0 and ν = 0.", naming N above 0, ν not 0, or reachability not shown;
  - "Jumble twists need Jumble on."
  (framework for the mapping; default for the wording)

### 9. Names, keys and terms

- **ADR-D02.41** Display names: Cap map, Cut slice, Raw view, Straddlers, Cap covers, Twist check, Word check, Repair
  list, ν marks, ν-corrections, ν-correction check, and the three checkpoint verdicts by name (retained checkpoint,
  label checkpoint, sticker test). "Release check" is a palette synonym of Twist check, and row labels are synonyms.
  An unavailable reason names the missing fact. (default)
- **ADR-D02.42** Keys: the four seat keys are held keys (ADR-D01's slot `keymap.key.held`, revision 4) with fixed
  seat glyphs, the tetrahedron
  each seat carries onto the cell; holding one shows its 12 twist keys with their rotation orders as numerals. The
  repair digits act only in the repair slot's context; the aids sit on chords. [data: CR-19, XM1] (framework for the
  held-seat form; default for the bindings)
- **ADR-D02.43** Every word these surfaces show passes LP4-11's glossary check. ADR-D02 uses the glossary's words
  (alignment domain, jumbling block, pose unit, seam, blocked, check) and phrases built from them (blocked cap, kept
  twist checks), and requests the additions and corrections of section 15. (framework)

### 10. The research output panel

- **ADR-D02.44** A centre peer pane `pane.research` in the Analyse layout, with tabs Records, Claims, Runs and
  Obligations. Lists are virtualised and grouped by stage, then family. A record opens in place with its one-paragraph
  finding, then a folded tree of named parts with their lengths. (default for the place; framework for the pane and its
  tabs)
- **ADR-D02.45** Common rules:
  - every control invokes a row through the command layer and the palette, so the panel is reachable from the keyboard
    and from scripts (ADR-D02.1);
  - the branch shows read-only;
  - "pinned: input <digest>" never turns stale on a commit;
  - a run's derived state opens in a free view, captioned "derived; reachability open", and is refused past the
    four-view envelope;
  - History shows no research runs;
  - checker stepping opens LP4's debugger pane on the check's per-claim trace.
  (framework)
- **ADR-D02.46** Verdict words, on one line with their scope, the scope wrapping and never cut, in one weight and no
  colour:
  - `Pass`: "holds within <scope>";
  - `Fail`: "fails within <scope>", and the expanded row names the first claim that does not hold, its code and the
    checker's reason;
  - `NotApplicable`: "not applicable (scope only)", counted as no check, its reason shown on expand;
  - an unavailable binding, or a ready check not run: "not checked", with its reasons and dependencies word for word,
    `historical_check_incomplete` included;
  - a cut-off: "not found within budget", never a verdict;
  - reproduction: "reproduced: bytes equal" or "differs at byte k";
  - a replay: "replayed from record, not re-executed";
  - a model's review: "reviewed by a model, not checked".
  Copy and save are allowed, with the note "Solving previews refuse it". No new provenance value. (framework)

### 11. The tuning panel

- **ADR-D02.47** A dock panel `panel.tuning`, built at the end of ADR-D02's shell. The palette opens it as a tab in
  the current layout's right dock (in Solve, beside History), so the views being judged stay visible. Its parts: Family; Look; the five seed
  knobs over live ramp strips; Compare (two tuning slots, Swap, a held key); Presets (shipped read-only, forked on
  edit); Advanced behind a switch, with search. A held colour-vision and high-contrast simulation runs over the real
  frame. In the product shell the core's frame lies beneath the Godot window, so until a core simulation stage and its
  look-cost row exist the simulation covers the chrome only and says so [data: a core simulation stage]. The existing
  Appearance panel stays beside it. Save checks contrast, a monotone height ramp and pairwise distinct contact-class colours under G4's
  simulations, and fails in place. (framework for the Save checks; default for the layout)

### 12. A round-4 moment on record

- **ADR-D02.48** At the commit that reaches N = 0, the last domain's outline fades in place. It can be turned off and
  never shows at a check or a preview. Its values and its motion-table row come in freeze round 4. (open)

### 13. The research module's working rules

- **ADR-D02.49** A researcher's records follow the research data's record formats, with long steps as trees of named
  parts (ADR-D02.3) and a one-paragraph finding. The checker replays every record twist by twist. (framework)
- **ADR-D02.50** A research run reads the frozen research data and archive; it does not run the frozen research
  scripts. Its seeds come from the declared pool (ADR-D02.52) and are made as the census made them. (framework)

### 14. Acceptance: Track J

ADR-D02 is accepted by Track J of the ADR-D03 solve-test plan, adopted by the owner on 23 November 2026, with the
owner's values of 26 November 2026.

- **ADR-D02.51** The setup: a researcher agent works only through the module, the language and scripts it writes
  itself. It runs on the owner's API only after the owner confirms the provider, the model and the budget. (framework)
- **ADR-D02.52** Seeds: the pool 7401 to 7499. The scored batches take fixed seeds in order: J2 uses 7401 to 7406
  (three at depth 10, three at depth 20); J3 uses 7407 to 7421 (three at each of depths 40, 100, 300, 1,000 and
  3,000). The researcher agent chooses seeds and depths freely from 7422 to 7499 for exploration. The pool is disjoint
  from the research seeds 7001 to 7109 and 7301 to 7307 and from the block 9000 to 9099 reserved for the shelved
  tutorial's test. (framework)
- **ADR-D02.53** Runner-time budget per depth batch of three scrambles: depths 10 and 20, one hour each; 40, 100 and
  300, twelve hours each; 1,000 and 3,000, twenty-four hours each; 24 hours of free exploration; about 110 hours in all.
  The researcher agent's API cost is separate. (framework)
- **ADR-D02.54** The bars, unchanged:
  - **J0 Validity** (gate): the run catches at least 80% of the defects planted in the module, and replays the recorded
    restoration of seed 7001 through the module with every digest matching.
  - **J1 Operability** (gate): every research operation is reachable from the keyboard; observation works as
    `jumbling-observation.md` requires; the repair list follows the 17 October boundary; built-in tools never choose
    (the boundary count is zero).
  - **J2 Reproduction** (gate): the six J2 scrambles reach the solved state, and an independent checker replays every
    record twist by twist.
  - **J3 Frontier** (scored): the fifteen J3 scrambles, run from shallow to deep, each scored against the frozen record
    at its depth.
  - **J4 Output** (gate): records in the research data's formats, long steps as trees of named parts, a one-paragraph
    finding; the checker passes every record.
  (framework)
- **ADR-D02.55** The owner's target for the jumbled puzzle (26 November 2026): substantive progress on restoration
  theory under this ADR. The target is not a time. A full run's finding states what it advances against the frozen
  research data, in checkable terms: a depth restored where the frozen runs stopped, a lower measure than the frozen
  record at the same depth (N, heights or Φ), or a claim the checker passes that the frozen archive does not hold. The
  checker confirms each stated advance; whether the progress is substantive is the owner's judgement, not a bar the run
  scores for itself. (framework)
- **ADR-D02.56** When it runs: reduced, after ADR-D02's shell, on sample data (J0 and J1); full, after K2's greybox
  wires the Jumble repair flow and the research panel to the engine (J2 to J4 and the target of ADR-D02.55). (framework)

### 15. Requests to other lines

ADR-D02 edits no command table, no ADR-D01 text, no frozen language chapter and no LP4 or LP5 crate. It asks the top
integrator for these rows and file changes. Existing rows are used as they are: `jumble.release.check`,
`jumble.covers`, `jumble.repair.list`, `jumble.corrections.check`, `jumble.correction.lookup` and `jumble.nu`
(`docs/progress/1.0/jumbling-interfaces.md`).

- **CR-14, view-kind rows:** the cap-map threshold `jumble.capmap.threshold.{set, clear}` with the formulas of
  ADR-D02.15; the slice's held split and its radius-shell control; the raw view mode and its clip; the entry slot and
  the two Global chart presets; the highlight-and-dim focus layer; a repair row's step focus; naming and un-naming a
  cap (ADR-D02.16, .19); switching the tint mode (.20); the domain and "height at least k" focus layers (.21, .23);
  marking a ν-correction into a set with a signed multiplicity (.39).
- **CR-15:** `jumble.repair.stage`: stage a picked repair row as a sealed, whole-only fresh preview that writes the
  repair-pair record.
- **CR-16, tuning:** save and apply a preset; set and swap the two tuning slots; the held compare; the held
  colour-vision and high-contrast simulation; the live-selector revert; a difference-file export.
- **CR-17, research:** a row that opens a check report's per-claim trace in LP4's debugger, and a row that opens a
  run's derived state in a free view with its own binding source.
- **CR-19, held seat keys:** chapter 05's key-map forms admit the four held seat keys of the owner's DT-M1 answer
  Q7 (a), so a held seat key with a twist key routes as one chord and passes the duplicate, prefix and layout checks.
- **Glossary (LP4-11's file):** eleven additions (Straddler, Cap status, Cap class, Named cap, Meeting cap, Contact
  class, Repair, Twist check at L1; Word check at L2; Step flag at L6; Undefined at L1); Seam becomes "a cut cell of one
  alignment domain's frame, covered by that domain on one side only", with Cut cell's 469,800 counted per frame; Clean
  grip becomes "the grip of an admissible cap"; Cap cover says "meeting cap", and the N_c, In/Out/A and Floor rows say
  they are undefined at a blocked cap.
- **The motion table** is not a command row: its version 2 (ADR-D02.37) is an item of this record.
- CR-18 is not a request of this record: A1 found its rows (`session.branch.replay`, `preview.side.*`) decided by
  ADR-D01 revision 6.

### 16. What this record does not decide

- The colours, hues, lightness steps and timings: freeze round 3. Signature moments, ADR-D02.48 included: round 4.
- A research layout (R1.C): it depends on LQ2's research profile, which the joint setting decides.
- An arrival motion for results (the row `slot.fill` with its cue `cue.check`): only if X1, X4 and X5, timed by outcome, show no
  difference, and then only for long jobs that enumerate their coverage.
- Dimming by default at a named blocked cap: never, unless the owner reopens it after X6.
- Each aid's principle record (LP4-12): every aid shown needs its own record before its help card ships.
- A Jumble on or off crossing inside one 1.0 log: `c600-log` holds one identity block; a note for LP4 and K2.
- The base puzzle's target belongs to ADR-D03's Track B, not to this record: an upper bound of 3,000 hours of real time
  for a true expert's one full solve without jumbling (owner, 26 November 2026; the reading of "real time" is recorded
  in the decision page).
- The `jumbling-interfaces.md` wording that calls a total N with missing coverage "undefined" conflicts with revision
  6's J-3; ADR-D02.22 follows J-3, and the conflict is recorded, not resolved, here.

## Data still owed

Items marked [data] wait for these; each is a lead for the engineering line, not a claim:
- R5's admission of the chart, slice and stereographic stages, with a gate row per chart preset. The drawing itself
  is built in the product shell (owner, 26 November 2026); Look Lab's L1 is retired.
- The exposed-face geometry.
- X1, X3, X4, X5 and X6 (the aid jobs' timings, the long-word bound and the dim path).
- A gate row for certificate-point glyphs: at least 1,654 points for one blocked grip at S1.
- Contact classes across commit, undo, redo, a branch switch and a reopen on a W-J fixture.
- The never-hidden set and the cut trace first in gate batch 1, on a W-J fixture and a deep export.
- A headless timing for `rim_summary` over 600 caps and `seam_facts(all)` at the two deep exports.
- M-RING and M-FOLD redone in the repository; the occlusion of both chart presets at the default camera.
- A gate row with a W-J fixture sweeping while a repair-list job and a research check job run in the runtime child.

## Evidence

Design records from source and documents, plus the A4 counts on the research model. Nothing here is Windows,
DirectX, input, performance or human evidence. The round records, the A4 data and its scripts are private working
files; the counts this record uses are stated above in full.
