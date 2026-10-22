# Jumbling: observation requirements (piece filters, raw view, cut slice)

Status: **requirements draft, 10 October 2026, written at the owner's request.** It says what a solver must be able to see in a jumbled state and which data the engine and the renderer must supply for it. It is not a layout: layout, colour, material and motion belong to the design track (stage 2.3), and G1 to G6 stay with the owner. It goes to the next design-table round (front-end designer and mathematician first) and is an input to the 2.5 freeze of the render contract, the command table and the view model.

Sources:
- the restoration method: [restoration](../../../research/jumbling/theory/restoration.md) sections 2 to 4 and [open problems](../../../research/jumbling/theory/open-problems.md);
- the render data contract: [render-contract](../../../research/jumbling/render-contract.md);
- the retained filter requirements: [solving-workflow](solving-workflow.md) section 4.3, and the commands `filter.open`, `filter.preview`, `filter.apply` and `structure.layer` of the [command table](command-table.md);
- the design decisions of 2 October 2026 ([owner-decisions-2026-10-02-design](../../wiki/decisions/owner-decisions-2026-10-02-design.md)) and the charter's D8, S3 and A3;
- what the research viewer does today: [viewer README](../../../research/jumbling/viewer/README.md), full-state page.

Evidence kind: requirements and source reading. Nothing here is Windows, Direct3D 12, input or performance evidence.

## 1. What the solver has to see

The restoration method has two phases, and each needs different things on screen.

| Phase | What the solver does | What the view must show |
| --- | --- | --- |
| U, shape | Reads the seams and closes misaligned cuts until the shape is restored (N = 0) | where the shape is wrong (seams, holes, overhangs); the misalignment N_c of each cap; the domain of each piece; which caps are blocked and why; whether a twist or word the solver proposes closes a cut or lowers N; at a cap the solver names, the three-twist repairs that lower N (rule 6) |
| L, labels | The retained method (orbit-first block building) on a lattice configuration | the retained views and filters, unchanged |

At a deep stop (open problem R1), the solver also needs:
- the floor F_c = |In_c − Out_c| of each misaligned cap: at the floor, no twist of that cap alone can lower N_c;
- the off-grip seams: misaligned cells on rotated cut hyperplanes that are no cap's cut, and the caps whose regions hold them.

So the views answer two questions, matching D8:
- **Global: where is the shape wrong, and how do the wrong places relate?**
- **Local: does this cut close, and which turn of this cap lines it up?**

## 2. Rules (from existing decisions and contracts)

1. **Filters change only the drawing.** No filter, view or tint changes a state, a label, admissibility or an inside set (`AGENTS.md`, mechanics; render contract section 1).
2. **The engine decides geometry.** Every predicate that depends on geometry (inside a cap, blocked, domain, seam) is evaluated exactly by the engine and handed to the view as a piece set with its revision and SHA-256. The view never derives one from floats.
3. **One revision.** A filter result, a count and a tint belong to one revision. After a commit they are recomputed or discarded, never shown against a newer state (render contract section 7; R-19).
4. **Three scopes stay separate** (solving-workflow 4.3): the full mechanical analysis always covers all 177,120 pieces; the work filter decides what can be picked and twisted; the reference display is drawn but not pickable.
5. **Never hidden by a filter:**
   - the straddling piece and its two certificate points of the grip the solver selects or hovers;
   - every piece a previewed twist or word moves (its collateral);
   - protection conflicts (as in the retained design);
   - hidden pieces that matter to the selected action are drawn as an outline in the reference scope, with a count.
6. **The tools never choose.** A check answers about a twist or word the solver proposes. No view lists the twists that close a cut or lower N, ranks caps by how promising they are, or marks a "next move" (human-solve boundary, 2 October 2026). Showing N_c, the floor and blocked caps is allowed: these are facts about the configuration, not choices. One exception (owner, 17 October 2026: [record](../../wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md)): for a cap c the solver names, the program may list the pairs (x, d) of the three-twist repair (manual section 2.3) that lower N, each with its y and z, and the solver picks one. The list is in menu order of x, then d, marks no pair as best, and is empty when no pair at c lowers N; the program does not choose the cap and makes no twist itself. Measured on one record: 1 to 61 pairs at the cap where a repair was found, a median of 5; from step 44 on a median of 3, with a single pair at 15 of 52 steps.
7. **The scramble journal is never shown.** "Pieces moved by scramble record t" is not a filter. The solver's own twist record is allowed ("pieces moved by my twist k"; reopening, U7).
8. **Hover only inspects.** Inspecting, selecting a cap, assigning a grip and moving the camera stay separate actions.

