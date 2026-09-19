# Development decisions and evidence

> **Historical shared-runtime / 0.3 log.** For released 0.4 behavior and evidence, start with the [0.4 release guide](RELEASE_0_4.md) and [documentation index](README.md). References to current work below retain their original historical meaning.

This log describes the current development work. It is not a release announcement or a claim that older test results certify newer source.

The published 0.2.4 release validation and its recorded source identity remain frozen. The structure, inspection, update-protocol, and auxiliary-view changes below form version 0.3. Its separate package validation must be read alongside these functional observations; the older download and its checksums do not certify this version.

## Version 0.3 packaging and research handoff

**Decision:** Publish the completed source and executable as **C600 Studio 0.3**, preserving 0.2.4 as a historical release. Version strings now share the native host's version constant; launcher settings migration considers 0.2.4 before older versions and preserves existing destination files. The final 13-source native hash is recorded in [0.3 validation](RELEASE_0_3_VALIDATION.md); the earlier measured rendering/interaction source differs only in its native version display.

**Packaging result:** The x86 native host compiled and its build-time fixture passed 19 groups. Eight isolated migration cases passed. The final frozen x64 launcher/engine passed seven portable verification groups over 135 resource files, including Unicode data paths, all labels, graceful reopen, parent-EOF recovery, and corrupted-resource rejection. A separate real normal startup passed six checks: live native bridge and window readiness, exact packaged host, all 259,800 solved labels, no startup fixture, graceful process closure, and released session lock. The private smoke guard initially rejected Windows' legitimate console-host child; correcting that exact system-file recognition required no application change. The installer compiled from this verified payload; an OS install/uninstall cycle remains untested.

**License collection:** The installed NumPy distribution stores its main notice in package metadata rather than a `licenses` directory. The builder now discovers recorded notice files safely and retains the complete main and component notices. Microsoft Managed DirectX remains external. Regression executables, prototypes, performance instrumentation, raw diagnostics, and personal data are excluded from distribution.

**Research result:** The updated 34-page English LaTeX report includes the canonical dual graph and incidence conventions, exact layers and whole-piece overlap, identity/destination inspection, independent auxiliary cameras, atomic native updates, and measured performance limits. Compilation and visual review passed. The original algorithms paper, curated replay archive, Puzzle Theory source/PDF, and proof-review records remain unchanged. The source and PDF are supplied together; hardware targets and historical observations remain explicitly separated.

**Remaining limits:** The final native functional and visual checks passed, but the short performance sample still missed 100 ms instant-turn p95 and continuous full-detail 1080p/30 FPS. One graph Enter-activation p95 measured 50.04 ms against a 50 ms gate. Actual mixed-monitor/high-DPI behavior, sustained performance, OS installation, and cross-restart auxiliary-camera persistence are not established. No failed architectural experiment was promoted. See [performance evidence](DEVELOPMENT_PERFORMANCE.md) and [the roadmap](LIMITATIONS_AND_ROADMAP.md).

## Structure explorer layout

**Problem:** A single long vertical flow mixed color navigation, layers, vertices, and filter review. It required scrolling past unrelated tasks and made preview/apply controls difficult to reach. Horizontal overflow elsewhere in the native tools panel also needs integrated layout verification.

**Decision:** Split structure navigation into Colors, Layers, and Vertices task tabs. Keep the selected cell-center origin/search row and filter expression/actions outside those tabs. Preserve origin and layer selection when switching tasks. Put predicate/composition settings under Options and complete composed rules under Review, retaining a compact count summary when review is collapsed. Use ordinary Windows controls, local scrolling, a substantial graph, explicit focus, and readable labels.

**Behavior retained:** Canonical C1–C600 identity; cell-rooted BFS layers; any-touch whole-piece membership; vertex lookup as navigation; local hover/search/compare; explicit centering, count preview, insertion, and apply events. The native host continues to own HTTP, context validation, persistence, and viewport changes.

**Verification scope:** The focused WinForms control fixture passed eight check groups with the native host's Segoe UI 9 font. It checks topology and navigation, keyboard/mouse handlers, stale-preview rejection, task-state preservation, and visible filter actions at 320, 330, and 600 pixels. A failed narrow-width run exposed a scrollbar-gutter sizing error; the corrected run passed, and control-only images were inspected. The fixture uses immutable model topology and no personal session or DirectX renderer. Later matching-source native integration and 96 DPI visual evidence are recorded below; these earlier fixture results are a separate scope.

## Structure navigation cost and preview ordering

