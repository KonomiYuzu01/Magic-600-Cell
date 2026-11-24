# Look Lab app and core (LL1–LL2)

`LookLab.Core` is the framework-neutral, net8.0 core of the local Look Lab.
It has no Godot, engine or NuGet dependency. It loads verified full-model
geometry, projects it in double precision, and owns the parameter, preset,
colour, turn, layout, flow and cost rules. The staged Godot .NET app consumes
these rules. The core checks supply no renderer evidence.

Run from the repository root:

```text
python tools/looklab/check.py
```

The standard-library harness restores and builds both projects offline, runs
the console tests, removes its unique temporary folder, and compares all
checkout files and directories before and after, including ignored files.
Build products, .NET CLI state and NuGet caches stay in that folder. Unknown
arguments exit 2. A failing step prints its diagnostics and exits 1;
a cleanup failure also fails the check. No installation is performed.
The recipe needs a .NET SDK with the net8.0 targeting pack and runtime already
installed. Node is optional: tests that call the original Taste Lab JavaScript
report a skip when it is absent. All other tests always run.

The test runner has 88 named tests, including 49 Python harness cases. It
prints a line for each named test and a summary, and exits nonzero on failure.
It reads the repository and committed fixtures only, and creates its own
temporary folders with mkdir under the system temporary directory. It opens
no real Taste Lab export, personal session or database.

## Staged Godot app and lanes (LL2)

```text
python tools/looklab/check.py --godot
python tools/looklab/check.py --godot-gpu
python tools/looklab/check.py --run
```

The CPU lane (`--godot`) first runs the core check, then builds a source-only
temporary copy of `app/` and `core/`, imports it headless, runs the minimal
scene's CPU probe, then runs the app smoke test and three negative controls.
The smoke test loads geometry with digest checks, builds 600 instances,
changes all 42 parameters once through the panels' validated setter, and saves
and reloads canonical preset bytes in the run's temporary folder. A skipped
parameter, wrong instance count and changed byte must each fail. Headless
Godot's dummy renderer provides no rendering evidence. The graphics lane
(`--godot-gpu`) uses a visible 640 x 360 window,
requires the Windows display server, Vulkan or Direct3D 12, and a non-empty
adapter name. Its stage-0 shader writes four known values into an RGBA32F
target; the exact readback must pass, and an injected shader offset must fail.
It then checks the drawing shader's geometry for all nine S-B cases (cameras
c0–c2, start/middle/end), comparing 2,700 `(ndc_x, ndc_y, w)` outputs against
both the core and the fixture. The cells-only frame must report 6,096,000
primitives and 600 visible instances. A shader-transposed Q and a missing
instance must each fail; an unrelated error cannot satisfy a negative control.
Only the graphics lane supplies rendering evidence. It is run by the integrator
outside other renderer sessions' measurement windows. `--run` stages, builds
and imports, then opens the app with the checkout root as its argument; it
does not run the core tests and is not a check.

Exit codes are **0** passed (or interactive run ended), **1** failed, **2**
usage, and **3** graphics lane not run because a window or real rendering driver
could not start. Exit 3 is never a pass. A headless display, dummy driver or
empty adapter fails with exit 1, as does missing graphics evidence.

`LOOKLAB_GODOT` may name the console executable. Otherwise the harness requires
exactly one `Godot_v4.7.2-stable_mono_win64_console.exe` recursively under
`%LOCALAPPDATA%/Microsoft/WinGet/Packages/GodotEngine.GodotEngine.Mono_*/`.
It checks for `4.7.2.stable.mono` before building. The staged `nuget.config`
clears inherited sources and uses only `GodotSharp/Tools/nupkgs/` next to that
executable. The private cache is checked for the 4.7.2 Godot packages, their
recorded source, and bytes matching the local archives. No machine path is
committed, and no network source or installation is used.

All children receive the parent environment without names containing `KEY`,
`SECRET` or `TOKEN`, case-insensitively; test-runner children inherit that
environment and also apply the same filter. `APPDATA`, `LOCALAPPDATA`, `TEMP`
and `TMP` point inside the run's temporary folder. A unique project name and
directory handshake protect both editor import and scene runs: each reports
its user, config, data and cache directories and waits for validation before
proceeding. The harness checks that real Godot AppData top-level names and
modification times are unchanged and that no real user-data directory has the
run's name. It removes the staging folder and verifies the checkout, including
ignored files. Build/core processes have 180-second limits, Godot imports and
probes 120 seconds (the A5 sweep 400 seconds), version probes 15 seconds, and the directory handshake
20 seconds. Interactive runs have a two-hour limit. Expiry ends the process
tree with `taskkill /T /F` on Windows and fails the run.

The viewport is always full detail: an unindexed base triangle list of 30,480
vertices is instanced 600 times. Three rows of each 4D cell frame occupy the
instance transform, and its fourth row occupies `INSTANCE_CUSTOM`. Vertex XYZ
and UV hold the verified 4D position and sticker index. The vertex shader
performs shrink/R, cell frame, moving-slot turn, Q, 4D projection, zoom and
aspect in the core's order. Labels use an R32F texture (u32 values up to
259,799 are exact in f32); all 259,800 entries are uploaded for each bound
revision. A core `TurnClock` drives the generator/inverse and those snapshots.
Colours follow `label / 433` and the core's proper ring colouring. Visibility
discards geometry without changing slots or labels. Drag in the viewport to
rotate Q in two 4D planes; inertia, damping and the settling curve affect the
camera response. Costs remain **not measured**.

There is one panel section per schema group. Number/integer sliders and fields,
enums, booleans, OKLCH components and Bezier control points update the look
immediately. Editing one colour or curve component preserves the exact values
of the other components. The fibration highlight uses ring 0, and the orbit
highlight uses vertex orbit 34: these fixed reference sets avoid adding
selection controls beyond LL2. The frame/layout controls adjust instrument
spacing, type size and the stage/panel split; greybox commands and flows remain
for LL4. Load and Save use the core's strict canonical preset format. Dialogs
and the file boundary both limit access to `work/loop-memory/looklab/presets/`
of the supplied checkout; links and paths escaping that directory are refused.
The directory is created on first Save, never during startup or a CPU check.