## 3. Piece filters

### 3.1 What carries over from the retained filters

A piece has a fixed identity (its id and its home region) wherever it is. A slot is a lattice position; off the lattice a piece is in no slot.

| Retained filter | In a jumbled state |
| --- | --- |
| Live query | Unchanged: re-evaluated at each revision. |
| Frozen identities (members follow the pieces) | Unchanged. |
| Frozen positions (occupants change) | Defined only for lattice slots. An off-lattice piece is in no frozen position; the panel counts such pieces separately. |
| Orbit (36 K⁺ orbits) | By home orbit: always defined. By current orbit position: on-lattice pieces only. |
| Cap (the pieces of a cap) | The certified inside set of cap c (section 3.2). Undefined while c is blocked: the view shows the straddling piece instead. |
| Sticker label (home facet colour) | Unchanged: a sticker keeps its home facet label. |
| Frame group, Hopf fibre (S3) | On-lattice pieces only; off-lattice pieces are counted, not placed. |

### 3.2 New predicates

| Predicate | Definition | Source |
| --- | --- | --- |
| Lattice status | on or off the lattice (pose in K⁺) | `pose_lattice` |
| Domain | the pieces of one domain γK⁺: same tree positions (t_L, t_R) (restoration T6); the lattice domain is (v₀, v₀) | new per-pose field (section 8) |
| Height | tree heights (d_L, d_R), for example "height at least 2" | new per-pose field |
| Inside cap c | the certified inside set of an admissible cap c | engine classification |
| Lens of c and d | inside c and inside d | engine |
| Domain size and containment | the number of pieces of a domain, and the caps that hold it whole | engine (domains, inside sets) |
| Carrier of c | an off-lattice domain that makes up most of the inside of cap c (at least half of it); one twist of c puts it back on the lattice and lifts less than it returns | engine |
| Fragment | a small off-lattice domain enclosed by the lattice domain; a twist of any cap that holds it whole lifts far more lattice pieces than it returns (I_a@40 stop: 5 fragments of 12 to 424 pieces) | engine |
| Blocked by | the straddling piece of a blocked cap | certificates |
| Rim of c | pieces with a 3-face on a misaligned cut cell of c | engine rim data |
| Off-grip seam | pieces with a 3-face on a misaligned cell of a hyperplane that is no cap's cut | engine |
| Moved by preview | the moving set of a previewed twist or word | engine |
| Moved by my twist k | from the solver's own record | session journal (solver's moves only) |

Predicates combine with and, or and not, and save as a live query or as frozen identities. `filter.preview` shows the counts of the current revision before `filter.apply`, as in the retained command table.

### 3.3 Tints

- **Domain tint** (restoration section 3, "the solver's view"): one colour per domain, the lattice domain neutral. Domains seen so far: 6 at the I_a@40 stop, 18 at an S4@60 stop; on the W-J I_a fixture path the count passes 20 at record 82 and reaches 37 at record 300 (`research/jumbling/theory/results/paths-I_a-300.json`, series `domains`). The palette must keep about 20 domains apart in both theme families, or fall back to "largest domains coloured, the rest one colour, with a count".
- **Off-lattice tint**: already in the research viewer.
- **Height tint**: optional, a ramp over d_L + d_R.
- A tint never replaces the sticker label: the label stays readable (A3: effects never hide structure).

## 4. Raw view

**Definition.** The posed puzzle as it is: facet and sticker shrink both 1, no filter, no tint, all 177,120 pieces in their actual poses. Its outer shape is the union of the posed pieces, not the 600-cell.