**Measured fix:** Each color change formerly destroyed and rebuilt four neighbor buttons and four vertex buttons, and rebuilt unchanged layer options. The explorer now reuses those controls and options while updating their current-origin targets. It retains the selected layer, filter composition, and stale-preview guards. A fixed isolated WinForms comparison measured Colors navigation at 119.19ms mean before and 20.21ms after, with mean layout events falling from 70.9 to 6.9. Each task had 20 measured calls; message pumping and painting were outside those individual timings. This identifies a control/layout improvement, not an end-to-end native latency guarantee. Broad layout suspension was tried privately and rejected because it increased layout work.

**Integrated observations:** A later native-host measurement separated color navigation plus painting, a graph Right-arrow step, and Enter activation. Their p95 values were respectively **45.33ms**, **6.18ms**, and **50.0411ms**. The first two passed the unchanged 50ms gate. Enter activation had a 38.80ms mean but its p95/maximum exceeded the gate, so that activation check **failed**. Neither rounding it to 50ms nor reporting only its mean constitutes a pass. Paired auxiliary-view interaction p95 values were at most 49.79ms in that run. These are short current-build observations; full performance acceptance remains separate from the earlier release validation.

**Rejected follow-up:** A private candidate deferred hidden Layers-page label/list updates until that page was opened, while retaining current BFS/filter semantics. One fixed before/after comparison reduced layout events but did not improve Colors navigation: mean 23.39ms became 26.82ms, and p95 29.38ms became 49.84ms. The candidate was rejected, was not promoted to production, and was not repeatedly measured to seek a passing result. The production explorer still updates the layer presentation on navigation.

**Preview ordering:** The host ignores responses belonging to earlier preview request IDs instead of clearing a newer accepted preview. A response for the current request still invalidates its preview if the state or applied filter rules changed. The dedicated host-fixture check deterministically delivers B before A, verifies that B's counts and reviewable rules remain intact, and separately checks state/rule changes. This exercises production response handling with a disconnected fixture rather than claiming a live network timing test.

**Public fixture entry point:** The [combined Windows controls runner](../tests/test_native_auxiliary_controls.py) now includes the structure explorer and passed **8 structure + 6 cell-view + 11 auxiliary-manager groups** in one fresh output directory against unchanged production source. Source hashes agreed before and after execution. This run verifies the documented entry point and the focused control contracts using real Forms, immutable topology, and fresh committed-state metadata; it loads no MPUlt renderer or personal session and establishes no DirectX performance result. All fixture-owned windows and processes exited.

## Turn presentation and state authority

**Decision:** Show native turns instantly after the backend has durably committed them. Remove pre-commit twist animation, which could present an uncommitted state while native callbacks and renderer work were still in flight. Keep camera rotation interactive and preserve finite legal moves, full collateral, and exact undo/redo.

**Verification scope:** Test successful turns, rejected turns, full native/backend color agreement, and undo/redo against the current source. This decision does not by itself demonstrate a speed improvement. Measure commit latency and frame presentation separately; do not label a full-detail frame-rate limit as input latency.

## Native sidebar sizing

**Finding:** A layout could have no horizontal scrollbar while its multiline filter editor had collapsed to zero height. The earlier general window/docking fixture did not check the editor's own bounds. A focused 280px-sidebar assertion reproduced the missing editor and retained the failure before the fix.

**Decision:** Preserve explicit editor heights, disable multiline textbox autosizing, and avoid zero-height maximum constraints on fixed controls or fixed-height flow panels. Give native selectors an explicit usable height. Validate vertical row heights and sibling separation as well as horizontal bounds. Use a dedicated Segoe UI font on the added workbench controls while preserving the original native form's font; changing the legacy form font can trigger designer autoscaling and enlarge its minimum window size. Keep the sidebar at least 320px wide (default 348px) so native tab padding still leaves a readable graph and form; allow local vertical scrolling rather than compressing controls below that minimum.

**Verification scope:** Retain the original eight host-fixture checks and add targeted checks for Filters, Solve, and Pieces at 320/348px, editable-control minima, local section expansion, control-state transitions, integrated Structure bounds, and legacy form sizing. The earlier 280px failure remains diagnostic evidence; 280px is below the new supported sidebar minimum. These are in-process WinForms checks, with model connection synthesized for control-state tests and no engine requests. Final integrated visual evidence remains separate.

## Remaining acceptance scope

