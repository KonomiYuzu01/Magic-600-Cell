# LX-1 sandbox time series

Decision **open**; decider **owner**. Timing is a weak lead only.

Raw ordered nanosecond samples (smoke, then three series of 100) are in `timings.json`.
Raw file SHA-256: `2e3ca7c51117fc5fe3e8ff14473f5ba398a61923102c3f7dcf8b4dc588071f3c`.
Regenerate: `python -B work/experiments/language-lx/lx1-cost/measure.py --record`.

Timer: `time.perf_counter_ns`. Python is in-process within a fresh worker per candidate/mode.
Each program gets a fresh synthetic host. RT2 Node reuses one child and creates a fresh VM context
per program; RT3 starts a fresh Node child per program. One workload run executes its entire
ordered list. Source reading, parsing/approval/compilation, execution and bridge work are timed;
worker startup and final payload hashing/validation are excluded. Counter hooks/opcode tracing
are disabled. Original logical metering and provenance overhead remain in every mode.

Cells run sequentially on a machine shared with other work. D6 power source, Windows power mode,
vendor mode, V-Sync and overlays are not controlled or recorded here. No GPU is used. Neither
these series nor their smoke runs are decisive B4-12 performance evidence.

Each entry below is **median / p99, microseconds**. Median is the mean of samples 50 and 51;
p99 is nearest rank, sample 99 of 100 sorted samples. Per-step and per-kernel-call entries divide
each whole-workload elapsed sample by its repeated count: amortized workload cost, not isolated
instruction or kernel latency. No cross-route step ordering or total is meaningful.

## rt1-own

