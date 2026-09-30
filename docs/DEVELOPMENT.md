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

## Exact checkout bytes

Model assets and build inputs are verified by the SHA-256 of their raw bytes, so `.gitattributes` turns off line-ending conversion for every file. A checkout then holds the committed bytes whatever the `core.autocrlf` setting. A Windows working tree checked out before this rule may still hold CRLF copies of LF files, which fail the model asset check. Before committing anything in such a tree, run:

```powershell
python tools/checkout_bytes.py --fix
```

It rewrites, from the index, only LF text files whose bytes are exactly the CRLF conversion of their committed bytes, and refreshes files Git still lists as modified although their bytes match. It never stages or deletes anything: each original is moved into a backup directory under the Git directory, whose location it prints; delete that directory once the checkout looks right. It refuses while a merge is unresolved or while Git would still convert line endings, and reports every other difference (edits, binary files, links, executable-bit changes) without touching it. Without `--fix` it only reports. Never use `git add --renormalize` or stage a whole converted tree: that would commit the CRLF copies.

## Building from a clean checkout

Source builds require Windows, the .NET Framework 4.x compiler targeting x86, and the supported [Managed DirectX assemblies](../DIRECTX.md), which remain external. The frozen 0.4 package includes its own Python runtime and compiled native host.

1. Clone the repository. `.gitattributes` keeps every file byte-exact, whatever `core.autocrlf` is.
2. Install 64-bit CPython 3.14.7. Then create the pinned engine environment (CPython 3.14.7 with hash-pinned NumPy 2.3.5 in `tools/.venv/engine`):

   ```powershell
   python tools/toolchain/bootstrap.py install engine-python
   ```

   The installer never downloads an interpreter or uses a uv-managed one, and refuses any other interpreter. Run every command below with `tools\.venv\engine\Scripts\python.exe`.
3. Build the native host without starting it:

   ```powershell
   tools\.venv\engine\Scripts\python.exe work/experiments/magic600-04/native_launch.py --mode g2 --build-only
   ```

   The build directory `native-build/<identity>/` holds `build.json`, a version 2 receipt. Its build identity covers:
   - the product sources, model and retained runtime;
   - the sealed compiler recipe;
   - the compiler files and reference assemblies;
   - the interpreter and NumPy.

   The checkout location, the Windows build and test-only files do not change it. The legacy compiler is not deterministic, so the executable itself is not bit-reproducible, and a build is reused only from a valid receipt for the same identity whose recorded executable hash still matches. Details: [build identity](wiki/concepts/build-identity-v2.md).
4. Compile the native regression harness and bind its inputs, without starting the engine or a window:

   ```powershell
   tools\.venv\engine\Scripts\python.exe work/experiments/magic600-04/tests/run_postapproval.py --compile-only
   ```

   A real harness run drives the desktop, so run it only on purpose. Desktop frames are captured only with `--recorder <ffmpeg.exe>`. Without it, each frame is logged as `SKIP`, and no recorder is part of the repository.
5. Run the WinForms startup regression with a fresh data directory:

   ```powershell
   tools\.venv\engine\Scripts\python.exe native/bootstrap.py --self-test-only --data <short fresh directory>
   ```

   Keep `--data` paths short: the legacy compiler fails with CS1619 when an output path is close to 260 characters.

`python work/experiments/magic600-04/native_launch.py --mode g2 --session development` starts a development session. `packaging/assemble.py` requires a matching version 2 receipt. Its identity must rehash and bind every input that the code declares, and its product files must match the current bytes. The public package-time [README](../work/experiments/magic600-04/packaging/README.md) retains its original candidate-stage wording, as explained in the [release guide](RELEASE_0_4.md).

[RELEASE_0_4_CONTINUITY.json](RELEASE_0_4_CONTINUITY.json) maps the 0.4 release build receipt onto this tree (`tools/provenance/continuity_04.py`). The stored identity rehashes, and 88 of its 96 inputs are byte-identical here. The other 8 are listed 0.4.1 adaptations (the step 1 build identity and the screening fixes) whose release bytes remain in Git history. The 0.4 build itself cannot be recreated on later machines, because its toolchain (CPython 3.12.14, csc 4.8.9232.0) differs.

## Verification boundaries

Shared backend regressions are separate from 0.4 native acceptance. With the source dependencies installed, their entry points include:

```powershell
python tests/test_core.py
python tests/test_reference_maps.py
python tests/test_crash.py
python tests/test_engine_lifecycle.py
```

These write `tests/core_report.json`, `tests/reference_map_report.json` and `tests/crash_report.json`, which Git ignores. Some other scripts under `tests/` are harness tools, not standalone checks, and need an argument:

- `python tests/test_http_boundary.py "<engine URL with its token>"` needs a running engine.
- `python tests/test_native_auxiliary_controls.py --output <fresh directory>` opens test windows on the desktop.
- `python tests/test_portable_package.py <bundle> <output>` needs a built package.

Use fresh disposable data for all checks. Never test destructive operations against a personal profile. Actual Windows/DirectX behavior, keyboard input, long sessions and performance require matching native evidence; headless results cannot establish those claims. No application checks were rerun for this documentation organization.

The checked-in generated assets are sufficient for runtime use. Optional asset regeneration also needs retained external geometry/reference inputs; do not change the immutable model manifest as part of documentation or UI work.

## Publishing changes

Stage only reviewed files. Keep conversation exports, local memory, credentials, personal sessions, logs, screenshots and raw machine diagnostics private. Root `.gitignore` excludes common local artifacts, but tracked files and archive contents still require review. Preserve Andrey Astrelin's credit and all third-party notices.

Historical 0.3 build instructions remain available at [tag 0.3](https://github.com/KonomiYuzu01/Magic-600-Cell/blob/0.3/docs/DEVELOPMENT.md). They must not be relabelled as a 0.4 build.
