# Magic 600 Cell 0.4

A Windows solving workbench for the full 600-cell puzzle, built around **[Magic Puzzle Ultimate by Andrey Astrelin](https://superliminal.com/andrey/mpu/)**. Primary credit for the original puzzle simulator and renderer belongs to Andrey. Earlier versions of this project were named C600 Studio.

**[Download 0.4](https://github.com/KonomiYuzu01/Magic-600-Cell/releases/tag/0.4)** · [Usage](USAGE.md) · [Release guide](docs/RELEASE_0_4.md) · [Documentation index](docs/README.md)

## Run 0.4

Download the Windows x64 portable ZIP, extract the complete folder, and open `Magic600Cell.exe`. Keep `Magic600Engine.exe` and `_internal` alongside it. No separate Python or compiler is needed. The native renderer requires .NET Framework 4.x and the supported [Managed DirectX runtime](DIRECTX.md).

The default G2 layout keeps the native puzzle visible. The alternate keyboard workspace is available with `Magic600Cell.exe --mode g1`. Personal data is stored separately under `%LOCALAPPDATA%\Magic600Cell\0.4`; older C600 Studio profiles are not automatically migrated. See [sessions and rollback](USAGE.md#sessions-and-rollback).

## What 0.4 includes

- A native solving workspace with explicit Prepare / Macro / Cleanup operations.
- Reusable macros, full-effect inspection, protected-orbit checks and checkpoint recovery.
- Linked Current / Operation / After cycle views, orientation and buffer tools, and endgame families.
- Physical keyboard input and configurable key sets, plus C600 / MPUlt v1 log exchange.

All operations retain the full puzzle state: 35 moving orbits, 177,120 surface pieces, 259,800 labelled sticker slots and 1,200 positive generators. Display filters do not change the mechanical model. The user chooses and executes solving steps; the application does not automatically solve the puzzle.

Performance optimization is scheduled for the next iteration. The [release record](docs/RELEASE_0_4.md) distinguishes verified checks from reused evidence and remaining limits. Historical performance measurements are not a 0.4 performance certification.

## Repository guide

| Location | Purpose |
| --- | --- |
| [USAGE.md](USAGE.md), [release guide](docs/RELEASE_0_4.md) | Start here for the released application. |
| [work/experiments/magic600-04/](work/experiments/magic600-04/) | Released 0.4 workspace source and packaging inputs; the original path is retained for reproducibility. |
| Root Python modules, [native/](native/), [assets/](assets/) | Shared backend, retained native integration and immutable full-model assets. |
| [Build guide](docs/DEVELOPMENT.md) | Source layout, prerequisites and public-checkout limitations. |
| [RELEASE.json](RELEASE.json), [SOURCE_MANIFEST.json](SOURCE_MANIFEST.json) | Version identity and source inventory; [release provenance](docs/RELEASE_0_4_PROVENANCE.json) binds the frozen package inputs. |
| [Research](research/README.md) | Mathematical references, reports and audits, with their original version and evidence scope. |
| [Documentation index](docs/README.md) | Current documentation, historical records and future proposals. |

Root launch scripts, `web/` and `packaging/` retain the earlier shared/0.3 paths; they are not the 0.4 portable-package entry point. The `work/experiments/magic600-04` name does not make the published 0.4 release a new experimental build.

## After 0.4

The next planned version is **0.41**, focused on optimizing 0.4, debugging and performance, alongside a dedicated local development workbench. This is a recorded plan, not a released feature set. The [1.0 architecture proposal](docs/architecture/1.0/README.md) remains future documentation and does not describe the shipped 0.4 renderer.

## Credits and license

Magic 600 Cell is a derivative work of Magic Puzzle Ultimate, © 2010 Andrey Astrelin, distributed under the same terms as the original project. Acknowledgements to ivan216 for source references and Nan Ma for historical solving material.

[Credits](CREDITS.md) · [MIT license](LICENSE) · [Third-party notices](THIRD_PARTY_NOTICES.md)