| Workload | Mode | Series | Executed form | Elapsed | Per step | Per kernel call | Per coarse unit |
| --- | --- | --- | --- | --- | --- | --- | --- |
| full | labels_off | 1 | typed IR instructions dispatched by rt1_vm.Machine | 47522.500 / 52450.100 | 1159.085 / 1279.271 | 3394.464 / 3746.436 | not applicable |
| full | labels_off | 2 | typed IR instructions dispatched by rt1_vm.Machine | 48147.750 / 54235.500 | 1174.335 / 1322.817 | 3439.125 / 3873.964 | not applicable |
| full | labels_off | 3 | typed IR instructions dispatched by rt1_vm.Machine | 47905.850 / 50889.400 | 1168.435 / 1241.205 | 3421.846 / 3634.957 | not applicable |
| jumbling | labels_off | 1 | typed IR instructions dispatched by rt1_vm.Machine | 14472.750 / 16925.000 | 904.547 / 1057.812 | 3618.188 / 4231.250 | not applicable |
| jumbling | labels_off | 2 | typed IR instructions dispatched by rt1_vm.Machine | 14335.700 / 16055.100 | 895.981 / 1003.444 | 3583.925 / 4013.775 | not applicable |
| jumbling | labels_off | 3 | typed IR instructions dispatched by rt1_vm.Machine | 14419.400 / 16131.900 | 901.212 / 1008.244 | 3604.850 / 4032.975 | not applicable |
| representations | labels_off | 1 | typed IR instructions dispatched by rt1_vm.Machine | 30749.050 / 35063.900 | 1138.854 / 1298.663 | 5124.842 / 5843.983 | not applicable |
| representations | labels_off | 2 | typed IR instructions dispatched by rt1_vm.Machine | 30822.500 / 35707.900 | 1141.574 / 1322.515 | 5137.083 / 5951.317 | not applicable |
| representations | labels_off | 3 | typed IR instructions dispatched by rt1_vm.Machine | 30937.900 / 35304.600 | 1145.848 / 1307.578 | 5156.317 / 5884.100 | not applicable |
| full | static_only | 1 | typed IR instructions dispatched by rt1_vm.Machine | 47781.500 / 49678.000 | 1165.402 / 1211.659 | 3412.964 / 3548.429 | not applicable |
| full | static_only | 2 | typed IR instructions dispatched by rt1_vm.Machine | 48347.600 / 53386.200 | 1179.210 / 1302.102 | 3453.400 / 3813.300 | not applicable |
| full | static_only | 3 | typed IR instructions dispatched by rt1_vm.Machine | 48387.850 / 50737.200 | 1180.191 / 1237.493 | 3456.275 / 3624.086 | not applicable |
| jumbling | static_only | 1 | typed IR instructions dispatched by rt1_vm.Machine | 14576.700 / 16096.400 | 911.044 / 1006.025 | 3644.175 / 4024.100 | not applicable |
| jumbling | static_only | 2 | typed IR instructions dispatched by rt1_vm.Machine | 14533.800 / 15328.800 | 908.362 / 958.050 | 3633.450 / 3832.200 | not applicable |
| jumbling | static_only | 3 | typed IR instructions dispatched by rt1_vm.Machine | 14552.000 / 16290.300 | 909.500 / 1018.144 | 3638.000 / 4072.575 | not applicable |
| representations | static_only | 1 | typed IR instructions dispatched by rt1_vm.Machine | 30984.350 / 35279.300 | 1147.569 / 1306.641 | 5164.058 / 5879.883 | not applicable |
| representations | static_only | 2 | typed IR instructions dispatched by rt1_vm.Machine | 31020.450 / 33489.100 | 1148.906 / 1240.337 | 5170.075 / 5581.517 | not applicable |
| representations | static_only | 3 | typed IR instructions dispatched by rt1_vm.Machine | 31091.600 / 33645.500 | 1151.541 / 1246.130 | 5181.933 / 5607.583 | not applicable |
| full | run_time | 1 | typed IR instructions dispatched by rt1_vm.Machine | 47865.400 / 53799.300 | 1167.449 / 1312.178 | 3418.957 / 3842.807 | not applicable |
| full | run_time | 2 | typed IR instructions dispatched by rt1_vm.Machine | 48614.400 / 52000.800 | 1185.717 / 1268.312 | 3472.457 / 3714.343 | not applicable |
| full | run_time | 3 | typed IR instructions dispatched by rt1_vm.Machine | 49154.700 / 56952.900 | 1198.895 / 1389.095 | 3511.050 / 4068.064 | not applicable |
| jumbling | run_time | 1 | typed IR instructions dispatched by rt1_vm.Machine | 14837.950 / 17017.200 | 927.372 / 1063.575 | 3709.488 / 4254.300 | not applicable |
| jumbling | run_time | 2 | typed IR instructions dispatched by rt1_vm.Machine | 14845.250 / 16604.000 | 927.828 / 1037.750 | 3711.312 / 4151.000 | not applicable |
| jumbling | run_time | 3 | typed IR instructions dispatched by rt1_vm.Machine | 14872.750 / 18548.700 | 929.547 / 1159.294 | 3718.188 / 4637.175 | not applicable |
| representations | run_time | 1 | typed IR instructions dispatched by rt1_vm.Machine | 32047.850 / 35292.300 | 1186.957 / 1307.122 | 5341.308 / 5882.050 | not applicable |
| representations | run_time | 2 | typed IR instructions dispatched by rt1_vm.Machine | 31580.550 / 35126.100 | 1169.650 / 1300.967 | 5263.425 / 5854.350 | not applicable |
| representations | run_time | 3 | typed IR instructions dispatched by rt1_vm.Machine | 31709.100 / 35819.100 | 1174.411 / 1326.633 | 5284.850 / 5969.850 | not applicable |

## rt2-py

