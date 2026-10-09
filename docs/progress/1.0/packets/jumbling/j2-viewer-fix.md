# Packet: fix the J2 viewer review findings V1–V5 (implementation)

Run as an implementation with the default reviewer model, effort `max` and speed tier `fast`, in the worktree the wrapper creates. Change only the allowed files.

## 1. Goal and acceptance
- Goal: fix the five findings of review `20261009T195031Z-2a42a035` on the J2 viewer (`research/jumbling/viewer/`).
  - **V1 (major).** Global shrink moves certificate points across their certified cut. Draw the two certificate points in Global at the exact posed vertex positions, before any cell or sticker shrink, in both perspective and stereographic projection. Shrink applies to sticker geometry only. Counterexample: `#s1-g1-p7` with cell shrink 0.78 and sticker shrink 0.86. Vertex 7 has exact h ≈ +0.003370, but the Global marker sits at h ≈ −0.001076.
  - **V2 (major).** Retained-turn previews describe moving pieces as on the lattice. Between exact states, a piece that moves in the current twist is "moving (float preview)". It must not be reported as on the lattice or sitting in a slot, even when both endpoints are lattice poses. Counterexample: `#s1-p1035` scrubbed to 1.5 shows "On the lattice: the piece sits in a slot." Lattice statements are allowed only at exact states.
  - **V3 (minor).** The loader ignores the exported binary digest. Verify the SHA-256 digest of the decoded scene bytes against the header with `crypto.subtle.digest` before building the model, on both the binary and the base64 path. A mismatch shows a clear load error and builds nothing.
  - **V4 (minor).** The headless certificate assertion accepts incorrect signs. In `check_viewer.mjs`, assert the signed h values and the vertex identities of the Certificate row: below must be negative, above positive, and both must match the exported certificate. Reversing either sign must make the assertion fail.
  - **V5 (minor).** Deep links accept grips without an exported pole. `readHash()` accepts a grip only when it is one of the exported grips of the patch; otherwise it ignores the grip and does not throw. Example: `#s2-g47` and `#s2-g600`.
- Acceptance: the acceptance check below passes. The integrator then runs the headless Playwright check (`node research/jumbling/viewer/check_viewer.mjs --vendor <dir>`) outside the sandbox. It must pass, and it must include a new assertion for V1 (Global certificate markers keep the sign of h at default shrink) and one for V5.
- Out of scope:
  - look, colour and motion choices;
  - `export_scene.py`, unless the digest field it writes is missing or wrong;
  - `research/jumbling/sim/` and `witness.py`.

## 2. Actual problem and reproduction
- The review result lists each finding with its counterexample. Use the deep links above on the page served by `node research/jumbling/viewer/check_viewer.mjs --serve`.

## 3. Environment and versions
- Branch `claude/jumbling-1-0`, committed head. Node 22. Three.js 0.160.0, loaded through the page's import map. Playwright with headless Chromium may not run inside the sandbox, because it has no network; the integrator runs it afterwards.

## 4. Necessary source and evidence
- `research/jumbling/viewer/viewer.js`:
  - loading near line 120;
  - lattice and moving status near lines 236 and 902;
  - the Global certificate markers near line 624;
  - the Local frame near line 746;
  - hash reading near line 1201.
- `research/jumbling/viewer/check_viewer.mjs`, the certificate assertion near line 163.
- `research/jumbling/viewer/export_scene.py`, the digest it writes near line 527.
- `research/jumbling/viewer/README.md`, which must stay accurate after the change.

## 5. Attempts so far
| # | Step | Result |
|---|---|---|
| 1 | Review `20261009T195031Z-2a42a035` | V1 and V2 major; V3, V4 and V5 minor |

## 6. Constraints and owned files
- Change only the allowed files below. Do not commit; the wrapper collects the patch.

```implement-contract
{"allowed_files": ["research/jumbling/viewer/viewer.js", "research/jumbling/viewer/check_viewer.mjs", "research/jumbling/viewer/README.md", "research/jumbling/viewer/index.html"], "acceptance_check": ["node", "--check", "research/jumbling/viewer/viewer.js"], "stop_condition": "all five findings are fixed in the allowed files and the acceptance check passes"}
```

## 7. Required return format
- Changes only in the assigned worktree. The final message lists:
  - the changed files;
  - per finding, what changed;
  - the acceptance result;
  - open points.
