# W-J fixtures (scripted scrambles replayed by J1)

Status: **mid-stage fixtures of the [jumbling plan](../../../docs/progress/1.0/jumbling-plan.md), 9 October 2026.** They serve the renderer packet [E-2.4-0J](../../../docs/progress/1.0/packets/renderer/E-2.4-0J-jumbling.md) and the engineering packet [ENG-0J](../../../docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md).

Evidence kind: source and synthetic geometry. Legality, digests and counts are exact (J1, Q(√5)); arrays and sampled positions are floats converted at the end. Times are cloud timings of research code, not performance evidence.

## Run

From the repository root, with Python 3 and NumPy:

```text
python research/jumbling/fixtures/wj.py check                      # replay every fixture with J1 and compare all references
python research/jumbling/fixtures/wj.py export S4 end <dir>        # renderer arrays for one stage
python research/jumbling/fixtures/wj.py build S4 <walk-output>     # rebuild a fixture from a walk.py output
```

Stages: `start`, `mid` (after half the records), `end`, and `sweep-before` and `sweep-after` around the swept twist.

## Fixtures

Each script is the journal of an exact random walk of `research/jumbling/theory/walk.py` (uniform caps and menu elements, blocked attempts skipped). Only the applied twists are kept, and J1 replays them under the exact menu: every record must apply as recorded.

| File | Menu (identity) | Walk seed | Twists | Off the lattice at the end | Pose-table entries (in K⁺) | Largest entry height | Blocked grips at the end | Swept twist: grip, angle, moving pieces |
|---|---|---:|---:|---:|---:|---:|---:|---|
| `wj-S4.json` | S4₀ (`793fcfb2…`) | 3 | 640 | 117,498 | 1,591 (386) | 32 | 578 | 209, 90°, 2,197 |
| `wj-I_a.json` | I_a (`fddc8532…`) | 2 | 841 | 121,184 | 1,333 (268) | 16 | 573 | 564, 72°, 2,926 |
| `wj-I_b.json` | I_b (`d70c122f…`) | 2 | 885 | 128,013 | 1,248 (242) | 16 | 563 | 249, 144°, 2,882 |

These are observed workloads, not worst cases. Whether the pose count is bounded for any of these menus is open (theory draft, item 3).

## Format (`magic600-jumbling-wj/1`)

- `menu`: the exact menu record (`TwistMenu.record()`); `check` rebuilds the menu with `close=False` and requires the same identity.
- `journal`: the J1 journal document with `model_identity`, `menu_identity` and `contract_revision`. `check` refuses a fixture whose identities differ from the checkout.
- `stages.<stage>`: records applied, exact J1 digest, pieces off the lattice, pose-table entries and how many are in K⁺, the largest entry height and denominator of the non-K⁺ poses, the SHA-256, shape and type of the three renderer arrays, and 48 sampled pieces (36 off the lattice and 12 moved on it at the end) with their pose id, lattice flag and posed centroid in the asset frame.
- `sweep`: the last record whose angle lies strictly between 0° and 180° (half-turns have no swept direction). Its grip, angle, moving set (count and SHA-256 of the sorted piece ids), the rotation plane (`plane_u`, `plane_v`) of the J2 family R(φ) = I + (cos φ − 1)(uuᵀ + vvᵀ) + sin φ (vuᵀ − uvᵀ), the digests and array hashes before and after, and 16 sampled moving pieces with their posed centroid at t = 0, 1/2 and 1.
- `end_survey`: the status of all 600 grips at the end (`a` or `b`), and for each blocked grip the straddling piece and its two exact certificate points, with float copies.
- `samples`: the sampled piece ids.

## Arrays (`export`)

`export` writes a state in the [render data contract](../render-contract.md) (`magic600-jumbling-render/1`, dimension 4). Little-endian, with `header.json` naming the dimension, the revision, the J1 digest, the identities and the SHA-256 of each file:

| File | Content |
|---|---|
| `pose_index.i32` | 177,120 pose ids, one per piece; pose 0 is the identity |
| `poses.f32` | one 4 × 4 float32 matrix per pose, row-major, x′ = M x in the asset frame |
| `pose_lattice.i32` | the K⁺ index of each pose, or −1 off the lattice |

The pose table is J1's canonical table (identity first, then by exact key), so equal configurations give equal arrays.
