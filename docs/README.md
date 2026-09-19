# Magic 600 Cell 0.4 documentation

The current release is **0.4**, published on 19 September 2026. Begin with the release and usage documents below. Older records keep their original dates, results and version labels; they do not supersede the release record.

## Current release

| Document | Use |
| --- | --- |
| [Usage](../USAGE.md) | Launch, solving workflow, keyboard, sessions and rollback. |
| [Release guide](RELEASE_0_4.md) | Delivered behavior and verification scope. |
| [Release provenance](RELEASE_0_4_PROVENANCE.json) | Frozen source, native build and archive hashes. |
| [Build and source guide](DEVELOPMENT.md) | 0.4 source locations and public build limitations. |
| [Dependencies](../DEPENDENCIES.md), [DirectX](../DIRECTX.md) | Required runtime components. |
| [Limitations and next work](LIMITATIONS_AND_ROADMAP.md) | Current limits and the planned 0.41 iteration. |
| [Research index](../research/README.md) | Full-model mathematics and versioned engineering reports. |

## Historical records supporting 0.4

| Record | Scope |
| --- | --- |
| [0.4 development snapshot](progress/0.4/README.md) | Archived 17 September state, published before release; includes failed and incomplete checks. |
| [0.3 validation](RELEASE_0_3_VALIDATION.md) | The frozen 0.3 executable only. |
| [Development log](DEVELOPMENT_LOG.md) | Earlier shared-runtime and 0.3 implementation decisions. |
| [Performance observations](DEVELOPMENT_PERFORMANCE.md) | Historical 0.3 samples; not a 0.4 benchmark. |
| [Structure explorer](STRUCTURE_EXPLORER.md) | Earlier shared-model notation and 0.3 interface reference. |
| [Native update protocol](NATIVE_UPDATE_PROTOCOL.md), [runtime provenance](RUNTIME_PROVENANCE.md) | Retained protocol and upstream-runtime background. |
| [Legacy packaging](../packaging/README.md) | 0.3 package instructions, retained with the original scripts. |

The [0.3](https://github.com/KonomiYuzu01/Magic-600-Cell/releases/tag/0.3) and [0.2.4](https://github.com/KonomiYuzu01/Magic-600-Cell/releases/tag/0.2.4) releases retain their original downloads and identities. Package-time candidate labels also remain part of the unchanged 0.4 artifact's provenance; the release guide records its subsequent publication.

## Future work

**0.41** is planned to optimize 0.4, address debugging and performance, and establish a dedicated local development workbench. No implementation is implied by this documentation update.

The [1.0 architecture](architecture/1.0/README.md) is a separate future proposal. Its PDFs and source package are not 0.4 runtime requirements or delivered 0.4 features.

## Public-file boundary

Publish application source, reviewed documentation, mathematical assets and anonymized summaries. Keep conversation/JSONL exports, shared personal memory, credentials, personal databases and logs, private paths, screenshots and raw diagnostic captures outside Git. Ignore rules reduce accidental additions but do not sanitize files already tracked or Git history. Retain upstream credits and licenses.