The RGBA32F checks use an explicit RenderingDevice framebuffer, because an
ordinary HDR SubViewport is half precision. Shaders are authored in Godot
shading language; the float check supplies minimal GLSL stage entry points for
the device's SPIR-V compiler. The geometry adapter takes both function bodies
directly from `cells.gdshader`, unchanged, supplies the Godot builtins from the
actual mesh/instance data and binds copies of the actual uploaded textures.
It contains no separate projection implementation. GPU comparisons use
`abs(actual - expected) <= 1e-4 + 1e-4 * abs(expected)`, the S-B GPU probe's
tolerance for single-precision matrices, trigonometry and division against the
core's double precision. The stage-0 powers-of-two values use exact equality.
CPU tests prove source-body reuse, parameter packing, shader fault binding and
the accessors' agreement with all 2,700 core projections; they cannot prove a
shader draw or readback. Graphics and interactive runs are reserved for the
integrator and must pass on the matching build before claiming rendering evidence.

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
cell_frames.f32, model.npz and slot_piece.u32. Each file is SHA-256 checked
against the `files` map of assets/manifest.json before its bytes are used;
the manifest is the trust root. Missing files, missing manifest entries and
mismatches name the file. The .npz reader uses System.IO.Compression. Its small
.npy reader refuses the wrong dtype, Fortran order, invalid shapes or lengths.
Normals require `<f8`; incidence metadata requires the retained `<i2`/`<i4` types.
Stickers shrink towards their anchors, as in the S-B specification (section 3):
the area-weighted centroid of each sticker's triangles, computed in float64 in
file order and rounded once to float32. `Geometry` refuses the assets unless
the SHA-256 of the 433 x 4 little-endian float32 anchors equals the S-B pin;
assets/mesh_centers.f32 is not read.

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
| `projection` | Structure / enum | perspective, stereographic, orthographic, hemispheres, hopfRings, ringSheet | 4D projection or Global layout; stereographic fixes d4 to one, and the three layouts place each cell in a chart. |
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
| `layoutId` | Layout / enum | central-stage, docked-workbench, split-views, focus-inset | Greybox layout structure id. |
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

