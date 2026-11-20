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
  - `Shell.cs` is the root node. When the environment names the core library (`M600_CORE_DLL`), the host library
    (`M600_HOST_DLL`) and the assets folder (`M600_ASSETS`), it opens the core's surface under the whole window,
    creates the host over it, follows window resizes, and submits one frame per process step (step S4, a draft).
    Charter E2's turns (a draft): when `M600_SESSION_DIR` (a new folder) and `M600_TWIST_MENU` (the twist menu's
    record) are set as well, the host opens a session there, and every key with a W3C code (`KeyChord.cs`: the
    physical key after Ctrl, Shift, Alt and Meta) goes through c600-input's router on its draft US QWERTY map. A
    twist key (`KeyJ`, `KeyK`, `KeyL`, with Shift for the inverse) commits `twist.apply` in the command layer over the
    session's journal, and the host adopts and publishes the new state at the next engine revision. A lost keyboard
    focus releases the router's held keys. The close runs the session's correctness control.
  - `EventLog.cs` (step S5, a draft) writes charter E2's event log, `tools/perf/c9`'s DRAFT
    `magic600-c9-shell-events-v1`, when `M600_EVENTS_DIR` names an absolute folder without a log and
    `M600_BUILD_IDENTITY` names the build (`M600_RUN_ID` is optional). Each submitted frame gets a `frame` row at the
    QPC read just before the call, and each confirmed presentation a `presented` row with the core's display time.
    `events.jsonl` is written on close, with the `run` row first.
    - Every measured press, a twist key's key-down (not a repeat) that committed or was refused, gets a `key` row in
      press order. A committed press adopted at engine revision R names the first submitted frame that draws
      revision R over a publication of revision R or later (the conservative DRAFT reading); a refused press, and a
      press no frame showed before the close, names no frame, which `key_to_frame.py` counts as unconfirmed. `frame`
      rows carry `engine_revision` and `publication_revision` as extra fields.
    - `correctness` is the session's correctness control, `journal-reopen-and-kernel-replay`, with its record as
      `correctness_detail`: the live state, the state a reopen of the journal replays, and the kernel's own replay of
      every committed press from the solved state must agree, and the newest presented frame's revision must show
      the replay's state after that many presses. An undo or redo (Ctrl+Z, Ctrl+Y on the draft map) is shown but
      fails the control, since its replay knows only twists. Without a session, `correctness` is `fail` with
      `correctness_control` `none`.
    - The core does not report its swap chain's address, so there is no `swap_chain` and no PresentMon cross-check.

    The E3 startup log (`magic600-c9-startup-v1`) is not written yet.
  - `InputLog.cs` and the echo in `Shell.cs` serve route B's check B1. When `M600_INPUT_ECHO` is `1`, a panel over the
    view echoes every key press (not a repeat) and mouse click (not the wheel) that reaches the window, and
    `input.jsonl` (`magic600-c9-shell-input-v1`, a C9 DRAFT) records each with the QPC of its receipt, the window
    pixel of a click, and a `run` row with the totals and whether the window kept the foreground. It is a file of its
    own, because `key_to_frame.py` refuses any row type it does not know. The echo works with or without the core.
    The foreground counts as lost once the window had it and lost it, or if it never had it (both logs).
  - `CoreSurface.cs` drives the core context's life through the shell calls: `sa2_create`, `sa2_surface_attach` and
    `sa2_scene_load` on open, `sa2_surface_move` and a drained `sa2_resize` while it lives, and `sa2_scene_unload`,
    `sa2_drain`, `sa2_surface_detach` and `sa2_detach` on close. It opens Godot's own adapter (the LUID of Godot's
    D3D12 physical device), not DXGI's first hardware adapter. Draft choices: the acceptance harness's default
    config, and scene 1.
  - `HostLibrary.cs` loads the native host library (`c600_shell_host.dll`, step S3), resolves its `m6h_*` exports,
    refuses another version than 2, and wraps each call. `Create` hands the library the core's table and a context.
  - `CoreLibrary.cs` loads the core from an absolute path, resolves the exports that `renderer/bridge`'s function
    table names, in its order, and refuses another ABI version than 5. It also resolves `ShellCalls`, the route-B
    calls the shell makes itself and the table does not hold: `sa2_create`, `sa2_scene_load`, the three surface
    calls, `sa2_resize`, `sa2_drain`, `sa2_scene_unload` and `sa2_detach`. The bridge crates load no library
    (CT-RPI-20), so the shell is the only place that loads the core.
