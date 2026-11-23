# ADR-D01: the design system

Status: **Accepted** (owner, 23 November 2026, at A7: UI/UX freeze round 1; [freeze record](../../../../wiki/decisions/owner-decisions-2026-11-23-a7-freeze.md)). Its
framework items and slot names are frozen; defaults, tuned values and open items are not (freeze rounds 2 to 4).
This file is revision 3a. Revisions 4, 5 and 6 beside it amend it, and a later revision wins where they differ
([README](README.md)). The owner approved the content and the dependency list on 22 November 2026 and decided that
this record follows the owner's picks where they conflict with frozen clauses, without a versioned-change review
([overrules](../../../../wiki/decisions/owner-decisions-2026-11-22-adr-d01-overrules.md)). A5 built it in Look Lab, A6 reviewed it, and the owner froze it at A7.

Revision 3a (22 November 2026, owner's approval of the design/art path): the two-cue sound cap in ADR-D01.27 and
ADR-D01.56 is lifted (owner: "do not cut"); the full sound table comes in the third freeze round. A5's chrome is built as a
library that both Look Lab and the product shell load. The Jumble and research surfaces go into a separate Jumbling
ADR (ADR-D02) after A7; K2 opens after A7 and ADR-D02. UI/UX freezes in four rounds: A7 is round 1 (slot names, not
values).

Rests on: the design charter, section 5 of `docs/progress/1.0/charter-2.0-draft.md` (signed 3 October 2026, amended
6 November 2026); the owner's design decisions of 2 October 2026
(`docs/wiki/decisions/owner-decisions-2026-10-02-design.md`); stage 2.5 of
`docs/progress/1.0/stage-2-experiment-protocol.md`; the frozen language architecture (ADR-L01 to ADR-L12) except
where "Owner overrules" below replaces a clause; the owner's answer to LQ12 (6 November 2026); ADR-R01 (renderer);
the command table `docs/progress/1.0/command-table.json`. Inputs of this line: the A1 collection of 48 professional
tools, the A2 reverse-engineering records, the A3 pairing records of 14 topics, and the owner's A4 picks and chat
answers of 21 and 22 November 2026.

How to read the tags. Every decision item ends with one tag:

- **(framework)** A host rule. Scripts, settings and themes cannot change it. Proposed for freezing at A7; after
  that it changes only through a new record that supersedes this one.
- **(default)** The shipped default, held as data in the default profile (ADR-D01.1). The solver may change it in
  settings or by keys, and a .6cs program may change it through the routes of ADR-D01.2.
- **(tuned)** A value set in Look Lab with the owner watching (A5, G4, G5) and stored as token data. This record
  fixes the slot, not the number.
- **(open)** Left to a later owner step (listed under "What this record does not decide").

The owner's general rule governs the whole record: "All of my answers are preferences only, not frozen commands for
the final product; do not freeze any component of ADR without my permission." And: compiled scripts should be able
to customise the design system to the greatest extent that stays consistent with the language framework, mainly
through customisable key bindings that can change the design system or the UI as a whole.

## Context

1.0 needs one design system for a dense expert tool: two linked views of 259,800 sticker slots, a language at the
centre of the work (600 Cell Script, `.6cs`), long keyboard-driven sessions, and a learning path that feels like a
game while the screen does not. The charter asks for two authored theme families with three scene looks each, one
encoding grammar, compact and graphic chrome, and user tuning. The renderer is selected (ADR-R01: the D3D12 core
hosted by Godot 4.7.2 .NET), and its gate must hold in the shipped look (charter E1).

The frozen language architecture keeps the core safety rules this record builds on: a program never commits (LG3.4),
every commit rechecks the preview revision, the full-state hash and the protected orbits, recordings never commit
(EV4.17), and view and workspace changes use command rows (DET.35). A check on 22 November 2026 found that several
owner picks conflict with other frozen clauses: live previews, a separate layout history, multi-step rollback, one-key
preview-and-commit, the solver's own program previews in a solve, the canvas primitive and install-time package
consent. The owner decided that this record follows the picks. "Owner overrules" lists each replaced clause and the
rule that takes its place, so the integrator can record them.

The owner wants the scripts to reach almost everything: layout, views, keys, themes, panels and the editor. The host
ships one good default set and exposes it as data and building blocks that scripts rearrange, without ever giving a
script a way to commit, to hide what state is shown, or to break the shared reading of the screen.

Nexus cards and annotated reference pairs are the development-stage method for collecting the owner's interests
(owner decision 4A, 2 October 2026; correction of 21 November 2026). They are not product content, and this record
designs no surface for them.

## Decision

### 0. Framework and defaults

- **ADR-D01.1** The design system has two layers. The framework is the set of host rules in this record tagged
  (framework). The default set is one design profile held as data: layout, workspaces, view bindings, keymap preset,
  density, type ramp, theme and motion preset. 1.0 builds one default set, mostly the A3 recommendations as the owner
  amended them. The other options the owner picked are reached through Rust building blocks that scripts compose, not
  built as parallel built-in modes. (framework)
- **ADR-D01.2** Scripts reach the design system by three routes:
  1. An entry point the solver invokes (by key, chord, palette row or radial menu) may run view and workspace command
     rows and prepare a preview under ADR-D01.8.
  2. A view publishes view-model nodes and display plans (panel content, overlays, free-view content) and may drive
     a live preview (ADR-D01.43). It never writes the workspace; a live `=` binding is a derived value (ADR-D01.23).
  3. Data files the solver loads by a command: themes, token overrides, keymaps, layouts and workspaces.
  Every change a script makes to the default set is a view or workspace step in the layout history (ADR-D01.14). A
  recording replays recorded view and workspace steps unattended and stops for one user action at the rows EV4.18
  names; it never commits. (framework)
- **ADR-D01.3** Key bindings are the main way to customise. Any key, chord or held key may be bound to a command row
  or to a registered .6cs entry point, including one that rearranges the layout, swaps the keymap preset, changes the
  theme or opens a script panel. Keymap presets are switchable and exportable through the `keymap.*` and
  `file.keymap.*` rows. (framework; presets default)
- **ADR-D01.4** What no script, setting or theme can change:
  1. A commit happens only by the solver's own action: a non-repeating key-down or an explicit pointer action on a
     host-drawn commit control or a commit key, or the one-key preview-and-commit of ADR-D01.12 on the solver's own
     macros and twists. Commit is never scriptable, and no recording plays one (LG3.4).
  2. Every commit rechecks the preview revision, the full-state hash and the protected orbits, and stops at the
     preview on any failure.
  3. The main Global and Local views show the committed state. A preview never replaces it; at most a marked pending
     marker or pending overlay is drawn over it (ADR-D01.10).
  4. Every script-drawn area carries a visible "script" frame, and commit controls are drawn only by the host.
  5. High contrast and a keyboard-only path to every command stay available; a script cannot switch them off.
  6. No built-in ranking, suggestion, radial content or quick fix is computed from the puzzle state to propose a
     next twist or macro (human-solve boundary, owner, 2 October and 3 November 2026).
  7. The meaning of each state and encoding role (preview, committed, stale, invalid, selected, the relationship-strip
     classes) is the same in every theme family and scene look. Its colour may be overridden (ADR-D01.51); its
     meaning and its shape, label or position cue may not.
  8. Protection conflicts and complete-operation effects always break through focus, dimming and hiding (owner,
     2 October 2026), and host statuses are composited after user stages (RPI.76).
  (framework)
- **ADR-D01.5** Product code refers to tokens by name only, checked by lint (charter D1). Tokens cover colour, type
  scale, spacing grid, control height, radius, elevation, motion durations and curves, and icon sizes. One resolved
  token set feeds the Godot theme and the D3D12 scene constants. (framework)
- **ADR-D01.6** Every view and panel has designed empty, loading, error, stale ("applied, display stale") and busy
  states (charter D2). No modal error dialogs; long work shows progress in place and can be cancelled. (framework)
- **ADR-D01.7** The charter's anti-goals 1 to 13 bind the default set and every host building block. Layouts allow
  other languages and longer strings: no text slot has a fixed width, and a label that does not fit is shortened with
  its full text on hover and in the status line. Subtitles stay undecided (owner, 2 October 2026). (framework)

### 1. Preview and commit

- **ADR-D01.8** Every move-producing action enters one preview state; a session holds one pending preview (KN5.11).
  Sources: the solver's typed twists and own macros, a .6cs preview program, a library macro, and the built-in tools.
  Built-in tools only transform the solver's own input (invert, mirror, conjugate, repeat or re-scope a solver macro,
  or a single twist the solver specified) and never offer a sequence they chose (2 October boundary). In a solving
  session a program's word is accepted when the program is the solver's own, written in the solver's workspace (owner,
  3 November 2026); a word from a package written by someone else is refused at preview there with its reason shown.
  Outside a solving session every source is accepted. (framework)
