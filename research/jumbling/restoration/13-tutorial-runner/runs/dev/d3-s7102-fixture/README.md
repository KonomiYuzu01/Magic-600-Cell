# D3 bulk fixture: the state after step 198 of the depth 100 development line

Test fixture for the exact speed-up of the bulk rules. It is not a run record; the run itself is
`runs/dev/d3-s7102/` on branch `claude/dev-d3-s7102` (seed 7102, depth 100, acceptance mode), which is still in its
bulk phase.

- `bulk-198.json`: the run's `tables/bulk-end.json` at checkpoint commit `0bca41f` (199 steps recorded, indexes 0
  to 198), byte for byte: the state after the bulk step with index 198. File sha256
  `cd7fe134dd9481c00cbe80d26d2fdbb6d139390a55556d6df002688a2b765160`, state digest `83abb70d…`; 24,789 pieces off
  the lattice (24,665 at height 2 and 124 at height 4), N = 27,644; 771 distinct poses in 20 domains.
- `steps-199-201.json`: the run's next three bulk steps as recorded at checkpoint commit `2731667` (rule, literal
  word, word sha256, and the before and after digest, off count and N of each step). Step 199 is `conj`
  (315, 1)(79, 4)(315, 1) to N = 27,600; step 200 is `cover` (38, 6) to N = 27,598; step 201 is `conj`
  (6, 1)(89, 2)(6, 1) to N = 27,566.

The cloud recorded 868, 30 and 1,064 seconds for these steps. An integrator profile of step 199 at this state
(23 October 2026, headless Linux, under `cProfile`) found the same word in 2,868 seconds of `conj`, of which
`rim.Domains.coset` took 2,166 seconds (exact Q(√5) matrix products), `Domains` rebuilds after every apply and undo
251 seconds over 248 rebuilds, and cap classification 307 seconds. Nothing here is Windows, DirectX or performance
evidence.