- `build_product.py`: the product build on route B, offline, into `work/build/c9-product/<stamp>/`: the core
  (`renderer/core/build.cmd`, the DLL with its shaders) in `native/`, a release build of `c600-shell-host` in
  `host/`, the shell (`build_shell.py`) in `shell/`, a copy of the model's tracked files in `assets/`, and a copy of
  the twist menu the session pins (`research/jumbling/fixtures/wj-I_a.json`, the I_a menu; G6 owns the menu) in
  `menu/`, a release build of `c600-runtime` in `runtime/`, and charter E3's session fixtures, which the host's
  `c600-shell-session-fixture` writes: Jumble off in `session-fixture/` and Jumble on (`--puzzle jumble-on`) in
  `session-fixture-jumble-on/`. A launch reopens a fresh copy of its puzzle's fixture. A release build of
  `c600-shell-lx4` (`renderer/bridge/lx4`, LX-4's editor library, C9 U8) goes in `lx4/`; the shell loads it only with
  `launch.py --lx4`. A build made before this part existed fails the check and must be rebuilt.
  `product.json` (`magic600-c9-product-build-v1`) names every part by its SHA-256 and gives the build identity, a
  digest of the parts. It never launches anything.
- `launch.py`: the one way to start the shell. It recomputes every part of a product build (default: the newest),
  creates the run folder `--out` (never an existing one), and starts the pinned Godot (the process that presents)
  on the build's shell with `M600_CORE_DLL`, `M600_HOST_DLL`, `M600_ASSETS`, `M600_EVENTS_DIR`,
  `M600_BUILD_IDENTITY`, `M600_RUN_ID`, `M600_SESSION_DIR` (`session` in the run folder) and `M600_TWIST_MENU`,
  plus `M600_INPUT_ECHO=1` with `--echo`. `--lx4 --lx4-fixtures DIR` (the two go together) adds `M600_LX4=1`,
  `M600_LX4_DLL` (the build's `lx4/c600_shell_lx4.dll`) and `M600_LX4_FIXTURES` (DIR, a folder that `c600-lx4 --out`
  wrote; it must hold `MANIFEST.json`). The fixtures are not part of the build, so `launch.json` records
  `MANIFEST.json`'s SHA-256 as `lx4_fixtures` and never the folder. It waits for the shell to exit
  and writes `launch.json` (`magic600-c9-launch-v1`) with the build and run folders written as `<build>` and
  `<out>`. `--dry-run` checks and prints and starts nothing. Start it from a PowerShell on the desktop: a window
  started from a background process never gets the foreground.
- `build_shell.py`: an offline build into `work/build/c9-shell/<stamp>/` (or a folder `build_product.py` names). It uses the harness's offline restore
  (`renderer/acceptance/prepare_host.py`), which takes the packages only from the pinned Godot install, and writes
  `build.json`. It never launches Godot and builds no core and no bridge.
- `tests/test_renderer_shell.py` checks the project's fixed points without a build:
  - Godot.NET.Sdk 4.7.2 on net10.0;
  - the offline package source;
  - the route-B window settings;
  - the export list and ABI version equal to the bridge's and the header's;
  - every shell call declared by the header and absent from the bridge's table;
  - the host library's exports, version, status codes, press codes, and frame, key and presented rows equal to its
    Rust source, and the correctness control's name;
  - the event log's `key`, `frame` and `presented` fields as `key_to_frame.py` reads them, and a passing
    correctness only from the session's control;
  - the chord codes `KeyChord.cs` names, all known to c600-input;
  - `CoreSurface.cs` using every shell call and no other;
  - the input log's rows, a file and row kinds apart from the E2 log's, and the echo only when asked;
  - over a fake build: `product.json`'s parts and identity, the refusal of a changed or added part, the launcher's
    command and environment, a dry run that creates and starts nothing, the refusal of a used run folder, `--lx4`
    refused without its fixtures, and a `launch.json` without private paths.

## Status

Step S2, the native host library: `c600-shell-host` in `renderer/bridge/host`, a member of the bridge's workspace and
outside the language workspace (CT-RPI-01). It is a draft:

- `m6h_create` takes the core's function table and a context the shell created with `sa2_create`. It loads the
  shipped model, adopts its solved state, opens one view and publishes it as one retained primitive of every slot.
- Then come `m6h_set_camera`, `m6h_set_viewport`, `m6h_pump`, `m6h_frame` (to the surface; it names the frame's
  sequence, device epoch and surface), `m6h_read_presented` (confirmed presentations, each with the display time the
  core read from the confirming statistics row), `m6h_recover` (which publishes the view again for the new device), `m6h_last_error` and
  `m6h_destroy`.
- ABI 2 adds the turns: `m6h_open_session`, `m6h_key_down` (it answers the press and the adopted revision),
  `m6h_key_up`, `m6h_focus_lost`, `m6h_close_session` (the correctness control) and `m6h_check` (its record).
  `src/turns.rs` holds the session: KN5's command layer on M6's journal store over the product disk (`FsDisk`, with
  its lock), LP3's input router, and KN3's twist menu pinned in both. `m6h_frame` names the frame's engine revision
  and the revision of the publication it draws.
- ABI 3 adds the session fixture (`m6h_open_session` with a fixture) and `m6h_load_runtime`, which starts the language
  runtime's process.
- ABI 4 adds the view jobs (C9 U8, `src/view.rs`): `m6h_run_view` schedules a program text as the view's job and
  returns at once; the runtime runs it on one worker thread, and `m6h_pump` or `m6h_frame` delivers a completed run's
  reference publication with the plan dependency `program` (the hex SHA-256 of the text). `m6h_view_result` reads a
  ticket's record (status, revision, source and value digests, first confirmed frames; a refusal's code and reason as
  JSON in `m6h_last_error`), and `m6h_view_draft` names the editor's digest so that the view reads stale until a run
  of that text publishes; a draft cancels no run. A run serves no host row, and its limits, deadline and output
  allowance are declared draft values. `src/bin/c600-shell-runtime.rs` is the runtime's child process for the tests.
