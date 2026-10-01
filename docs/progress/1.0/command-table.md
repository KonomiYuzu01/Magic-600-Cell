# H-08 Command table and layer boundaries (draft)

Status: **first version for the design track, stage day 1**; due day 4. This draft is built from the [top-down inventory](inventory/top-down.md) only. Two later inputs change it:
- On day 3 the merged inventory adds the code-only functions.
- On day 4 the 2.2 dispositions mark each command keep, redesign, delete or automate.

Every command is `pending-2.2` until then. The architecture freeze (2.5) freezes the table and the layer interfaces.

Data: [command-table.json](command-table.json), 86 commands, built and checked by [build_command_table.py](build_command_table.py) (`python docs/progress/1.0/build_command_table.py`). The script fails in these cases:
- a row names an unknown inventory ID;
- a row uses a value outside the vocabularies below;
- a committing command may repeat or skips the busy gate;
- an inventory function has no command and is not listed as "not a command".

## 1. The six layers

Each layer may call only the layer below it. The right-hand column is what crosses the boundary.

| Layer | Owns | Must not | Gives the layer above |
|---|---|---|---|
| **Engine** | Mathematics: the model, legal moves, effects, protection checks, previews, names and residuals. It is proven equal to 0.4 by the differential oracle. | Hold UI state, read the clock for decisions, or write files. | Results with an evidence state (`verified`, `recorded`, `unknown`, `unchecked`), typed errors, and the availability of each command |
| **Session store** | The journal, checkpoints, workspace (drafts, macros, worksheets, keymaps, filters) and preferences, plus migration import. One transaction per commit (R-06). | Interpret mathematics; write a response while holding the session lock (R-15). | Revisions (`state`, `workspace`, `protection`), durable command receipts, recovery after a crash |
| **Command layer** | The command table: command IDs, gates, preview rule, undo class, contexts and job handling. It is the only entry for anything that changes state, workspace or files. | Decide availability itself (it asks the engine), or have a second path that skips a gate (R-05, R-07). | One `invoke(command_id, args)` with a receipt, and availability per command for the current context |
| **View model** | What every panel shows: selections, the forecast in view, filter results and inspection. It is built from engine snapshots and revisions. | Call the engine synchronously on the UI thread (R-14); keep its own copy of the puzzle state. | Observable view state; it updates only when its inputs change (R-19) |
| **Shell** | Layout, panels, motion, focus and input routing (keys, pointer, onscreen keyboard), following the layout specification and keyboard model from the design track. | Change state except through the command layer; enable a control the engine says is unavailable. | Command invocations; view and camera state for the renderer |
| **Renderer** | Direct3D 12 drawing of 259,800 stickers, the frame and overlays; picking from a screen point to a sticker. | Change or relabel mechanical state; draw a state that is not committed (instant turn). | Frames, pick results, device-loss reports (R-17) |

Rules that cross the layers (R numbers refer to [requirements-from-screening](requirements-from-screening.md)):
- **Commit outcome is separate from refresh.** A committed command is never reported as failed. "Applied, display out of date" is its own state (R-02).
- **Durable command identity.** A retried command keeps its ID and is committed at most once ([1.0 architecture V03.5](../../architecture/1.0/10_V1_ARCHITECTURE.md), proposal).
- **Typed errors.** The error categories are those of the architecture V06 proposal: input, unverified reference, protection, stale, cancelled, resource limit, consistency, unknown outcome, auth and busy. Each category has its own continuation. A hard failure is never shown as a low score.
- **Three things the shell never merges:** evidence state, score and permission.

## 2. Command row format

| Field | Meaning |
|---|---|
| `id` | A stable dotted ID, for example `operation.execute`. Keys, buttons, menus and scripts all name this ID. |
| `label` | A working name; the final wording comes from the design track. |
| `kind` | `commit` changes the puzzle state through the journal. `workspace` changes drafts, protection, macros or preferences. `view` changes only presentation. `query` is read-only. `file` writes an export. `app` is process or job control. |
| `inventory` | The top-down inventory rows it serves. |
| `gates` | The checks that must pass: `protection`, `fresh_preview` (a preview of the current revisions), `confirm` (explicit user confirmation of scope), `model_match`, `not_busy`, `file_check` (a check of the same file digest). |
| `preview` | `required` means the command runs only from a current preview. |
| `undo` | `journal` (undo and redo), `workspace` (the workspace's own undo) or `none`. |
| `contexts` | Where the command is available: `any`, `puzzle`, `solve`, `operation`, `keyboard`, `filter`, `macro_base`, `editor`. |
| `repeat` | Whether a held key may repeat it. It is always false for `commit` (R-18). |
| `job` | Runs as a cancellable job; the shell stays responsive. |
| `key_0_4` | The 0.4 binding, for reference only. 1.0 keys come from the keyboard model (G6). |
| `disposition` | Set by 2.2. |

## 3. What the table shows now

| Kind | Commands | Examples |
|---|---|---|
| workspace | 32 | phases, buffers, intents, blocks, protection, macros, keymap edits |
| view | 27 | panels, cycle modes, inspection, display controls, camera |
| query | 12 | check, preview, buffer analyser, recommendations, filter counts |
| commit | 11 | execute, live twist, undo, redo, new solve, reset, log import, checkpoint save and restore, timer start and stop |
| file, app | 4 | exports, resume, stop a job |

Only eleven commands change the puzzle. Everything else edits the work around it or the view. This is the main point for the layout: the commit path is narrow (preview, then execute; or a live twist), while most of the interface is preparation and inspection.

Eleven inventory rows are not commands. They are launch arguments, storage policy, process ownership, an input routing rule, a renderer rule, an engine event, and the separate browser client. They are listed in `not_commands`.

## 4. Use by the design track

- **Greybox layouts (G1):** every control in a greybox names a command ID. A layout passes the reachability check when every kept command is reachable in it.
- **Keyboard model (G6):** keys bind to command IDs and contexts. A key in `editor` context never reaches a puzzle command. Repeat is allowed only where `repeat` is true.
- **Flow scripts:** a core flow is a sequence of command IDs. The script measures steps, pointer travel and time on each layout.
- **Pointing needs (H-05)** return to the engineering track as the commands that need a pick target (`object.inspect`, `grip.select`, `twist.apply`, `camera.rotate`, `camera.center`).

## 5. Open points

1. Command names for the code-only functions (day 3 merge).
2. Whether `session.timer.*` writes to the journal or only to the workspace. 0.4 documents an explicit timer but not where it is stored.
3. Whether the structure commands (`structure.*`, `view.cell_views`, `camera.center`, `set.save`) are kept. They come from 0.3 documents only.
4. Whether `twist.apply` with draft destination should be a separate command (`operation.phase.append_twist`) so that its gate set is simpler.
5. The availability protocol between the command layer and the engine (V04.3 proposal) needs the Astra plan check of the architecture before 2.5.