- **ADR-D01.9** A new preview may replace a shown one: by a command the solver invokes, or by a live preview
  (ADR-D01.11, ADR-D01.43). The replacement takes a new preview ID only when it is built (KN5.11), the preview view
  marks the change, and the replaced preview can no longer be committed. After a replacement the commit key is
  ignored for a short settle interval, so a key-down never commits a preview the solver has not seen. A recording
  never replaces a preview. (framework; settle interval tuned)
- **ADR-D01.10** A preview is drawn in its own preview view, a free view outside the main pair. The main Global and
  Local views keep the committed state and show only a small pending marker while a preview waits. The preview view
  shows the full collateral effect, the witness sequence, the effect on every orbit and on protection (signature
  moment S4), with a before-to-after scrub and move-by-move stepping. Preview twists animate differently from
  committed twists. A script may add a marked pending overlay over the committed state in both main views (the older
  arrangement). (default)
- **ADR-D01.11** One slim commit bar summarises the preview: the staged .6cs statement, source, move count, status
  (ready, impossible, stale, refused), the scrub, and inline parameters that re-evaluate the preview live as they
  change. The commit key (Ctrl+Enter by default outside the editor, ADR-D01.58) writes one labelled entry whose
  provenance is the statement; Esc discards; Promote saves it as a macro. Commit is disabled with a stated reason when
  the preview is impossible, stale or refused. The move strip opens on demand from the bar. (framework for commit and
  recheck; bar layout default)