**What it must show.**
- Overhangs: stickers that a jumble twist carried off the 600-cell's boundary.
- Holes: faces of pieces that a twist exposed. These faces carry no sticker: stickers lie only on the 600 boundary cells, and the asset set holds sticker meshes only (`mesh_sticker.u32`). **The research viewer cannot draw them today**; a hole is now an unexplained gap.
- By restoration M1, a cut is closed exactly when every contact across it joins one domain and nothing on it sticks out or is left open. The raw view shows the second half (holes and overhangs). A contact between two domains without a gap is visible only through the domain tint. Phase U is done when neither shows anything (N = 0, restoration M3).

**Requirements.**
1. Exposed faces are drawn in a neutral material, distinct from every sticker colour, optionally tinted by domain. This needs new geometry: the 3-faces of each piece on the cut hyperplanes of its region (section 8). Its size is not known and must be measured before the contract revision.
2. The cut hyperplane of a selected cap can be shown as a translucent overlay, without hiding geometry.
3. Projection: the existing chain (rotation Q, then perspective with eye distance d4 or stereographic from a named pole, then the 3D camera). The raw view adds one preset, **cap-centred**: stereographic from the antipode of the selected cap's pole, so that cap sits undistorted in the middle.
4. The raw view is a view mode, not a filter: switching to it changes no selection or saved filter, and switching back restores the previous shrink and filters.

**Known difficulty.** Without shrink, the 4D-to-3D projection stacks the 600 cells, so the raw view of the whole puzzle is hard to read. It is meant for one cap or a few caps in a cap-centred projection, and for the solved check at the end of phase U. Whether a full raw view is useful at all is a question for the design table.

## 5. Cut slice view (Local)

This is the new view that phase U needs most.

**Idea.** The cut hyperplane H_c of a cap c is a three-dimensional space. Every twist of c fixes the pole n_c, so it maps H_c to itself as an ordinary 3D rotation about the point where the pole meets it. On H_c, the cut cells of each domain form two layers:
- the faces of the pieces just inside the cap;
- the faces of the pieces just outside it.

Closing the cut (U2) is lining up the inside layer with the outside layer by a turn of the cap. In the slice this is a 3D picture of a combination lock.

**What it shows for a selected admissible cap c.**
1. Each cut cell of each domain in one of three states: matched on both sides by one domain, inside only, outside only.
2. The counts In_c, Out_c, A_c, N_c = In_c + Out_c − 2A_c and the floor F_c = |In_c − Out_c|. When N_c = F_c, the view says that no turn of this cap alone can lower N_c.
3. A turn preview: the solver drags or keys a turn of the cap; the inside layer rotates in the slice while the outside layer stays. The float preview is labelled as such. On release the engine's exact check tells whether this turn closes the cut, lowers N_c, or neither. The view never offers the closing turn itself (rule 6).
4. Two layouts for the design table to choose from: overlay (both layers superimposed, mismatches marked) or split (inside and outside side by side, linked cursor).

**Blocked caps.** The slice shows the straddling piece's section crossing H_c and the two certificate points, below and above. The solver then knows which piece must move before the cap can turn.

**Off-grip seams.** For a misaligned cell on a hyperplane that is no cap's cut, the same slice view is offered for that hyperplane (in the frame of its domain), with the list of caps whose regions hold the cell. A twist cannot close such a seam directly (restoration M2); this view shows which caps would have to carry it onto a cut.

## 6. Global view

- **Cap map**: all 600 caps on a fixed layout (the layout is a design-table question). Each cap shows N_c, blocked or admissible, and whether it is at its floor.
- **Totals**: N, grip N (the sum of N_c), the number of off-grip seam cells, the number of domains and off-lattice pieces, all for the current revision.
- **Linked to Local**: selecting a cap in Global opens its cut slice in Local; selecting a piece in either view selects it in both.
- **Domain tint** in the 4D projection, with the lattice domain neutral.
- **Text budget**: the trial budgets of 20, 50 and 80 words per screen by context apply. Counts are numbers, not sentences.

### 6.1 Layouts tried in a view lab (11 October 2026)

