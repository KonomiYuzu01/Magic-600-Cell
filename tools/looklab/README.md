# Look Lab core (LL1)

`LookLab.Core` is the framework-neutral, net8.0 core of the local Look Lab.
It has no Godot, engine or NuGet dependency. It loads verified full-model
geometry, projects it in double precision, and owns the parameter, preset,
colour, turn, layout, flow and cost rules. The later Godot app consumes these
rules. LL1 contains no UI, shaders or renderer evidence.

Run from the repository root:

```text
python tools/looklab/check.py
```

The standard-library harness restores and builds both projects offline, runs
the console tests, removes its unique temporary folder, and compares all
checkout files and directories before and after, including ignored files.
Build products, .NET CLI state and NuGet caches stay in that folder. Extra
arguments exit 2. A failing step prints its last 60 lines and returns its exit
code; a cleanup failure prints the folder. No installation is performed.
The recipe needs a .NET SDK with the net8.0 targeting pack and runtime already
installed. Node is optional: tests that call the original Taste Lab JavaScript
report a skip when it is absent. All other tests always run.

The test runner has 38 named tests, including six Python harness cases. It
prints a line for each named test and a summary, and exits nonzero on failure.
It reads the repository and committed fixtures only, and creates its own
temporary folders with mkdir under the system temporary directory. It opens
no real Taste Lab export, personal session or database.

## Core entry points

```csharp
var schema = ParameterSchema.Load(Path.Combine(root, "tools/looklab/data/parameters.json"));
var preset = Presets.Load(Path.Combine(root, "tools/looklab/presets/default.json"), schema);
var geometry = new Geometry(root);
var turn = new TurnData(Path.Combine(root, "tools/looklab/fixtures/w3-turn.json"));
var stopwatch = Stopwatch.StartNew();
var clock = new TurnClock(turn, preset.Params.Number("turnMs"),
    preset.Params.Number("easeA"), preset.Params.Number("easeB"),
    () => stopwatch.Elapsed.TotalMilliseconds);
var frame = clock.Frame();
var projected = geometry.Project(0, preset.Params.Structure(1.6),
    Geometry.Identity(), turn, frame.Theta);
```

The caller supplies the repository path, aspect, Q camera matrix and time.
`Geometry.Project` also accepts `StructureValues(cs, ss, d4, zoom, aspect,
projection)` directly. Q and cell frames are column-major; `Rotate` updates
the columns in the S-B chronological order. The output is `(NdcX, NdcY, W)`.
Perspective uses S-B's d4 formula, stereographic uses that formula at d4 = 1,
and orthographic uses world.xyz. All then use the same 3D zoom/aspect projection.
No clipping, sampling or visibility parameter changes a slot, label or cell id.

`Geometry` reads only mesh.json, mesh_vertices.f32, mesh_sticker.u32,
mesh_centers.f32, cell_frames.f32 and model.npz. Each file is SHA-256 checked
against the `files` map of assets/manifest.json before its bytes are used;
the manifest is the trust root. Missing files, missing manifest entries and
mismatches name the file. The .npz reader uses System.IO.Compression. Its small
.npy reader refuses the wrong dtype, Fortran order, invalid shapes or lengths.
Normals require `<f8`; incidence metadata requires the retained `<i2`/`<i4` types.

`TurnData` keeps two verified 259,800-label u32 little-endian snapshots. A
generator copies labels from source to destination using the entire previous
snapshot; its inverse restores the solved snapshot. `TurnClock` uses back-to-back
half-open intervals, alternating generator and inverse, binding revision t in
turn t. A supplied binding is checked against the expected full-state digest,
including at the first frame of the odd revision. `slot / 433` gives its cell.

## Parameters (`magic600-look-parameters`, version 1)