| Workload | Mode | Series | Executed form | Elapsed | Per step | Per kernel call | Per coarse unit |
| --- | --- | --- | --- | --- | --- | --- | --- |
| full | labels_off | 1 | CPython 3.15.1 opcode events in rendered code objects | 63813.700 / 69579.000 | 116.448 / 126.969 | 4558.121 / 4969.929 | not applicable |
| full | labels_off | 2 | CPython 3.15.1 opcode events in rendered code objects | 64731.150 / 70058.500 | 118.123 / 127.844 | 4623.654 / 5004.179 | not applicable |
| full | labels_off | 3 | CPython 3.15.1 opcode events in rendered code objects | 64331.450 / 72128.000 | 117.393 / 131.620 | 4595.104 / 5152.000 | not applicable |
| jumbling | labels_off | 1 | CPython 3.15.1 opcode events in rendered code objects | 21450.950 / 24190.900 | 101.184 / 114.108 | 5362.738 / 6047.725 | not applicable |
| jumbling | labels_off | 2 | CPython 3.15.1 opcode events in rendered code objects | 22011.550 / 28130.500 | 103.828 / 132.691 | 5502.887 / 7032.625 | not applicable |
| jumbling | labels_off | 3 | CPython 3.15.1 opcode events in rendered code objects | 25228.450 / 28528.800 | 119.002 / 134.570 | 6307.113 / 7132.200 | not applicable |
| representations | labels_off | 1 | CPython 3.15.1 opcode events in rendered code objects | 49880.100 / 55176.000 | 131.263 / 145.200 | 8313.350 / 9196.000 | not applicable |
| representations | labels_off | 2 | CPython 3.15.1 opcode events in rendered code objects | 43515.750 / 51598.200 | 114.515 / 135.785 | 7252.625 / 8599.700 | not applicable |
| representations | labels_off | 3 | CPython 3.15.1 opcode events in rendered code objects | 44598.800 / 55643.700 | 117.365 / 146.431 | 7433.133 / 9273.950 | not applicable |
| full | static_only | 1 | CPython 3.15.1 opcode events in rendered code objects | 69865.900 / 79514.200 | 127.493 / 145.099 | 4990.421 / 5679.586 | not applicable |
| full | static_only | 2 | CPython 3.15.1 opcode events in rendered code objects | 65226.300 / 79146.000 | 119.026 / 144.427 | 4659.021 / 5653.286 | not applicable |
| full | static_only | 3 | CPython 3.15.1 opcode events in rendered code objects | 64107.150 / 71649.400 | 116.984 / 130.747 | 4579.082 / 5117.814 | not applicable |
| jumbling | static_only | 1 | CPython 3.15.1 opcode events in rendered code objects | 22045.750 / 25582.900 | 103.989 / 120.674 | 5511.438 / 6395.725 | not applicable |
| jumbling | static_only | 2 | CPython 3.15.1 opcode events in rendered code objects | 26133.450 / 33551.200 | 123.271 / 158.260 | 6533.363 / 8387.800 | not applicable |
| jumbling | static_only | 3 | CPython 3.15.1 opcode events in rendered code objects | 26350.150 / 32976.000 | 124.293 / 155.547 | 6587.538 / 8244.000 | not applicable |
| representations | static_only | 1 | CPython 3.15.1 opcode events in rendered code objects | 45339.500 / 56098.000 | 119.314 / 147.626 | 7556.583 / 9349.667 | not applicable |
| representations | static_only | 2 | CPython 3.15.1 opcode events in rendered code objects | 44672.750 / 57553.700 | 117.560 / 151.457 | 7445.458 / 9592.283 | not applicable |
| representations | static_only | 3 | CPython 3.15.1 opcode events in rendered code objects | 44075.000 / 54265.000 | 115.987 / 142.803 | 7345.833 / 9044.167 | not applicable |
| full | run_time | not_run | CPython 3.15.1 opcode events in rendered code objects | Run-time labels require a modified VM (RTE.12/RTE.24); CPython is unmodified | not_run | not_run | not_run |
| jumbling | run_time | not_run | CPython 3.15.1 opcode events in rendered code objects | Run-time labels require a modified VM (RTE.12/RTE.24); CPython is unmodified | not_run | not_run | not_run |
| representations | run_time | not_run | CPython 3.15.1 opcode events in rendered code objects | Run-time labels require a modified VM (RTE.12/RTE.24); CPython is unmodified | not_run | not_run | not_run |

## rt2-node

