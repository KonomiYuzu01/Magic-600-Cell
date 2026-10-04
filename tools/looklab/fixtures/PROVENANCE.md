# Fixture provenance

The S-B source branch is `claude/renderer-sb`, at commit
`19341fd70913845bcf8624148979c3e065027f48`. These are synthetic model/probe inputs, not session data.

`sb-reference.json` retains 300 samples per camera/pose, at source positions
`floor(i * (9066 - 1) / 299)` for i = 0..299. Both endpoints are included.
`w3-turn.json` is the original turn file, copied byte for byte.

Production command for both fixtures:

```text
python tools/looklab/fixtures/make_sb_fixture.py
```

Inputs (paths relative to the S-B source copy):

| File | SHA-256 |
|---|---|
| `SOURCE_COMMIT` | `91ad488905634fdba4463d53d6a5e12fb1ed317909849553c13e77776e9cddb9` |
| `SPEC.md` | `1f426c167533eacd5d959d171fc2aace3c113a345c8869befabe4c71cdda675a` |
| `reference_geometry.py` | `caba8d8b7daf2b253abaf2a87a60454432c606a4d5dda30e6c10c3d053e19837` |
| `cameras.json` | `6bb4288cc46e9ad4516b99d74322293dfe9eb076f12f58dc0a7d0e51b6403c50` |
| `workload/turn.json` | `13811e74385b7a07e2e66d7177178b41fdac7921a6e2fd1287e6eac9504445e1` |
| `reference/index.json` | `9f6596fc76d81c945af8f9c1d3321d877c3fe69bc91ed4af938529ce5fb8995c` |
| `reference/sample.u32` | `d05055c5114bd42592f5d019b7406bb88abd26aacdab0ce42a28df5ea0c605a0` |
| `reference/c0_start.f32` | `8334baf9c516ea593cb161f8f15ef3bcc9817333ae8d5b6a5d9b1aeb1878b118` |
| `reference/c0_mid.f32` | `795dbe2059577e2a7c4605b722666129c8b801783b73799c3f468b2a53ace1c1` |
| `reference/c0_end.f32` | `3ffbfb05e99550068dad44ce5fe81a7d8e221a665c535d7651281b37b23a70f5` |
| `reference/c1_start.f32` | `281dc158e263356c32d1114cf150ee7e937427e9bd380212e545001c1c240a54` |
| `reference/c1_mid.f32` | `4841d6d5e9058e8fa4cf8cc6114cc6ee387408466ac9f12c382493819078e11b` |
| `reference/c1_end.f32` | `51aa6d29c28e2b622c305198d53b2f0a2498aa500120396fe0139412ec910e0b` |
| `reference/c2_start.f32` | `e0a7e1173709bd6fd25c8c47d50baa9074fd41a4741fccf2a109f73c5634b83e` |
| `reference/c2_mid.f32` | `14d4b921c605fa8e071bc0231bbadb3e845427679da9b3ce45012436ae9f58fc` |
| `reference/c2_end.f32` | `547135116accbca11b36004abd44d20d5a4e3137003a54b2b48e9c0fefae3e30` |

Asset input digests used by the original reference:

| File | SHA-256 |
|---|---|
| `assets/cell_frames.f32` | `2dfceb37625b3f37985ea94193eca79c6e50e6ad00563eb4ba13a54dd7d3001f` |
| `assets/manifest.json` | `b5db4469f3da9ba3cdbd93806a703f594153e61b08ce32d4fcce9de9c5cc60bc` |
| `assets/mesh.json` | `357fb767272d8e08caa8220903d4f3fc687be47c771b96b170f2cff4ac6ddb03` |
| `assets/mesh_centers.f32` | `91ac80a0be89c3ec2f78ff8ecb214e9949266c8bbdbabf30d18b441c3cbe1cbb` |
| `assets/mesh_sticker.u32` | `34d15cf938814957a40c360822889bc7f4e4c9ac65388d63242af31b170e26c7` |
| `assets/mesh_vertices.f32` | `696f82b56ba7562126a6fc158936cdfe837c594ba666b9b342815bde66a7f4df` |

`rings.json` is a separate deterministic engine-order fixture. Its command is:

```text
python tools/looklab/fixtures/make_rings_fixture.py
```

Input `assets/model.npz`: `680de6710e8a05b12dd4ba2ea22c866e645af032a31faf985d869798521f8b13` (verified against the manifest).
Only normals.npy, orbit_id.npy, face_offsets.npy and face_values.npy are read.
The helix enumeration, tuple sorting, lowest-uncovered-cell search and 7-regular
acceptance rule port tools/tastelab/page/geometry.js and sim/geometry_fixture.py.
Vertex ids retain core.py's ascending orbit-34 piece-position order; cells retain engine ids.

Tools: Python standard library (the packet environment is CPython 3.14.7).
The generators contain their editable production source. Rerunning them requires
the scoped read-only S-B source copy for the S-B fixtures; acceptance only needs committed fixtures.
