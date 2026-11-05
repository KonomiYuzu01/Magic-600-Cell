# Named-device compute comparison

Status before an actual capture: **not run here**. Decisions remain **open** for the
owner. `main.cpp` creates a D3D12 compute queue on the exact named hardware adapter,
refuses software/fallback adapters, uploads the frozen inputs, dispatches all four
stages, waits at most 30 seconds and reads every result back. There is no window,
swap chain, PresentMon trace or timing result.

From the repository root, outside the sandbox:

```text
python work/experiments/language-lx/lx7-charts/check.py
python work/experiments/language-lx/lx7-charts/device/run.py --adapter "NVIDIA GeForce RTX 4070 Laptop GPU"
python work/experiments/language-lx/lx7-charts/device/compare_device.py work/experiments/language-lx/lx7-charts/device/device-capture.json
```

`run.py` verifies the freeze, invokes `device/build.cmd`, creates fresh data inside
`build/`, records the requested and actual device and the input, HLSL, DXIL, native
executable and output SHA-256 values, validates the comparison, then writes
`device/device-capture.json` and `device/device-summary.json`. Both are small public
records; raw result words are retained in JSON, with no host paths. `build/` and
all binaries are deleted in `finally`, including on build/device/comparison failure.
It refuses an existing build directory to avoid overlapping runs. Do not run checks
and device work concurrently. Direct `build.cmd` is provided for inspection; the
Python runner owns cleanup.

For a sandbox trial only, append `--sandbox-lead`; its separate
`sandbox-lead-capture.json` and `sandbox-lead-summary.json` cannot replace the
integrator's run. No successful process exit alone establishes a named-device
result: metadata, raw readback and binding validation must all succeed.

Wire formats are little endian. Each input is 96 bytes: vertex float4, four row-major
camera float4s, float d, uint stage, two zero uints. Stage IDs 0..3 are built-in
perspective, generated perspective, stereographic and gnomonic. Each output is 16
bytes: uint coverage then three binary32 coordinate bit patterns (zero placeholders
for missing coverage). Order is stage, pose, vertex, as `reference.cases` defines.
Output buffers start at 0xffffffff to expose missing writes. Count is a root
constant; 64 threads per group, last-group bounds check, explicit resource
transitions, a queue fence, then readback. No app state enters the harness.

The comparator reports min/p50/p95/p99/max per coordinate, using nearest-rank
quantiles. Absolute error is in projected units. Relative error omits exact zero
and is dimensionless. ULP distance compares binary32 ordering to the CPU, collapsing
signed zero. Counts and full IDs of CPU/exact coverage disagreements are retained.
Nonfinite results never enter distributions. A malformed capture is refused;
numerical disagreement is reported without a precision pass/fail. RPI.36's metric,
exclusions and triangle-crossing handling still need agreement before judging LX-7.

The binary32 request in this candidate uses shader model 6.2 and preserved denormals.
Compiler flags and versions are captured. It uses no vendor shader feature, no
third-party library, no installation and no external service. Device names alone
do not prove provenance: retain the exact source, manifest and capture command
together. Changed sources cannot inherit an older capture.