## Taste Lab import (`tastelab-export`, versions 2 to 4)

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
Version 3 adds `images` (the Images tab's ratings) and version 4 a love
flag on those ratings; the Look Lab ignores `images`, and `presets` is the
same in all three. Other export versions are refused. A null look is skipped and reported with
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

The defaults in data/layouts/ are central-stage.json (a central stage and contextual
overlay instruments), docked-workbench.json (stage and multiple docked panels),
split-views.json (Global and Local side by side) and focus-inset.json (Global with a Local inset).
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

fixtures/PROVENANCE.md records the S-B source branch main, its immutable
commit, the anchor rule and pin, input SHA-256 digests and production commands. sb-reference.json
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

## LL3 swipe, compare and capture

The editor's Swipe, Compare and Capture buttons open the corresponding controls.
Swipe shows the current best on the left and one candidate on the right. Better
promotes the candidate; worse keeps the best. Use best in editor returns the best
to the existing preset editor, where it can be saved. The displayed seed and
initial canonical preset plus the answer sequence reproduce the candidate run.
Restart starts from the current best with the chosen seed; it writes nothing.

Candidates use a fixed unsigned 32-bit LCG (1664525*x + 1013904223, modulo 2^32),
changing one to three distinct schema ids. Numbers and integers move one schema
step, enums move to a neighbouring option, booleans flip, and colours and curves
move one component by 0.01 (one degree for hue). Changes clamp to their schema
ranges and reverse direction at a bound. Preset metadata is preserved. Each
answer appends one UTF-8, LF-terminated JSON line to a seed-named run file under
work/loop-memory/looklab/swipe/. Its five fields are time (UTC ISO 8601),
best_sha256 and candidate_sha256 (SHA-256 of canonical preset bytes before the
answer), verdict (better or worse), and changed_parameter_ids in schema order.
The answer writes no preset, capture or other record.

Compare selects two to four presets from tools/looklab/presets/ and the same
work/loop-memory/looklab/presets/ folder as the editor. Presets may be repeated
when only one exists. Each pane has its own full-detail viewport. One shared 4D
camera object supplies Q to every pane; local Godot 3D cameras only provide the
viewport's culling and per-preset effects. Dragging any pane updates all panes.
One shared TurnClock, using the first selected preset's timing and easing,
supplies a single sampled frame to all panes. Swipe uses each preset's own turn
timing so motion candidates remain visible.

Save PNG captures the viewport at its native pixel size. Save clip captures one
to ten seconds at a fixed 24 fps, streaming native RGBA frames into a locally
installed FFmpeg with libx264 and writing MP4/H.264. Odd clip dimensions are padded
to the next even pixel for yuv420p compatibility; stills are never resized.
LOOKLAB_FFMPEG can select an existing local executable; otherwise PATH is used.
The graphics harness also discovers an existing WinGet FFmpeg before redirecting
AppData. No encoder is installed or downloaded. Without FFmpeg the clip control
is disabled with that reason; stills work. The editor captures its stage; compare
offers a capture-pane selector, and swipe captures the best pane.

Captures stay under work/loop-memory/looklab/captures/ and byte-identical copies
go under work/gallery/looklab/ for the workbench's existing bounded GalleryScanner
(.png images and .mp4 videos). These private outputs must not be committed.
The --run checkout accounting permits changes only under work/loop-memory/looklab/
and this gallery mirror.

The CPU acceptance command remains python tools/looklab/check.py --godot. It adds
scripted better/worse answers with exactly two valid records and compare runs
with two, three and four panes sharing one camera and clock at five turn times.
Disabling the answer handler or shared clock must fail its own check. Core tests
also exercise seeded replay, bounds, PNG CRC/decoded size validation, fixed clip
frame timing, gallery-copy rules and the gallery's own scanner on prepared files.
The graphics lane captures a native PNG and, when local FFmpeg exists, a one-second
clip, decodes every clip frame to check dimensions and timing, runs the gallery's
own scanner, and requires a disabled capture handler to fail. Only the integrator
runs that lane; headless checks and prepared fixtures provide no graphics evidence.

## LL4: greybox and flow runner

Enable **Greybox and flows** at the top of the parameter panels to put grey
regions beside the full-detail views. The four structures are **central-stage**
(contextual overlays), **docked-workbench** (docked panels), **split-views**
(Global and Local side by side) and **focus-inset** (Local inset over Global). Select a
layout and a catalogue context to see its command ids in the assigned regions.
Each region has a scrollable command list; clicking an id highlights it.
`any` remains a named context. It is not expanded into a wildcard. Commands
listed as hidden by a layout are omitted.

Choose one of the six draft flows, set the model window width and height in
pixels, and use **Run flow** and **Next step**. The runner highlights each
command in its own context and region. It previews the script and executes no
puzzle command. It resolves the complete flow before publishing its report;
an unreachable command is refused by id and context with no partial or previous
report. Changing the layout, flow, context or model dimensions clears the report.
Model dimensions describe the window being compared; the greybox scales its
fractional regions to the available display area.

The report preserves every number from `FlowMetrics`: each step's command,
context, region, centre-to-centre travel, target width and estimated seconds,
plus total travel and seconds. It displays the Shannon-form model and its
constants **a = 0.1 s**, **b = 0.15 s/bit**. Target width is the destination
region's smaller pixel dimension. These are estimates of region acquisition,
not measured task times or performance results.

**Save report** creates a unique `flow-<id>.json` only under
`work/loop-memory/looklab/flows/` of the supplied checkout. No output directory
is created during startup or headless checks. Saved reports use
`magic600-look-flow-report` version 1, with camelCase keys: `format`, `version`,
`layout`, `flow`, `name`, `draft`, `windowWidthPixels`, `windowHeightPixels` and
`report`. The nested report retains the core's `steps`, `travelPixels`,
`estimatedSeconds`, `aSeconds` and `bSecondsPerBit`. UTF-8 JSON uses a two-space
indent, LF and a final newline; numeric values round trip without precision
loss. Generated filenames avoid using flow metadata as paths, existing reports
are retained, and paths through links are refused.

`python tools/looklab/check.py --godot` adds three headless result lines: exact
metrics for the fixed docked layout and 14-step piece-operation flow, refusal
of an unreachable final command, and exact catalogue coverage in all eight
contexts on all four layouts: `layouts=4 contexts=32 commands=209 placements=348` (209 with LP5's research
delta, `docs/progress/1.0/command-rows-research.json`, whose rows are hidden in context `any`).
The harness also requires the refusal-handler and
catalogue-filter faults to fail their own checks. Core tests cover all six
flows on all four layouts, progression, refusal/reset, JSON precision and scoped
output writes; Python tests cover missing/repeated results and negative-control
requirements. These CPU checks supply no rendering or performance evidence.
The integrator runs the graphics and interactive lanes.

## Colour vision, palette panel and cost meter (LL5)

The scrollable instrument panel starts with a colour vision selector (normal
by default, protan, deutan and tritan), the current preset's palette report,
and its full-detail cost entries. The selector is preview state, not a preset
parameter. CVD runs after the drawing shader's lighting, edges and fog, on its
clamped linear RGB, with the core's Machado severity-one coefficients and the
same final clamp. The clear background uses the same uploaded coefficients.
The step defaults off and is bypassed by the existing geometry probe. No
mechanical state, labels, geometry or core results change.

The palette panel shows all four modes' minimum OKLab distance and minimum
background lightness distance, the zero-based class pairs, the same-ring face
adjacency count, and every gamut failure's class (or background) and reason.
Unavailable distances stay unavailable. `PalettePanel` accepts optional
`PaletteThresholds` from its caller and passes them to the core; without that
input it displays values and no pass/fail marks, including for gamut failures.
Preset edits and loads refresh these controls while retaining the CVD mode.

The meter loads `data/cost.json` through `CostTable` and follows the schema's
`costFeature` mappings for the preset's settings. It includes zero/disabled
settings, because the table gives feature entries and has no activation or
parameter-value rule. Each null entry reads **not measured**; a measured entry
shows its stored milliseconds and source verbatim. An unmapped table feature
is not shown. The status line uses the meter's summary. Values are never
measured, estimated, scaled, added or extrapolated by the meter.

`check.py --godot` adds three result lines: CVD (52 fixed colours across four
modes, plus five background checks), cost (ten null entries and one synthetic
measured entry with its source and a live setting/status update), and palette
(caller thresholds and live structured gamut failures). Disabling the CVD
transform or the meter's null-text branch must fail its own check. The managed
runner has 88 named tests. The consolidated LL7 harness retains the LL2, LL5 and LL6
isolation and fault cases and checks the updated combined schedules; it has 49 tests.

The graphics lane adds a readback of the same 52 known linear colours through
the actual drawing fragment body into RGBA32F; a wrong uploaded matrix must
fail. Its procedural vertex input supplies the known colours with unit
diffuse and no material effects, rather than authoring another CVD shader.
The CPU checks cover the coefficients, fragment-body reuse, matching varyings
and uniform/fault packing. Both CVD checks require an absolute error of at
most **2e-6 per linear RGB component**: three single-precision products and
additions plus clamping are compared with the double-precision core. RGBA32F
avoids 8-bit and half-float quantization. CPU checks supply no GPU evidence;
the integrator runs the graphics lane and its wrong-matrix fault.

## Global and Local greyboxes (LL6)

**split-views** places Global on the left and Local on the right, with instruments,
commands and review below. **focus-inset** places Local over the lower right of
Global, below the review overlay. These are provisional layouts for G1 exploration.
The layout format remains `magic600-look-layout`, version 1. It accepts either
stage regions or exactly one docked `global` and one docked/overlay `local`,
with identical context lists and no stage region. An overlay Local must fit within
Global. All eight contexts remain available; only the four stage commands route
to Global. No catalogue command is added. A paired layout always reserves the
parameter panel's width, as in docked placement, so the panel covers neither view.

The two views share one look, context, rotation and bound turn revision. Global
owns the clock and inertia. A drag in either pane rotates that camera once;
Local copies its rotation and binds the same turn frame after each Global step.
Local is created on first selection of a paired layout and stops processing and
rendering when hidden. CVD mode, coefficients and clear background follow both
views, including a Local created after a mode has already been selected.
The existing layouts continue to use a single view filling the original stage
rectangle; greybox mode arranges the views within its right-half stage area.

Local frames the moving slots' rest anchors at theta zero. The anchors use the
current gap, shared rotation and Local projection/aspect. Smooth weights fade
at clip w from 0.5 to 1, at each NDC frame edge from 1 to 1.25, and at the 4D
denominator from 0.02 to 0.1. Weighted focus and magnification vary continuously
towards the neutral frame when nothing is visible. Magnification lies in [1,3];
the centre is clamped to keep the target window inside the full frame. The
displayed frame follows exponentially with a 0.12 s time constant, capped per
axis at 2 NDC units/s and at 4 magnification units/s. First appearance sets the
target directly; non-finite or non-positive time steps leave the follower unchanged.
Global always has the neutral `view_frame`. Production position and geometry
readback derive from the same framed clip vector.

`check.py --godot` adds four LL6 result lines: arrangements for four layouts at
two sizes in normal and greybox modes with overlay and docked panel placement,
with neither paired view under the parameter panel; exact-once drags, inertia and
shared frames across two turn boundaries; all four CVD modes through background
edits and layout switches; and settled framing plus per-step follower bounds. Six
injected faults must fail their own checks: hidden Local, a parameter panel
covering Local, ignored Local drag, an independent Local clock, CVD applied only
to Global, and neutral Local framing. Core tests
cover parser refusals, projection, arrangement, the three reviewed focus regressions,
seeded sweeps and crossings of every guard, empty transitions and the follower.

The GPU lane includes the original nine neutral cases (2,700 samples) and one
non-neutral frame (300 samples), checked at the existing `1e-4 + 1e-4*abs(expected)`
tolerance. Removing the production framing statement at shader load must make
the non-neutral case fail. The integrator runs this lane; headless checks provide
no rendered, window-input or performance evidence.

## Global layouts and the G1 catalogue (LL7)

The editor's `projection` parameter switches among six structure modes. The
catalogue in `presets/g1-*.json` supplies one preset for each, plus a
stereographic variant with the polar cap hidden:

| Mode | Preset | Name |
| --- | --- | --- |
| perspective | `g1-perspective.json` | G1 perspective |
| stereographic | `g1-stereographic.json` | G1 stereographic |
| orthographic | `g1-orthographic.json` | G1 orthographic |
| hemispheres | `g1-hemispheres.json` | G1 two hemispheres |
| hopfRings | `g1-hopf-rings.json` | G1 Hopf rings |
| ringSheet | `g1-ring-sheet.json` | G1 ring sheet |
| stereographic | `g1-stereographic-cap.json` | G1 stereographic, polar cap hidden |

These use the default look; the orthographic preset sets `fieldOfView` to
0.5935577461607933 radians to fit the view. The other six keep the default
1.4314871793377604 radians. Compare mode lists these presets, so up to four
modes can be placed side by side.

At the starting orientation the plain stereographic preset puts the fixed
camera inside the 20 cells around the projection pole: their stickers reach
w > 1, past the projection's singularity at w = 1, so the view shows one cell
from inside. The cap variant turns on the w band and hides w above 0.5, the
60-degree cap around the pole: 70 cells whole and 60 in part. The band's lower
bound stays at -1, the lowest value the schema allows, so it also trims the 20
cells around the opposite pole, whose stickers reach just below w = -1. At rest
with identity Q, every corner of a drawn triangle is inside the frame and at
least 2 units in front of the camera, and the drawn part, including the points
where the band cuts a triangle, stays in the middle half of the frame. The band
is fixed in the model frame, so a drag turns the hidden cap away from the
projection pole and the camera can end up inside drawn cells again. The variant
is meant for the starting orientation only.

The three Global layouts place cells using charts of their 4D points. Hopf
rings uses the chosen right Hopf cover: each ring is a circle of 30 cells on
the tangent circle of its base point, with ring 0 facing the viewer. Hopf rings
and the ring sheet use affine charts. In the ring sheet, rows are the Hopf rings
in path order, cut open at half-step 0; each cell's face neighbours in the rows
above and below sit one half-step to its left or right, a few of them across
the cut. Two hemispheres puts each cell in the ball of its centre's side under
Q, mirrors the north ball and clamps the projection denominator at 0.25.
This hemisphere chart applies its clamped division after the 4D rotation.

A moving sticker is drawn between its own chart and its target cell's chart,
with blend weight `clamp(abs(theta / Angle), 0, 1)`. At `|theta| = Angle` it
reaches its destination slot's rest placement up to the retained-input residual;
the core test maximum is 9.3e-8 at gap 0.24 before the chart scale. This is an
approximate handoff, with the chart's scale or denominator affecting the error.

Every drag turns both Q and Spin, including inertia. The projections and
hemispheres show Q, Hopf rings shows Spin, and the ring sheet shows neither
and stays facing the viewer. Compare panes and linked Global/Local views copy
both rotations. Local framing uses the same placement and Spin as the drawing;
Global keeps its neutral frame.

For Hopf rings and the ring sheet, each Hopf ring has one colour: an equitable
proper colouring of the Hopf ring graph with `classes` colours (4 to 8). The
other modes keep the Look Lab ring colouring. The palette panel reports the
colouring that is drawn, including after projection edits, class edits and
preset loads. The deterministic `fixtures/hopf-rings.json` records the chosen
cover and its 3-regular graph. The retained `rings.json` mixes ten rings from
that right cover with ten from a left cover and has a 7-regular graph. See
`fixtures/PROVENANCE.md` for the inputs, generator and comparison.

```text
python tools/looklab/check.py --stills
```

This opens a real graphics window, stages the app and captures all seven G1
presets with identity Q and Spin, at rest and at mid-turn, at 1600 x 1000.
The fourteen PNGs go to `work/loop-memory/looklab/captures/`, with a mirror in
`work/gallery/looklab/`. The tool prints each preset, pose and relative path,
then quits. It is a tool for tuning and for the owner's G1 pick, not a
measurement. Only the app's output folders may change during this run.

Core LL7 tests recompute the Hopf facts and fixture, chart orientation and bead
identity, separation and fit, target tables and retained-input handoffs,
equator margin, drag signs, equitable colouring, palette reports, framing, the
seven presets, and the cap variant's hidden cells and frame fit.
`check.py --godot` appends four headless results, each with one injected fault
that must fail its own check:

| Result | Injected fault |
| --- | --- |
| `LOOKLAB_LL7_PRESETS_PASS` | `ll7-projection-clamped` |
| `LOOKLAB_LL7_SPIN_PASS` | `ll7-compare-spin-not-copied` |
| `LOOKLAB_LL7_LAYOUT_PASS` | `ll7-palette-old-colouring` |
| `LOOKLAB_LL7_FRAMING_PASS` | `ll7-framing-ignores-spin` |

These check all uniforms, the slice band and six kinds across seven presets,
exact drag/inertia and shared rotations, exact layout/class texture data with
the displayed palette report (also after the panels and the view leave the
tree and return), and settled Local framing for all three layouts. Texture
checks read back the bound image and require it to equal the exact uploaded
bytes; if a renderer returns no image, they compare private copies instead.
`LOOKLAB_LL7_TEXTURE readbacks=... uploads=...` says which path was used.

The GPU lane retains its ten geometry cases and adds six layout cases: each
layout at rest and mid-turn, with non-identity Q and Spin and 150 moving plus
150 stationary vertices per case. Both turn target tables are exercised. The
new result is `LOOKLAB_LAYOUT_GEOMETRY_PASS cases=6 samples=1800 target=RGBA32F`;
removing the production blend statement must fail as `layout-no-blend`.
The integrator runs `--godot-gpu` and `--stills` after integration. Headless
checks provide no rendered, window-input or performance evidence.

## The G2 shot (LL8)

A shot is a fixed camera drag path and turn schedule, stored in format
`magic600-look-shot`. The committed `data/shots/g2.json` is ten seconds at
24 fps and 1600 x 1000: a slow drift, one turn and its inverse, with frame 60
as the hero still. It is called a shot because a preset's `scene` already
means a scene look. Every preset renders the same 240 states. The shot sets
all timing, including easing; preset `turnMs`, `easeA`, `easeB`, `inertia`,
`cameraDamping` and `settle` do not apply.

```text
python tools/looklab/check.py --shot [preset ...]
```

Supply preset stems such as `default g1-hopf-rings`, or omit them to render
every preset in ordinal order. This requires a real graphics window. Each
run writes one `<stem>.png` hero still per preset and `manifest.json` under
`work/loop-memory/looklab/shots/<run>/`. The manifest format is
`magic600-look-shot-run`; it records input and still digests, the track digest,
capture waits, tool versions and source identity: commit, clean flag and a
digest of the staged app/core sources. It names the graphics adapter and
stays private. Dirty sources produce a draft run. Clean, committed Look Lab
sources produce a formal run: once every preset has passed, its stills are
copied byte for byte to `work/gallery/looklab/shots/<run>/`; the manifest is
not mirrored.

After the sequence, every preset re-captures the hero, first and last frames
with eight frame waits, comparing their bytes with the sequence captures
(three waits each). A mismatch means stale readback or history-dependent
effects. The hero image must also differ from the first and last images;
otherwise the readback is frozen or the turn does not show in that look. The
first and last images may match: the ring sheet ignores the camera, and the
shot starts and ends at rest with the same labels. The shot must therefore
put its hero frame inside a turn, at an angle that differs from the first and
last frames. Either failure stops the run before its manifest is written.

`check.py --godot` adds `ll8-shot`, a synthetic headless check of camera and
turn bindings, capture order and waits, hero pixels, gallery copies and every
manifest field. Its three injected faults cover a camera never pushed to the
material, stale capture and frozen capture. `check.py --godot-gpu` adds
`ll8-capture`: 48 encoded colours through production readback, with delayed
and frozen faults, then the shot's spot check with ambient occlusion and
depth of field enabled. Headless checks provide no GPU evidence; the
integrator runs the GPU probe and `--shot` after integration.

Clips are not made until the owner approves FFmpeg. Renders and manifests
stay private and are never committed.

## A5: live data and the kept view

The interactive lab (`python tools/looklab/check.py --run`) polls the JSON files
under `tools/looklab/data/` and `tools/looklab/presets/` and the command table
twice a second. When one changes and has been quiet for one poll, the lab is
rebuilt from the files in the same window. A file the lab refuses leaves the
last good lab on screen, with the reason in the notice line at the top left.

The camera, the look being edited and the panel state are kept across a rebuild
and across a restart in `work/loop-memory/looklab/view.json` (format
`magic600-look-view`, version 1: `q` and `spin` as 16 numbers each, the sha256
of the default preset the look was edited from as `defaultPreset`, the look as
`preset`, and panel state as strings under `ui`). The file is written at most
once a second and when the window closes. When `presets/default.json` changes
on disk, the new default is shown instead of the kept look. The interactive lab
opens with the values of `tools/looklab/data/opening.json` (format
`magic600-look-opening`) over the default preset: the Hopf-ring layout drawn
large, with smaller cells and stickers, so every cell is the same size and the
structure is easy to read. Checks, stills and shots do not read it. A panel adds its
state through `LabApp.ViewParts`.

Inside the window frame the Local view has its own shape, read from
`tools/looklab/data/local-view.json` (format `magic600-look-local-view`:
`projection` perspective or orthographic, `d4`, `gap` and `stickerShrink`
inside their parameter ranges, `margin` in radians, `fill` and `spread`). It
draws only the moving cells and the cells within the margin around them,
centred, at a fixed size and moved apart by `spread` so the inner cells show; the linked rotation turns them in 3D. Without the file Local is the
re-framed copy of Global. The lab saves a picture of its window, `frame.png`,
beside `view.json` four and twelve seconds after it opens. The window opens at
most of the screen.

The frame carries the two inspector tiers over one field model (a strip at
the top edge of the views and the docked inspector, collapsed by default) and
the bottom index (Problems, Console and Tasks, collapsed, with count badges).
The inspector's View fields are the lab's own parameters; the selection, the
preview parameters, the problems and the task are sample data and are marked
so. A typed `=` expression is kept as a live binding (view part
`frame.bindings`) and is not evaluated.
Labels are off by default; the tag button in the top bar turns them on (view
part `frame.labels`). The theme follows the Windows
light or dark setting until a choice is kept.

The script interface (ADR-D01 section 6 and ADR-D01.57 to ADR-D01.59) is in
the chrome library: the `.6cs` editor pane with diagnostics beside the code,
the Scripts dock, the console line, the palette's typed mode for a whole
line, two script panel slots in the right dock and the canvas. Every exported
`fn` or `command` of a program is a command row `script.<program>.<entry>`,
so the palette lists it and a key binds to it. Own bindings are kept apart
from the default keymap (view part `frame.keymap`) and over the preset in
force (`data/design/keymaps/`); `file.keymap.export` writes them as a
versioned keymap file. Ctrl+Enter in the editor runs the entry point at the
caret and Ctrl+Shift+Enter is the commit key there. Look Lab has no
interpreter: the entry points of the sample programs in `data/samples/` are
wired by hand in `app/Live/LabScripts.cs`, their values are sample values,
and an edited program is checked but not saved.

`python tools/looklab/check.py --godot` adds one result line,
`LOOKLAB_A5_LIVE_PASS reloads=1 refusals=1 restart=kept parts=15 checks=7`, and
three faults that must fail their own check.

The sweep (`app/Live/Sweep.cs`, mode `a5-sweep`) uses the whole frame the way
a person does: time runs past the end of the sample task, every available
command runs through the entry the palette, the keys and the radial menus
share, every dock panel, pane, workspace, theme family, scene look and
density is shown, every visible button takes a real mouse press through the
root viewport, and the close request keeps the view. A logged engine error, a
listener the chrome caught (`Safe.Raise`) or a button whose press did not
reach it fails the check. Its line is
`LOOKLAB_A5_SWEEP_PASS commands=N clicks=M errors=0`, with two faults (a
token no theme has, a listener that throws). While the window is open the
same failures light the error lamp in the top bar, become Problems rows and
are written to `work/loop-memory/looklab/errors.log`.

Local has two shapes on file, compared on F7: `data/local-view-cap.json`
(the default: stereographic, centred on the moving cells, the cells around
them faint and behind) and `data/local-view.json` (the moving cells spread
apart). F8 compares the two Global layouts (rings and the ball), and F9 steps
through the screens of the current showing. Global is fitted to its pane by
scale alone. A kept arrangement from an earlier showing is not restored (view
part `frame.showing`); a fresh start takes the scene of its theme family.

### A5 stage 7: the script interface

The Script workspace opens on `sweep.6cs`. The editor, the Scripts dock, the console, the script panel slots, the
mounts, the canvas and the package sheet are the chrome library (`app/Chrome/Model/Editor.cs`,
`app/Chrome/Ui/ScriptPanels.cs`, `app/Chrome/Ui/ScriptWidgets.cs`); `app/Live/LabScripts.cs` wires them to the
sample programs under `data/samples/`, whose entry points give fixed results.

- Every run waits in a visible queue and ends in a preview, a read result or a view change, never in a commit.
  A read result stays under its region and dims, with the name of what came between, after a commit or an undo.
- Diagnostics show in the gutter, at the end of the line, on the scrollbar and in the list; a fix shows as ghost
  text and Tab takes it. The bug button gives one fault of each severity.
- `mounts.6cs` mounts in six places (palette entries, a context action, an inspector section, a docked panel, a
  free view, an overlay annotation), each with a switch in the Scripts dock. Two panel slots, then tabs.
- `data/samples/packages/ring-tools.json` is a package by another author: its digest binds its source, the consent
  sheet lists what it asks for, and in a solving session its preview is refused before anything is prepared.
- A key that commits is never given to a run. Restart scripts and safe mode are in the Scripts dock; F4 opens the
  command bar; F9 steps through the ten screens of this stage.
- `a5-sweep` also fails when a script frame holds a commit control.

### A5 stage 8: colour, type, icons, motion and the component gallery

Look Lab opens on the Solve workspace. F9 steps through the screens of this stage; F6 (and the palette row
"Theme: next of the six presets") steps through Graphite and Paper with their three scene looks; key 5 opens the
Gallery workspace.

- The two families are two characters (`character` in `data/design/themes/*.json`): Graphite builds depth with
  hairlines and square corners, Paper with tone steps and soft corners. The Appearance dock can draw a family the
  other way, to compare. Each family gives committed, preview, script and selection an accent of their own.
- A scene look sets the ground, the fog, the glow and the edges through the lab's own parameters, inside their ranges.
- The gallery (`app/Chrome/Ui/Gallery.cs`, model in `app/Chrome/Model/Gallery.cs`) has seven pages: every widget in
  every state, type with the notation glyphs each face chain misses, both icon tiers at 16 px with the command
  coverage, the motion table (`data/design/motion.json`) with a play button per entry, the candidate sticker palettes
  (`data/design/palettes.json`) with their smallest OKLab distances under four kinds of colour vision, a list of
  10,000 rows that builds only the rows in view, and the five designed states of a pane.
- Only a palette that passes is offered; choosing one writes the lab's palette parameters, so both views show it.
- `data/design/command-icons.json` gives every registered command an icon or marks it label only on purpose.
- The top bar has readout cells; each view has pick-layer keys (6 to 0, F10 cycles how the other layers show).
  The layers change marks only: the core does not draw per-layer opacity yet.
- Reduced motion (Appearance dock, the gallery's Motion page, `motion.reduced.set`) shows every motion's still
  picture at once. Cue slots carry no audio.
- The core tests include a token lint: no colour is written in `app/Chrome/Ui` or in the lab's wiring of it.

### A5 stage 9: the same frame in the product shell

The chrome library has no second copy: `renderer/shell/build_shell.py` stages `app/Chrome` and `data/design` into the
shell's build. Three members exist for a host whose picture lies beneath a transparent window: `ChromeFrame.OpenGround`
(the ground is painted everywhere but the Global view), `ChromeFrame.Opening` (that rectangle) and
`ChromeFrame.Claims` (whether the chrome takes a key). Look Lab sets none of them. See `renderer/shell/README.md`,
"The frame (A5 stage 9)".

### A5 stage 10, point 1: the relationship strip

The strip sits on the shared edge of the locked pair (`ChromeFrame.SetSeam`), never in a dock: Target with its
Occupant, the Wanted piece with where it sits now, Buffer A and Buffer B with their Occupants, the Target frame
(Home or Captured) and the working orbit's Residual, which expands in place into its exact classes. Next is a quiet
bookmark at the strip's end. Each item has an anchor in Local and a marker in Global; a hovered item is linked to
them. An ill-formed tuple is shown as invalid and is never corrected.

- Orbit ids, piece counts, orientation groups, the stage order and the seed dependencies are read from
  `assets/seed_atlas.json`. Everything else is demo data and is marked so on the strip: three demo states (mid-solve,
  the endgame at the last buffer, a fresh scramble), switched with F11 or the chip at the strip's head. The places
  of the anchors on the pictures are demo places too: the lab has no engine that knows where a position is drawn.
- Keys: E steps the Target through the unfinished positions (position-id order), A and B move a buffer, H changes
  the Target frame, O goes to the following orbit of the rail, Shift+N sets Next, Alt+N clears it, N activates it.
- The commands keep the ids of the command table; the lab's display words for them are in
  `data/design/command-labels.json`.
- Token slots: `strip.ground`, `strip.cell.*`, `strip.link`, `strip.next`, `anchor.*`; motion row `strip.change`.
- Not shown: the stale state of an item (the demo has no commit that makes one stale).

### A5 stage 10, point 2: the orbit rail

The rail lies along the head of the locked pair (`ChromeFrame.SetPairHead`), never in a dock: 35 cells, each with
the orbit id, the orientation-group glyph (one kind per group type), the residual level drawn as a length, and
three separate marks in three fixed places (solved, lock, Next); the working orbit has a border and a bar.

- The order never depends on the state: the seed stage order, labelled "one admissible order", or orbit-id order
  with the button at the rail's end. No cell is emphasised for being unfinished and nothing moves at idle.
- A click selects the working orbit; a click on the working cell opens it in place into the exact residual classes
  with exact numerals ("unknown" is hatched on the cell and spelled out, never zero) and the unfinished positions in
  position-id order. Hold O and type two digits to select an orbit; the rail shows what is typed.
- On hover a cell marks the orbits its seed touches (a line above) and the orbits whose seeds touch it (a line
  below), from the seed atlas; they are never lit from the state.
- Token slots: `rail.ground`, `rail.cell.*`, `rail.lock.*`, `rail.status.*`, `rail.moment.complete`; motion rows
  `orbit.complete`, `lock.set`, `lock.release`.
- Not shown: setting a lock on a cell, and the completion moment (no demo action completes an orbit). The list of
  unfinished positions shows the first 120 and counts the rest.
- The tour key now also works while a radial menu is open.

### A5 stage 10, point 3: the draft line

- A row over the commit bar, under the pair (ADR-D01.66): the route switch (Draft or Direct) with the route mark, the
  three phases Prepare, Core and Cleanup with the active one marked, the Goal slot with its own result, the check-mode
  chip (Net or Strict) and the settling lamp. The line draws no commit control: Commit stays on the host's bar.
- In Draft a twist joins the active phase and the draft is staged as the preview, so the commit bar commits it and a
  commit or a discard empties the draft. In Direct a twist is one complete operation and is committed at once.
- The Goal never gates a commit: an unmet Goal leaves Commit enabled. The switch between Draft and Direct lets go of
  every held key (the orbit jump among them) and changes the grip marker in Local: broken for Draft, whole for Direct.
- Keys: `Q` Draft or Direct, `P` next phase, `J` append a sample twist, `U` take the last twist of the active phase
  away, `Y` declare the sample Goal or take it away. The five commands `input.destination.set`, `intent.set`,
  `operation.phase.select`, `operation.phase.edit` and `operation.phase.append_twist` also run from the palette, so
  all twelve commands of the solving surface work on the demo state.
- Demo data: the twists, the Goal result (met or unmet by a fixed rule) and the Net and Strict results (a Prepare
  without a Cleanup passes Net and fails Strict). No engine evaluates them.
- Token slots: `mark.route.*`, `draft.phase.*`, `commit.goal.*`, `chip.check_mode.*`, `commit.status.settling`;
  motion row `route.switch`.
- Not shown: inline parameters, editing a twist inside a phase by hand (the command takes a typed argument), and the
  motion of the route switch.

### A5 stage 10, point 4: the key map

- The keymap in force drawn as a keyboard (ADR-D01.74), over the frame and never modal: keys keep working under it.
  Hold the help key (`Shift+/`) and it shows; let go and it leaves. A short press pins it, and so do its Pin button
  and the keyboard button of the top bar; a short press again or `Esc` lets it go. Its List button opens the earlier
  list of every binding, which the palette command still opens too.
- Each cap shows what the key does where the pointer is, with the modifiers that are held. The header names the
  active key set (the preset, and "with your bindings" when you have bound keys), the place and the modifiers; a
  change of key set is also said in the notice line. So a key never changes meaning without a name for the change.
- Five cap states, each with its own drawing and token slot `keymap.key.*`: a whole outline for a key bound here, a
  double outline for a twist key, a broken outline with the place for a key bound somewhere else, a faint outline for
  an unbound key and a filled cap for a held key. Look Lab's own keys (tour, demo state, orbit jump) show as bound.
- Tour stop 18 shows it pinned with the pointer over Local.
- Not shown: the ordered marks that keycaps share with the Grip frame (no Grip frame is built), key sets per orbit
  (the presets are the two keymap files), and numpad or navigation keys.

### A5 stage 10, point 5: inline parameters on the commit bar

- What was left of the commit bar after the draft line: the scrub and the commit of a prefix were already there, so
  this point adds inline parameters (ADR-D01.11). The numbers of the staged statement show as small number fields
  on the bar, each named by the word before it (cell, axis, ring) or as turns for a signed count. Text in quotes is
  never a parameter. A change rebuilds the preview under a new id, so the settle interval applies again.
- The bar only asks: the host rebuilds. For a staged draft the changed number goes into the twist of the draft it
  belongs to, and the draft line shows it. Up to four fields show; more are counted.
- Demo data: the ranges (cell 0 to 599, axis 0 to 5, ring 0 to 11, turns -2 to 2), and after a change the moves and
  the effects stay those of the sample, which the preview says.
- Tour stop 18 shows a typed twist with its three fields.
- Not shown: parameters that are not whole numbers, and a parameter of a macro's own definition.

### A5 stage 10, point 14: two automatic screen checks

- Keyboard reach, in the tests that `python tools/looklab/check.py` runs: every key set (the default and each
  preset) opens the palette, closes an overlay and shows the key map; the palette lists every command; every action
  that an element raises by name is a command, or is listed under `pointer_only` in
  `data/design/screen-words.json` (empty since the A6 re-check: History's roll to a layout step is the draft row
  `layout.roll`); on the Library, the History dock and the draft line no button, chip or row acts by itself, each
  sends a registry command; eleven of the twelve solving commands have a key and
  the twelfth is the held orbit jump.
- Words per screen, in the sweep of the Godot lane (`check.py --godot`): for each of the five workspaces the sweep
  counts the words of the labels, buttons and empty fields shown in the window and fails over the limit in the same
  file. Counts on 22 November 2026: solve 129, script 99, analyse 160, reference 96, gallery 141. Counts on 23
  November 2026, with the Library dock and History in the right dock: solve 140, script 151, analyse 151,
  reference 184, gallery 195 (limits 155, 165, 175, 200, 215). A screen is counted after the sweep's first press in
  it, so the count includes what earlier presses left open (strip problems, History rows).
- Time: the sweep presses about 760 buttons and takes about 230 seconds; it has its own 400-second limit in
  `lane_a5.py`. On main at 7a8c4cca it already ran past the general 120-second limit.
- Limits of the checks: the word limits are those counts with a margin, kept so that a screen does not grow
  unnoticed; they are not the owner's text budgets for trial content. Text an instrument draws itself (rail ids,
  keycaps) and tooltips are not counted. The reach check reads source text: a control whose handler acts without
  raising a command by name is not seen by it, and focus order by Tab is not checked.

### A5 stage 10, point 7: History in full

- Already there before this point: abandoned redo steps kept as a branch, the partial undo (one move back inside the
  last applied entry, recorded as its own action) and "roll to here".
- New (ADR-D01.16): each kept branch has a row with its starting entry, its entries and moves, and two controls.
  Switch makes the branch the line: it asks first (Confirm or Cancel in the row), the later entries of the current
  line become a branch in its place, and nothing is rewritten or lost. Switch is off while an entry is partly undone
  or when the branch's starting entry is no longer on the line, and its tooltip says why. Replay brings the branch's
  moves back as a preview that the commit bar commits.
- The command `session.branch.select` runs from the palette too: there it only opens the confirm gate in History.
- Tour stop 18 shows a partial undo inside an entry and a kept branch.
- Demo data: the entries and moves are the sample session's; the puzzle state does not follow the history.
- Not shown: names for branches, a tree drawing of more than one level, and restoring a named version. Replay is the
  command `session.branch.replay` (a draft row of the record, see the A6 section below).

### A5 stage 10, points 9 to 11: the palette's row menu, readouts with lamps, the five panel states

Done together at the owner's request.

- Palette (ADR-D01.18). Pins and typed mode were there. New: a row menu, opened by a right click or by the Menu key
  or `Shift+F10` on the highlighted row, with Pin (`P`), Alias (`A`) and Bind a key (`B`). An alias is a second
  name of your own: a query finds the row by it, the row shows it, and it never changes the order of the rows. It
  is typed after `=` and kept with the pins. Bind uses the same "press the key" step as the Scripts dock and binds
  the command without its argument. Typed mode now also takes an argument of your own, as in `> orbit.switch O05`.
- Top bar readouts. Each of the three cells (revision, history position, run queue) has a lamp and opens on a click
  to a second line of words. The lamp is told apart by shape: a ring at rest, filled when the history is behind its
  newest entry or runs wait, filled inside a ring when an entry is partly undone. Token slots `lamp.idle`,
  `lamp.active`, `lamp.attention`. No Timer and no Session cell.
- Panel states (ADR-D01.33). Every dock panel sits in a box that can show one of five states besides ready, each
  with a word, a glyph of its own shape and the reason: Empty, Loading and Error take the content's place; Stale
  and Busy keep the content under a banner (stale content is dimmed). Error offers Retry and Stale offers Refresh.
  Nothing spins. `F12` steps every dock panel through the five; they are demo states and their text says so.
- Tour stops 19 (readouts), 20 (panel states) and 21 (the palette's row menu).
- Not shown: opening a readout from the keyboard (a click only), panel states on panes outside the docks, a state
  that a real panel sets by itself (only the demo key sets them), and a binding that keeps a row's argument.

### Stage 10 point 8: the inspector's two tiers and the shared field gestures

Both tiers read one field model (ADR-D01.22, ADR-D01.23). The strip above the views shows the selection and the
Essential preview parameters, and a count (`+n`) of the fields the Inspector dock holds beyond it; pressing the count
shows or hides the dock. In the dock, the head of a section counts the numbers that are away from their defaults and
resets them together. Neither tier has a commit control, and a test reads the source to keep it so.

Every numeric field has the same gestures in both tiers:

- drag the value; Shift is ten times coarser and Alt one step a notch for that drag. No gesture goes finer than the
  step: every value a field takes lies on its grid and its display has the decimals that grid needs, so a change of
  value always changes what the field shows (Q2 of the A6 review);
- the value ladder: three rungs of 100, 10 and 1 of the field's steps for each notch, never finer than the step. The
  small ladder beside the value shows the rung, and both tiers show the same one. Changing the rung is not an edit;
- the keyboard, with the field focused: Left and Right step by the rung, Up and Down change the rung, Enter types a
  value, `=` starts an expression that stays a live binding, Delete resets;
- the mark beside the label names where the value comes from, and clicking the label resets the field.

Not built: the View section's scope switch (the lab's View fields always act on both views), and the ladder as a
pop-up under the pointer.

### Stage 10 point 6: the preview view's before and after, and the range a preview touches

The stage of the preview view compares the committed state with the state after the whole sequence (ADR-D01.10):
one row of cells for each orbit, the share that will move outlined before and filled after. The button in the head
steps through both sides, before only and after only (`preview.side.next`; each side is also its own command,
`preview.side.both`, `preview.side.before` and `preview.side.after`). The line under the panes is the scrub, one notch
for each move.
Above the per-orbit bars one line names the range: how many orbits and how many pieces the sequence moves. The
comparison itself is a picture with no control, and the commit stays on the host's commit bar.

The orbits, their sizes and the moved counts are sample numbers (the orbits are named "sample orbit"). The comparison
draws the two ends only; the state between them, the collateral pieces one by one and the different animation of a
preview twist are the core's to draw and are not built here.

### A6 review: build fixes

The A6 review of the A5 build found build defects against rules the record already states, so the record does not
change. Each fix has a headless test in `tests/A6FixTests.cs`; A5 tests whose sample was the defect were changed.

- Commit safety. A draft whose protection check fails is staged as an impossible preview and never commits (B-1). A
  typed line never writes the move history; in Direct a typed twist is staged, and only a twist key commits (B-2). A
  prefix taken at the scrub is prepared and checked again as its own preview before it can commit (B-3). A ready
  preview remembers the head it was built on and goes stale when anything moves the head, a Direct twist included
  (B-13). An edited statement is checked again and never keeps its old witness (B-12). A branch switch commits only
  the answer to the open question, for its branch and at its head (B-5).
- Effects and drafts. Only evaluated effects may say an orbit is untouched; a preview without them says "Not
  evaluated" (B-4). Each orbit keeps its own draft; activating Next in another orbit saves the present one (B-7).
- Gallery, scripts, anchors. The gallery's pane states are the docks' five, Busy among them (B-8). A script's frame
  follows the recorded author class: a built-in tool has the mark alone, the solver's own script a thin border, other
  authors a patterned border (B-9). The strip's anchors sit where each view's own projection puts their position, so
  they follow the camera (B-10, `core/ScenePlaces.cs`; a position of an orbit stands for its p-th piece, sample data).
- Library (B-6). Solve opens the Library on the left and History on the right. The Library holds macros, Templates,
  sets, blocks and saved states in one tree with a Built-in branch (sample records). Its filter box filters in place;
  Up and Down select; Enter adds the selected macro to the active phase (Core by default) as one change
  (`macro.add`); Shift+Enter previews it alone (`operation.preview` with `macro:<name>`). Rows lead with the kind's
  glyph and the effect signature on the sample orbits, which reads as unverified. Analyse opens the Library on the
  left too; History is in the right dock of every layout. The live check drives the filter with real key events and
  prints `LOOKLAB_A6_LIBRARY filter=swap core=3 changes=1 history=right`.
- Commands (Q5). Replaying a kept branch (`session.branch.replay`) and the preview's sides (`preview.side.both`,
  `preview.side.before`, `preview.side.after`, `preview.side.next`) are rows of the registry, added from
  `data/design/command-rows-draft.json` until the command table takes them; the buttons send these commands.