At the owner's request, a scratch WebGL 2 page (private, not in the repository) drew all 259,800 stickers of jumbled states in five layouts. It used the S-B shrink chain (facet shrink 0.76, sticker shrink 0.82, d4 = 1.18) and per-piece poses from the J1 reference; each posed sticker is drawn in the chart of the cell nearest to it. States: the W-J fixtures, the I_a@40 stop, random scrambles of 150 to 1,400 twists, and the deep scramble of section 9. Screenshots came from headless Chromium with software rendering. Nothing here is Windows, Direct3D 12, input or performance evidence.

| Layout | How it places the 600 cells | What it showed on jumbled states |
| --- | --- | --- |
| 1.0 probe perspective | rotation Q, then p = d4 · xyz / (d4 − w) | Cell size varies about 12 times between the near and the far side (1.18/0.18 against 1.18/2.18); the far half collapses into the core; a deep jumble reads as one blob. Fit for the raw view (section 4), not for Global. |
| Stereographic | x.xyz / (1 + x.w), cut to a ball around a chosen cell | Angles are kept, but sizes still vary strongly and the outer cells are very large. |
| Two hemispheres | each half stereographically, side by side, the second mirrored | Every cell appears once at moderate distortion; neighbours across the equator land far apart. |
| Hopf rings | the 20 rings below, each a circle of 30 rigid cell blocks around its base point on a sphere | Every cell has the same size and no distortion; pieces and defects can be read cell by cell; neighbours along a ring are face neighbours. |
| Ring sheet | the 20 rings as the rows of a 20 × 30 grid | The most compact overview of all 600 cells; columns line up only roughly (rows were ordered greedily). |

Facts the ring layouts use (scratch computation in floating point, script not kept; to be redone in the repository if the design table picks a ring layout):
- the 600 cells split exactly into 20 rings of 30 face-connected cells (Boerdijk–Coxeter helices), found by an exact cover over the images of one helix; a screw rotation of order 30 steps each ring along itself;
- the rings are the fibres of one Hopf fibration: with u, v spanning the best-fit plane of a ring, conj(u) · v is the same unit quaternion J for all 20 rings;
- the 20 base points u · J · conj(u) are the vertices of a regular dodecahedron (pairwise angles 41.8, 70.5, 109.5, 138.2 and 180 degrees).

Update (22 October 2026): a kept script now reproduces these facts. `tools/looklab/fixtures/make_hopf_fixture.py` (standard library only) recomputes them from `assets/model.npz` and writes `tools/looklab/fixtures/hopf-rings.json`, which the Look Lab's ring layouts use. It finds 240 helices. Their right invariants fall into ten classes of 12 helices and six of 20, and so do their left invariants. The six 20-classes on each side are exact covers, so there are 6 + 6 Hopf fibrations, and together they partition the 240 helices. Within a right cover, conj(u) · v is the same J up to sign; within a left cover, v · conj(u) is. Every cover's ring graph is 3-regular with 30 edges. In the cover the Look Lab uses (the right cover that holds helix 0), the base points are the vertices of a regular dodecahedron, the ring graph is its edge graph, and the chain step is a screw of order 30 that advances 12 degrees. The Look Lab's own rings (`tools/looklab/fixtures/rings.json`) are not one Hopf fibration: 10 of them lie in the right cover that holds helix 0 and 10 in one left cover, and their ring graph is 7-regular.

The owner's reference is Andrey Astrelin's MC7D, which folds the extra dimensions into each sticker so that the picture stays clean. The ring layouts apply the same idea to cells: the 4D place of a cell becomes a position on a ring and a ring on a sphere, while the cell itself is drawn rigidly in its own chart.

Colour reading: with one hue per home cell (600 hues), neighbouring hues cannot be told apart. Random scrambles also leave large single-colour areas: in the 1,400-twist sample 17.5% of the stickers were still in their home cell, and stickers that only 2 to 5 caps can move cover about two thirds of a cell's area. Two candidates for section 3: a filter "not in its home cell", and a per-cell count of how mixed the cell is.

