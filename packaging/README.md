# C600 Studio 0.3 for Windows

> **Legacy 0.3 packaging.** For the released 0.4 application, use the [0.4 release guide](../docs/RELEASE_0_4.md) and [source/build guide](../docs/DEVELOPMENT.md). This directory retains its original 0.3 scripts and package instructions.

**Andrey Astrelin created Magic Puzzle Ultimate**, whose original puzzle engine,
geometry conventions, and renderer underpin this application. See
[Andrey's original project page](https://superliminal.com/andrey/mpu/) and the
included `CREDITS.md` for acknowledgements and upstream sources.

Extract the complete ZIP into a folder, then double-click **C600Studio.exe**.
Keep its accompanying `_internal` folder and `C600Engine.exe` together with it.
The package includes its Python interpreter, NumPy, and compiled native host;
you do not need to install Python or a C# compiler.

The application uses the original MPUlt renderer and requires 64-bit Windows
with .NET Framework 4.x and the legacy DirectX components needed by MPUlt.
The supplied release was tested on Windows 10 with Intel HD Graphics 620.
This is a portable folder application, not an installer.

This 0.3 package includes the structure and auxiliary-view improvements.
The older GitHub 0.2.4 download and its historical validation remain unchanged.

Microsoft Managed DirectX DLLs are not redistributed in this package. The app
uses the required version already installed on your computer, or an existing
verified C600 Studio runtime cache. If startup reports missing Managed DirectX,
download [Microsoft DirectX End-User Runtimes (June 2010)](https://www.microsoft.com/en-us/download/details.aspx?id=8109),
extract that Microsoft package, run its `DXSETUP.exe`, then start the app again.

Normal startup opens the puzzle directly. Developer WinForms fixture windows
run during the build, never during normal application startup.

Your session, checkpoints, settings, and native keys are stored under
`%LOCALAPPDATA%\C600Studio`. Existing C600 Studio sessions are preserved.
Close the application before moving its extracted folder. The private engine
closes with the application; `C600Engine.exe` is not a second user interface.

Rendering filters affect only the displayed scene. All 259,800 labelled stickers
remain in the puzzle state. During a dense scene's camera motion, the smooth
motion option temporarily reduces drawn detail and restores full detail when
motion stops.

Use **Cell views** to open the **Global overview** and **Focused neighborhood**.
Both windows can remain open; drag their title bars or resize them as needed.
The local view has independent rotation/zoom, **Follow selection** and **Pin**,
and an explicit **Center main** action. Narrow windows use the **Views** drawer.
Press **F8** to show or hide the tools. The **Structure** tab provides C1–C600
lookup, shared-face neighbors, cell-centered layers, and reviewed filters.

Shift+left click inspects a piece's home Cell Centers; Shift+right click locates
the piece required at the clicked destination. Shift+left drag retains 4D
rotation. Hidden pieces remain noninteractive. See the project's
[structure guide](https://github.com/KonomiYuzu01/C600-MPUlt/blob/main/docs/STRUCTURE_EXPLORER.md)
and [measured performance limits](https://github.com/KonomiYuzu01/C600-MPUlt/blob/main/docs/DEVELOPMENT_PERFORMANCE.md).

The C600 Studio MIT license is in `LICENSE`; the original MPUlt MIT notice is
preserved in `licenses/MPUlt-MIT.txt`. Other bundled third-party license texts
are in `licenses`. No session databases,
personal logs, account information, or remote telemetry are included.

For developers, the source recipe is `packaging/build_windows.py`, using the
pinned build dependencies in `packaging/requirements-build.txt`. Developer
regressions remain available in the source distribution.
