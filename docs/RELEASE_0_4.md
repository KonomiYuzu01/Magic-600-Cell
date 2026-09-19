# Magic 600 Cell 0.4

Magic 600 Cell 0.4 adds the native solving workspace with explicit Prepare / Macro / Cleanup operations, reusable macros, protection policies, checkpoint recovery, orientation and buffer tools, and physical keyboard input.

The default G2 layout keeps the native puzzle visible. Minimized tools restore normally and the main window can return to the foreground.

Download the Windows x64 portable ZIP, extract it, and run Magic600Cell.exe. No separate Python or compiler is required. Microsoft Managed DirectX remains an external prerequisite; see [DIRECTX.md](../DIRECTX.md).

Performance optimization is scheduled for the next iteration.

The accompanying recordings use real application footage, with waiting and repeated navigation removed. English captions identify the demonstrated actions. Source and artifact hashes are included in the release provenance.

Original puzzle simulator and renderer: Magic Puzzle Ultimate by Andrey Astrelin. See the included credits and third-party notices.

## Run and source

The native default is G2. G1 remains available for its alternate keyboard workspace. Start the package normally for G2; use the launcher with `--mode g1` for G1. Keep personal profiles separate from test profiles.

The exact 0.4 source is under `work/experiments/magic600-04/`, with shared backend, native and asset files at the repository root. This existing directory name is retained so the frozen source paths and package manifest remain reproducible. Source development entry: `python work/experiments/magic600-04/native_launch.py --mode g2 --session development`. Windows native build requires the documented .NET compiler and Managed DirectX. See the [source/build guide](DEVELOPMENT.md) for packaging inputs, pinned tools and the public checkout's missing harness-input limitation.

## Source and package binding

The frozen source-file hashes, native inputs and ZIP hash are recorded in [RELEASE_0_4_PROVENANCE.json](RELEASE_0_4_PROVENANCE.json). The portable package contains its build-time package manifest. Its original candidate-stage label is retained as build provenance; this accompanying release record identifies the accepted unchanged artifact.

The final package's resource check, legal engine transaction and recovery entry, dependency check, actual keyboard sequence, normal exit and default native view were checked. Earlier unchanged solver/protection/checkpoint evidence was reused. Recording edits do not represent timing measurements. Raw private recordings and failed attempts are retained locally; they are not uploaded with this source snapshot.

See the [0.4 release assets](https://github.com/KonomiYuzu01/Magic-600-Cell/releases/tag/0.4) for the portable ZIP and SHA256SUMS.txt. The captioned demonstration is delivered separately; it is not included in the GitHub release assets.

[Usage](../USAGE.md) · [Documentation index](README.md)