| Workload | Mode | Series | Executed form | Elapsed | Per step | Per kernel call | Per coarse unit |
| --- | --- | --- | --- | --- | --- | --- | --- |
| full | labels_off | 1 | JavaScript on unmodified V8; instruction count not available here | 66177.650 / 72212.700 | not available here | 4726.975 / 5158.050 | executed JavaScript domain-helper callbacks: 1034.026 / 1128.323 |
| full | labels_off | 2 | JavaScript on unmodified V8; instruction count not available here | 66965.500 / 71979.800 | not available here | 4783.250 / 5141.414 | executed JavaScript domain-helper callbacks: 1046.336 / 1124.684 |
| full | labels_off | 3 | JavaScript on unmodified V8; instruction count not available here | 66117.500 / 71902.900 | not available here | 4722.679 / 5135.921 | executed JavaScript domain-helper callbacks: 1033.086 / 1123.483 |
| jumbling | labels_off | 1 | JavaScript on unmodified V8; instruction count not available here | 19595.900 / 22085.100 | not available here | 4898.975 / 5521.275 | executed JavaScript domain-helper callbacks: 699.854 / 788.754 |
| jumbling | labels_off | 2 | JavaScript on unmodified V8; instruction count not available here | 19755.850 / 21463.600 | not available here | 4938.962 / 5365.900 | executed JavaScript domain-helper callbacks: 705.566 / 766.557 |
| jumbling | labels_off | 3 | JavaScript on unmodified V8; instruction count not available here | 20034.400 / 23243.100 | not available here | 5008.600 / 5810.775 | executed JavaScript domain-helper callbacks: 715.514 / 830.111 |
| representations | labels_off | 1 | JavaScript on unmodified V8; instruction count not available here | 41943.650 / 57845.800 | not available here | 6990.608 / 9640.967 | executed JavaScript domain-helper callbacks: 953.265 / 1314.677 |
| representations | labels_off | 2 | JavaScript on unmodified V8; instruction count not available here | 41546.650 / 62166.000 | not available here | 6924.442 / 10361.000 | executed JavaScript domain-helper callbacks: 944.242 / 1412.864 |
| representations | labels_off | 3 | JavaScript on unmodified V8; instruction count not available here | 41673.050 / 44901.300 | not available here | 6945.508 / 7483.550 | executed JavaScript domain-helper callbacks: 947.115 / 1020.484 |
| full | static_only | 1 | JavaScript on unmodified V8; instruction count not available here | 66184.250 / 73007.100 | not available here | 4727.446 / 5214.793 | executed JavaScript domain-helper callbacks: 1034.129 / 1140.736 |
| full | static_only | 2 | JavaScript on unmodified V8; instruction count not available here | 67062.000 / 73205.800 | not available here | 4790.143 / 5228.986 | executed JavaScript domain-helper callbacks: 1047.844 / 1143.841 |
| full | static_only | 3 | JavaScript on unmodified V8; instruction count not available here | 67180.750 / 93847.900 | not available here | 4798.625 / 6703.421 | executed JavaScript domain-helper callbacks: 1049.699 / 1466.373 |
| jumbling | static_only | 1 | JavaScript on unmodified V8; instruction count not available here | 19829.550 / 21579.800 | not available here | 4957.387 / 5394.950 | executed JavaScript domain-helper callbacks: 708.198 / 770.707 |
| jumbling | static_only | 2 | JavaScript on unmodified V8; instruction count not available here | 19779.000 / 21308.400 | not available here | 4944.750 / 5327.100 | executed JavaScript domain-helper callbacks: 706.393 / 761.014 |
| jumbling | static_only | 3 | JavaScript on unmodified V8; instruction count not available here | 19963.700 / 22121.500 | not available here | 4990.925 / 5530.375 | executed JavaScript domain-helper callbacks: 712.989 / 790.054 |
| representations | static_only | 1 | JavaScript on unmodified V8; instruction count not available here | 41340.300 / 44779.800 | not available here | 6890.050 / 7463.300 | executed JavaScript domain-helper callbacks: 939.552 / 1017.723 |
| representations | static_only | 2 | JavaScript on unmodified V8; instruction count not available here | 41885.600 / 57023.200 | not available here | 6980.933 / 9503.867 | executed JavaScript domain-helper callbacks: 951.945 / 1295.982 |
| representations | static_only | 3 | JavaScript on unmodified V8; instruction count not available here | 42648.450 / 58101.600 | not available here | 7108.075 / 9683.600 | executed JavaScript domain-helper callbacks: 969.283 / 1320.491 |
| full | run_time | not_run | JavaScript on unmodified V8; instruction count not available here | RTE.12/RTE.24: borrowed VM is unmodified; no native run-time labels | not_run | not_run | not_run |
| jumbling | run_time | not_run | JavaScript on unmodified V8; instruction count not available here | RTE.12/RTE.24: borrowed VM is unmodified; no native run-time labels | not_run | not_run | not_run |
| representations | run_time | not_run | JavaScript on unmodified V8; instruction count not available here | RTE.12/RTE.24: borrowed VM is unmodified; no native run-time labels | not_run | not_run | not_run |

