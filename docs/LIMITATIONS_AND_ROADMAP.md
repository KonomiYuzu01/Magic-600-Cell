# Magic 600 Cell 0.4 limitations and next work

## Current release and planned 0.41

The [0.4 release guide](RELEASE_0_4.md) records delivered behavior and verification scope. The native workspace retains the full mechanical model, explicit operation selection, full collateral inspection, protection and recoverable sessions. Finite endgame witnesses do not prove arbitrary protected-state reachability or a human full solve. Microsoft Managed DirectX remains an external prerequisite.

Performance optimization is scheduled for the next iteration. **0.41** is planned to optimize the 0.4 program, address debugging and performance, and build a dedicated local development workbench. This is a recorded direction, not an implemented feature set or a new passing performance result. The [1.0 architecture](architecture/1.0/README.md) remains separate future documentation.

## Historical shared-runtime / 0.3 assessment

The assessment below retains its original interface, evidence and priorities. It is background for 0.4 development, not an exhaustive description of the released 0.4 workspace. Historical measurements apply only to their identified source/build.

C600 Studio adds state management and solving tools around Andrey Astrelin's Magic Puzzle Ultimate. Its full-model journal, certified operations, inspection, and structure navigation provide more than additional keybindings. These are descriptions of project capabilities and proposed work, not claims of historical priority over other puzzle software. Upstream credit and runtime boundaries are documented in [runtime provenance](RUNTIME_PROVENANCE.md).

## Historical boundaries

| Area | Current boundary | Practical consequence |
| --- | --- | --- |
| Mechanical model | One immutable full 600-cell profile, with complete sticker identities and legal generator maps. | A display filter cannot become a smaller puzzle or suppress a move's hidden collateral. |
| Cell layers | Exact shared-face BFS cell shells; whole pieces use any-hosting-cell membership. | Cell shells partition cells, but their piece selections overlap. A completed layer is not automatically protected against later moves. |
| Protection | Commit protection is expressed in complete orbits. | Protecting an arbitrary color, layer, or selected piece set requires additional full-effect checks. |
| Structure graph | A local one- or two-hop schematic adjacency graph with explicit cell and vertex lookup. | Its screen distances and angles do not measure the geometry or move group. Canonical IDs, rather than swatches, identify colors. |
| Auxiliary geometry | Global and focused views project the retained four-dimensional vertices and tetrahedral cells, with separate cameras. | Projected cells can overlap; these are navigation views, not additional sticker engines or proof of a solving strategy. Native integration passed on the tested Windows configuration; short paired-view p95 measurements passed 50 ms, with longer individual outliers. |
| Auxiliary state and counts | Active-orbit, exact-visible, and eligible per-cell incidences are distinct committed-state summaries. | Eligibility is active orbit intersected with the exact interaction mask. Multi-cell pieces occur in more than one cell's totals, and visible colors are not a substitute for exact identity/orientation correctness. |
| Auxiliary window state | Close/reopen and drawer transitions retain cameras and the pinned origin during the current application run. | Auxiliary camera, placement, and pin settings are not persisted across application restart. Mixed-monitor DPI and screen changes need integrated verification. |
| Buffer analysis | Certified fixed buffer positions and oriented frame searches. | Buffer occupants move. Arbitrary editable buffers cannot be substituted without proving guarded reachability and relocation. |
| Insertion | Finite witnessed plans with full collateral, checked against the current state. | Setup-tree depth and one-star length do not establish a globally shortest solution or a complete insertion cost. |
| Saved sets | Frozen identities or frozen positions, limited to 10,000 members. | Saving an expression does not create a live query. Use predicates for larger or state-dependent groups. |
| Reports | Orbit progress and current-branch transaction totals; an explicit session timer. | Exact identity/orientation completion differs from visible-color completion. The timer includes thinking time while running. It is not inferred active-work time. |
| Client parity | The native and browser clients have different control sets and preset workflows. | Do not assume native structure navigation or every browser preset-management action exists in both clients. |
| Rendering | A retained Windows DirectX renderer, with optional adaptive camera-motion detail. | Smooth sampled motion and full-detail rendering are different measurements. Complete visible detail must return after motion. |
| Twist presentation | Native turns appear instantly after durable commit; pre-commit twist animation is removed. | This preserves the authoritative state boundary. It does not establish a latency target or promise animated turn playback. |

