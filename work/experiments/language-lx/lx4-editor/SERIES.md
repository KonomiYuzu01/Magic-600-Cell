# LX-4 headless function-timer series

Experiment E-2.4-L4. Decision: **open**; decider: **owner**.

**Function timers, a lead only. The proposed bars below are context and are not compared.**
No key-to-frame interval, editor, window, PresentMon, renderer gate or E2 was measured.

RTE.49 proposed context only, p95 / p99 milliseconds:

| Interval | S | M | L |
| --- | --- | --- | --- |
| Typed echo | 50 / 67 | 50 / 67 | 50 / 67 |
| Syntax feedback | 67 / 100 | 67 / 100 | 100 / 150 |
| Completion list shown | 100 / 150 | 100 / 150 | 150 / 250 |
| Type/effect diagnostics from last burst key | 150 / 250 | 200 / 300 | 500 / 750 |
| Stale state shown on view | 100 / 150 | 100 / 150 | 100 / 150 |
| Reference view re-run presented | 250 / 400 | 250 / 400 | 500 / 750 |

W proposed dependent re-check context: 1,000 / 1,500 ms. **Not compared.**

Function times below use milliseconds. All three 100-sample series are shown separately.
One preceding smoke sample per row is retained in `timings.json`; it is excluded from quantiles.
p95/p99 use nearest rank; median averages the middle pair. `cold` means fresh parser/checker work
without a retained result cache, in an already running process; it does not mean cold OS caches
or application startup. `cold_check` and `dependents_check` receive already parsed trees.
Cold W covers all 50 modules; W edits cover W00 (400 lines); W dependent checks cover 49 modules.
Syntax-break checks refuse the draft. Completion measures parsing the broken tail, not constructing
or displaying a completion list. See RESULT.md for host/build and exclusions.