## rt3-wasm

| Workload | Mode | Series | Executed form | Elapsed | Per step | Per kernel call | Per coarse unit |
| --- | --- | --- | --- | --- | --- | --- | --- |
| full | labels_off | 1 | generated WebAssembly; instruction count not available here | 881607.100 / 1140819.500 | not available here | 62971.936 / 81487.107 | executed Wasm-to-host import calls: 4238.496 / 5484.709 |
| full | labels_off | 2 | generated WebAssembly; instruction count not available here | 1093486.750 / 1271443.500 | not available here | 78106.196 / 90817.393 | executed Wasm-to-host import calls: 5257.148 / 6112.709 |
| full | labels_off | 3 | generated WebAssembly; instruction count not available here | 1145171.150 / 1332138.200 | not available here | 81797.939 / 95152.729 | executed Wasm-to-host import calls: 5505.631 / 6404.511 |
| jumbling | labels_off | 1 | generated WebAssembly; instruction count not available here | 218623.800 / 239748.000 | not available here | 54655.950 / 59937.000 | executed Wasm-to-host import calls: 1383.695 / 1517.392 |
| jumbling | labels_off | 2 | generated WebAssembly; instruction count not available here | 211841.950 / 290150.600 | not available here | 52960.488 / 72537.650 | executed Wasm-to-host import calls: 1340.772 / 1836.396 |
| jumbling | labels_off | 3 | generated WebAssembly; instruction count not available here | 203407.600 / 226433.400 | not available here | 50851.900 / 56608.350 | executed Wasm-to-host import calls: 1287.390 / 1433.123 |
| representations | labels_off | 1 | generated WebAssembly; instruction count not available here | 469522.800 / 502042.600 | not available here | 78253.800 / 83673.767 | executed Wasm-to-host import calls: 3194.033 / 3415.256 |
| representations | labels_off | 2 | generated WebAssembly; instruction count not available here | 457502.050 / 496135.500 | not available here | 76250.342 / 82689.250 | executed Wasm-to-host import calls: 3112.259 / 3375.071 |
| representations | labels_off | 3 | generated WebAssembly; instruction count not available here | 487984.450 / 520675.000 | not available here | 81330.742 / 86779.167 | executed Wasm-to-host import calls: 3319.622 / 3542.007 |
| full | static_only | 1 | generated WebAssembly; instruction count not available here | 878671.450 / 1086185.400 | not available here | 62762.246 / 77584.671 | executed Wasm-to-host import calls: 4224.382 / 5222.045 |
| full | static_only | 2 | generated WebAssembly; instruction count not available here | 827505.300 / 976945.800 | not available here | 59107.521 / 69781.843 | executed Wasm-to-host import calls: 3978.391 / 4696.855 |
| full | static_only | 3 | generated WebAssembly; instruction count not available here | 1066404.000 / 1271082.300 | not available here | 76171.714 / 90791.593 | executed Wasm-to-host import calls: 5126.942 / 6110.973 |
| jumbling | static_only | 1 | generated WebAssembly; instruction count not available here | 268936.150 / 310163.700 | not available here | 67234.038 / 77540.925 | executed Wasm-to-host import calls: 1702.128 / 1963.061 |
| jumbling | static_only | 2 | generated WebAssembly; instruction count not available here | 263604.150 / 382169.700 | not available here | 65901.038 / 95542.425 | executed Wasm-to-host import calls: 1668.381 / 2418.796 |
| jumbling | static_only | 3 | generated WebAssembly; instruction count not available here | 206317.100 / 246023.600 | not available here | 51579.275 / 61505.900 | executed Wasm-to-host import calls: 1305.804 / 1557.111 |
| representations | static_only | 1 | generated WebAssembly; instruction count not available here | 506648.500 / 578725.900 | not available here | 84441.417 / 96454.317 | executed Wasm-to-host import calls: 3446.588 / 3936.911 |
| representations | static_only | 2 | generated WebAssembly; instruction count not available here | 491566.950 / 629949.900 | not available here | 81927.825 / 104991.650 | executed Wasm-to-host import calls: 3343.993 / 4285.373 |
| representations | static_only | 3 | generated WebAssembly; instruction count not available here | 503820.200 / 561948.000 | not available here | 83970.033 / 93658.000 | executed Wasm-to-host import calls: 3427.348 / 3822.776 |
| full | run_time | 1 | generated WebAssembly; instruction count not available here | 1097015.700 / 1206144.700 | not available here | 78358.264 / 86153.193 | executed Wasm-to-host import calls: 4628.758 / 5089.218 |
| full | run_time | 2 | generated WebAssembly; instruction count not available here | 953351.150 / 1274637.600 | not available here | 68096.511 / 91045.543 | executed Wasm-to-host import calls: 4022.579 / 5378.218 |
| full | run_time | 3 | generated WebAssembly; instruction count not available here | 1104931.700 / 1241537.600 | not available here | 78923.693 / 88681.257 | executed Wasm-to-host import calls: 4662.159 / 5238.555 |
| jumbling | run_time | 1 | generated WebAssembly; instruction count not available here | 221021.350 / 239363.800 | not available here | 55255.338 / 59840.950 | executed Wasm-to-host import calls: 1323.481 / 1433.316 |
| jumbling | run_time | 2 | generated WebAssembly; instruction count not available here | 210674.550 / 225795.600 | not available here | 52668.637 / 56448.900 | executed Wasm-to-host import calls: 1261.524 / 1352.069 |
| jumbling | run_time | 3 | generated WebAssembly; instruction count not available here | 205632.600 / 234280.000 | not available here | 51408.150 / 58570.000 | executed Wasm-to-host import calls: 1231.333 / 1402.874 |
| representations | run_time | 1 | generated WebAssembly; instruction count not available here | 500697.950 / 592721.400 | not available here | 83449.658 / 98786.900 | executed Wasm-to-host import calls: 3053.036 / 3614.155 |
| representations | run_time | 2 | generated WebAssembly; instruction count not available here | 476956.800 / 582822.000 | not available here | 79492.800 / 97137.000 | executed Wasm-to-host import calls: 2908.273 / 3553.793 |
| representations | run_time | 3 | generated WebAssembly; instruction count not available here | 470701.700 / 518356.300 | not available here | 78450.283 / 86392.717 | executed Wasm-to-host import calls: 2870.132 / 3160.709 |