The schema is data/parameters.json. Load validates unique ids, the seven groups,
kinds, numeric ranges and steps, enum options, defaults, optional cost-feature
ids and one-line descriptions. `ParameterSet` provides Number, Integer, Choice,
Boolean, Colour and Curve access. Colours are OKLCH objects `{L,C,h}` (L and C
in [0,1], hue in degrees [0,360]); curves are `{a,b}` in [0,1], defining the
Bezier through (0,0), (a,0), (1-b,1), (1,1). Numeric step is a control increment,
not quantization: a loaded number keeps its exact value.

A minimal schema example:

```json
{
  "format": "magic600-look-parameters",
  "version": 1,
  "parameters": [
    {
      "id": "gap",
      "group": "Structure",
      "kind": "number",
      "min": 0,
      "max": 0.3,
      "step": 0.01,
      "default": 0.24,
      "costFeature": null,
      "description": "Cell shrink amount: cs equals one minus gap."
    }
  ]
}
```

The first 18 ids below retain the Taste Lab meanings and ranges. `classes`
counts ring colour classes. `gap` shrinks cells, so cs = 1 - gap; stickerShrink
is the independent S-B ss. fieldOfView is in radians and zoom = 1/tan(fov/2).
The Look Lab follows the plan's Structure group for `gap` because it defines
cell shrink cs = 1 - gap, although the protocol table lists sticker gaps under Material.
The colour-vision preview and grid overlay belong to app state.

| Id | Group / kind | Range or values | Description |
|---|---|---|---|
| `hueRotation` | Colour / number | 0..360; step 1 | Sticker palette starting hue in degrees. |
| `hueSpread` | Colour / number | 60..360; step 1 | Hue span distributed over the ring colour classes. |
| `lightness` | Colour / number | 0.45..0.85; step 0.01 | Sticker OKLab lightness before alternating offsets. |
| `lightnessAlt` | Colour / number | 0..0.2; step 0.01 | Alternating negative and positive sticker lightness offset. |
| `chroma` | Colour / number | 0.04..0.2; step 0.01 | Requested OKLCH sticker chroma, mapped into sRGB gamut. |
| `classes` | Colour / integer | 4..8; step 1 | Number of proper ring colour classes. |
| `bgLightness` | Colour / number | 0.05..0.95; step 0.01 | Background OKLab lightness. |
| `bgHue` | Colour / number | 0..360; step 1 | Background OKLCH hue in degrees. |
| `bgTint` | Colour / number | 0..0.05; step 0.001 | Background OKLCH chroma. |
| `gap` | Structure / number | 0..0.3; step 0.01 | Cell shrink amount: S-B cs equals one minus gap. |
| `edgeWeight` | Material / number | 0..3; step 0.05 | Barycentric edge treatment weight. |
| `edgeBrightness` | Material / number | 0..1; step 0.01 | Linear black to white edge brightness. |
| `gloss` | Material / number | 0..1; step 0.01 | Specular gloss amount; retains the Taste Lab meaning. |
| `glow` | Material / number | 0..1; step 0.01 | Rim glow amount. |
| `fog` | Light and depth / number | 0..1; step 0.01 | Depth fog amount. |
| `turnMs` | Motion / number | 150..900; step 1 | Duration in milliseconds of each generator or inverse turn. |
| `easeA` | Motion / number | 0..1; step 0.01 | First easing control x at (a,0). |
| `easeB` | Motion / number | 0..1; step 0.01 | Second easing control x at (1-b,1). |
| `projection` | Structure / enum | perspective, stereographic, orthographic | 4D projection kind; stereographic fixes d4 to one. |
| `d4` | Structure / number | 1.01..4; step 0.01 | 4D perspective eye parameter used before the 3D camera. |
| `stickerShrink` | Structure / number | 0..1; step 0.01 | S-B ss: sticker shrink around each sticker centre. |
| `highlight` | Structure / enum | none, fibration, orbit | Structure highlighting mode. |
| `cellsVisible` | Structure / boolean | true / false | Show the cell geometry; labels and slots are unaffected. |
| `sliceVisible` | Structure / boolean | true / false | Restrict the visible geometry to the selected normalized 4D w band. |
| `sliceMin` | Structure / number | -1..1; step 0.01 | Lower normalized 4D w bound of the visibility band. |
| `sliceMax` | Structure / number | -1..1; step 0.01 | Upper normalized 4D w bound of the visibility band. |
| `accentPrimary` | Colour / colour | OKLCH | Primary instrument accent in OKLCH. |
| `accentSecondary` | Colour / colour | OKLCH | Secondary instrument accent in OKLCH. |
| `finish` | Material / enum | matte, gloss, glass | Surface finish, with gloss controlled independently. |
| `keyLight` | Light and depth / number | 0..2; step 0.01 | Directional key-light gain; default matches S-B shading. |
| `fillLight` | Light and depth / number | 0..2; step 0.01 | Ambient fill-light gain; default matches S-B shading. |
| `depthOfField` | Light and depth / number | 0..1; step 0.01 | Depth-of-field amount, disabled by default. |
| `ambientOcclusion` | Light and depth / number | 0..1; step 0.01 | Ambient-occlusion amount, disabled by default. |
| `inertia` | Motion / number | 0..1; step 0.01 | Camera inertia amount. |
| `settle` | Motion / curve | a,b in [0,1] | Normalized settling response with controls (a,0) and (1-b,1). |
| `cameraDamping` | Motion / number | 0..1; step 0.01 | Camera damping amount. |
| `fieldOfView` | Structure / number | 0.2..2.8; step 0.01 | Vertical field of view in radians; zoom is one over tan(fov/2). |
| `panelDensity` | Frame / number | 0.5..2; step 0.05 | Instrument spacing and row density scale. |
| `typeScale` | Frame / number | 0.75..2; step 0.05 | Typography scale relative to the app's base type sample. |
| `layoutId` | Layout / enum | central-stage, docked-workbench | Greybox layout structure id. |
| `viewportShare` | Layout / number | 0.3..0.9; step 0.01 | Requested fraction of the window devoted to the stage. |
| `panelPlacement` | Layout / enum | docked, overlay | Instrument panel placement mode. |