Integrator's proposal, not a decision: Global = Hopf rings or ring sheet; Local = cap chart plus the cut slice (section 5); the 1.0 perspective as the raw view. Layout, colour and motion belong to the design table and the owner (G1 to G6).

## 7. Checks the solver can request

All exact, on the current revision, about a proposal the solver makes:
- would the twist (c, x) close c's cut, and what would N_c become (one cap's count, by restoration M2);
- would the word w lower N, and by how much (a sum of own-cut changes);
- why cap c cannot improve alone (its floor);
- is cap c blocked, and by which piece;
- at a cap c the solver names, which pairs (x, d) of the three-twist repair lower N, each with its y and z (the one exception to rule 6, owner, 17 October 2026).

The engine answers within the command layer's preview and gates (`fresh_preview`). Apart from the repair list, no check returns a list of good twists.

## 8. Data the engine and the render contract must add

| Data | Per | Use | Note |
| --- | --- | --- | --- |
| Domain id, tree heights (d_L, d_R) | pose | domain tint and filter, height filter | small: one row per pose in the pose table |
| Rim summary: N_c, In_c, Out_c, A_c, blocked | cap, per revision | Global cap map, floor | 600 rows; exact |
| Cut cell states of one cap | cap, on request | cut slice view | exact; computed when the solver opens the slice |
| Off-grip seam cells, with holding caps | revision | Global count, seam slice | exact |
| Repair list of one cap: the pairs (x, d) whose repair lowers N, with y and z | cap, on request | the repair list (rule 6) | exact; the research script tries every pair at the cap, 118 to 1,593 at the 63 repair steps of one record, in 3 to 5 minutes per cap |
| Exposed-face meshes | piece region | raw view holes, cut slice | new asset; size to be measured |
| Filter results | query, per revision | piece sets with SHA-256 | as the retained filters |

These extend `magic600-jumbling-render/1` and the view model of ENG-0J. They are proposals for the 2.5 freeze, not authorized changes. The research code computes all of them except the exposed-face meshes (`rim.py`, `seams.py`, `trees.py`, and `research/jumbling/restoration/11-scramble-census/scripts/density.py` for the repair list), headless and in research timings only. How fast a product engine computes them is an open engineering question, not a performance claim.

## 9. What exists today

The full-state research page (`research/jumbling/viewer/full.html`) has:
- filters all pieces, off-lattice only, on-lattice only, isolate one piece;
- off-lattice tint;
- facet and sticker shrink;
- perspective and stereographic projection;
- a blocked-grip list that highlights the straddling piece and its two certificate points;
- the swept twist animation.

It has no domain tint, no exposed faces, no cap map, no cut slice and no checks.

**Deep test input.** Random admissible scrambles leave about a third of the pieces on the lattice (I_a: 63 to 69% off from 700 to 1,400 twists; 71% after the 1,000-twist scramble of the [scramble census](../../../research/jumbling/restoration/11-scramble-census/README.md)), so they under-test every view and filter here. `research/jumbling/theory/deep_scramble.py` gives a 360-twist I_a scramble with 93.5% of the pieces off the lattice ([open problems](../../../research/jumbling/theory/open-problems.md), structure results). By the lattice-prefix rule it can follow any retained scramble, which mixes the colours as well, without changing which pieces are off the lattice. Views, filters and the cut slice are to be judged on it as well as on the W-J fixtures and the I_a@40 stop.

## 10. Questions for the design table and the owner

1. The cut slice: overlay or split layout, and how it sits in Local while Global stays visible (D8: two views only).
2. The domain palette across both theme families, and the fallback past about 20 domains.
3. The material of exposed faces, distinct from every sticker colour.
4. The cap map layout for 600 caps.
5. Whether a full-puzzle raw view is useful, or the raw view is cap-centred only.
6. The default filters and view on entering Jumble.
7. The motion of the turn preview in the slice (part of G-values; owner).
8. The Global layout for Jumble: Hopf rings, ring sheet, two hemispheres or the 1.0 perspective (section 6.1).
9. How the repair list of a cap is shown and picked from, and how the solver steps through its three twists.

Not run: no Windows, Direct3D 12, input or performance check applies to this page.