- **ADR-D01.12** The solver may commit a prefix of the preview, up to the scrub point, in one action; the prefix is
  prepared and rechecked as its own preview before it commits. A one-key preview-and-commit binding is allowed only
  for the solver's own macros and twists: one key-down prepares, rechecks and commits, and any failed recheck leaves
  the preview on screen uncommitted. Recordings and scripts cannot trigger it. (framework)
- **ADR-D01.13** Direct twists use grips (`twist.apply`). Hover ghosting of what a click would do appears only while
  a modifier is held. A click, or the release of a drag on a grip, commits one direct twist; release-to-commit never
  applies to macros or program output. Nothing commits as a side effect of focus loss, deselection, another command,
  a script re-run or a recording. (framework for commit rules; modifier ghosting default)

### 2. History

- **ADR-D01.14** Two histories with their own keys, both in the Rust session store and surviving restarts:
  1. The move history: committed transactions and the puzzle-related workspace actions (macros, operation drafts,
     records). One hand commit, macro commit, or commit the solver made from a script's preview is one named entry
     tagged by origin. Ctrl+Z and Ctrl+Y by default.
  2. The layout history: camera, view filter, layout, workspace arrangement and design-profile changes (theme,
     density, type, motion, keymap, panel mounts), whether made by hand or by a script. Its own keys (default
     Ctrl+Alt+Z and Ctrl+Alt+Y).
  Neither history holds the other's steps, and an undo in one never moves the other. Previews and read-only runs are
  never recorded. (framework; keys default)
- **ADR-D01.15** The History dock shows the move history as .6cs: one header row per transaction (origin, name, short
  id, time) that expands to its move lines, collapsed to headers by default. A rollback cursor moves by the undo and
  redo keys, by dragging, or by "Roll to here", each as one solver action that may take several steps. A committed
  entry expands into its moves, and the solver can step back inside it move by move; a partial undo is recorded as its
  own action naming the entry and the move, so nothing is rewritten. Selected lines can be copied as .6cs, saved as a
  macro, or replayed as a preview. A script may switch the dock to the plain list of named transactions. (default;
  append-only recording framework)
- **ADR-D01.16** Abandoned redo steps are kept as a recoverable branch. Inserting new moves mid-history records a new
  branch from that point; the later moves of the old line come back as a preview the solver commits. Nothing is
  rewritten in place. Switching branches is `session.branch.select` with its confirm gate. Restoring a named version
  adds a new entry. (framework)

### 3. Commands and keys

- **ADR-D01.17** One command registry in the Rust command layer, fed by the command table and by registered .6cs
  entry points, which become overlay rows with KN5.3's row, kind and contexts (LG3.3). Each command has a stable
  namespaced id, label, synonyms, kind (query, view, workspace, commit, file, app), argument schema and optional
  default binding. Menus, context menus, radial menus, the keymap, the shortcut sheet and the palette are generated
  from it. (framework)
- **ADR-D01.18** The palette is an in-window overlay. It ranks only by the solver's own recency, frequency and pins;
  each row shows label, binding, location and an unavailable tag; the highlighted row shows short help (about 20
  words) and the full reason it is unavailable. Right-click binds, pins or aliases a row. Its typed mode accepts
  command calls and full 600 Cell Script, with history; a run ends in a read result or a preview only. Opened over a
  selection, it filters to commands valid for it, with a one-key "show all". (default; ranking rule framework)
- **ADR-D01.19** Commands use single keys by default, scoped by context (global, Global view, Local view, script
  editor, preview active) so that twist keys and command keys do not collide. Keys route to the hovered view first and
  fall back to the focused view, with a visible focus indicator. Held keys act as quasi-modes. Keys open by soft
  disclosure, and the palette always offers every command (owner, 2 October 2026). (default)
- **ADR-D01.20** Hotkey-held radial menus serve frequent families (twist directions on a grip, view and filter
  toggles). Each radial menu is itself a command, user-editable, and may hold .6cs entries; an entry behaves exactly
  like its palette row. (default)