Inspect current implementation and tests before extending a feature. The [structure guide](STRUCTURE_EXPLORER.md) is the authoritative user-facing contract for color and layer notation. Older zero-based filter notation must remain compatible.

The explorer uses separate Colors, Layers, and Vertices tasks, with a shared origin and persistent filter actions. Options and complete rule review are collapsible. The [development log](DEVELOPMENT_LOG.md) records the rationale and distinguishes control-fixture evidence from actual native integration and visual checks.

The auxiliary windows use the same neutral English Windows interface: compact controls, real IDs and counts, canonical palette fills, and explicit selection/hover markers. Follow/Pin changes the local neighborhood origin; Center main is separate from selection. The Views drawer keeps the geometry available on narrow or short screens without compressing the main viewport. Auxiliary rotation and selection remain available during a main-state commit, but statistics describe the last committed snapshot until the next revision arrives. Native integration passed 28 groups, and reviewed control/DirectX captures cover 1300 × 740 and 1000 × 650 windows at 96 DPI. These are not desktop-level captures or mixed-monitor tests. [Current performance observations](DEVELOPMENT_PERFORMANCE.md) record substantial gains alongside unmet instant-turn and full-detail targets and one borderline explorer activation failure. Published 0.2.4 validation applies only to its frozen release build.

## Concrete optimization work

1. **Keep navigation off the state-update path.** Fetch immutable topology once per model, then search, compare, traverse adjacency, and calculate small BFS shells locally. Hover must not call the engine, fetch a native snapshot, recolor 259,800 slots, or force a DirectX frame. Only an explicit camera or filter action should affect the viewport.
2. **Measure preview costs separately from rendering.** Record cold and warm expression parsing, mask evaluation, rule composition, serialization, native snapshot application, and frame draw time. Include current rules, pin policy, selected context, and model/state identity in cache keys where relevant. Cache immutable distances or masks with bounded memory; avoid an unbounded cache of one full boolean piece array for every possible query.
3. **Preserve exact filter semantics while caching.** Compile reusable parser structure separately from state-dependent masks. Predicates such as `color`, `home_layer`, `unsolved`, `selected`, and `preview` have different invalidation dependencies. An unchanged expression is insufficient evidence that its result is unchanged.
4. **Make long reports incremental.** Current-branch totals can be cached by journal head or maintained with each committed event. Verify them against an independent ancestry reduction after undo, redo, checkout, import, and reopen. Do not silently aggregate abandoned branches into current solution totals.
5. **Optimize full-detail rendering with evidence.** Reuse buffers, reduce allocations and duplicate projection work, and measure draw/upload costs. Compare complete rendered pixels and all authoritative labels before claiming equivalence. Report sampled-motion responsiveness separately from full-detail frame latency; changing the sample limit does not prove an improvement to full-detail rendering.
6. **Share validated preset commands.** Use one expression/rule/composition contract across clients, with explicit exact/pin policy and count preview. Preserve first-match hide/style rules during union and intersection. Saved view dictionaries must merge unrelated keys rather than replace another client's settings.
7. **Bound auxiliary rendering and metadata work.** Share immutable topology and parsed status between both views; update count data by committed revision rather than on every hover or camera event. Keep auxiliary cameras independent from the full sticker viewport, cache projection until its inputs change, and suspend closed/minimized/hidden views. Measure the combined UI with zero, one, and two auxiliary windows open, including rapid selection during a durable turn. GDI redraw time and full DirectX sticker-frame time must be reported separately.

Future changes require short actual samples from the matching source/build. Targets are acceptance criteria, not automatically achieved results. The current report identifies hardware, runtime, exact source, filter, camera path, detail policy, frame latency distribution, UI latency, bounded idle-paint observations, memory limitations, and full-detail restoration. Run timed samples without simultaneous build or regression workloads.

## Potential extensions

These items are proposals, not implemented promises:

