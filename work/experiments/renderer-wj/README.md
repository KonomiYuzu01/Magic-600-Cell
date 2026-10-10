# W-J Direct3D 12 probe (E-2.4-0J)

The pose probe builds and its source/fixture checks run headlessly. **CPU lattice agreement passes with the SPEC section 3 area-centroid shrink anchors and unchanged assets.** This is source/fixture evidence, not a GPU or performance result. Do not mark W-J accepted or the Jumble switch invisible on the strength of `check_wj.py`.

This directory starts from the S-B C++20/HLSL probe. Only this copy changes. It draws all 600 instances of the 30,480-vertex mesh, all 259,800 labelled stickers and all 177,120 pieces. A piece index selects four row-major matrix rows. Home cell/sticker shrink precedes the pose; the certified moving pieces then receive the swept rotation; S-B projection and colour follow. The renderer consumes J1's data and decides no legality.

The implementation plan is one matrix-table method, whole-state uploads, an independent stdlib reference, exact first-use readbacks and owner-run measurements. No second transform method or optimisation is justified before those measurements. The packet assigns integration and review of this Codex-authored candidate to Claude; this sandbox invocation makes no nested model calls, network calls or Git mutations.

## Build and prepare

Use the pinned engine Python (CPython 3.14.7, NumPy 2.3.5), MSVC x64, Windows SDK 10.0.26100.0/DXC, CMake/Ninja from the renderer-spike environment. No dependency download or installation is part of these commands. Run from the repository root:

```powershell
$env:PATH = (Resolve-Path tools/.venv/engine/Scripts).Path + ';' + $env:PATH
work/experiments/renderer-wj/build.cmd
$wjData = Join-Path (Resolve-Path work/loop-memory).Path 'wj-fixtures'
python work/experiments/renderer-wj/prepare_wj.py --out $wjData
work/experiments/renderer-wj/build/wj_probe.exe --selftest --scene wj --menu S4 --data $wjData
python work/experiments/renderer-wj/check_wj.py
```

`build.cmd [build-directory]` builds `wj_probe.exe` and seven DXIL blobs. Its build identity hashes the executable, then `draw_vs`, `draw_ps`, `count_vs`, `geometry_cs`, `lattice_cs`, `overlay_vs`, `overlay_ps` in that order. For the restricted Windows token, use the existing serial Ninja-command workaround:

```powershell
$env:M600_WJ_SERIAL_NINJA = '1'
work/experiments/renderer-wj/build.cmd
```

`prepare_wj.py` exports all five stages in one J1 journal replay per fixture. It also writes the certified sorted moving set, the motion document and solved/retained lattice controls with J1's labels and destination map. A new menu, geometry or contract requires new fixtures and re-acceptance, not edits to their arrays. Exports and captures are private synthetic evidence, outside this source directory.

The no-GPU `check_wj.py` runs the shared renderer asset check, checks pole length and all 600 frames (including float32 pole transports within 1e-8), and captures each fixture stage before the next journal record applies. One comparison checks the exact `State.digest()`, all three array hashes, shapes/types and the off-lattice count. Without replaying again, it rejects five copies with a changed stage digest and one with a changed array hash. Sampled exact centroids and the sweep plane/moving set are also checked. All exports and all recomputed reference files go into a fresh plain-mkdir temporary directory outside the repository, deleted in `finally`, including on failure. Imports disable bytecode writes. `--menu S4` is the focused development check; the default checks all three menus.

`reference_wj.py` uses only the standard library and the immutable S-B reference's math helpers. It consumes checked exports; its output includes the six states `start`, `mid`, `end`, `sweep0`, `sweep05`, `sweep1`, at cameras c0/c1/c2. Samples include every 4,099th global vertex, one vertex per moving sticker and one per sticker of a sampled fixture piece. Each `ref_<menu>_index.json` fixes asset/fixture/camera digests, all five stage array descriptions, sample order, output hashes and the 1e-4 + 1e-4*abs(reference) GPU tolerance. The anchors come from `sticker_anchors`: the area-weighted triangle centroid in SPEC section 3, computed in float64 in file order and rounded once to float32. Each index records their rule and little-endian SHA-256 `0b6ead284d3f62719e3e6ec893d6c199d49698d246a59164bc362446e39faca6`; `mesh_centers.f32` is no longer an input. Regeneration is explicit:

```powershell
python work/experiments/renderer-wj/reference_wj.py --exports $wjData --menu S4 --out <new-reference-directory>
```

## Geometry and lattice agreement

After building, run these untimed checks on the owner's GPU. Geometry output is refused if any reference is absent or has the wrong digest. The draw-counter check requires 30,480 vertex-shader invocations for each of the 600 cells.

