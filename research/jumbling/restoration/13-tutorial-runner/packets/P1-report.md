Implemented P1 in the two allowed files:

- [rules_height.py](research/jumbling/restoration/13-tutorial-runner/scripts/rules_height.py): replaced the stub with child execution, validated parsing, deterministic pair selection, proposals, trees and end mappings.
- [test_rules_height.py](research/jumbling/restoration/13-tutorial-runner/scripts/test_rules_height.py): added fixture, geometry, clock, failure and record-hygiene checks.

**Acceptance passed:** exit 0, 21 tests, **114.591 s total**. The worktree remained unchanged during acceptance, including ignored files.

After `(12,14)`, R22D-36 matched S2 in full:
`7cd868f6466953eb6ea1e0d466ed66857d34edaea7dd7c554c1e74975005541b`.

The single H4 call returned `h4/L4`, letter `(35,13)`, τ `35 → null`, u `0 → 0`, ending at:
`3ab25d214b6c4ad759ac7158a0fc1cabae1584aa711d3f757af7b91c43971837`.
The live state stayed unchanged until the runner replayed and accepted the proposal.

Per-test wall times:

| Class / test | Seconds |
|---|---:|
| Committed / r22d81 and pairs | 0.002 |
| Committed / census tiers | 0.002 |
| Committed / H4 and crashes | 0.001 |
| Committed / pair rounds | 0.059 |
| Committed / missing/conflicting FIRST lines | 0.061 |
| Exact / R22D-36 and H4 | 33.500 |
| Exact / clock sentinel | 50.016 |
| Exact / wrong claimed digest | 0.400 |
| Stub / r22d75 | 0.623 |
| Stub / r22d81 families and ends | 0.872 |
| Stub / pairs and NOT_APPLICABLE | 1.424 |
| Stub / H4 families and guards | 0.931 |
| Stub / pair steps and exhaustion | 0.768 |
| Stub / shard errors and timeouts | 2.809 |
| Stub / clocks and signals | 7.295 |
| Stub / crash, truncation and cancellation | 2.232 |
| Stub / malformed and mismatched steps | 3.708 |
| Stub / missing steps | 1.012 |
| Stub / clock wiring | 0.508 |
| Stub / scripts and snapshot | 0.012 |
| Stub / pair order and validation | 0.016 |

Geometry setup took 6.500 s. Real child wall times were:

| Child run | Seconds |
|---|---:|
| r22d75, first step | 9.381 |
| r22d75, second step | 12.225 |
| H4 at S2 | 10.684 |
| r22d75, missing snapshot | 5.634 |
| r22d81, missing snapshot | 6.432 |
| r22d81_pairs, missing snapshot | 8.250 |
| H4, missing snapshot | 7.487 |
| two_letter_u, missing snapshot | 6.852 |
| r22d81, solved snapshot | 7.678 |
| r22d81_pairs, S2 snapshot | 7.607 |

No disagreement with section 4 was found. No runner or API interface change was needed. No P1 issues remain open; the excluded `test_runner.py` stub-expectation update remains with the integrator. Evidence is headless only; Windows/DirectX checks were not run.