# Using Magic 600 Cell 0.4

Download the [0.4 portable package](https://github.com/KonomiYuzu01/Magic-600-Cell/releases/tag/0.4), extract it, and open **Magic600Cell.exe**. Keep the complete package together. Install the supported [Managed DirectX runtime](DIRECTX.md) if prompted; Python and a compiler are not required for normal startup.

## Workspace and solving

G2 is the default native workspace and keeps the puzzle visible. Launch `Magic600Cell.exe --mode g1` for the alternate keyboard workspace.

Use **Solve** to choose a macro, prepare its steps, inspect its complete result and protection, then explicitly preview and execute. **Current**, locked **Next**, and protected state are distinct. In the cycle view, **Current / Operation / After** distinguish the actual residual, the selected operation and its predicted residual.

Endgame families are available inside Solve. Choose the family, auxiliary positions and finite parameters, then inspect the full collateral effect before saving a macro. Saving does not select, insert or execute it. Reference variants require explicit source and destination ordered frames; equivalent net effects do not imply equivalent intermediate-motion protection.

Changing a protection policy invalidates an earlier preview. Cancel that preview and check it again under the new policy before executing. The application does not search for setup words, choose macro combinations or execute a solution for you.

The full model has 35 moving orbits, 177,120 surface pieces, 259,800 labelled sticker slots and 1,200 positive generators. Rendering filters affect visibility, never mechanical state or a move's hidden collateral effects.

## Keyboard and session tools

The onscreen keyboard shows the active set and its functions. Use its set picker to inspect or edit bindings; **Functions** groups utility actions separately from direct solving keys. Custom bindings take precedence, so consult the active map rather than assuming shortcuts from earlier releases.

**Session** provides New, Resume, explicit timer controls, staged seeded scrambles and recoverable reset. A recorded whole-solve completion requires an actual journal commit; importing Home, undoing or restoring does not manufacture a completion.

C600 and MPUlt logs can be exported, checked and explicitly imported. Imports preserve current workspace preferences and provide a recovery checkpoint. MPUlt compatibility is restricted to `MPUltimate v1 600-cell-Full`; model, turns, checksum and full colour state must verify. Keymap files use a separate export/check/apply flow and contain bindings, not puzzle state or executable commands.

## Sessions and rollback

The default 0.4 profile is `%LOCALAPPDATA%\Magic600Cell\0.4`, separate from older `%LOCALAPPDATA%\C600Studio` profiles. No old profile is silently imported, renamed or deleted. Logs and checkpoints belong to the data folder, not the application folder.

For a deliberate compatibility check, close the old application, copy its entire session folder to a separate location, then launch `Magic600Cell.exe --data "<copied-session-folder>"`, replacing the placeholder with that copy's path. Never run two versions against the same profile. Preserve the original as a rollback copy; older applications may not understand newer workspace preferences. Legacy native key files and top-level macro/binding preferences are not automatically adopted as 0.4 settings.

To roll back, close 0.4 and run the earlier application against its untouched original profile. Keep or archive the separate 0.4 profile. Extract each application version into a new complete folder rather than mixing package files.

## Evidence and support material

The [release guide](docs/RELEASE_0_4.md) and [provenance](docs/RELEASE_0_4_PROVENANCE.json) identify the published package and verification scope. Finite endgame witnesses do not prove arbitrary protected-state reachability or a human full solve. Performance optimization is planned for the next iteration.

Keep personal sessions, logs, credentials, conversation exports, screenshots and raw diagnostics private. Public reports should use a minimal anonymized reproduction and identify the application version.

See the [documentation index](docs/README.md) for mathematical references and historical material.