- Numeric gestures (Q2). No gesture goes finer than a field's step (Alt drags one step a notch); every value lies
  on the field's grid and shows with the grid's decimals.

Deferred: B-11 (help at the cursor as a geometry-backed term card linked to the Learn layout). The lab has no Learn
layout and no term cards, so the key's label alone would point at a missing surface; it needs its own step.

### A6 scoped re-check: build fixes

The scoped re-check of the fixes above left build findings open; each fix has a headless test, in
`tests/A6RecheckTests.cs` or as an extension of the A6 or A5 test it names.

- Checks. A staged draft carries its own prefix check: a prefix is cut from the phases in order and checked under the
  mode the draft was staged with, so a Strict draft's prefix stays Strict (B-3). A word whose effects move the locked
  sample orbit fails the draft's check in both modes, as the same twist staged alone (N-2). A twist of a cell outside
  0 to 599 or an axis outside 0 to 5 is no witness and gives its reason; a Direct twist is checked before it commits
  (N-3). The sample previews are checked by the sample rule when they are staged (`PreviewSamples`), so one witness
  shows one collateral however it was staged (N-6).
- Anchors. In Local the anchors follow the view's spread of the cells (`focus_spread`) and its cap (`focus_cos`), the
  shader's own formula (B-10, `LocalFocus` in `core/ScenePlaces.cs`): a position on a cell the view does not draw has
  no anchor.