- **ADR-D01.21** A typed command line is the palette's typed mode, not a permanent prompt. User overrides are stored
  apart from the defaults and exported as a versioned keymap file (`file.keymap.export`). The default keymap is drafted
  after reading Hyperspeedcube's keybind notes. (default)

### 4. Inspector

- **ADR-D01.22** Two tiers bound to one selection and one field model. Tier 1 is a thin strip at the edge of the main
  views with the selection count and kind and the three to six Essential preview parameters. Tier 2 is one docked
  inspector, collapsed by default, with fixed sections: Selection facts (read-only puzzle state), View (scope: both
  views by default), and Preview and script parameters generated statically from typed .6cs declarations. Neither
  tier has a commit control. (default; read-only state and no commit framework)
- **ADR-D01.23** Every numeric field shares one gesture set: drag, a value ladder with a keyboard alternative, a typed
  value, and `=` plus a .6cs expression. An `=` expression stays a live binding stored in the session store; it is
  evaluated in the runtime process under the read capability, or the preview capability for preview parameters, and
  may bind only view, filter, workspace and preview parameters, never puzzle state. The stored binding is the
  expression; its current value is derived each time and never written back as a workspace action, so a live binding
  adds no undo steps. A live binding on a preview parameter re-prepares the preview under ADR-D01.9. Each field shows
  its provenance (default, theme, session, override, script); clicking the label resets it. (framework for capability
  and derived values; gestures default)
- **ADR-D01.24** The solver may pin a capped number of cards and one reference state whose values appear beside the
  current ones. In-place controls next to a selection (floating band, hover readouts, swatch popovers) are building
  blocks a script may enable; they are not in the default set. (default)

### 5. Feedback

- **ADR-D01.25** Feedback appears at its source first. .6cs diagnostics run when editing pauses and show as
  non-reflowing end-of-line notes, gutter shapes and scrollbar ticks, in three severities coded by colour plus shape.
  Diagnostics about the puzzle paint the offending cells with a reserved, view-only error overlay in both Global and
  Local; it never changes state or relabels pieces. Every script, check and analysis shows pending, ok or failed on
  its own item. (default; overlay is view-only framework)
- **ADR-D01.26** Progress is drawn in place (pending bars over the script range, a ring on the originating panel),
  and one quiet status slot shows runtime health and the running-task count. Long analyses are cancellable tasks in
  the runtime process; Esc cancels, and a commit that changes the state a task reads cancels it and marks its queued
  previews stale. (framework for cancel and stale; drawing default)
- **ADR-D01.27** A bottom panel with Problems, Console and Tasks tabs is the index, collapsed by default, with count
  badges, severity filters, merged repeats and Find Last Error; each row links to the source line and the puzzle
  element. Completion of a long task is a status flash; any sound goes through G5 and the sound table
  (ADR-D01.56). (default)
- **ADR-D01.28** Built-in quick fixes are limited to syntax and contract fixes and to the transforms the boundary
  already allows on the solver's own macro (invert, mirror, conjugate, repeat, re-scope). A fix shows read-only as
  ghost text, like a code editor's suggestion, and enters the source only when the solver presses Tab; nothing applies
  itself. Running the changed macro is still a preview the solver commits. Fixes that would choose new moves wait
  until the owner and the integrator set the exact boundary; fix scripts the solver writes are the solver's own tools
  (owner, 3 November 2026). (framework)

### 6. Script interface

- **ADR-D01.29** Custom views, scripts and visualisations are among the most important parts of the language. The
  Rust side provides comprehensive building blocks for them: panels, typed parameter controls, tables, trees, charts,
  overlay annotations, free views, and the canvas below. Each one is in the display-primitive inventory
  (ADR-L11.5). (framework)
- **ADR-D01.30** A program declares its kind, typed parameters and mount points: palette entry, context action,
  inspector section, docked panel in a slot, free view, or overlay annotation in a view. The host draws declared
  panels with design-system widgets and tokens. Panels have stable ids restored from the session store, a visible
  target with a pin, the selection as input, and a placeholder when the script is missing. Hot reload, an error list
  beside the code, a restart-scripts command and a safe-mode start are part of the contract. (framework)
- **ADR-D01.31** A canvas primitive lets a script draw vector content with design tokens, inside panels and as view
  overlays, with minimal restrictions. It is display only, it never commits, and the host repaints it on a theme
  change and composites host statuses over it (RPI.76). Custom pixels in the scene still go through user stages
  (ADR-L11.12). The canvas is part of the display-primitive inventory. (framework)
- **ADR-D01.32** Solvers may install packages written by others. In 1.0 they are shared as local files with digests;
  there is no registry (LQ12). A package declares its capabilities in a manifest, and the solver consents at install;
  the consent covers later runs until the solver revokes it in settings. Two docked slots for script panels sit
  beside the main views before tabs group them. Script icons come from the provided icon set. (framework for manifest
  and consent; slot count default)