```powershell
foreach ($menu in 'S4','I_a','I_b') {
    work/experiments/renderer-wj/build/wj_probe.exe --scene wj --menu $menu --data $wjData --geometry-check --out "work/loop-memory/perf/renderer/wj/geometry-$menu"
}
work/experiments/renderer-wj/build/wj_probe.exe --scene w3 --geometry-check --out work/loop-memory/perf/renderer/wj/w3-geometry
work/experiments/renderer-wj/build/wj_probe.exe --scene wj --menu S4 --data $wjData --lattice-check --out work/loop-memory/perf/renderer/wj/lattice
python work/experiments/renderer-wj/check_lattice.py
```

The separate CPU lattice check exits 0. For both the solved control and J1's retained generator 1, all 259,800 projected labels and transported frames agree, and shrink centres and transformed shrunken sticker bounds agree within 2e-6 world units. Bounds are computed from every vertex of each moved sticker, after the exact required home shrink. Equal geometry must have equal bounds, irrespective of triangle order or different interior tessellation. The bounds comparison therefore does not rely on matching triangulation vertices.

The GPU lattice check reads J1's label/destination controls and compares every home sticker against its destination: exact identity/label, shrink centre, all-vertex bounds for moved stickers and lattice flag. It writes `lattice_check.json` and returns 1 on any mismatch. The GPU lattice check has not been run with the new anchors; no GPU result is claimed here. Identity stickers have identical vertex streams. Bounds agreement is a necessary condition, not a proof of complete surface agreement.

`check_wj.py` is deliberately the packet's asset/replay/reference acceptance command; it does not silently weaken or declare acceptance item 2 passed. `check_lattice.py` checks item 2 separately and must pass both controls with bound and centre errors at most 2e-6. Before any comparison it refuses an asset whose SHA-256 differs from `assets/manifest.json` and anchors whose SHA-256 differs from the SPEC pin. The probe's asset loading refuses such anchors too, so no run uploads others. The owner chose the SPEC anchor rule on 10 October 2026; the assets and model identity are unchanged. The passing CPU check closes item 2 at source level; an invisible lattice handoff still needs the GPU check.

## Timed trace and exact checks

See [PROTOCOL.md](PROTOCOL.md) for fields and rejection rules. Two fenced upload contexts feed three default-heap bundles. On each adoption the whole pose index, the whole logical matrix table and the whole lattice table are written together. All three resources bind from the same bundle. The table allocation covers the largest fixture stage; unused capacity is zero padded, and its logical entry count travels with the binding. Colours use immutable home sticker labels in W-J.

W-J alternates the fixture's swept twist and its inverse, 190 ms each, with smoothstep easing and W2's 0.002-radian camera step every frame. Even pose revisions use `sweep-before`; odd ones use `sweep-after`. Revision zero is also checked. Immediately after every revision's first draws, in the same command list, `copyBound()` copies the label buffer and the three pose resources selected by those draws. Each first-use copy is preserved in a disjoint readback slot for the whole capture. No mapping, hashing or array comparison occurs during the capture. After the queue drains, every logical array is compared by SHA-256 and by every 32-bit element (float32 bits included). Binding, upload, first-use and copy frames are also checked. Stale data cannot be rescued by matching colour or by a label-only check.

The preserved host readbacks use roughly 1.8–2 GiB for a 192 s W-J run, depending on the menu. Allocation happens before the trace; failure starts no timing run. These are readback-heap bytes, not reported as local VRAM. Peak local VRAM is sampled through `QueryVideoMemoryInfo` as in S-B. `run.json` has no host paths or fixture export directory.

## Owner's measurements

The gate accepts `scene: wj` since the separately reviewed `renderer-wj-packets/judge-wj.md` change (commit `7e68b50`); a gate without it must reject these runs. Do not relabel captures as W3 or change thresholds. Preserve the exact built executable/shader digests with every run.

`run_scene.ps1` copies S-B's PresentMon 2.6 capture/stop/visibility procedure, requires an administrator PowerShell for PresentMon, creates fresh private run directories and uses the same capture mutex to exclude concurrent S-B captures. It launches a new probe process per cold run, waits 20 s between runs, stops the named PresentMon session, confirms the CSV is complete, fills the swap-chain identity and asks for the existing post-run attendance declaration. The visible probe uses `-NoNewWindow`; helpers stay hidden.

On mains power in high-performance discrete GPU mode, with native resolution, no frame generation/upscaling, default V-Sync off, attend every full-screen run:

```powershell
foreach ($menu in 'S4','I_a','I_b') {
    work/experiments/renderer-wj/run_scene.ps1 -Scene wj -Menu $menu -Data $wjData -Runs 3 -Overlays 'none running'
}
work/experiments/renderer-wj/run_scene.ps1 -Scene w3 -Runs 3 -Overlays 'none running'
```