- **Shortest color paths.** Retain BFS predecessors to display a shortest shared-face path between two canonical colors, with keyboard traversal. A shortest graph path is a navigation aid, not a shortest legal move sequence.
- **Vertex and color bookmarks.** Save validated V/C identities with a model ID, a short label, and optional camera context. Keep bookmarks separate from frozen piece sets and from live filter queries.
- **Color/layer progress views.** Show exact and color-only completion side by side, with current-position and home-identity interpretations. For overlapping layers, compute a union for global totals or explicitly label repeated membership.
- **Optional piece ownership layers.** A separately named `owned_layer` could assign each piece to the minimum graph distance among its hosting cells. This would partition pieces but differs from the current any-touch selection. It must not silently replace `layer`.
- **Protection for arbitrary piece sets.** Persist a selected set with identity/position semantics and reject a proposed operation whose complete net effect violates it. A geometric layer is a view grouping, so protection still needs labelled full-state checks, orientation, and collateral.
- **Cost-aware insertion planning.** Rank available certified choices using complete expanded-word costs, protection conflicts, and the user's target policy. Explain the chosen destination and alternatives. Keep finite legal witnesses, cancellation, and current-state validation even when using a heuristic.
- **Live named queries.** Separate live expressions from frozen identity/position sets in storage and UI. Show membership counts and the state revision used for evaluation, and bound query complexity and cache memory.
- **A collateral and provenance inspector.** Link an operation's target, oriented buffers, setup, affected orbits, certificate, and journal event. Replayable evidence would make assisted work easier to inspect and explain than an opaque macro button.
- **Accessible color recognition.** Add persistent text labels, searchable legends, optional symbols or patterns, and color-blind palettes. Expose owner-drawn graph nodes to screen readers and verify keyboard focus across every view. Keep palette changes as presentation preferences and retain canonical IDs in reports and exports.
- **Persistent auxiliary workspaces.** Save independent camera values, window placements, and pinned origins with validated model and display context. Recover off-screen placements after monitor changes, and expose a placement reset. Current in-memory camera retention is not this persisted feature.
- **Session branch comparison and playback.** Compare exact labels and complete operation support from a shared journal ancestor, then play committed events with explicit pause and speed controls. Keep abandoned branches out of current-branch totals and avoid modifying the source session during comparison.
- **Automatic renderer calibration.** Measure warmed frame costs at the current resolution and choose a bounded motion-detail budget from actual observations. Always restore full visible detail at idle and preserve exact picking; do not infer an untested GPU recommendation from the calibration.
- **Guided solving workflows.** Offer a user-reviewed checklist of targets, protected groups, and checkpoints. Keep planning recommendations separate from execution; do not claim a universal optimal solve order or solve arbitrary buffer configurations without supporting algorithms.

## Verification priorities

Use isolated sessions. Never test reset, failure injection, import, or scramble against a personal database.

| Contract | Required evidence |
| --- | --- |
| Canonical IDs | C1/C600 boundaries; reject C0/C601 in new predicates; retain legacy `home(C000)` and `current(C599)`; verify native palette permutations do not relabel canonical colors. |
| Topology | 600 cells, four symmetric shared-face neighbors each, connected graph; 120 vertices with 20 incident cells each; four vertices per cell; model ID unchanged. |
| Cell layers | Independently compute BFS shells from every origin; L0 is one cell, L1 its four neighbors; all cells occur once across shells. Verify overlapping whole-piece membership independently. |
| State-dependent filters | Solved and legally moved states, undo/redo and reopen; `color`/`home_layer` follow identities, `cell`/`layer` remain positional. Styles remain uniform across a physical piece. |
| Preview/apply | No mutation during counts; replacement/intersection/union preserve documented rules; stale contexts and malformed requests reject without changing labels, journal head, preferences, or pending work. |
| Native interactions | Keyboard focus and text editing; local hover/navigation; click versus drag; ordinary multi-click twists; hidden and annotation-only pieces stay noninteractive after camera, filter, and display changes. |
| Auxiliary views | Real retained tetrahedral geometry and canonical IDs; independently checked status scopes; shared selection without implicit main-camera changes; Follow/Pin; distinct cameras; closed/minimized/inactive drawing suspension; busy-state input; narrow/wide layout; actual native shortcut routing and error recovery. |
| Reports and buffers | Independent count reductions; correct destination versus required identity; fixed buffer positions versus occupants; candidate frame costs and complete preview support; meaningful stale-analysis errors. |
| Persistence and performance | Checkpoint/reset/import/reopen preferences; bounded caches; actual current-build Windows/DirectX samples with explicit detail policy. |

`tests/test_piece_filters.py` provides existing parser, whole-piece, and preference-failure checks. The structure, cell-view, and auxiliary-window fixtures exercise local controls, immutable geometry, and container lifecycle; they are not GPU performance tests or substitutes for the actual native-host input path. Run the core, reference-map, crash, lifecycle, and native regressions appropriate to any modified boundary as described in [development](DEVELOPMENT.md).