Defaults preserve S-B cs = 0.76, ss = 0.82, d4 = 1.18, zoom = 1.15, turnMs = 190,
smoothstep easing and its sRGB background (0.13, 0.145, 0.16), converted to OKLCH.
Key/fill gains are 0.5/0.5 and additional material/depth effects start disabled.
The sticker palette is an authored ring palette, using the Taste Lab rules;
S-B's per-cell HSV colouring has no equivalent ring preset. Accent, frame and
layout defaults are draft exploration values, with no owner taste approval implied.

## Presets (`magic600-look-preset`, version 1)

The six keys are exactly format, version, name, family, scene and params, in that
order. family is f1, f2 or null; scene is solving, inspecting, celebrating or null.
Every schema id is required. Unknown or missing ids, duplicate keys, wrong types,
unknown format/version, non-finite and out-of-range values are refused with the
id in the error. There are no implicit migrations. `PresetMigrations.Migrate`
is the explicit version hook; no older-version migration is registered yet.
Parsing checks the source format and version before applying that hook, then
validates the migrated text's current keys and values. Tests inject a migration
through an internal per-call overload, leaving the production registry unchanged.

Canonical bytes use UTF-8 without BOM, LF, two-space indent, schema parameter
order, culture-invariant shortest round-trip doubles, integral integer values
without decimal points, and a final newline. `CanonicalBytes`, `Save` and
`Load` implement the byte-for-byte round trip. The full default example is
also committed as presets/default.json:

```json
{
  "format": "magic600-look-preset",
  "version": 1,
  "name": "S-B geometry default",
  "family": null,
  "scene": null,
  "params": {
    "hueRotation": 0,
    "hueSpread": 360,
    "lightness": 0.7,
    "lightnessAlt": 0.05,
    "chroma": 0.12,
    "classes": 6,
    "bgLightness": 0.26208135203457433,
    "bgHue": 248.1740890656654,
    "bgTint": 0.009079176564054185,
    "gap": 0.24,
    "edgeWeight": 0,
    "edgeBrightness": 0,
    "gloss": 0,
    "glow": 0,
    "fog": 0,
    "turnMs": 190,
    "easeA": 0.3333333333333333,
    "easeB": 0.3333333333333333,
    "projection": "perspective",
    "d4": 1.18,
    "stickerShrink": 0.82,
    "highlight": "none",
    "cellsVisible": true,
    "sliceVisible": false,
    "sliceMin": -1,
    "sliceMax": 1,
    "accentPrimary": {
      "L": 0.75,
      "C": 0.09,
      "h": 220
    },
    "accentSecondary": {
      "L": 0.75,
      "C": 0.09,
      "h": 40
    },
    "finish": "matte",
    "keyLight": 0.5,
    "fillLight": 0.5,
    "depthOfField": 0,
    "ambientOcclusion": 0,
    "inertia": 0,
    "settle": {
      "a": 0.3333333333333333,
      "b": 0.3333333333333333
    },
    "cameraDamping": 0,
    "fieldOfView": 1.4314871793377604,
    "panelDensity": 1,
    "typeScale": 1,
    "layoutId": "central-stage",
    "viewportShare": 0.7,
    "panelPlacement": "overlay"
  }
}
```

`ThemeSet.Check` reports missing/duplicate family-scene entries and every
Structure-group difference across the two families and three scenes. It
does not silently choose a common projection or modify presets.

## Taste Lab import (`tastelab-export`, versions 2 and 3)

```json
{
  "kind": "tastelab-export",
  "version": 2,
  "presets": [
    {"family": "f1", "scene": "solving", "name": "Unrated", "look": null}
  ]
}
```