The W3 rerun is required: this build changes copied shaders/bindings/uploads shared by its label and pose paths. Earlier S-B W3 measurements remain valid only for their original build. For each fixture keep `run.json`, `pose_check.json`, `trace.jsonl`, PresentMon CSV and the judge's summary. Every run must pass exact label and pose checks and the unchanged 30 fps / 33.3 ms p99 gate, including the pooled frames. Above 7 GiB local VRAM is a recorded risk under the unchanged rule.

Run each fault in turn 20 during an attended 10 s direct probe run; each must produce `pose_check.status: fail` and exit 2. These direct runs are correctness evidence, not gate captures:

An exit code alone does not show that the fault was caught: the probe also exits 2 when turn 20 is never seen (for example when one long frame spans its start, so the injection is not reached) and when the clean path fails. So first run the same direct probe without a fault and require exit 0, then require for each fault that the injection was applied and that the failure is the one the fault causes:

```powershell
$probe = 'work/experiments/renderer-wj/build/wj_probe.exe'
$negatives = 'work/loop-memory/perf/renderer/wj'
& $probe --scene wj --menu S4 --data $wjData --duration 10 --out "$negatives/negative-clean"
if ($LASTEXITCODE -ne 0) { throw 'The clean control must pass before the fault runs' }
$at20 = { param($pose, $array) @($pose.checks | Where-Object { $_.revision -eq 20 -and $_.arrays[$array].element_mismatches -gt 0 }).Count -gt 0 }
$others = { param($pose) @($pose.checks | Where-Object { $_.revision -ne 20 -and $_.status -ne 'pass' }).Count -eq 0 }
$expected = @{
    'corrupt-index'    = { param($pose) (& $at20 $pose 0) -and (& $others $pose) }
    'swap-same-colour' = { param($pose) (& $at20 $pose 0) -and (& $others $pose) }
    'stale-pose'       = { param($pose) (& $at20 $pose 1) -and -not (& $at20 $pose 0) -and (& $others $pose) }
    'delay-adoption'   = { param($pose) $pose.late_adoptions -gt 0 -and $pose.mismatches -eq 0 }
    'stale-binding'    = { param($pose) $pose.binding_mismatches -gt 0 -and $pose.mismatches -eq 0 }
}
foreach ($fault in 'corrupt-index','swap-same-colour','stale-pose','delay-adoption','stale-binding') {
    $out = "$negatives/negative-$fault"
    & $probe --scene wj --menu S4 --data $wjData --duration 10 --inject $fault --out $out
    if ($LASTEXITCODE -ne 2) { throw "Expected exit 2 for $fault" }
    $run = Get-Content "$out/run.json" -Raw | ConvertFrom-Json
    $pose = Get-Content "$out/pose_check.json" -Raw | ConvertFrom-Json
    if (-not $run.injection_applied -or $pose.injection_not_reached) { throw "$fault was not injected; repeat the run in a new folder" }
    if (-not (& $expected[$fault] $pose)) { throw "$fault failed, but not with the failure it injects" }
}
```

The two binding faults are caught by the adoption and binding records, not by the GPU content comparison: the frame drawn with the old bundle is compared, if at all, with revision 19, which it matches.

The five faults change one used pose index, swap distinct pose indices of two monochrome pieces of the same home-facet colour, substitute a previous revision's matrix at a correct current pose index, delay adoption by one drawn frame, and bind the prior bundle for exactly the first captured frame of turn 20. The last fault is absent during preroll and all untimed checks. An injection not reached is a failure. The CPU self-test exercises these same comparison/injection functions and shows that the stale matrix alone passes a label-only check; it does not replace the GPU negatives.

Overlays are provisional experiment drawings: green/red fixture-end grip poles, purple/yellow certificate points, yellow straddling pieces with both points, and a swept-angle gauge. Grip status, straddle identities and the two world points come only from the fixture's end survey; points are divided by R, never taken from sticker meshes or re-posed. The survey is explicitly the fixed **end** snapshot during the alternating sweep workload; it is not a live claim of grip admissibility at other revisions. The angle gauge uses the fixture's plane/angle and the current preview angle. No final UI, pointer handling or design choice is introduced.

Measure each overlay alone, with the same build/menu/settings and the baseline immediately adjacent; report mean-frame-time and p99 differences from actual PresentMon captures. Frame-time differences are run-level measurements, not matched-frame comparisons: camera rotation remains per frame as the packet requires. Keep that distinction in the cost table. For every fixture, within the time box:

```powershell
foreach ($overlay in 'grips','certificates','straddles','angle') {
    work/experiments/renderer-wj/run_scene.ps1 -Scene wj -Menu S4 -Data $wjData -Overlay $overlay -Runs 3 -Overlays 'none running'
}
```

Repeat with I_a/I_b as time permits. Write unmeasured for every missing cost; estimate none. Fill [RESULT.md](RESULT.md) only with the exact matching build's verified GPU/capture evidence. Preserve Andrey Astrelin's primary MPUlt credit and upstream notices in the repository; this probe bundles no Managed DirectX DLLs.