## rt4-pyhost

| Workload | Mode | Series | Executed form | Elapsed | Per step | Per kernel call | Per coarse unit |
| --- | --- | --- | --- | --- | --- | --- | --- |
| full | labels_off | 1 | CPython 3.15.1 opcode events in rendered code objects | 45587.900 / 54168.400 | 46.048 / 54.716 | 3256.279 / 3869.171 | not applicable |
| full | labels_off | 2 | CPython 3.15.1 opcode events in rendered code objects | 47110.200 / 57891.300 | 47.586 / 58.476 | 3365.014 / 4135.093 | not applicable |
| full | labels_off | 3 | CPython 3.15.1 opcode events in rendered code objects | 47505.200 / 58477.200 | 47.985 / 59.068 | 3393.229 / 4176.943 | not applicable |
| jumbling | labels_off | 1 | CPython 3.15.1 opcode events in rendered code objects | 14895.600 / 17129.400 | 35.635 / 40.979 | 3723.900 / 4282.350 | not applicable |
| jumbling | labels_off | 2 | CPython 3.15.1 opcode events in rendered code objects | 14689.200 / 17834.500 | 35.142 / 42.666 | 3672.300 / 4458.625 | not applicable |
| jumbling | labels_off | 3 | CPython 3.15.1 opcode events in rendered code objects | 14823.550 / 17391.100 | 35.463 / 41.606 | 3705.887 / 4347.775 | not applicable |
| representations | labels_off | 1 | CPython 3.15.1 opcode events in rendered code objects | 31540.250 / 36158.700 | 40.802 / 46.777 | 5256.708 / 6026.450 | not applicable |
| representations | labels_off | 2 | CPython 3.15.1 opcode events in rendered code objects | 31390.450 / 39406.000 | 40.609 / 50.978 | 5231.742 / 6567.667 | not applicable |
| representations | labels_off | 3 | CPython 3.15.1 opcode events in rendered code objects | 31682.100 / 44763.600 | 40.986 / 57.909 | 5280.350 / 7460.600 | not applicable |
| full | static_only | 1 | CPython 3.15.1 opcode events in rendered code objects | 56966.700 / 83082.100 | 57.542 / 83.921 | 4069.050 / 5934.436 | not applicable |
| full | static_only | 2 | CPython 3.15.1 opcode events in rendered code objects | 59130.400 / 83215.800 | 59.728 / 84.056 | 4223.600 / 5943.986 | not applicable |
| full | static_only | 3 | CPython 3.15.1 opcode events in rendered code objects | 72591.000 / 81941.700 | 73.324 / 82.769 | 5185.071 / 5852.979 | not applicable |
| jumbling | static_only | 1 | CPython 3.15.1 opcode events in rendered code objects | 23796.150 / 30286.200 | 56.929 / 72.455 | 5949.038 / 7571.550 | not applicable |
| jumbling | static_only | 2 | CPython 3.15.1 opcode events in rendered code objects | 24310.050 / 33401.200 | 58.158 / 79.907 | 6077.512 / 8350.300 | not applicable |
| jumbling | static_only | 3 | CPython 3.15.1 opcode events in rendered code objects | 25451.450 / 33960.400 | 60.889 / 81.245 | 6362.863 / 8490.100 | not applicable |
| representations | static_only | 1 | CPython 3.15.1 opcode events in rendered code objects | 39293.400 / 44644.500 | 50.832 / 57.755 | 6548.900 / 7440.750 | not applicable |
| representations | static_only | 2 | CPython 3.15.1 opcode events in rendered code objects | 40373.500 / 51314.700 | 52.230 / 66.384 | 6728.917 / 8552.450 | not applicable |
| representations | static_only | 3 | CPython 3.15.1 opcode events in rendered code objects | 40555.050 / 56334.500 | 52.464 / 72.878 | 6759.175 / 9389.083 | not applicable |
| full | run_time (incomplete wrapper attempt) | 1 | CPython 3.15.1 opcode events in rendered code objects | 45393.200 / 74735.800 | 45.852 / 75.491 | 3242.371 / 5338.271 | not applicable |
| full | run_time (incomplete wrapper attempt) | 2 | CPython 3.15.1 opcode events in rendered code objects | 45490.450 / 53691.000 | 45.950 / 54.233 | 3249.318 / 3835.071 | not applicable |
| full | run_time (incomplete wrapper attempt) | 3 | CPython 3.15.1 opcode events in rendered code objects | 45547.400 / 55566.800 | 46.007 / 56.128 | 3253.386 / 3969.057 | not applicable |
| jumbling | run_time (incomplete wrapper attempt) | 1 | CPython 3.15.1 opcode events in rendered code objects | 15014.400 / 18775.000 | 35.920 / 44.916 | 3753.600 / 4693.750 | not applicable |
| jumbling | run_time (incomplete wrapper attempt) | 2 | CPython 3.15.1 opcode events in rendered code objects | 14622.400 / 15656.100 | 34.982 / 37.455 | 3655.600 / 3914.025 | not applicable |
| jumbling | run_time (incomplete wrapper attempt) | 3 | CPython 3.15.1 opcode events in rendered code objects | 14362.800 / 18415.900 | 34.361 / 44.057 | 3590.700 / 4603.975 | not applicable |
| representations | run_time (incomplete wrapper attempt) | 1 | CPython 3.15.1 opcode events in rendered code objects | 29775.000 / 35620.700 | 38.519 / 46.081 | 4962.500 / 5936.783 | not applicable |
| representations | run_time (incomplete wrapper attempt) | 2 | CPython 3.15.1 opcode events in rendered code objects | 29474.800 / 32668.400 | 38.130 / 42.262 | 4912.467 / 5444.733 | not applicable |
| representations | run_time (incomplete wrapper attempt) | 3 | CPython 3.15.1 opcode events in rendered code objects | 29700.650 / 36201.000 | 38.423 / 46.832 | 4950.108 / 6033.500 | not applicable |