Current-build performance observations are recorded in [development performance](DEVELOPMENT_PERFORMANCE.md), with source identity, hardware, detail policy, sample sizes, and separate turn, GDI, and DirectX timings. Instant-turn and full-detail targets remain unmet, and one graph-activation sample narrowly missed 50 ms. These observations include both auxiliary views; earlier release or isolated backend timings do not replace them.

The native integration and visual evidence below cover the tested 96 DPI configuration. They do not establish real mixed-monitor/per-monitor-DPI behavior or every desktop input path. A pure 125% layout-policy assertion is not a test on a second physical display. The available images combine control capture and the DirectX render target; desktop-level screen capture was unavailable.

See [the structure guide](STRUCTURE_EXPLORER.md), [limitations and priorities](LIMITATIONS_AND_ROADMAP.md), and [verification instructions](DEVELOPMENT.md). Keep raw machine screenshots, personal session data, and diagnostic paths out of public source.

## Auxiliary geometry views and visual direction

**Decision:** Add two independent views of the retained cell geometry: a global overview and a focused one- or two-hop neighborhood. Both consume one validated immutable topology object containing the real 120 four-dimensional vertices and 600 tetrahedral cells. The structure explorer's existing graph remains a schematic adjacency diagram. Neither auxiliary control owns puzzle labels, executes moves, calls HTTP, or changes the main DirectX camera.

The wide-window layout uses two owned, modeless windows with separate cameras, drag/resize placement, and nonoverlapping default positions along the screen's right edge. The Views tools tab supplies a drawer when the owner is narrower than 1100 logical pixels or the working area is too short to stack the windows. Reparenting, closing, and reopening retain the same view instances and camera values. Follow tracks the latest selected color; Pin retains the neighborhood origin independently. Center main is an explicit action. These auxiliary cameras, positions, and pin settings are currently in-memory state, not saved session preferences.

**Visual direction:** Use native English Windows controls, Segoe UI 9, a neutral light canvas, compact toolbars, and substantial geometry. Avoid decorative cards, gradients, glows, duplicate headings, and repeated instructional footers. Canonical palette fills identify cells; a solid teal outline marks selection and a dashed charcoal outline marks hover. A hollow marker indicates no active-orbit positions in that cell; a dark dot indicates at least one unsolved exact-visible position. Buffer and selected-piece context use distinct diamond and square markers. The renderer supplies actual IDs and count labels; tooltips and the guide carry longer explanations. Screen overlap and projection depth are not solving-progress metrics.

**State and responsiveness:** Both controls share one parsed, revisioned status object from the committed snapshot. Active-orbit, exact-visible, and eligible counts have different scopes; eligible means the intersection of the active orbit and exact interaction mask. Correctness uses labelled identity and orientation. Counts are per-cell incidences, so adding them across cells repeats multi-cell pieces. Pins and inspection annotations do not enlarge exact visibility or eligibility. Hover is local and does not request a backend refresh. During a main-puzzle commit, auxiliary rotation and selection remain usable, while explicit main centering waits; the host retains the latest selection request for the next context update.

Closed, minimized, hidden, and inactive-drawer views suspend geometry drawing. There is no auxiliary animation timer. Suspension also clears shared hover; disposal releases both owned windows, controls, tooltips, and the manager's font. Geometry or status failure disables the auxiliary display with a reconnect explanation, and is handled separately from the main puzzle renderer.

**Focused evidence:** The [documented standalone runner](../tests/test_native_auxiliary_controls.py) passed six [geometry-control](../tests/native/NativeCellViewRegression.cs) groups and eleven [window-manager](../tests/native/NativeAuxiliaryViewsRegression.cs) groups against a source set checked before and after execution. It uses retained geometry, a generated test palette, and production cell-status data from a fresh solved SQLite session that closes before the GUI runs. Control checks cover validation, exact neighborhood membership, shared immutable data, local selection/hover, camera/input handlers, and inactive rendering. Manager checks cover simultaneous owned windows, distinct nondefault cameras produced by in-process drag handlers, close/reopen and minimize, narrow/wide reparenting, Follow/Pin, busy-state camera/selection, error/reconfigure, nonoverlapping placement, logical screen thresholds, and owner shutdown. All fixture-owned windows and processes exited.

The monitor/reset review found that moving an owner without resizing could retain the previous screen's drawer policy. The manager now reevaluates working-area/DPI changes and explicit placement reset; ordinary moves within the same working area preserve manual placements. The passing pure policy check covers the width and short-screen thresholds at 100% and 125% scaling. A real mixed-monitor check remains pending.

