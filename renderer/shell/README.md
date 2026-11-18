# The product shell (C9)

The shell is the Godot 4.7.2 .NET application of Magic 600 Cell 1.0. It owns the window, the UI and the input. The
S-A2 core (`renderer/core`, `sa2_interop.dll`) draws the puzzle, and Godot's renderer draws no part of it (ADR-R01.2).
The product targets net10.0, a supported .NET release (ADR-R01.5), and not the acceptance harness's net8.0.

Route B, accepted for now by the owner on 18 Nov 2026, pending three owner-machine checks
(`docs/wiki/decisions/owner-decisions-2026-11-18-route-b.md`): the core presents in a composition layer of its own
beneath the shell's window. The window therefore allows per-pixel transparency, and its background is transparent.
These window settings are the route-B probe's (`work/experiments/c9-present-probe`, pull request 245).

## What is here

- `project/`: the Godot project.
  - `Shell.cs` is the root node.
  - `CoreLibrary.cs` loads the core from an absolute path, resolves the exports that `renderer/bridge`'s function
    table names, in its order, and refuses another ABI version than 5. It also resolves `ShellCalls`, the route-B
    calls the shell makes itself and the table does not hold: `sa2_create`, `sa2_scene_load`, the three surface
    calls, `sa2_resize`, `sa2_drain`, `sa2_scene_unload` and `sa2_detach`. The bridge crates load no library
    (CT-RPI-20), so the shell is the only place that loads the core.
- `build_shell.py`: an offline build into `work/build/c9-shell/<stamp>/`. It uses the harness's offline restore
  (`renderer/acceptance/prepare_host.py`), which takes the packages only from the pinned Godot install, and writes
  `build.json`. It never launches Godot and builds no core and no bridge.
- `tests/test_renderer_shell.py` checks the project's fixed points without a build:
  - Godot.NET.Sdk 4.7.2 on net10.0;
  - the offline package source;
  - the route-B window settings;
  - the export list and ABI version equal to the bridge's and the header's;
  - every shell call declared by the header and absent from the bridge's table.

## Status

Step S2, the native host library: `c600-shell-host` in `renderer/bridge/host`, a member of the bridge's workspace and
outside the language workspace (CT-RPI-01). It is a draft:

- `m6h_create` takes the core's function table and a context the shell created with `sa2_create`. It loads the
  shipped model, adopts its solved state, opens one view and publishes it as one retained primitive of every slot.
- Then come `m6h_set_camera`, `m6h_set_viewport`, `m6h_pump`, `m6h_frame` (to the surface), `m6h_read_presented`
  (confirmed presentations), `m6h_recover` (which publishes the view again for the new device), `m6h_last_error` and
  `m6h_destroy`.
- Every call checks its pointers, catches a panic and answers a status code.
- The host context is c600-display's declared fixture context with the core's own support report, and the limit
  profile is a declared draft value. Picking, previews, the command layer and the session store come later.
- Its tests drive the C interface over the bridge's C shim and the fake core: frames confirmed, a lost device and
  recovery, and the refusals. `tests/test_render_ffi.py` runs them. Evidence kind: source and fixture.

Step S1 of the shell, moved to the core's ABI 5:

- The project skeleton builds offline on net10.0. On 18 November 2026 it built with .NET SDK 10.0.401 and Godot.NET.Sdk
  4.7.2, with no warning, before and after the move to ABI 5.
- It was never launched. Nothing here is Windows/DirectX, input or performance evidence.
- Evidence kind: source build.

These parts are drafts of C9 that the sources leave open:

- The engine, the session store, the command layer, the view model and the display host run in the shell process, as
  one Rust library that the C# side calls. The runtime keeps a process of its own (RTE.21).
- That library lives outside the language workspace, beside `renderer/bridge`, for CT-RPI-01.
- The shell hands the core's export addresses to the bridge's table.

Next steps:

- the C# calls into the native host library;
- the window and layout with the core's surface attached, through the shell calls;
- the frame loop and the E2 and E3 log rows.