### 7. UI toolkit and accessibility

- **ADR-D01.33** All chrome is built from Godot Controls and Containers, styled by Theme resources generated at run
  time from the token registry. The main views and free views are D3D12 surfaces embedded as Controls and own their
  camera, picking and preview overlays. Docking is an app-owned dock model saved in the session store. Long lists are
  virtualised; UI scale uses named steps with per-monitor and fractional DPI; a component gallery renders every widget
  under every theme and look; the Godot version is pinned. (framework)
- **ADR-D01.34** An embedded web frame (WebView2 class, Windows-only, inside the platform layer) is used only where
  Godot Controls are not enough. An embedded code-editor component may be added the same way if Godot's text editing
  falls short. Both are in the approved dependency list as conditional entries. (framework for the condition)
- **ADR-D01.35** Accessibility for 1.0 is high contrast and keyboard-only use; screen-reader support is not in 1.0.
  The owner's criteria are read strictly in order. (framework)

### 8. Layout and workspaces

- **ADR-D01.36** One window with a fixed frame: top bar, three docks (left, right, bottom) each showing one panel at a
  time, and a status line under each main view. The centre is a pane group of splits and tabs holding the main pair
  as one locked container, plus free views and the .6cs editor as peer panes. The pair cannot be closed, neither view
  can be tabbed behind the other, and a drop that would hide one is refused with a visible reason. Maximising either
  view maximises the pair. Focus mode keeps only the pair. (framework for the locked pair; frame default)
- **ADR-D01.37** Scripts may change the layout freely within the locked-pair rule: different Global and Local
  arrangements, free tiling of any pane, new split tabs and new workspaces. Floating tool windows and detaching a pane
  to a second monitor are not in 1.0. (framework for the rule; arrangements default)
- **ADR-D01.38** Named workspaces are values in the session store and also per user, with a "load the session's
  layout" switch. Shipped presets: Solve, Script, Analyse, Reference. They are edited from the UI, undone in the
  layout history, reset to preset and exported. A program may read workspaces and prepare one as a layout preview;
  applying it is the solver's action, a bound entry point or a recorded step. (default)
- **ADR-D01.39** "Mode" in the layout means arrangement only. A workspace never changes the command vocabulary or
  capabilities and never enables or disables commits. A separate explore state, in which commits are paused, is a
  script-provided state signalled on every view frame; it may only pause commits, never enable them. (framework)

### 9. Linked views

- **ADR-D01.40** Global and Local are the main linked views and always show the committed state. Previews and other
  visualisations of a solving workflow are separate views, not the main pair. Local is not a zoomed Global: it is the
  view the solver reads at all times, and scripts may shape it for each solving workflow. (framework)
- **ADR-D01.41** All views read one shared context in the view model: selection, hover and the history cursor.
  Selection and hover are linked by default. The 4D rotation of Global and Local is linked by default and can be
  unlinked; camera links are named and marked on both frames. Local re-frames on a selection change by default;
  auto-framing can be turned off per view or by script. (default)
- **ADR-D01.42** Free views carry a binding chip that names their source: committed state, a preview, a saved
  reference or a history position. They can be pinned, compared with an A/B wipe, and returned to committed. There
  are no numbered link channels. (default)
- **ADR-D01.43** The .6cs caret is a focus source for the preview view. In a preview program it sets the preview to
  the state after the caret line, debounced (a live preview under ADR-D01.9); in a read program it highlights that
  line's result. Pin, "go to pinned line" and Pause are provided. Picking a sticker or overlay reveals the producing
  .6cs line. Each script output keeps one stable colour across its row and its overlay, and marks use shape plus
  colour, never red/green alone. (default; debounce tuned)

### 10. Density

- **ADR-D01.44** The default is compact and graphic: closer to music production software than to office software,
  mostly unlabelled controls on a strict grid, borders before shadows, no nested cards, and enough visual body to be a
  canvas for the design and art session. Labels are off by default with a labels toggle; search always covers every
  setting; an advanced switch never hides a value that differs from its default. (default; values tuned)
- **ADR-D01.45** One density token table derived from one unit times the display scale, with compact, default and
  comfortable presets; compact ships. Each main view has its own status line (Hyperspeedcube style) that names the
  hovered element, shows its gestures as icons and shows set and effective values, under a capped length.
  (default; values tuned)
- **ADR-D01.46** Focus layers in the views: pick layers on single keys shown beside the view, one cycling key for
  normal, dimmed and hidden inactive parts, opacity per element class, orbit highlight, and saved view presets.
  Picking follows visibility: a dimmed or hidden sticker cannot be picked. Protection conflicts and complete-operation
  effects break through every focus layer (ADR-D01.4 item 8). In the editor: folding, collapsible inline outputs,
  stale outputs dimmed after undo, "select unused definitions" and first-line error underlines. All are view aids
  that never run or commit. (default; the breakthrough framework)