These focused GDI/WinForms and backend-data checks remain distinct from the later native integration below, mixed-monitor behavior, personal-session recovery, and performance acceptance. Test compilation and package source lists include both new production modules. The [standalone fixture command](DEVELOPMENT.md) now invokes all three focused helpers without starting MPUlt or an HTTP engine.

## Current native integration and visual checks

The matching development production sources passed **28 native feature groups** on Windows with the original MPUlt host and actual DirectX renderer; the test process exited successfully. The checks include shared auxiliary focus/status, independent cameras and Follow/Pin, compact input routing, native gestures and full-state agreement, checkpoints, macros, insertion and reports, and renderer recovery from injected device loss. The device-loss case is controlled fault injection, not a claim that every display-driver failure was reproduced.

Two regression fixes have explicit native coverage. Enter on a focused compact auxiliary control leaves a pending puzzle preview uncommitted. Canceling the owned busy-close dialog preserves the original native `Puz` object and permits a subsequent turn: the legacy close handler now runs only after the workbench accepts closure. Focus synchronization also preserves the canonical header through camera restoration and keeps structure-filter origins consistent with auxiliary selection. This is current development evidence; it does not alter the published release's validation record.

The separate visual check passed and its captures were reviewed at **96 DPI**:

| Main window | Main native viewport | Auxiliary layout observed |
| --- | --- | --- |
| 1300 × 740 | 931 × 604 | Both 420 × 348 floating views; global view shows all 600 cells and the one-hop local view shows five cells. |
| 1000 × 650 | 631 × 514 | Views drawer with working Follow/Pin and coherent global/local geometry. |

The images combine actual WinForms/GDI control capture with actual DirectX render-target capture. They establish the inspected application layout and rendered content, not operating-system compositor behavior or an independent desktop screenshot. Auxiliary camera/placement/pin remain in-memory state only. The later matching-source [performance observations](DEVELOPMENT_PERFORMANCE.md) distinguish passed checks from unmet targets.

## Development handoff

Work in an isolated checkout and read `AGENTS.md` before continuing. Preserve the immutable full model, canonical/legacy ID contracts, durable transaction boundary, and existing upstream credit. Inspect current implementations before adding features. Keep native navigation local, validate filter previews against their current context, and retain full move witnesses and collateral. Use the focused control/layout fixtures to catch UI regressions before integrated Windows/DirectX checks. Report current-build evidence and unresolved limitations separately; do not publish personal data or claim a proposed optimization has already passed.

## Backend inspection and atomic native updates

**Correctness decisions:** Inspection records the exact clicked sticker, its label, the source position, and source-state hash. Shift-left marks every fixed home-cell center of the occupying identity; its inspector follows that identity's current position after undo/redo. Shift-right treats the clicked position as the destination, locates the identity that belongs there, and follows that required piece. Its buffer analysis uses the destination's orbit without changing the active filter orbit. Fixed centers and setup buffers receive an explicit unavailable-analysis reason. Refresh and reconnect restore the inspector without repeatedly switching the user's tab; old insertion candidates remain subject to their state check.

Annotations are durable preferences separate from `selected`, filter rules, and the exact interaction mask. Clearing an annotation preserves the prior selection and filter. Highlighting a hidden home center or required piece does not make it clickable. Native inspection rejects stale state/profile information and hidden hits before writing preferences. Filter preview counts use first-match rule precedence with pins and annotations excluded; apply validates the unchanged state/preferences/pending-preview context before saving those reviewed rules.

**Protocol decision:** Keep the legacy full snapshot and opt into v2 for atomic full/delta updates. A native turn may return its committed status and native snapshot under the same lock. The client validates the profile and exact delta base, then publishes complete materialized arrays. Unknown or evicted bases fall back to a full snapshot. Separate state, color, style, interaction, and annotation revisions prevent a filter or annotation update from being confused with a mechanical move. See [the protocol contract](NATIVE_UPDATE_PROTOCOL.md).

**Verification scope:** The backend feature regression passed 16 groups covering topology, canonical predicates/layers, inspection, durable metadata, filter precedence, stale/hidden rejection, authenticated HTTP, and atomic full/delta reconstruction. Its HTTP geometry is synthetic and shuffled; it is not evidence of mouse input or rendering. The feature implementation also passed the core, reference-map, and crash regressions. After adding current-position inspection fields, a separate four-group SQLite check passed home/required tracking through undo/redo, selected-filter independence, annotation clearing, and byte-identical recovery of all 259,800 labels. These focused results do not certify later native UI changes.