`TasteImport.Parse` maps each populated family/scene/name/look entry, requiring
all 18 Taste Lab ids and using schema defaults only for new Look Lab ids.
Version 3 adds `images` (the Images tab's ratings), which the Look Lab
ignores; `presets` is the same in both. Other export versions are refused. A null look is skipped and reported with
family, scene and reason. It produces no default preset. A malformed non-null
look fails the entire import and names the entry and parameter. Other export
metadata is ignored. `TasteImport.Write(text, schema, outputFolder)` validates
the entire export before removing its previous preset-NNN.json files in the
output folder and writing the new set. Other filenames and subfolders are
untouched; an invalid export removes and writes nothing. Its
production caller must pass `work/loop-memory/looklab/presets/`; tests pass a
fresh temporary folder. Filenames do not derive from exported names.

## Colour and motion reports

`Colour` ports the Taste Lab's published OKLab matrices and computed inverses,
OKLCH, sRGB conversion, 40-step gamut mapping, alternating palette lightness,
palette hue spacing and background. CVD uses the same Machado 2009 severity-1
linear-RGB matrices for protan, deutan and tritan, with RGB clamping.

`Colour.Report` builds class pairs from face-adjacent cells in different rings,
using the deterministic proper ring colouring. Same-ring edges are counted
separately (600 undirected adjacencies) and contribute no distance minimum.
For normal and each CVD mode it reports the minimum OKLab class-pair distance
and minimum lightness distance to the background. In a CVD mode the background
is simulated too. Optional `PaletteThresholds` come from the caller; without
them a report with a valid gamut assigns no marks. Normal-mode minima and all colour-distance
minima are compared with the original space.js hardCheck in the Node test.
Gamut mapping failures return `GamutFailures`, each with a zero-based class
index (null for the background) and the mapping reason. `GamutOk` is false,
all mode marks and `MeetsThresholds` are false, and uncomputed minima are null.
G4 callers use `MeetsThresholds`: it is null for valid gamut without caller
thresholds, true when all supplied marks pass, and false for any failed check.

`Easing.Ease` uses preview.js's 40-step bisection and Bezier controls in the
interior; clamped endpoints return exact 0 and 1. This avoids floating-point
plateaus at flat endpoint derivatives and stays within the JS tolerance.
`TurnAngle` is angle * ease(clamp(elapsedMs/turnMs)); the clock alternates its sign.
Tests cover endpoints, monotonicity over the full control domain, clamping,
mid-turn, revision boundaries and the original JS values where Node exists.

## Layouts (`magic600-look-layout`, version 1)

A layout has an id, positive grid dimensions, fractional regions, and one
placed/hidden command map per command-table context. Regions have a unique id,
stage/panel/overlay kind, rect, docked/overlay placement and active contexts.
Rects must fit the window. Every command is placed in an available region or
explicitly hidden in every context it lists. Unknown ids, contexts, duplicate
placements and incomplete coverage fail. `any` is a named command-table context;
it is not expanded into an inferred wildcard.

This complete tiny example is for a catalogue with commands a and b, both in any:

```json
{
  "format": "magic600-look-layout",
  "version": 1,
  "id": "tiny",
  "grid": {"columns": 5, "rows": 5},
  "regions": [
    {"id": "left", "kind": "stage", "rect": {"x": 0, "y": 0, "width": 0.2, "height": 0.2}, "placement": "docked", "contexts": ["any"]},
    {"id": "right", "kind": "panel", "rect": {"x": 0.4, "y": 0, "width": 0.2, "height": 0.2}, "placement": "docked", "contexts": ["any"]}
  ],
  "contexts": {"any": {"placed": {"a": "left", "b": "right"}, "hidden": []}}
}
```

The defaults are data/layouts/central-stage.json (a central stage and contextual
overlay instruments) and docked-workbench.json (stage and multiple docked panels).
They cover the actual docs/progress/1.0/command-table.json without invented commands.
They are greyboxes for exploration, not final owner-approved layouts.

## Flows (`magic600-look-flow`, version 1)

```json
{
  "format": "magic600-look-flow",
  "version": 1,
  "id": "open",
  "name": "Read a session",
  "draft": true,
  "steps": [
    {"command": "session.resume", "context": "any"},
    {"command": "view.residuals", "context": "any"}
  ]
}
```

Each step names a real command and one of its contexts. Unknown commands and
unavailable contexts are refused on load. A flow cannot run metrics on a
layout that hides a step. The six draft flows follow solving-workflow.md:
read session, piece operation, protection/recovery, macro reuse, endgame,
and next orbit. They remain draft until the owner's storyboards replace them.
The runner reports command steps; it executes no puzzle commands.

`FlowMetrics.Report` takes window width and height in pixels. It starts the
pointer at the first region's centre and sums Euclidean centre-to-centre travel.
Target width W is the smaller pixel dimension of the destination region.
Every step, including the first with D = 0, has Shannon-form time
`a + b*log2(1 + D/W)`, with **a = 0.1 s and b = 0.15 s/bit**.
These constants and times are not measurements. On the tiny layout at 100x100,
travel is 40 pixels, W is 20 and total time is `0.2 + 0.15*log2(3)` seconds.
These are coarse region-acquisition estimates, not timings of actual controls,
reading, thought, keyboard input or solver effort.

## Costs (`magic600-look-cost`, version 1)

```json
{
  "format": "magic600-look-cost",
  "version": 1,
  "features": {
    "edgeWeight": null,
    "gloss": null,
    "glow": null,
    "fog": null,
    "highlight": null,
    "finish": null,
    "keyLight": null,
    "fillLight": null,
    "depthOfField": null,
    "ambientOcclusion": null
  }
}
```

Every schema costFeature is required. A future measured entry has the shape
`{"milliseconds": 1.25, "source": "full-detail measurement identifier"}`;
milliseconds must be finite and nonnegative, and source must be nonempty.
The real data/cost.json contains only null entries, because H-06 is not delivered.
`CostTable.Status` says **not measured** for null entries. The core makes no
performance estimates or performance claims and does not assume costs add.

## Fixture provenance and port choices

fixtures/PROVENANCE.md records the S-B source branch claude/renderer-sb, its
immutable commit, input SHA-256 digests and production commands. sb-reference.json
has nine cases, each with 300 samples evenly spread across 9,066 source indices.
The tests compare all 2,700 results with absolute-plus-relative tolerance
`1e-5 + 1e-5*abs(reference)`. w3-turn.json is copied byte for byte and checked
against the original reference index's input digest. Acceptance needs none of
the external read-only source copies.
three-cycle-turn.json is a separate test-only permutation with its file and
full-label digests recorded in PROVENANCE.md; it makes inverse-array use observable.

The engine's orbit-34 piece positions determine ascending vertex ids, following
core.py. Their retained face incidence determines vertex_cells and cell_vertices;
only those incidence arrays and normals are read from model.npz. This avoids
inventing a geometry-based vertex numbering. Each cell has four maximal-dot
face neighbours with a clear gap to the fifth, each vertex meets twenty cells,
and each cell has four vertices. Geometry is also checked against retained planes.

The helix port enumerates sorted engine-cell vertex permutations, sorts unique
30-cell memberships lexicographically in engine cell ids, searches the lowest
uncovered engine cell, and takes the first 7-regular exact cover, as Taste Lab
does in its own cell order. The 240 candidates give twenty 30-cell rings.
Rings store sorted membership like Taste Lab; RingChains additionally gives a
deterministic closed face-neighbour traversal. rings.json pins vertex positions,
engine-cell-to-ring ids, memberships and the ring graph. ProperColouring tries
rings and colours in ascending order and is tested for classes 4 through 8.

Additional choices required where the plan leaves representation open:

- gap, stickerShrink and fieldOfView use Structure, following the plan's
  geometry contract: they define cs, ss and zoom respectively.
- Visibility is cellsVisible plus an optional normalized w slice band. It is
  view data, with all labels and slots retained. Inverted slice bounds describe
  an empty band; the core never reorders them or relabels state.
- Settle is a normalized Bezier response, independently of the shared turn
  easing ids. New numeric ranges and control increments are exploration bounds,
  documented in the schema, without value quantization.
- Canonical colour keys are L, C, h and curve keys are a, b. CVD reports simulate
  the background along with the palette; normal mode follows hardCheck exactly.
- The net8 JSON writer's platform newlines are normalized to LF. Easing returns
  exact clamped endpoints; all interior bisection steps retain the JS order.
- Flow steps explicitly carry context, and use the smaller region dimension as
  the Fitts target width. The first acquisition contributes the stated intercept.
- Draft region assignments use real command ids, with no hidden commands in the
  defaults. Import filenames use deterministic indices, avoiding private names.

Generation commands (standard library only):

```text
python tools/looklab/fixtures/make_sb_fixture.py <input_folder>
python tools/looklab/fixtures/make_rings_fixture.py
python tools/looklab/data/make_defaults.py
```

## Privacy and stop condition

Taste Lab and Look Lab outputs stay under **work/loop-memory/**: imported
presets at work/loop-memory/looklab/presets/, and later captures and swipe records
under work/loop-memory/looklab/. Do not commit real exports, captures, user names,
session data, logs or private paths. Only code, defaults and small synthetic
fixtures belong here. Committing an imported preset remains the owner's choice.

LL1 is complete when check.py exits 0, every required core feature and test is
present, the check leaves the checkout unchanged, and only tools/looklab/ was
changed. Rendering, graphics checks, UI, capture and swipe are later packets.