### 11. Type

- **ADR-D01.47** Three type roles reached only through role tokens: ui, content and code. Faces: IBM Plex Sans for
  ui, IBM Plex Serif for content (annotations, script docs, teaching text), Lilex for code, notation and identifiers.
  Each role has its own small closed size ramp; number fields use tabular figures; type metrics are the same in every
  theme family and look. No commissioned house face. (default; ramps tuned)
- **ADR-D01.48** Maths symbols are plain Unicode entered with a backslash input method, not typeset inline. A notation
  check on the face chain is the acceptance for charter D6; if the three faces fail it, a maths face joins the chain.
  Viewport labels use a separate rendering path. (framework for Unicode; the fallback is conditional)

### 12. Colour and themes

- **ADR-D01.49** Colour is layered semantic tokens in one data file per theme family: raw ramps generated from OkLCH
  seeds, semantic roles shared by all themes, component tokens, with groups for chrome, scene look (three per
  family), puzzle-data palette, state and .6cs syntax. (framework)
- **ADR-D01.50** Users tune through ranged knobs that move only the raw seeds (tone, contrast, black level, accent
  hue, grid intensity), with a live selector (Esc reverts), A/B slots and saved presets. An advanced panel overrides
  semantic tokens per theme as a difference file, with a contrast check on save. The theme follows the Windows light
  or dark setting by default. (default; the light-source model tuned in Look Lab)
- **ADR-D01.51** Everything may be recoloured, through settings or a script, including the puzzle-data palette and
  the state roles. A script names tokens; an explicit colour it sets keeps its value across theme switches, is flagged
  as an override and can be reset to its token. When a state or encoding role differs from its shipped colour, the
  view frame shows a small "non-standard colours" mark, and screenshots and recordings carry the token set used.
  (default; the mark is framework)
- **ADR-D01.52** The sticker palette is designed in OKLab and must pass G4's adjacent-cell distance and
  colour-vision-deficiency checks; the owner picks among passing palettes. (tuned)

### 13. Icons and motion

- **ADR-D01.53** Tier 1: monochrome command icons on one grid, addressed by semantic id and size token, with fills
  bound to theme roles. 1.0 starts from a verified open icon library as placeholder; the final set is drawn by the
  design and art session from the project's geometry (charter D3). (default; set open)
- **ADR-D01.54** Tier 2: content glyphs for cells, grips, axes, macros, previews and proof obligations, rendered from
  the same encoding tokens as the scene, after a 16 px legibility test; until it passes, content rows use tier-1 icons
  with labels and the same marks. Most of this work is in the later design and art session. (default)
- **ADR-D01.55** Motion appears only where state or view changed, and every motion has a meaning in the motion table
  (H-03). The engine commits before any animation, so skipping or reducing motion never changes state. Twists animate
  at the faster preset and speed up while the queue is non-empty; view transitions are eased and repeated clicks add
  up; an evaluated .6cs span flashes, then keeps a gutter marker. Reduced motion is off by default and available.
  Rewards are in-place glyph or marker changes, never modal, and can be switched off. (framework for commit-before-
  animate; presets default; curves tuned in G5)
- **ADR-D01.56** Sound is not decided here: one G5 comparison, then the full sound table in the third freeze round,
  with no cap on the number of cues (owner, 22 November 2026: "do not cut"). Each cue has a visual equivalent and mute, and
  no cue differs with whether a check or repair result is good or bad (neutral order). Cue ids are slot names for
  A7; the sounds are tuned later. (open, G5)

### 14. Language editor

- **ADR-D01.57** The .6cs editor is a dockable pane in the main frame, by default a right-hand column beside the main
  pair, built on Godot CodeEdit, with language services from the Rust host (semantic tokens, completion, hovers, span
  diagnostics, the backslash input method). Below it one tabbed panel holds the console (a REPL plus a read-only log
  that echoes every command-layer action as a .6cs line) and diagnostics. (default placement; editor framework)
- **ADR-D01.58** Ctrl+Enter in the editor runs a selection or region: a read program shows its result inline; a
  preview program opens the preview view under ADR-D01.8. A run key and the commit key never share a key in one
  context: in the editor the commit key is Ctrl+Shift+Enter by default, and outside it Ctrl+Enter. Enter in the
  command bar never commits. Results computed before a commit dim and name the entry they were computed at, with a
  Rerun action. Runs sit in a visible queue with an Abort key that restarts the runtime process without touching
  puzzle state. Dragging a cell, sticker, orbit or macro into the editor inserts a typed reference. (framework for key
  separation and no commit from a run; keys default)
- **ADR-D01.59** Lighter entry points to the same language: the palette's typed mode, a one-line command bar over the
  selection, and `=` expressions in inspector fields (ADR-D01.23). A notebook-style cell column is a building block a
  script may arrange; it is not the default editor. (default)
