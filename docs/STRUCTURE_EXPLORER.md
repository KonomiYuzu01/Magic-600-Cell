# Colors, cell layers, and the structure explorer

> **Shared-model and historical 0.3 interface reference.** Mathematical IDs and notation remain useful, but window names and controls below are not a complete 0.4 usage guide. Start with [Using 0.4](../USAGE.md).

The **Structure** tab helps locate cells and build whole-piece filters in the full 600-cell model. Its **Colors**, **Layers**, and **Vertices** task tabs share one selected cell. The origin/search row and filter actions remain outside those tabs, so changing tasks does not lose the origin or require scrolling through unrelated tools.

The graph is a schematic navigation tool. Searching, hovering, comparing, and changing the graph selection do not turn the puzzle or recenter the native viewport. Use **Center viewport** for an explicit camera change.

## Stable identifiers

Canonical color IDs run from **C1 to C600**. C1 is immutable lab cell 0, and C600 is lab cell 599. A color identifies a cell in the solved model; it is not an RGB value or an index into the native renderer's palette. Swatches help recognition, but the written ID is authoritative. Six hundred swatches need not be perceptually distinct.

Older filter arguments retain their original zero-based numbering. For example, `color(C1)` and `home(C000)` select the same physical identities, while `cell(C1)` and `current(C000)` select the same fixed positions. `home(C013)` means lab cell 13, corresponding to canonical C14. This compatibility rule also applies to the older `cap` and `shell` arguments. Use the new predicates for canonical C1–C600 notation.

Vertices use **V1 to V120**. Their order is derived from the existing O34 position IDs. Each vertex meets 20 tetrahedral cells, and each cell meets four vertices. Selecting a vertex lists its incident colors; it does not define a layer origin. Choose one of those colors to make that cell center the origin.

## Navigate the structure

1. Enter `C17` in **Find color** and press Enter or click the button. The selected color, its lab index, and its four shared-face neighbors appear.
2. In **Colors**, choose **1 hop** or **2 hops**. A graph edge means that two tetrahedral cells share a triangular face. Screen distances and angles are schematic, not geometric measurements or shortest-path lengths.
3. Click a node or a neighbor button to navigate locally. In the graph, arrow keys move keyboard focus; Enter or Space selects the focused node. Hover shows an ID without querying the engine.
4. Enter another color in **Compare**. The result distinguishes identical cells, shared-face neighbors, and more distant cells, and gives the shortest shared-face graph distance. A comparison color outside the displayed graph need not have a visible node.
5. In **Vertices**, use the selected cell's four vertex buttons, or the V-number control, to inspect a vertex's 20 incident colors. Select an entry and choose **Go to incident color**, or double-click it, to navigate to that cell and return to **Colors**.

Task pages scroll locally when necessary. The filter expression, count summary, **Preview counts**, **Apply filter**, and **Insert expression** remain below the task area. **Options** expands the predicate and composition controls; **Review** expands the complete count/rule text. Controls can be reached with Tab, including the graph, lists, expression, and preview text.

In **Colors**, **Use color**, **Use cell**, and **Use neighbors** prepare the corresponding predicate below without applying it. The sidebar has a 320px minimum width and a 348px default; task contents can scroll vertically at the minimum width.

## Auxiliary cell views

Use **Cell views** in the main toolbar, or the **View** menu, to open **Global overview** and **Focused neighborhood**. Both views use the model's actual tetrahedral cells and four-dimensional coordinates. They are navigation aids, not additional puzzle engines. Their color IDs refer to the same canonical C1–C600 cells as the structure explorer.

On a wide main window, the views open as two owned, resizable windows. Drag their title bars to reposition them. **Reset window positions** restores nonoverlapping positions along the screen's right edge without resetting either view's camera. The manager uses the **Views** tools tab instead when the main window is narrower than 1100 logical pixels, or the screen is too short to stack both windows. The inner **Global overview** and **Focused neighborhood** tabs share that drawer; the inactive tab stops rendering. Switching between drawer and floating windows preserves each view's camera and the local origin.

Selecting a cell synchronizes the canonical selection across the views and structure explorer. It does not move the main puzzle camera. Use **Center main** for that separate action. In the focused view, **Follow selection** moves the neighborhood origin with the selected cell. **Pin this origin** keeps the current origin fixed while other cells are selected; returning to Follow uses the latest selection. Choose **1 hop** or **2 hops** to change the neighborhood radius in the shared-face graph.

Auxiliary camera rotation and selection remain available while the main puzzle commits a turn. The host retains the latest selection request for the next state-context update; **Center main** waits until the main operation finishes. Visible solving statistics continue to describe the last committed state until the new status arrives.

The palette represents canonical cell colors. A solid teal outline marks selection and a dashed charcoal outline marks hover; these outlines do not represent solving progress. A hollow marker means the cell has no positions from the active orbit. A dark dot means it has at least one unsolved position in the exact visible selection. Buffer markers are diamonds and selected-piece markers are squares. Any status counts shown belong to the current committed state and must be read using their displayed labels. Auxiliary selection and status display do not change sticker colors or the full mechanical state.

**Reset camera** affects only that auxiliary view. **Close view** stops its rendering and retains its camera and focus for reopening. Minimizing a floating view, hiding the tools drawer, or minimizing the owner also suspends the corresponding drawing. No background animation timer runs for these views. If geometry or status cannot be loaded, the auxiliary view explains the failure and asks for a reconnect; the main puzzle remains independently usable.

## Cell-centered layers

A layer is an exact breadth-first-search shell in the shared-face cell graph, rooted at the **selected cell center**:

- L0 contains only the selected cell.
- L1 contains its four face-sharing neighbors.
- Lk contains cells whose shortest shared-face path from the selected cell has exactly k edges.

