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
    table names, in its order, and refuses another ABI version. The bridge crates load no library (CT-RPI-20), so
    the shell is the only place that loads the core.
- `build_shell.py`: an offline build into `work/build/c9-shell/<stamp>/`. It uses the harness's offline restore
  (`renderer/acceptance/prepare_host.py`), which takes the packages only from the pinned Godot install, and writes
  `build.json`. It never launches Godot and builds no core and no bridge.
- `tests/test_renderer_shell.py` checks the project's fixed points without a build:
  - Godot.NET.Sdk 4.7.2 on net10.0;
  - the offline package source;
  - the route-B window settings;
  - the export list and ABI version equal to the bridge's and the header's.

## Status

Step S1 of the shell:

- The project skeleton builds offline on net10.0. On 18 November 2026 it built with .NET SDK 10.0.401 and Godot.NET.Sdk
  4.7.2, with no warning.
- It was never launched. Nothing here is Windows/DirectX, input or performance evidence.
- Evidence kind: source build.

These parts are drafts of C9 that the sources leave open:

- The engine, the session store, the command layer, the view model and the display host run in the shell process, as
  one Rust library that the C# side calls. The runtime keeps a process of its own (RTE.21).
- That library lives outside the language workspace, beside `renderer/bridge`, for CT-RPI-01.
- The shell hands the core's export addresses to the bridge's table.

Next steps:

- the native host library and its C interface;
- the C# calls into it;
- the window and layout with the core's surface attached, which needs the core's ABI 5;
- the frame loop and the E2 and E3 log rows.