| Host | Size / function | Series | n | median ms | p95 ms | p99 ms | largest ms |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| CPython/LX-8 candidate-a | S/cold_parse | 1 | 100 | 8.174 | 11.888 | 12.934 | 12.978 |
| CPython/LX-8 candidate-a | S/cold_parse | 2 | 100 | 8.089 | 9.808 | 12.386 | 12.589 |
| CPython/LX-8 candidate-a | S/cold_parse | 3 | 100 | 8.005 | 10.012 | 11.005 | 11.088 |
| CPython/LX-8 candidate-a | S/cold_check | 1 | 100 | 1.357 | 1.734 | 2.105 | 2.127 |
| CPython/LX-8 candidate-a | S/cold_check | 2 | 100 | 1.364 | 1.717 | 1.963 | 1.964 |
| CPython/LX-8 candidate-a | S/cold_check | 3 | 100 | 1.347 | 2.411 | 2.682 | 2.941 |
| CPython/LX-8 candidate-a | S/body_parse_check | 1 | 100 | 9.573 | 13.760 | 14.018 | 14.206 |
| CPython/LX-8 candidate-a | S/body_parse_check | 2 | 100 | 9.519 | 12.659 | 15.986 | 16.206 |
| CPython/LX-8 candidate-a | S/body_parse_check | 3 | 100 | 9.509 | 15.370 | 15.538 | 15.572 |
| CPython/LX-8 candidate-a | S/signature_parse_check | 1 | 100 | 9.716 | 12.440 | 13.886 | 14.091 |
| CPython/LX-8 candidate-a | S/signature_parse_check | 2 | 100 | 9.466 | 11.341 | 12.201 | 13.254 |
| CPython/LX-8 candidate-a | S/signature_parse_check | 3 | 100 | 9.496 | 13.681 | 14.216 | 14.344 |
| CPython/LX-8 candidate-a | S/completion_parse | 1 | 100 | 9.368 | 13.005 | 14.015 | 15.085 |
| CPython/LX-8 candidate-a | S/completion_parse | 2 | 100 | 8.030 | 8.462 | 9.833 | 10.123 |
| CPython/LX-8 candidate-a | S/completion_parse | 3 | 100 | 7.964 | 12.377 | 12.760 | 12.804 |
| CPython/LX-8 candidate-a | S/syntax_break_parse_check | 1 | 100 | 7.937 | 9.160 | 9.642 | 11.530 |
| CPython/LX-8 candidate-a | S/syntax_break_parse_check | 2 | 100 | 7.972 | 9.067 | 11.075 | 11.129 |
| CPython/LX-8 candidate-a | S/syntax_break_parse_check | 3 | 100 | 7.958 | 8.916 | 10.972 | 12.204 |
| CPython/LX-8 candidate-a | M/cold_parse | 1 | 100 | 45.213 | 64.854 | 72.505 | 75.495 |
| CPython/LX-8 candidate-a | M/cold_parse | 2 | 100 | 44.538 | 47.670 | 49.138 | 52.023 |
| CPython/LX-8 candidate-a | M/cold_parse | 3 | 100 | 44.198 | 47.496 | 50.264 | 50.494 |
| CPython/LX-8 candidate-a | M/cold_check | 1 | 100 | 7.213 | 8.286 | 9.989 | 10.218 |
| CPython/LX-8 candidate-a | M/cold_check | 2 | 100 | 7.209 | 9.412 | 10.194 | 11.389 |
| CPython/LX-8 candidate-a | M/cold_check | 3 | 100 | 7.220 | 8.752 | 9.508 | 9.613 |
| CPython/LX-8 candidate-a | M/body_parse_check | 1 | 100 | 52.667 | 80.836 | 85.862 | 86.212 |
| CPython/LX-8 candidate-a | M/body_parse_check | 2 | 100 | 53.181 | 81.554 | 91.047 | 91.436 |
| CPython/LX-8 candidate-a | M/body_parse_check | 3 | 100 | 55.376 | 81.865 | 84.226 | 85.164 |
| CPython/LX-8 candidate-a | M/signature_parse_check | 1 | 100 | 53.754 | 77.256 | 79.287 | 82.671 |
| CPython/LX-8 candidate-a | M/signature_parse_check | 2 | 100 | 53.686 | 83.966 | 86.590 | 86.821 |
| CPython/LX-8 candidate-a | M/signature_parse_check | 3 | 100 | 52.949 | 71.638 | 84.633 | 90.730 |
| CPython/LX-8 candidate-a | M/completion_parse | 1 | 100 | 46.458 | 73.374 | 77.140 | 78.046 |
| CPython/LX-8 candidate-a | M/completion_parse | 2 | 100 | 46.537 | 65.534 | 70.183 | 76.067 |
| CPython/LX-8 candidate-a | M/completion_parse | 3 | 100 | 45.443 | 69.437 | 72.730 | 74.858 |
| CPython/LX-8 candidate-a | M/syntax_break_parse_check | 1 | 100 | 45.881 | 65.683 | 71.849 | 71.892 |
| CPython/LX-8 candidate-a | M/syntax_break_parse_check | 2 | 100 | 45.988 | 73.555 | 76.314 | 77.424 |
| CPython/LX-8 candidate-a | M/syntax_break_parse_check | 3 | 100 | 45.749 | 72.667 | 76.099 | 77.696 |
| CPython/LX-8 candidate-a | L/cold_parse | 1 | 100 | 271.469 | 397.383 | 406.600 | 416.248 |
| CPython/LX-8 candidate-a | L/cold_parse | 2 | 100 | 316.874 | 405.773 | 420.485 | 428.140 |
| CPython/LX-8 candidate-a | L/cold_parse | 3 | 100 | 355.144 | 409.406 | 417.911 | 420.467 |
| CPython/LX-8 candidate-a | L/cold_check | 1 | 100 | 68.983 | 78.543 | 84.634 | 94.527 |
| CPython/LX-8 candidate-a | L/cold_check | 2 | 100 | 70.688 | 83.868 | 88.999 | 91.181 |
| CPython/LX-8 candidate-a | L/cold_check | 3 | 100 | 70.160 | 80.802 | 88.968 | 90.901 |
| CPython/LX-8 candidate-a | L/body_parse_check | 1 | 100 | 476.519 | 515.961 | 531.220 | 536.044 |
| CPython/LX-8 candidate-a | L/body_parse_check | 2 | 100 | 303.865 | 482.519 | 502.978 | 515.595 |
| CPython/LX-8 candidate-a | L/body_parse_check | 3 | 100 | 289.988 | 313.003 | 324.923 | 333.385 |
| CPython/LX-8 candidate-a | L/signature_parse_check | 1 | 100 | 299.850 | 324.373 | 342.619 | 344.656 |
| CPython/LX-8 candidate-a | L/signature_parse_check | 2 | 100 | 297.557 | 321.453 | 326.573 | 329.385 |
| CPython/LX-8 candidate-a | L/signature_parse_check | 3 | 100 | 297.528 | 310.723 | 313.856 | 320.493 |
| CPython/LX-8 candidate-a | L/completion_parse | 1 | 100 | 255.993 | 274.277 | 283.671 | 284.039 |
| CPython/LX-8 candidate-a | L/completion_parse | 2 | 100 | 257.148 | 272.163 | 276.341 | 286.331 |
| CPython/LX-8 candidate-a | L/completion_parse | 3 | 100 | 259.610 | 293.254 | 300.303 | 353.329 |
| CPython/LX-8 candidate-a | L/syntax_break_parse_check | 1 | 100 | 289.900 | 366.877 | 388.637 | 414.243 |
| CPython/LX-8 candidate-a | L/syntax_break_parse_check | 2 | 100 | 294.123 | 341.405 | 365.639 | 402.909 |
| CPython/LX-8 candidate-a | L/syntax_break_parse_check | 3 | 100 | 294.884 | 346.362 | 391.012 | 408.421 |
| CPython/LX-8 candidate-a | W/cold_parse | 1 | 100 | 1001.480 | 1210.091 | 1409.353 | 1419.471 |
| CPython/LX-8 candidate-a | W/cold_parse | 2 | 100 | 986.129 | 1533.224 | 1582.641 | 1584.286 |
| CPython/LX-8 candidate-a | W/cold_parse | 3 | 100 | 1101.906 | 1469.131 | 1522.262 | 1524.585 |
| CPython/LX-8 candidate-a | W/cold_check | 1 | 100 | 194.520 | 236.925 | 246.458 | 270.796 |
| CPython/LX-8 candidate-a | W/cold_check | 2 | 100 | 215.426 | 291.708 | 302.414 | 304.880 |
| CPython/LX-8 candidate-a | W/cold_check | 3 | 100 | 275.450 | 307.172 | 311.756 | 315.608 |
| CPython/LX-8 candidate-a | W/body_parse_check | 1 | 100 | 38.655 | 39.912 | 43.396 | 44.145 |
| CPython/LX-8 candidate-a | W/body_parse_check | 2 | 100 | 38.167 | 39.279 | 41.345 | 41.352 |
| CPython/LX-8 candidate-a | W/body_parse_check | 3 | 100 | 28.149 | 38.367 | 40.927 | 41.217 |
| CPython/LX-8 candidate-a | W/signature_parse_check | 1 | 100 | 24.498 | 29.305 | 35.322 | 35.426 |
| CPython/LX-8 candidate-a | W/signature_parse_check | 2 | 100 | 24.591 | 33.622 | 36.463 | 36.559 |
| CPython/LX-8 candidate-a | W/signature_parse_check | 3 | 100 | 24.349 | 27.231 | 28.939 | 31.243 |
| CPython/LX-8 candidate-a | W/completion_parse | 1 | 100 | 21.510 | 31.026 | 32.193 | 32.322 |
| CPython/LX-8 candidate-a | W/completion_parse | 2 | 100 | 20.928 | 27.739 | 30.070 | 30.131 |
| CPython/LX-8 candidate-a | W/completion_parse | 3 | 100 | 21.169 | 26.452 | 31.410 | 32.230 |
| CPython/LX-8 candidate-a | W/syntax_break_parse_check | 1 | 100 | 21.802 | 27.959 | 30.477 | 30.613 |
| CPython/LX-8 candidate-a | W/syntax_break_parse_check | 2 | 100 | 23.596 | 32.746 | 35.019 | 35.718 |
| CPython/LX-8 candidate-a | W/syntax_break_parse_check | 3 | 100 | 25.657 | 33.148 | 34.186 | 36.234 |
| CPython/LX-8 candidate-a | W/dependents_check | 1 | 100 | 214.796 | 237.409 | 243.720 | 251.226 |
| CPython/LX-8 candidate-a | W/dependents_check | 2 | 100 | 218.750 | 241.806 | 250.155 | 250.820 |
| CPython/LX-8 candidate-a | W/dependents_check | 3 | 100 | 158.297 | 202.334 | 217.615 | 222.799 |
| Rust/std candidate-a | S/cold_parse | 1 | 100 | 3.622 | 3.886 | 4.125 | 4.138 |
| Rust/std candidate-a | S/cold_parse | 2 | 100 | 3.486 | 3.664 | 3.959 | 4.920 |
| Rust/std candidate-a | S/cold_parse | 3 | 100 | 3.204 | 3.529 | 3.607 | 3.675 |
| Rust/std candidate-a | S/cold_check | 1 | 100 | 2.781 | 3.179 | 3.352 | 3.365 |
| Rust/std candidate-a | S/cold_check | 2 | 100 | 2.759 | 2.849 | 2.993 | 3.150 |
| Rust/std candidate-a | S/cold_check | 3 | 100 | 2.737 | 2.871 | 2.962 | 3.207 |
| Rust/std candidate-a | S/body_parse_check | 1 | 100 | 6.700 | 7.370 | 7.570 | 7.753 |
| Rust/std candidate-a | S/body_parse_check | 2 | 100 | 5.917 | 6.875 | 7.230 | 7.679 |
| Rust/std candidate-a | S/body_parse_check | 3 | 100 | 5.815 | 6.829 | 7.189 | 7.220 |
| Rust/std candidate-a | S/signature_parse_check | 1 | 100 | 6.493 | 6.918 | 7.362 | 7.502 |
| Rust/std candidate-a | S/signature_parse_check | 2 | 100 | 5.583 | 6.423 | 6.534 | 6.628 |
| Rust/std candidate-a | S/signature_parse_check | 3 | 100 | 5.508 | 6.502 | 6.736 | 6.763 |
| Rust/std candidate-a | S/completion_parse | 1 | 100 | 3.665 | 3.772 | 3.880 | 4.034 |
| Rust/std candidate-a | S/completion_parse | 2 | 100 | 3.431 | 3.726 | 4.057 | 4.186 |
| Rust/std candidate-a | S/completion_parse | 3 | 100 | 3.220 | 3.473 | 3.534 | 3.588 |
| Rust/std candidate-a | S/syntax_break_parse_check | 1 | 100 | 3.765 | 3.982 | 4.192 | 4.508 |
| Rust/std candidate-a | S/syntax_break_parse_check | 2 | 100 | 3.704 | 3.955 | 4.034 | 4.229 |
| Rust/std candidate-a | S/syntax_break_parse_check | 3 | 100 | 3.430 | 3.760 | 3.854 | 3.949 |
| Rust/std candidate-a | M/cold_parse | 1 | 100 | 21.115 | 22.477 | 22.810 | 23.335 |
| Rust/std candidate-a | M/cold_parse | 2 | 100 | 20.877 | 24.245 | 25.269 | 25.728 |
| Rust/std candidate-a | M/cold_parse | 3 | 100 | 22.920 | 27.641 | 29.918 | 30.710 |
| Rust/std candidate-a | M/cold_check | 1 | 100 | 19.268 | 21.196 | 21.760 | 22.554 |
| Rust/std candidate-a | M/cold_check | 2 | 100 | 19.232 | 22.441 | 27.058 | 29.220 |
| Rust/std candidate-a | M/cold_check | 3 | 100 | 17.370 | 21.900 | 22.756 | 26.106 |
| Rust/std candidate-a | M/body_parse_check | 1 | 100 | 40.813 | 54.332 | 59.205 | 59.952 |
| Rust/std candidate-a | M/body_parse_check | 2 | 100 | 37.817 | 42.985 | 43.634 | 49.036 |
| Rust/std candidate-a | M/body_parse_check | 3 | 100 | 37.461 | 43.294 | 46.111 | 46.950 |
| Rust/std candidate-a | M/signature_parse_check | 1 | 100 | 40.804 | 44.496 | 46.263 | 46.648 |
| Rust/std candidate-a | M/signature_parse_check | 2 | 100 | 44.797 | 53.855 | 55.449 | 57.375 |
| Rust/std candidate-a | M/signature_parse_check | 3 | 100 | 41.706 | 55.505 | 57.306 | 59.474 |
| Rust/std candidate-a | M/completion_parse | 1 | 100 | 21.439 | 22.378 | 23.481 | 23.680 |
| Rust/std candidate-a | M/completion_parse | 2 | 100 | 21.568 | 24.797 | 26.472 | 26.742 |
| Rust/std candidate-a | M/completion_parse | 3 | 100 | 23.059 | 26.428 | 28.112 | 30.522 |
| Rust/std candidate-a | M/syntax_break_parse_check | 1 | 100 | 21.296 | 22.134 | 22.684 | 23.171 |
| Rust/std candidate-a | M/syntax_break_parse_check | 2 | 100 | 21.436 | 24.761 | 26.343 | 28.410 |
| Rust/std candidate-a | M/syntax_break_parse_check | 3 | 100 | 22.871 | 25.177 | 25.733 | 26.082 |
| Rust/std candidate-a | L/cold_parse | 1 | 100 | 126.518 | 136.069 | 140.308 | 151.170 |
| Rust/std candidate-a | L/cold_parse | 2 | 100 | 123.645 | 150.866 | 160.540 | 167.815 |
| Rust/std candidate-a | L/cold_parse | 3 | 100 | 122.249 | 146.922 | 163.773 | 164.340 |
| Rust/std candidate-a | L/cold_check | 1 | 100 | 107.056 | 140.777 | 144.915 | 152.063 |
| Rust/std candidate-a | L/cold_check | 2 | 100 | 99.722 | 122.321 | 128.706 | 133.207 |
| Rust/std candidate-a | L/cold_check | 3 | 100 | 96.903 | 126.011 | 136.319 | 145.778 |
| Rust/std candidate-a | L/body_parse_check | 1 | 100 | 226.375 | 276.748 | 295.514 | 295.670 |
| Rust/std candidate-a | L/body_parse_check | 2 | 100 | 224.936 | 250.357 | 270.620 | 284.157 |
| Rust/std candidate-a | L/body_parse_check | 3 | 100 | 225.386 | 253.204 | 267.784 | 274.042 |
| Rust/std candidate-a | L/signature_parse_check | 1 | 100 | 236.165 | 293.046 | 303.090 | 306.136 |
| Rust/std candidate-a | L/signature_parse_check | 2 | 100 | 308.742 | 345.013 | 354.231 | 361.924 |
| Rust/std candidate-a | L/signature_parse_check | 3 | 100 | 316.072 | 334.783 | 343.426 | 347.950 |
| Rust/std candidate-a | L/completion_parse | 1 | 100 | 120.109 | 134.921 | 147.442 | 159.053 |
| Rust/std candidate-a | L/completion_parse | 2 | 100 | 122.922 | 139.488 | 143.609 | 152.916 |
| Rust/std candidate-a | L/completion_parse | 3 | 100 | 123.953 | 153.443 | 158.098 | 166.612 |
| Rust/std candidate-a | L/syntax_break_parse_check | 1 | 100 | 126.420 | 144.877 | 160.044 | 163.783 |
| Rust/std candidate-a | L/syntax_break_parse_check | 2 | 100 | 123.599 | 135.579 | 148.956 | 155.505 |
| Rust/std candidate-a | L/syntax_break_parse_check | 3 | 100 | 123.534 | 142.420 | 151.906 | 157.004 |
| Rust/std candidate-a | W/cold_parse | 1 | 100 | 443.086 | 480.377 | 506.537 | 567.479 |
| Rust/std candidate-a | W/cold_parse | 2 | 100 | 462.735 | 549.776 | 575.277 | 580.065 |
| Rust/std candidate-a | W/cold_parse | 3 | 100 | 428.078 | 620.469 | 656.179 | 666.853 |
| Rust/std candidate-a | W/cold_check | 1 | 100 | 306.750 | 326.306 | 421.333 | 452.970 |
| Rust/std candidate-a | W/cold_check | 2 | 100 | 299.811 | 349.678 | 372.404 | 406.396 |
| Rust/std candidate-a | W/cold_check | 3 | 100 | 314.557 | 499.668 | 537.386 | 545.888 |
| Rust/std candidate-a | W/body_parse_check | 1 | 100 | 16.355 | 19.539 | 20.801 | 21.595 |
| Rust/std candidate-a | W/body_parse_check | 2 | 100 | 17.596 | 23.210 | 25.596 | 26.173 |
| Rust/std candidate-a | W/body_parse_check | 3 | 100 | 15.017 | 17.753 | 19.263 | 19.554 |
| Rust/std candidate-a | W/signature_parse_check | 1 | 100 | 15.810 | 17.375 | 17.924 | 18.009 |
| Rust/std candidate-a | W/signature_parse_check | 2 | 100 | 14.681 | 16.028 | 16.337 | 16.366 |
| Rust/std candidate-a | W/signature_parse_check | 3 | 100 | 14.487 | 15.710 | 15.999 | 17.612 |
| Rust/std candidate-a | W/completion_parse | 1 | 100 | 10.219 | 11.033 | 12.507 | 12.848 |
| Rust/std candidate-a | W/completion_parse | 2 | 100 | 8.434 | 9.243 | 9.604 | 9.794 |
| Rust/std candidate-a | W/completion_parse | 3 | 100 | 8.437 | 9.370 | 9.607 | 9.875 |
| Rust/std candidate-a | W/syntax_break_parse_check | 1 | 100 | 9.553 | 10.434 | 10.699 | 10.817 |
| Rust/std candidate-a | W/syntax_break_parse_check | 2 | 100 | 9.303 | 10.196 | 10.443 | 10.520 |
| Rust/std candidate-a | W/syntax_break_parse_check | 3 | 100 | 9.119 | 10.112 | 10.615 | 11.070 |
| Rust/std candidate-a | W/dependents_check | 1 | 100 | 214.108 | 350.475 | 368.003 | 375.651 |
| Rust/std candidate-a | W/dependents_check | 2 | 100 | 311.461 | 342.226 | 354.186 | 359.264 |
| Rust/std candidate-a | W/dependents_check | 3 | 100 | 210.510 | 274.779 | 281.851 | 290.577 |

These are synthetic source workloads in a shared sandboxed machine run. No bar has a pass/fail result.
The two-puzzle 10,800-sample key-to-presented-frame series and all foreground work are **not run here**.
Raw nanoseconds and source/build/input identities: `timings.json`.
