# E-2.4-03 S-D result card

This is the experiment card ([protocol section 2](../../../docs/progress/1.0/stage-2-experiment-protocol.md)) for packet [E-2.4-03](../../../docs/progress/1.0/packets/renderer/E-2.4-03-sd-qt.md). It is filled in as results arrive: an item that has not run is listed as open, never estimated. Raw logs and run records stay private; only sanitised summaries are published.

| Field | Content |
|---|---|
| ID | E-2.4-03 |
| Question | Can a Qt Quick application host the S-B drawing method in a `QQuickRhiItem` on Qt's QRhi Direct3D 12 backend inside the selection gate, and what does it constrain (H-09)? |
| Decision it feeds | The day-7 go/no-go (E-2.4-04), the selection (E-2.4-05), and PR #43 section 7 (the framework paper's interop questions). |
| Hypothesis | Level 1: a texture written by our own D3D12 code on the device and queue Qt Quick submits to displays correctly when it is handed over in the state Qt's tracker holds, on Qt's own device (route A), on a QRhi created with our device and queue and adopted through `QQuickGraphicsDevice::fromRhi` (route B), and with our device only through `fromDeviceAndContext` (route C). |
| Method | Level 1 smoke test: the SA2 producer DLL ([../renderer-sa2/native/](../renderer-sa2/native/README.md)), loaded by a C++ Qt Quick application (packet [SD-Q](../renderer-sd-packets/SD-Q-qt.md)), run matrix Q0 to Q15 in [README.md](README.md). |
| Qt version | 6.10.3 (`msvc2022_64`), the newest release whose repository layout the pinned aqtinstall 3.3.0 reads. PR #43 asks about Qt 6.12; nothing here holds for another Qt version. |
| Time box | Window days 3–6 (stage days 5–8), shared with E-2.4-02. |
| Kill criteria | Level 1 not passed by the end of window day 6. |
| Evidence class | Source/fixture until the owner-machine runs; then actual Windows/DirectX for the stated source and build. No performance evidence. |
| Result | Level 1 in progress. Qt 6.10.3 is installed through the reviewed installer and the harness builds against it. A first run stopped at Q0 because Windows Smart App Control refused the freshly built, unsigned self-test executable. Private shakedown runs then found one harness defect, fixed before the record run: Qt allocates render targets without zeroing, and the harness read new ones before writing them (SDQ-R-01). |
| Decision | Open. |

## Qt smoke test (level 1)

The record run has not been made yet. The answers (device and queue identity per route, where our `Signal` and `Wait` go on Qt's queue, the handover state Qt expects, resize, `releaseResources()` and teardown, the sequence numbers shown, and the deployed package size) go here and to PR #43 section 7.

## Acceptance items (packet section 1)

| # | Item | Status |
|---|---|---|
| 1 | Interop smoke test | In progress: harness written, reviewed and built against Qt 6.10.3; the record run is not done. |
| 2 | Geometry port | Not started (needs item 1). |
| 3 | Three cold W3 runs | Not started. |
| 4 | Layout specification and feature list | Not started. |
| 5 | Renderer constraints (H-09) | Not started. |

## Not claimed

- The source checks are not a Qt result: device adoption, Qt's state tracking, display, resize, teardown and device loss are untested until the smoke test runs.
- Nothing here holds for another Qt version, build, driver or machine. There are no performance claims.
