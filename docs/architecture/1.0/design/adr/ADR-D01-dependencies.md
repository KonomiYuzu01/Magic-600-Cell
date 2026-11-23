# ADR-D01 dependencies and licences (22 November 2026)

**Approved by the owner on 22 November 2026** ("I approve all the rest", with the ADR-D01 draft), as the one approval of new
product dependencies that `AGENTS.md` "Ask the owner" requires. The approval covers the entries as listed; an entry
whose shipped licence differs from this list, or whose terms are still marked "to verify" and turn out different,
is blocked until the owner sees the difference. Licences are as recorded
in the A3 records or as commonly published; each one is checked against the file actually shipped when it is
installed, and a mismatch blocks it. Already approved and not repeated here: Godot 4.7.2 .NET (MIT) and a supported
.NET release (ADR-R01.5).

## New product dependencies

| # | What | Role in ADR-D01 | Licence | Notes |
| --- | --- | --- | --- | --- |
| 1 | IBM Plex Sans | ui type role (D01.47) | OFL-1.1 | Reserved Font Name "Plex": a modified build may not ship under that name. Ship the OFL text |
| 2 | IBM Plex Serif | content type role (D01.47) | OFL-1.1 | Same as 1 |
| 3 | Lilex | code type role (D01.47) | OFL-1.1 | Derived from IBM Plex Mono; ship the OFL text |
| 4 | Maths fallback face, only if the glyph check fails | D01.48, charter D6 | OFL-1.1 | Candidates: IBM Plex Math (availability and coverage to verify), STIX Two Math, Noto Sans Math |
| 5 | Placeholder icon library | tier-1 icons until the design and art session draws the set (D01.53) | Lucide: ISC | Alternatives: Phosphor (MIT), Tabler Icons (MIT). Placeholder only; charter D3 wants a custom set |
| 6 | Embedded code-editor component, only if Godot CodeEdit falls short | D01.34 | CodeMirror 6: MIT | Needs 7 |
| 7 | Embedded web frame, only if Godot Controls are not enough | D01.34 | Microsoft Edge WebView2 runtime and SDK: Microsoft terms, to verify; a Godot binding (for example one built on wry, MIT OR Apache-2.0) to verify | Windows-only, so it lives in the platform layer; redistribution terms to check before approval |
| 8 | Fuzzy matcher for the palette | D01.18 | nucleo-matcher: flagged by A3 as possibly MPL-2.0, to verify | Alternative: a small matcher written in the project, or an MIT crate. MPL-2.0 is file-level copyleft: usable as a dependency, but files changed in it stay MPL-2.0 |

Items 4, 6 and 7 are conditional: they are approved now so that the check that triggers them needs no second
question, but they are installed only if that check fails.

## Code and patterns ported (no new dependency; notices kept)

Copied or closely adapted code keeps its copyright and licence notice, and Apache-2.0 sources keep their NOTICE file.
All of these go into the product's third-party notices.

| Source | Licence | What |
| --- | --- | --- |
| TiXL | MIT | Edit lifecycle (prepare, update, commit), layout handling with F-keys and focus mode, parameter relevancy levels |
| Rerun | MIT OR Apache-2.0 | Layout blueprint data model, shared view context, progress ring, type-to-editor registry, list-item density, UI writing rules |
| Hyperspeedcube | MIT OR Apache-2.0 | Queue-aware twist speed, log validation and replay, per-view status line, piece-filter expressions (credit HactarCE), keybind notes |
| Godot | MIT | Theme and Control patterns, editor-scale idea, colour remap pattern, preview-then-one-undo pattern |
| Robert Penner easing (via Godot) | MIT | Easing equations for view and 4D-rotation transitions |
| Material Maker | MIT | Float field interaction (drag, fine modifier, click to type), typed parameter declarations |
| marimo | Apache-2.0 | Density and spacing rules, reducer with tested inverses (reference) |

## Reference only (no code copied)

| Source | Licence | Used for |
| --- | --- | --- |
| Zed | GPL-3.0-or-later | Panel and dock contract, action log, activity indicator, modifier-held subtle preview: reimplemented from the idea |
| Blender | GPL-2.0-or-later | Typed undo steps, property-driven widgets, area model: ideas only |
| Penpot | MPL-2.0 | History row layout and shortcut table shape: shape only, no files copied |
| Closed tools (FL Studio, Bitwig, Houdini, Dorico, Onshape and others) | proprietary | Analysis of public material only; nothing copied |