## Immutable mapping cache: measured candidate

**Finding:** Every snapshot rebuilt a NumPy array from the verified profile's 259,800 native-to-lab IDs. A private in-process stage profile measured that conversion at about 14–15ms per request. Snapshot construction also evaluates render and interaction filters, hashes complete arrays, determines changed slots, and serializes the response. The full labelled-state hash is cached on the puzzle state; snapshot construction does not recalculate it.

**Decision:** Retain an owned, read-only native-to-lab array and inverse palette in the existing snapshot cache. Both come from the verified bridge profile, occupy 1,041,600 bytes together, and are replaced on profile change; replacing them invalidates previous delta bases. No request-provided mapping, extra worker, journal change, or mutable shared model was introduced.

**Focused verification:** Six cache regression groups passed against a retained verified native mapping: independent slot/palette agreement, immutable bounded storage and reuse, exact full/delta materialization after a legal move, legacy v1 compatibility, replacement-profile invalidation, and complete SQLite reopen. The replacement-profile branch uses a deliberately transformed mapping fixture to test invalidation; it does not claim a second geometry handshake passed. See [the cache regression](../tests/test_native_snapshot_cache.py).

The stage diagnostic used fresh isolated SQLite journals with WAL and synchronous FULL, the same legal turn sequence and retained native mapping, two warmups, then six measured turns per filter. Mean wall times were:

| Backend stage | Original 0.2.4, active / all | Development v2 before cache, active / all | Development v2 with cache, active / all |
| --- | ---: | ---: | ---: |
| Turn, durable commit, snapshot and JSON | 77.16 / 81.85ms | 71.61 / 68.29ms | 50.86 / 44.25ms |
| Snapshot construction, included above | 29.88 / 35.40ms | 35.70 / 33.13ms | 15.05 / 13.60ms |

With the cache, the largest measured totals were 56.99ms for active and 53.98ms for all. Journal transaction time averaged about 1.3–1.4ms; state refresh and the first progress/status calculation remained material costs. Every measured session reopened with all labelled bytes intact.

These are private CPU/backend diagnostics with a small sample, nested stage timers, and no HTTP transport, thread dispatch, native UI, GPU, or frame presentation. They identify an avoidable conversion cost and support the bounded cache; they do **not** establish a 100ms UI p95, a frame-rate improvement, or a release-wide performance guarantee. Raw machine reports are retained privately. Integrated current-build timings remain a separate acceptance step.

## Exact auxiliary counts and atomic inspected-cell focus

**Decision:** Derive the coarse auxiliary geometry from the retained model's real cell planes. Share 120 WXYZ vertices, 600 tetrahedra and their centers; retain the full high-resolution puzzle only in its existing mechanical/native owners. Compute six per-cell count arrays from authoritative labelled-piece correctness, the active orbit and the exact unpinned interaction mask. Cache those counts across focus-only updates. They count incidences and overlap between cells; no estimated progress percentage is introduced.

**Focused backend evidence:** Six geometry/count/focus groups passed independent plane/vertex and incidence checks, exact count oracles across five filters and moved states, empty eligibility, cache reuse, strict focus validation, injected SQLite failure, checkpoint/full-state reopen, and authenticated HTTP. A private six-turn-per-filter diagnostic measured the added count stage at 3.42ms mean for active and 2.96ms for all; complete in-process backend totals were 54.67ms and 47.40ms. This remains a small CPU diagnostic, excluding HTTP scheduling, native input and rendering.

**Atomic inspection refinement:** A successful native inspection now saves its exact clicked annotation and corresponding physical-cell focus in the same existing preference transaction. The native UI receives both in one atomic snapshot. Stale/profile-mismatched, hidden, malformed or failed writes change neither value. Clearing the piece inspection preserves the independently chosen cell focus; original selected identity, filters, protection and interaction remain separate.

The dedicated [inspection/focus regression](../tests/test_inspection_focus.py) passed five groups over fresh SQLite and authenticated HTTP with a synthetic shuffled native mapping. It verifies one transaction, both gesture semantics, selected-filter invariance, injected write rollback, checkpoint/reopen of all 259,800 labels, coherent snapshot/count focus metadata and stale/hidden rejection. Core, all 1,200 reference maps and crash recovery were then rerun serially and passed on this source. These backend checks do not claim actual auxiliary rendering; the focused native integration fixture separately covers HWND input, shared status, focus-outline pixels, busy queuing, independent cameras and lifecycle.