- ABI 5 adds the session's puzzle: `m6h_open_session` takes `M6H_PUZZLE_JUMBLE_OFF` (0) or `M6H_PUZZLE_JUMBLE_ON` (1),
  the indices of the shell's puzzle names. With Jumble on, a new session commits `jumble.on` at once, the twist keys
  commit `jumble.twist.menu` under the pinned menu, and each adoption is a jumbling engine bundle (`pose_index`,
  `poses`, `pose_lattice`) with one posed plan from `c600_display::jumble`. A fixture names its puzzle and opens only as
  that puzzle. c600-render hands the core the bundle's pose arrays over the home labels and draws every slot of the
  posed plan, none pickable (J13 stays unsupported).
- ABI 6 adds the preload (charter E3's startup, a draft; `src/preload.rs`): `m6h_preload` starts `m6h_create`'s model
  loads, and with a session the exact model's that the twist menu reads, each on a thread of its own, and returns at
  once. The shell calls it before it opens the core's surface. `m6h_create_preloaded` is `m6h_create` with those loads:
  it joins the kernel's and the geometry's where `m6h_create` makes them, and `m6h_open_session` joins the exact
  model's, so each load answers where it did before. `m6h_preload_free` frees a preload that no call took. The model
  loads themselves hash, unpack and check their files side by side (`c600_kernel::parallel`), taking each result in
  its old order, so they give the same model and the same first error.
- Every call checks its pointers, catches a panic and answers a status code.
- The host context is c600-display's declared fixture context (with Jumble on, its jumbling context for the menu's
  identity) with the core's own support report, and the limit profile is a declared draft value. Picking and previews come later.
- Its tests drive the C interface over the bridge's C shim and the fake core: frames confirmed, a lost device and
  recovery, and the refusals; and the turns: twist keys committed and adopted, the two revisions of the frames, keys
  that are no press, an undo that fails the control, a shown revision the host never adopted, the session's refusals,
  and Jumble on (the entry, the menu row, the pose replay, a jumble-on fixture, the posed bundle and plan that
  c600-display validates, and the core's adoption of its pose arrays); and the view jobs (`tests/view.rs`): a run that publishes, parse and admission refusals, a run that a newer
  run supersedes, a twist before a run's delivery and before its installation, a destroy with a run in flight, and
  drafts. `tests/test_render_ffi.py` runs them. Evidence kind: source and fixture.

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

Steps S3, S4 and S5 built offline on 18 November 2026 with no warning. They were never launched.

The product build, the launcher and the input echo: on 18 November 2026 `build_product.py` built the core, the host
and the shell offline with no warning, and `launch.py --dry-run` checked the build. The shell was not launched: its
first launch (ABI 5's GPU smoke, part 2) and check B1 are owner-machine steps of the one sitting.

The turns: on 18 November 2026 the host's tests passed over the fake core and the shell built offline with no
warning. They were never launched; E2's key-to-frame run (runbook step A3) is an owner-machine step of the one
sitting.

Next steps:

- the E3 startup log;
- the owner-machine run of S4 (route B check B1 first), in the one sitting.