- History. A Replay with no branch number (a key bound through the palette) acts on the branch last named in History
  and refuses when there is none; it never prepares branch 0 by default (Q5). Opening or cancelling the switch
  question records nothing (`MoveHistory.Asked`), so state-reading tasks keep running and script results stay fresh
  (N-4). A change of demo state (F11) carries the drafts by state and orbit (N-10).
- Library. Shift+Enter with nothing selected says so and leaves the pending preview (N-5; the live check presses it on
  an empty filter). The effect signature has 35 slots in the rail's order and follows the rail's order setting: a full
  bar where the word moves the orbit, a base-line tick where it was measured not to, a short upright hair where
  nothing is known. The five sample orbits stand on O34, O06, O00, O17 and O02 (sample mapping, demo data); every
  other slot is unknown (N-8). The Template binds its roles Target, Buffer A and Buffer B to the strip of the
  working orbit (a position names a sample cell, sample mapping); Enter adds the bound word and Shift+Enter previews
  it, and another Target gives another word and a fresh preview (N-9).
- Commands (N-7). The Library's branch heads, History's Moves and Layout tabs, its entries, its Switch and Cancel and
  its layout steps, and the check-mode chip send registry commands. The chip uses the command table's
  `protection.policy.set` (net or strict). Four rows are Look Lab's own proposal, in
  `data/design/command-rows-draft.json` after the record's five: `library.branch.toggle`, `layout.roll`,
  `history.show` and `history.entry.toggle`. `history.show` without an argument shows the moves, History's default
  view.

All build findings of the re-check are fixed on this branch, each with a test. Their full acceptance moves to the
stage 2 greybox check, where the five flows run on the backend. The record findings J-6, R-3, J-7 and N-1 belong to
the design-system ADR's record; the lab makes no build change for them.
