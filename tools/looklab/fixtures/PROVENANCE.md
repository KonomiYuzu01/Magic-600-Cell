# Fixture provenance

The S-B source branch is `main`, at commit
`c4b31b0a733cac04d1e514540a7a6ba04439d20e`. These are synthetic model/probe inputs, not session data.

Shrink anchors: area-weighted triangle centroid, SPEC.md section 3. SHA-256 of the 433 x 4 little-endian float32 anchors:
`0b6ead284d3f62719e3e6ec893d6c199d49698d246a59164bc362446e39faca6`.

`sb-reference.json` retains 300 samples per camera/pose, at source positions
`floor(i * (9066 - 1) / 299)` for i = 0..299. Both endpoints are included.
`w3-turn.json` is the original turn file, copied byte for byte.

Production command for both fixtures:

```text
python tools/looklab/fixtures/make_sb_fixture.py <input_folder>
```

`input_folder` is the read-only S-B source copy containing reference/ and workload/.

Inputs (paths relative to the S-B source copy):

| File | SHA-256 |
|---|---|
| `SOURCE_COMMIT` | `d062a04810589944c7b8b6f6ea3a99bd3b596e5353514a3beff3aec92dbdeb91` |
| `SPEC.md` | `8f336c04b618dd7925cd4e8e531b96911840bf377a063d14a9f993015be50101` |
| `reference_geometry.py` | `abadf064763a90cd8c8c890751f27e1038c09a799a22ee39a4a6087c76586efc` |
| `cameras.json` | `6bb4288cc46e9ad4516b99d74322293dfe9eb076f12f58dc0a7d0e51b6403c50` |
| `workload/turn.json` | `13811e74385b7a07e2e66d7177178b41fdac7921a6e2fd1287e6eac9504445e1` |
| `reference/index.json` | `9f7f3dedb3c8f22cbcc59a7002cbdf441d865390ae190188752d1b3c257c75c2` |
| `reference/sample.u32` | `d05055c5114bd42592f5d019b7406bb88abd26aacdab0ce42a28df5ea0c605a0` |
| `reference/c0_start.f32` | `5debaba2ef94ffe15054ed1beffb2c3b0b79c67aaad7545829b78107e380b6cb` |
| `reference/c0_mid.f32` | `3de7d99d2d2d709578ab36b6e9549b770d12da88cf576118ea2b4971f4c0ea0a` |
| `reference/c0_end.f32` | `06056c46471e83ee80084bf70c4366ff098696850a1884e2fe1ebaa0b1a4b527` |
| `reference/c1_start.f32` | `3e7e6916ce48c1603e71eabc2b8616c266b51f94aca479925b0392b66c1fa062` |
| `reference/c1_mid.f32` | `0c83ef87b4bb1b0a343eee16b7f1a35792ca53194703cfb3386aba756c41ee07` |
| `reference/c1_end.f32` | `e35cadc3c9133bd8ad7dc517c8bfa676e62c7862e4987729e323cedcd5917935` |
| `reference/c2_start.f32` | `24a983360b8adad198be82c57633446ce2b5462f06953151d16a46442b7ffb86` |
| `reference/c2_mid.f32` | `59828c55bf4f7d1d8f4f9d40a9a2871bb5a9783fe223419db2a77e26c9f333ba` |
| `reference/c2_end.f32` | `32b82353c750c6d2cd6c8f4079f3d7dd946ab7d1b7f078d5cc0c8a85e76a7c41` |

Asset input digests used by the original reference:

| File | SHA-256 |
|---|---|
| `assets/cell_frames.f32` | `2dfceb37625b3f37985ea94193eca79c6e50e6ad00563eb4ba13a54dd7d3001f` |
| `assets/manifest.json` | `b5db4469f3da9ba3cdbd93806a703f594153e61b08ce32d4fcce9de9c5cc60bc` |
| `assets/mesh.json` | `357fb767272d8e08caa8220903d4f3fc687be47c771b96b170f2cff4ac6ddb03` |
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

## Synthetic inverse-path fixture

`three-cycle-turn.json` is a test-only 3-cycle of slots 0, 1 and 2, with angle
2*pi/3 and an orthonormal coordinate plane. It is not a legal model generator
and is never used for rendering. The other 259,797 slots retain their labels.
Its explicit inverse restores solved labels; applying its forward move twice does not.

Production command (also regenerates the S-B fixtures above):

```text
python tools/looklab/fixtures/make_sb_fixture.py <input_folder>
```

Fixture SHA-256: `702e667319e5cbd463baaa823e43d5c16660abbb9019964efa43cd8e5c297c07`.
Solved-label SHA-256: `88cc6e660964145096c29d46575f6ab5e78ac5d6ab3f7e5f4e14ad45fde3f9c6`.
Turned-label SHA-256: `9ca93cc22eb14ab832639c9f589a1255051f54b611a81fff819c283ccd8e16b6`.
Label digests cover all 259,800 labels encoded as u32 little-endian, like W3.
