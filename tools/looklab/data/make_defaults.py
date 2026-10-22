"""Regenerate the authored LL1 schema, preset, greyboxes, draft flows and null costs.

Standard library only. Draft placements do not imply owner UX approval.
"""
import json
import math
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
LAB = HERE.parent


def write(path, data):
    text = json.dumps(data, indent=2, ensure_ascii=False, allow_nan=False) + "\n"
    if path.exists() and path.read_bytes() == text.encode("utf-8"):
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8", newline="\n")


def main():
    parameters = []

    def add(id, group, kind, default, description, min=None, max=None, step=None, values=None, cost=None):
        p = {"id": id, "group": group, "kind": kind}
        if kind in ("number", "integer"):
            p.update(min=min, max=max, step=step)
        if kind == "enum":
            p["values"] = values
        p.update(default=default, costFeature=cost, description=description)
        parameters.append(p)

    def number(id, group, lo, hi, step, default, description, cost=None):
        add(id, group, "number", default, description, lo, hi, step, cost=cost)

    # Convert the S-B sRGB background to the Taste Lab OKLCH controls.
    rgb = [x / 12.92 if x <= .04045 else ((x + .055) / 1.055) ** 2.4 for x in (.13, .145, .16)]
    m1 = ((.4122214708, .5363325363, .0514459929), (.2119034982, .6806995451, .1073969566), (.0883024619, .2817188376, .6299787005))
    m2 = ((.2104542553, .7936177850, -.0040720468), (1.9779984951, -2.4285922050, .4505937099), (.0259040371, .7827717662, -.8086757660))
    lms = [sum(a * b for a, b in zip(row, rgb)) ** (1 / 3) for row in m1]
    lab = [sum(a * b for a, b in zip(row, lms)) for row in m2]
    bg_hue = math.degrees(math.atan2(lab[2], lab[1])) % 360
    bg_tint = math.hypot(lab[1], lab[2])
    number("hueRotation", "Colour", 0, 360, 1, 0, "Sticker palette starting hue in degrees.")
    number("hueSpread", "Colour", 60, 360, 1, 360, "Hue span distributed over the ring colour classes.")
    number("lightness", "Colour", .45, .85, .01, .7, "Sticker OKLab lightness before alternating offsets.")
    number("lightnessAlt", "Colour", 0, .2, .01, .05, "Alternating negative and positive sticker lightness offset.")
    number("chroma", "Colour", .04, .2, .01, .12, "Requested OKLCH sticker chroma, mapped into sRGB gamut.")
    add("classes", "Colour", "integer", 6, "Number of proper ring colour classes.", 4, 8, 1)
    number("bgLightness", "Colour", .05, .95, .01, lab[0], "Background OKLab lightness.")
    number("bgHue", "Colour", 0, 360, 1, bg_hue, "Background OKLCH hue in degrees.")
    number("bgTint", "Colour", 0, .05, .001, bg_tint, "Background OKLCH chroma.")
    number("gap", "Structure", 0, .3, .01, .24, "Cell shrink amount: S-B cs equals one minus gap.")
    number("edgeWeight", "Material", 0, 3, .05, 0, "Barycentric edge treatment weight.", "edgeWeight")
    number("edgeBrightness", "Material", 0, 1, .01, 0, "Linear black to white edge brightness.")
    number("gloss", "Material", 0, 1, .01, 0, "Specular gloss amount; retains the Taste Lab meaning.", "gloss")
    number("glow", "Material", 0, 1, .01, 0, "Rim glow amount.", "glow")
    number("fog", "Light and depth", 0, 1, .01, 0, "Depth fog amount.", "fog")
    number("turnMs", "Motion", 150, 900, 1, 190, "Duration in milliseconds of each generator or inverse turn.")
    number("easeA", "Motion", 0, 1, .01, 1 / 3, "First easing control x at (a,0).")
    number("easeB", "Motion", 0, 1, .01, 1 / 3, "Second easing control x at (1-b,1).")
    add("projection", "Structure", "enum", "perspective", "4D projection or Global layout: perspective, stereographic (d4 fixed to one) and orthographic project the cells; hemispheres, hopfRings and ringSheet place each cell in a chart.", values=["perspective", "stereographic", "orthographic", "hemispheres", "hopfRings", "ringSheet"])
    number("d4", "Structure", 1.01, 4, .01, 1.18, "4D perspective eye parameter used before the 3D camera.")
    number("stickerShrink", "Structure", 0, 1, .01, .82, "S-B ss: sticker shrink around each sticker centre.")
    add("highlight", "Structure", "enum", "none", "Structure highlighting mode.", values=["none", "fibration", "orbit"], cost="highlight")
    add("cellsVisible", "Structure", "boolean", True, "Show the cell geometry; labels and slots are unaffected.")
    add("sliceVisible", "Structure", "boolean", False, "Restrict the visible geometry to the selected normalized 4D w band.")
    number("sliceMin", "Structure", -1, 1, .01, -1, "Lower normalized 4D w bound of the visibility band.")
    number("sliceMax", "Structure", -1, 1, .01, 1, "Upper normalized 4D w bound of the visibility band.")
    add("accentPrimary", "Colour", "colour", {"L": .75, "C": .09, "h": 220}, "Primary instrument accent in OKLCH.")
    add("accentSecondary", "Colour", "colour", {"L": .75, "C": .09, "h": 40}, "Secondary instrument accent in OKLCH.")
    add("finish", "Material", "enum", "matte", "Surface finish, with gloss controlled independently.", values=["matte", "gloss", "glass"], cost="finish")
    number("keyLight", "Light and depth", 0, 2, .01, .5, "Directional key-light gain; default matches S-B shading.", "keyLight")
    number("fillLight", "Light and depth", 0, 2, .01, .5, "Ambient fill-light gain; default matches S-B shading.", "fillLight")
    number("depthOfField", "Light and depth", 0, 1, .01, 0, "Depth-of-field amount, disabled by default.", "depthOfField")
    number("ambientOcclusion", "Light and depth", 0, 1, .01, 0, "Ambient-occlusion amount, disabled by default.", "ambientOcclusion")
    number("inertia", "Motion", 0, 1, .01, 0, "Camera inertia amount.")
    add("settle", "Motion", "curve", {"a": 1 / 3, "b": 1 / 3}, "Normalized settling response with controls (a,0) and (1-b,1).")
    number("cameraDamping", "Motion", 0, 1, .01, 0, "Camera damping amount.")
    number("fieldOfView", "Structure", .2, 2.8, .01, 2 * math.atan(1 / 1.15), "Vertical field of view in radians; zoom is one over tan(fov/2).")
    number("panelDensity", "Frame", .5, 2, .05, 1, "Instrument spacing and row density scale.")
    number("typeScale", "Frame", .75, 2, .05, 1, "Typography scale relative to the app's base type sample.")
    add("layoutId", "Layout", "enum", "central-stage", "Greybox layout structure id.", values=["central-stage", "docked-workbench", "split-views", "focus-inset"])
    number("viewportShare", "Layout", .3, .9, .01, .7, "Requested fraction of the window devoted to the stage.")
    add("panelPlacement", "Layout", "enum", "overlay", "Instrument panel placement mode.", values=["docked", "overlay"])
    write(HERE / "parameters.json", {"format": "magic600-look-parameters", "version": 1, "parameters": parameters})
    write(LAB / "presets/default.json", {"format": "magic600-look-preset", "version": 1, "name": "S-B geometry default", "family": None, "scene": None,
                                       "params": {p["id"]: p["default"] for p in parameters}})
    for id, name, projection in [("perspective", "G1 perspective", "perspective"),
                                 ("stereographic", "G1 stereographic", "stereographic"),
                                 ("orthographic", "G1 orthographic", "orthographic"),
                                 ("hemispheres", "G1 two hemispheres", "hemispheres"),
                                 ("hopf-rings", "G1 Hopf rings", "hopfRings"),
                                 ("ring-sheet", "G1 ring sheet", "ringSheet")]:
        params = {p["id"]: p["default"] for p in parameters}
        params["projection"] = projection
        if projection == "orthographic":
            params["fieldOfView"] = 2 * math.atan(1 / 3.27)
        write(LAB / f"presets/g1-{id}.json", {"format": "magic600-look-preset", "version": 1, "name": name,
                                            "family": None, "scene": None, "params": params})
    features = dict.fromkeys((p["costFeature"] for p in parameters if p["costFeature"]), None)
    write(HERE / "cost.json", {"format": "magic600-look-cost", "version": 1, "features": features})
    commands = json.loads((ROOT / "docs/progress/1.0/command-table.json").read_bytes())
    contexts = commands["vocabularies"]["contexts"]

    def region(id, kind, x, y, width, height, placement="docked"):
        return {"id": id, "kind": kind, "rect": {"x": x, "y": y, "width": width, "height": height},
                "placement": placement, "contexts": contexts}

    for id, central in [("central-stage", True), ("docked-workbench", False), ("split-views", False), ("focus-inset", False)]:
        regions = ([region("stage", "stage", 0, 0, 1, 1), region("instruments", "overlay", .02, .08, .23, .84, "overlay"),
                    region("review", "overlay", .75, .08, .23, .84, "overlay"), region("commands", "overlay", .28, .9, .44, .08, "overlay")]
                   if central else [region("stage", "stage", .25, 0, .5, .75), region("instruments", "panel", 0, 0, .25, 1),
                                    region("review", "panel", .75, 0, .25, 1), region("commands", "panel", .25, .75, .5, .25)])
        if id == "split-views":
            regions = [region("global", "global", 0, 0, .5, .78), region("local", "local", .5, 0, .5, .78),
                       region("instruments", "panel", 0, .78, .36, .22), region("commands", "panel", .36, .78, .28, .22),
                       region("review", "panel", .64, .78, .36, .22)]
        elif id == "focus-inset":
            regions = [region("global", "global", 0, 0, 1, 1), region("local", "local", .66, .52, .32, .40, "overlay"),
                       region("instruments", "overlay", .02, .08, .23, .84, "overlay"),
                       region("review", "overlay", .66, .08, .32, .40, "overlay"),
                       region("commands", "overlay", .28, .90, .36, .08, "overlay")]
        placements = {context: {"placed": {}, "hidden": []} for context in contexts}
        for command in commands["commands"]:
            name = command["id"]
            target = ("stage" if name in ("camera.rotate", "object.inspect", "grip.select", "twist.apply") else
                      "review" if name.startswith(("operation.", "protection.", "worksheet.")) or name == "view.findings" else
                      "instruments" if name.startswith(("view.", "target.", "buffer.", "macro.", "endgame.", "filter.", "block.", "orbit.", "structure.", "intent.")) else "commands")
            if target == "stage" and id in ("split-views", "focus-inset"):
                target = "global"
            for context in command["contexts"]:
                placements[context]["placed"][name] = target
        write(HERE / f"layouts/{id}.json", {"format": "magic600-look-layout", "version": 1, "id": id,
                                            "grid": {"columns": 12, "rows": 12}, "regions": regions, "contexts": placements})
    flows = [
        ("read-session", "Open a session and inspect its orbit state", [("session.resume", "any"), ("view.residuals", "any"), ("orbit.switch", "any"), ("view.local", "any"), ("view.global", "any")]),
        ("piece-operation", "Target, compose, check, preview, execute and review", [("target.next.set", "any"), ("object.inspect", "any"), ("buffer.analyse", "solve"), ("view.local", "any"), ("view.global", "any"), ("macro.select", "solve"), ("operation.phase.select", "solve"), ("operation.phase.edit", "solve"), ("operation.cleanup.from_prepare", "solve"), ("operation.check", "operation"), ("operation.preview", "operation"), ("operation.execute", "operation"), ("view.residuals", "any"), ("target.next.activate", "any")]),
        ("protect-recover", "Protect completed work, save and recover", [("protection.policy.set", "any"), ("protection.position.protect", "any"), ("session.checkpoint.save", "any"), ("session.undo", "any"), ("session.redo", "any"), ("session.checkpoint.restore", "any")]),
        ("macro-reuse", "Save and reuse the solver's macro with a fresh review", [("worksheet.save", "any"), ("macro.save", "any"), ("macro.base.open", "any"), ("macro.select", "macro_base"), ("operation.reuse", "any"), ("operation.check", "operation"), ("operation.preview", "operation"), ("operation.execute", "operation")]),
        ("endgame", "Inspect a residual, choose a correction and review exact completion", [("view.residuals", "any"), ("endgame.configure", "any"), ("macro.select", "solve"), ("operation.check", "operation"), ("operation.preview", "operation"), ("operation.execute", "operation"), ("view.residuals", "any"), ("view.findings", "any"), ("session.report", "any")]),
        ("next-orbit", "Continue with locked Next or select the next orbit", [("view.residuals", "any"), ("target.next.activate", "any"), ("orbit.switch", "any"), ("view.recommendations", "any"), ("view.global", "any")]),
    ]
    for id, name, steps in flows:
        write(HERE / f"flows/{id}.json", {"format": "magic600-look-flow", "version": 1, "id": id, "name": name, "draft": True,
                                          "steps": [{"command": command, "context": context} for command, context in steps]})
    write(HERE / "shots/g2.json", {"format": "magic600-look-shot", "version": 1, "id": "g2",
                                 "name": "G2 shot: slow drift, one turn and its inverse", "size": [1600, 1000],
                                 "seconds": 10, "heroFrame": 60, "ease": [1 / 3, 1 / 3],
                                 "camera": [{"seconds": 10, "dragX": .5, "dragY": .15}],
                                 "turns": [{"start": 2, "seconds": 1}, {"start": 6, "seconds": 1}]})
    print(f"Defaults: {len(parameters)} parameters, four greyboxes, six draft flows, all costs null, one G2 shot")


if __name__ == "__main__":
    main()