In **Layers**, choose L0 through L15 and inspect the exact list of colors in that shell. **Use current layer** or **Use home layer** prepares the corresponding predicate in the persistent filter area; these buttons do not apply it. The interface derives the available range from the topology. Camera orientation and the current scramble do not change this graph.

Cell shells are disjoint and together contain all 600 cells. **Whole-piece filters for those shells can overlap.** A piece is included when at least one of its hosting cells belongs to the chosen shell; all its stickers are displayed. A piece touching cells in adjacent shells can therefore appear in both selections. Do not add piece counts from separate layers and call the result a count of unique pieces.

Vertex-centered geometric bands are not the layer definition used here. A possible future rule assigning each piece to its nearest shell would be a different, explicitly named ownership convention.

## Exact filter semantics

| Predicate | Meaning |
| --- | --- |
| `color(C17)` | Follow every physical piece whose solved identity bears color C17, wherever that piece is now. |
| `cell(C17)` | Select current piece positions touching cell C17, regardless of their occupants. |
| `adjacent(C17)` | Select current positions touching any of C17's four face-neighbor cells. The root cell is not included as a cell, although shared multi-cell pieces can also touch it. |
| `layer(C17,L3)` | Select current positions touching any cell exactly three shared-face steps from C17. |
| `home_layer(C17,L3)` | Follow the physical identities whose solved positions touch that same shell. |

`layer(C17,L0)` equals `cell(C17)`, `home_layer(C17,L0)` equals `color(C17)`, and `layer(C17,L1)` equals `adjacent(C17)`. These are piece predicates, not instructions to recolor, turn, or extract a smaller mechanical puzzle.

Combine predicates with parentheses, `!`, `&`, and `|`; the word forms `not`, `and`, and `or` are also accepted. Negation binds before intersection, which binds before union.

```text
color(C17) & unsolved
color(C17) & color(C42)
cell(C17) | adjacent(C17)
layer(C17,L3) & O33
home_layer(C17,L2) & orientation_wrong
```

The second example finds pieces bearing both colors. The third includes the selected cell and its immediate neighboring cells. The final example follows home identities rather than fixed current locations. Existing orbit, correctness, named-set, rank, and sticker-count predicates remain available.

## Preview, compose, and apply

Open **Options** in the filter area to choose a predicate and composition:

| Composition | Effect |
| --- | --- |
| Replace | Use the new predicate as one solid rule. |
| Intersect | Keep only matches of both the existing rules and the new predicate, preserving the existing rule styles and order. |
| Union | Include the new predicate as well as the current visible selection; existing hide rules are restricted so they do not conceal the added selection. |

Click **Preview counts** to inspect the resulting whole-piece and sticker-slot counts. Open **Review** for additions/removals and the complete scrollable composed rules. Review can expand automatically when there is enough vertical space; collapsing it retains the count summary and preview. This is a read-only calculation. The counts exclude optional buffer/selection/preview pins and inspection annotations. The rules use first-match priority, including explicit hide rules.

Click **Apply filter** after reviewing the preview. Applying a structure filter uses exact mode. A changed puzzle or filter context requires a fresh preview; stale results must not silently apply. **Insert expression** instead copies only the generated predicate to the filter editor for manual editing. It does not apply it.

The Filters tab also has shortcuts for the selected color, current cell, neighbors, center layer, and home layer. These structure shortcuts prepare an expression and counts for review. Existing ordinary presets such as Active orbit and All pieces apply immediately. The **Hide 600-cell frame** setting is independent of either kind of preset.

Hiding the frame removes the background framework. The focused cell's cyan outline remains as a navigation annotation; it is not an extra interactive piece and does not broaden the filter.

Rendering and picking are separate. Hidden pieces remain in the full mechanical state. Pins or inspection annotations can reveal context outside an exact filter without granting interaction with those otherwise excluded pieces.

## Piece inspection and insertion context

Shift+left **click** on a pickable piece identifies its physical occupant and highlights that identity's solved home cell centers in cyan. Shift+right **click** treats the clicked position as a destination, locates the identity required there, and highlights that identity at its current location. The insertion destination remains the clicked position. These annotations do not change sticker colors in the authoritative state or broaden the interaction mask. Use **Clear inspection** to remove the inspection context.

Shift+left **drag** remains a 4D camera rotation. Clicking, dragging, and multi-click twist grips must remain distinct interactions. Empty space and pieces excluded by the active filter cannot trigger inspection.

Native twists appear instantly after the backend has durably committed the move. Pre-commit twist animation has been removed so an uncommitted state is not displayed as a completed turn. Camera motion remains interactive; this decision concerns twist presentation and does not change the legal move or undo history.

For a moving nonbuffer destination, the buffer analyser can show fixed buffer positions, their current occupants, candidate oriented frames, setup depth and word, and primitive-turn cost. A fixed cell center or fixed setup-buffer destination is explained explicitly. A candidate star is a finite operation, not a promise that the selected star alone completes an insertion. Preview checks its complete full-model effect and protected-orbit conflicts before commit.

## Implementation and scope

`GET /api/structure` provides immutable topology once: canonical cell adjacency, vertex incidence, inverse incidence, and the model ID. The native control derives BFS distances and navigates locally. `POST /api/filter-preview` evaluates a proposed expression or rule list against the current state without saving preferences. `POST /api/filter-apply` checks the preview context before saving the reviewed exact rules.

The feature does not change the immutable model, its 259,800 labelled sticker slots, or its 1,200 legal generators. It does not define an automatic layer-by-layer solution or a new orientation system. See [limitations and development priorities](LIMITATIONS_AND_ROADMAP.md) for remaining work and [development](DEVELOPMENT.md) for verification requirements.