- **ADR-D01.60** External-editor support (file watch, hot reload from outside, a language server for other editors)
  is not in the first release. (default)

## Owner overrules of frozen clauses (22 November 2026)

The owner decided that this record follows the owner's picks where they conflict with frozen clauses, without a
versioned-change review. Each clause below is replaced for the 1.0 product as stated; every other part of the frozen
architecture holds. The integrator records the decision and marks each clause (wiki decision page, contradiction or
supersession notes in the chapters, and the format packet's pin refresh where a pinned chapter changes).

| Clause | Frozen text, short | Replaced by |
| --- | --- | --- |
| LG3.6 (and LG3.7) | A preview is prepared only inside a command the solver invokes; views have no preview effect | A view may drive a live preview (caret, live parameters, `=` bindings on preview parameters); ADR-D01.9, ADR-D01.23, ADR-D01.43 |
| LG3.1, LG3.5 | Programs cannot invoke view-kind rows; workspace changes only as requested changes inside an invoked command | An invoked entry point may run view and workspace rows; views still never write the workspace, and live bindings are derived values; ADR-D01.2, ADR-D01.23 |
| KN5.12 [O] | Whether a program may replace a shown preview is open | Answered: replacement allowed, with a new ID, a visible mark and a settle interval; ADR-D01.9 |
| KN5.14 | Commit token from a key-down on the preview shown on screen | Also the one-key preview-and-commit on the solver's own macros and twists; ADR-D01.12 |
| KN5.19 | One undo key and one durable order across journal and workspace | Two histories with their own keys: moves and layout; ADR-D01.14 |
| KN5.20 | `session.undo` and `session.redo` are the only entries, one step each | Multi-step rollback by one action, and partial undo inside an entry as its own recorded action; ADR-D01.15 |
| ONE.74, KN4.6 | A branch comes only from a commit after undo | Also mid-history insert, with the old line's later moves offered as a preview; ADR-D01.16 |
| EV4.16, BND.84 (and BND.15) | No solver-tool solving preview before the joint setting; program-produced words refused at preview in a solving session | The solver's own programs prepare solving previews; others' packages stay refused there; ADR-D01.8 |
| DET.37 | Capabilities granted per profile and per run | Consent at install, revocable; ADR-D01.32 |
| ADR-L11.5 | Display primitives are a stated inventory | The inventory gains the canvas and any building block of ADR-D01.29 it lacks; ADR-D01.31 |

Unchanged and still binding: no program commits (LG3.4, DET.35); every commit rechecks; recordings never commit and
stop where EV4.18 says; the 2 October boundary for built-in tools and the 17 October repair-list exception (INV-11).

## What this record does not decide

- **Values.** Palette (G4), motion curves and durations (G5), density numbers, type sizes, the settle interval, the
  debounce and the default keymap are set in Look Lab with the owner and stored as token data. This record fixes
  their slots.
- **The manifesto and the metaphor.** Charter items the owner writes; the candidate metaphors stay an owner draft.
- **The core flows for G6.** Proposed three: one cap turn with preview and commit (S2, S4); building one block under
  protection, orbit-first (`docs/progress/1.0/solving-workflow.md`); writing and running a .6cs macro. The owner names
  them at A5.
- **Sound** (G5) and the **tier-1 icon set** (design and art session).
- **The exact human-solve boundary** for built-in tools that choose new moves (owner and integrator, open since
  3 November 2026). ADR-D01.4 item 6 and ADR-D01.28 keep the built-in tools inside the current boundary.
- **Multi-monitor detach, a screen reader, external editors**: after 1.0.
- **Renderer feature costs.** A graphic default must still pass the gate in the shipped look (charter E1); a feature
  that breaks it alone returns to the design line with its measured cost (E-2.4-05).

## Why

- The owner's criteria, read strictly in order, put compactness and a professional look first; the A3 pairs show the
  strongest compact tools (Hyperspeedcube, Bitwig, FL Studio, TiXL, Rerun, Zed) win by focus and keyboard reach, not
  by emptiness.
- Separating framework from defaults answers the owner's two wishes at once: maximum script freedom, and nothing that
  lets a script commit, hide the committed state or break the shared reading of the screen.
- Key bindings carry the reach: a bound entry point runs as an action the solver took.
- Keeping previews out of the main pair keeps the one question each main view answers (charter D8) and makes the
  committed state visible without any action.
- One default set keeps 1.0 buildable; the extra picks become building blocks instead of parallel modes that each need
  tests, docs and themes.
- The owner chose the picks over the conflicting frozen clauses (22 November 2026); the safeguards that carry the
  frozen intent (the settle interval, derived live bindings, append-only partial undo, refusing others' packages at
  solving preview) keep commits deliberate and history recoverable.

## Alternatives considered

| Alternative | Why it was turned down |
| --- | --- |
| Previews drawn as a pending layer in both main views (preview-commit A as written) | The owner wants the main pair on the committed state; kept as a script option over the committed state (ADR-D01.10) |
| Local follows the focus and shows previews (linked-views B as written) | Owner: previews are separate views; binding chips moved to free views |
| Keep the frozen clauses and offer the nearest allowed forms (revision 2) | The owner chose the picks over those clauses on 22 November 2026 |
| Invisible undo stack (history A) | Move sequences are the solver's real product; history must be visible and reusable |
| Keymap without command search (commands D alone) | Hundreds of commands and a growing macro library cannot all be bound |
| No script panels (extension-ui A) | Against the owner's priority on custom views and visualisations |
| Script-drawn UI in an isolated window as the main path (extension-ui C) | Visual drift and floating windows over the views; kept only as the canvas primitive and, if ever needed, the web frame |
| Immediate-mode GPU chrome (ui-toolkit B) | Loses Godot's text input, DPI handling and keyboard navigation |
| Free floating windows (layout C as the frame) | Lost panels and fragile layouts; free tiling stays available to scripts inside the frame |
| Show-on-demand chrome (density B) | Hides state an expert needs; its safety valves are kept |
| One monospace for everything, or a commissioned house face (type C, D) | Owner excluded the house face; one monospace is too austere for teaching text |
| Seed-only themes or an open editor for every colour (colour A, C) | Combined as tiers plus knobs plus overrides (option D) |
| Text-only minimalism (icons D) | Against anti-goal 2 and charter D3 |
| Editor outside the app or taking over the screen (editor D) | The views must stay visible while the language is used |

## Consequences

- **Frozen texts.** The integrator records the overrules above; no versioned-change review (owner, 22 November 2026).
- **Command table rows**, added without the versioned-change review:
  - view and workspace rows for theme select and tune, token override and reset, density, type scale and motion
    presets, reduced motion, applying a stored arrangement, script panel mount and unmount, radial menu edit, palette
    pin and alias, explore state, preview view open and pin, A/B wipe, history dock modes;
  - layout-history undo and redo (two rows beside `session.undo` and `session.redo`);
  - multi-step rollback and partial undo (commit rows), and one-key preview-and-commit (a commit row with the recheck
    gates of `operation.execute`).
  Existing rows already cover layout (`work.layout.set`, `workspace.set`), keymaps (`keymap.*`, `file.keymap.*`),
  preview (`operation.preview`, `view.preview.scrub`), undo (`session.undo`, `session.redo`), branches
  (`session.branch.select`) and commit (`operation.execute`, `twist.apply`). Rows with `confirm` or `job` stop a
  recording for one user action (EV4.18).
- **Record kinds.** Layouts, dock state, theme overrides, keymaps, display presets, the layout history and `=` bindings
  need record kinds in PS1.27's partition (`c600-layout-draft` is reserved).
- **Engineering checks.** Tests for: the settle interval (no commit of a preview replaced within it); live bindings add
  no undo steps; partial undo and multi-step rollback are append-only and restore exactly; the two histories never
  mix; no key both runs and commits in one context; a package from others is refused at solving preview. The renderer
  must draw the preview view and up to two more states (A/B wipe) at full detail inside the gate.
- **The gate in the shipped look.** A graphic, compact default with free views and the preview view must be measured
  with the full chrome on the owner's machine (charter E1, ADR-R01).
- **Scope control.** Each building block beyond the default set needs a test and a token-driven look, but no
  separate built-in mode, theme or documentation set.
- **Packages from others** are a security surface: manifest, consent, the script frame and safe mode are required
  before any package installs.
- **Look Lab.** A5 builds the default set in Look Lab; LL first needs its layouts brought in line with the command
  table (A5's step 0).
- **Dependencies** are approved (owner, 22 November 2026; `ADR-D01-dependencies.md` beside this record).

## Evidence

Classes: source and public reference only. The A1 to A3 records cover 48 tools from public documents, source and
talks; closed tools were analysed from public material only. The owner's picks and answers of 21 and 22 November 2026
are preferences and decisions, not evidence. The frozen clauses cited above were read at 6d65d361 and the command
table on origin/main, on 22 November 2026. No Windows/DirectX, input, latency or performance claim rests on this
draft. The first visual evidence is the A5 build in Look Lab; the gate evidence is a later measurement in the shipped
look.

## What the owner accepts

The owner approved the content and the dependency list on 22 November 2026. Nothing is frozen until A7, where the
owner freezes item by item:

1. The (framework) items as the host contract; after freezing they change only through a new record that supersedes
   this one.
2. The (default) items as the initial content of the default profile. The profile is data: a later change goes
   through Look Lab and the owner's sign-off and is logged, without a new record.
3. The (tuned) items as token data that Look Lab may change.

Freezing this record answers no question beyond those, changes no gate and accepts no other ADR.
