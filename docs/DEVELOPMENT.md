# Magic 600 Cell 0.4 source and build guide

For normal use, download the [0.4 portable package](https://github.com/KonomiYuzu01/Magic-600-Cell/releases/tag/0.4). A source build is separate from the accepted binary and requires its own verification.

## Source layout

The 0.4 workspace lives under `work/experiments/magic600-04/`. The original directory name is retained because imports, build receipts and manifests use it. Root Python modules, `native/` and `assets/` supply its shared backend, retained renderer and full mechanical model.

| Path | Purpose |
| --- | --- |
| `work/experiments/magic600-04/native_launch.py` | Native source build and G1/G2 development launch. |
| `work/experiments/magic600-04/engine.py` | 0.4 engine entry point. |
| `work/experiments/magic600-04/native/` | Native workspace controls. |
| `work/experiments/magic600-04/packaging/assemble.py` | 0.4 portable package assembly. |
| `work/experiments/magic600-04/packaging/requirements-build.txt` | Pinned packaging tools. |
| `work/experiments/magic600-04/packaging/check_package.py` | Isolated package checks. |
| Root `packaging/`, launch scripts and `web/` | Retained earlier paths, not the 0.4 packaging workflow. |

[SOURCE_MANIFEST.json](../SOURCE_MANIFEST.json) inventories the public runtime source. [Release provenance](RELEASE_0_4_PROVENANCE.json) records the frozen package inputs. Do not rewrite those hashes to make later edits appear part of the accepted artifact.

## Prerequisites and public-checkout limitation

Source builds require Windows, 64-bit CPython, NumPy, the .NET Framework 4.x compiler targeting x86, and the supported [Managed DirectX assemblies](../DIRECTX.md). The frozen package includes its Python runtime and compiled native host. Microsoft Managed DirectX remains external.

**The current public checkout is not a self-contained native build kit.** `native_launch.py` includes `tests/run_postapproval.py` in its hashed harness inputs, but that file is absent from the published `work/experiments/magic600-04/tests/` directory. The checked-in source launch/build therefore cannot be presented as a verified clean-checkout recipe. This documentation-only update does not supply a replacement fixture or change the build code.

The recorded development entry point is:

```powershell
python work/experiments/magic600-04/native_launch.py --mode g2 --session development
```

It requires the complete matching build inputs, not just the published subset. `--build-only` selects compilation without starting the application. A matching native build receipt is required by `packaging/assemble.py`; use fresh output and work directories and the pinned toolchain. The public package-time [README](../work/experiments/magic600-04/packaging/README.md) retains its original candidate-stage wording, as explained in the [release guide](RELEASE_0_4.md).

## Verification boundaries

Shared backend regressions are separate from 0.4 native acceptance. With the source dependencies installed, their entry points include:

```powershell
python tests/test_core.py
python tests/test_reference_maps.py
python tests/test_crash.py
python tests/test_engine_lifecycle.py
```

Use fresh disposable data for all checks. Never test destructive operations against a personal profile. Actual Windows/DirectX behavior, keyboard input, long sessions and performance require matching native evidence; headless results cannot establish those claims. No application checks were rerun for this documentation organization.

The checked-in generated assets are sufficient for runtime use. Optional asset regeneration also needs retained external geometry/reference inputs; do not change the immutable model manifest as part of documentation or UI work.

## Publishing changes

Stage only reviewed files. Keep conversation exports, local memory, credentials, personal sessions, logs, screenshots and raw machine diagnostics private. Root `.gitignore` excludes common local artifacts, but tracked files and archive contents still require review. Preserve Andrey Astrelin's credit and all third-party notices.

Historical 0.3 build instructions remain available at [tag 0.3](https://github.com/KonomiYuzu01/Magic-600-Cell/blob/0.3/docs/DEVELOPMENT.md). They must not be relabelled as a 0.4 build.
