"""LL8 shot checks, graphics probe and private shot-run provenance validation."""
from pathlib import Path
import hashlib
import json
import re
import runpy
import subprocess
from datetime import datetime
from types import SimpleNamespace


SHOT = "tools/looklab/data/shots/g2.json"
TURN = "tools/looklab/fixtures/w3-turn.json"
PARAMETERS = "tools/looklab/data/parameters.json"
SHADER = "tools/looklab/app/shaders/cells.gdshader"
PRESETS = "tools/looklab/presets"
RUNS = "work/loop-memory/looklab/shots"
GALLERY = "work/gallery/looklab/shots"
CLIPS = "not made: no clip encoder until FFmpeg is approved"
RUN_PATTERN = r"[a-z0-9][a-z0-9-]*-[0-9]{8}T[0-9]{6}Z-[0-9a-f]{8}"
HEADLESS = "LOOKLAB_LL8_SHOT_PASS presets=2 frames=240 camera=bound turn=bound spot=identical stills=2 manifest=exact checks=1"
GRAPHICS = "LOOKLAB_LL8_CAPTURE_PASS frames=48 waits=3 effects=identical checks=1"
SHOT_FAULTS = ("ll8-shot-stale-camera", "ll8-shot-stale-capture", "ll8-shot-frozen-capture")
CAPTURE_FAULTS = ("ll8-capture-delayed", "ll8-capture-frozen")


def run(godot, stage, temporary, env, common, godot_run, failure):
    result = godot_run(godot, stage, temporary, env, ["--headless", *common, "ll8-shot"])
    if sum(re.fullmatch(HEADLESS, line) is not None for line in result.stdout.splitlines()) != 1:
        raise failure("ll8-shot: result missing, repeated or wrong counts")
    for fault in SHOT_FAULTS:
        godot_run(godot, stage, temporary, env, ["--headless", *common, "ll8-shot", fault], expected_failure=fault)
        print("looklab LL8: injected fault rejected: " + fault, flush=True)
    print("looklab LL8: 1 headless check and 3 injected faults passed", flush=True)


def graphics(godot, stage, temporary, env, common, godot_run, failure):
    result = godot_run(godot, stage, temporary, env, [*common, "ll8-capture"], graphics=True, timeout=600)
    if sum(re.fullmatch(GRAPHICS, line) is not None for line in result.stdout.splitlines()) != 1:
        raise failure("ll8-capture: result missing, repeated or wrong counts")
    for fault in CAPTURE_FAULTS:
        godot_run(godot, stage, temporary, env, [*common, "ll8-capture", fault], graphics=True, timeout=600, expected_failure=fault)
        print("looklab LL8: injected fault rejected: " + fault, flush=True)
    print("looklab LL8: graphics capture and both injected faults passed", flush=True)


def source_identity(check):
    check = SimpleNamespace(**check) if isinstance(check, dict) else check
    def git(*args):
        try:
            result = subprocess.run(["git", *args], cwd=check.ROOT, env=check.scoped_env(), capture_output=True, text=True, timeout=30)
        except (OSError, subprocess.TimeoutExpired) as exc:
            raise check.CheckFailure("shot: source identity git failed") from exc
        if result.returncode:
            raise check.CheckFailure("shot: source identity git failed")
        return result.stdout
    commit = git("rev-parse", "HEAD").strip()
    if re.fullmatch(r"[0-9a-f]{40}", commit) is None:
        raise check.CheckFailure("shot: source identity commit must be 40 lowercase hex")
    clean = not git("--no-optional-locks", "status", "--porcelain", "--untracked-files=all", "--", "tools/looklab")
    return {"commit": commit, "clean": clean, "staged": check.source_digest()}


def shot(check, godot, stage, temporary, env, common, stems):
    check = SimpleNamespace(**check) if isinstance(check, dict) else check
    resolved = list(stems) if stems else sorted(p.stem for p in (check.ROOT / PRESETS).glob("*.json"))
    command = "python tools/looklab/check.py --shot" + (" " + " ".join(stems) if stems else "")
    identity = source_identity(check)
    ctx_env = dict(env, LOOKLAB_SHOT_CONTEXT=json.dumps({"command": command, "source": identity}))
    result = check.godot_run(godot, stage, temporary, ctx_env, [*common, "shot", *resolved], graphics=True, timeout=1800)
    name = shot_report(check, result.stdout, resolved, identity, command)
    print(f"looklab shot: {name} {'formal' if identity['clean'] else 'draft'} presets={len(resolved)} stills={len(resolved)}; no clips until FFmpeg is approved", flush=True)


def shot_report(check, output, stems, identity, command):
    check = SimpleNamespace(**check) if isinstance(check, dict) else check
    def require(condition, message):
        if not condition:
            raise check.CheckFailure("shot: " + message)

    def keys(value, *expected):
        require(isinstance(value, dict) and list(value) == list(expected), "manifest keys differ")

    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError("duplicate manifest key")
            result[key] = value
        return result

    def digest(path):
        return hashlib.sha256(path.read_bytes()).hexdigest()

    def local(relative):
        path = check.ROOT / relative
        require(check.inside(path, check.ROOT), "output is outside the checkout")
        for current in (path, *path.parents):
            require(not current.is_symlink() and not current.is_junction(), "output must not pass through links")
            if current == check.ROOT:
                break
        return path

    def files(folder, expected):
        require(folder.is_dir() and sorted(p.name for p in folder.iterdir()) == sorted(expected), "run or gallery file list differs")
        for name in expected:
            require(local((folder / name).relative_to(check.ROOT)).is_file(), "output file is missing")

    try:
        shot_data = json.loads((check.ROOT / SHOT).read_text(encoding="utf-8"))
        frames = round(shot_data["seconds"] * 24)
        width, height = shot_data["size"]
        hero = shot_data["heroFrame"]
        lines = output.splitlines()
        runs = [line for line in lines if line.startswith("LOOKLAB_SHOT_RUN")]
        require(len(runs) == 1, "RUN result missing or repeated")
        match = re.fullmatch(r"LOOKLAB_SHOT_RUN manifest=" + RUNS + r"/(" + RUN_PATTERN + r")/manifest.json formal=([01])", runs[0])
        require(match is not None, "malformed RUN result")
        name, formal = match.groups()
        require(name.startswith(shot_data["id"] + "-") and bool(int(formal)) == identity["clean"], "RUN identity or formal flag differs")
        require([line for line in lines if line.startswith("LOOKLAB_SHOT ")] ==
                [f"LOOKLAB_SHOT preset={s} still={s}.png spot=identical" for s in stems], "preset results differ")
        require([line for line in lines if line.startswith("LOOKLAB_SHOT_PASS")] ==
                [f"LOOKLAB_SHOT_PASS presets={len(stems)} stills={len(stems)} clips=0 frames={frames} size={width}x{height}"], "PASS result differs")
        folder = local(f"{RUNS}/{name}")
        manifest = json.loads(local(f"{RUNS}/{name}/manifest.json").read_text(encoding="utf-8"), object_pairs_hook=unique)
        keys(manifest, "format", "version", "run", "created", "command", "source", "formal", "shot", "inputs", "tools", "capture", "presets", "clips")
        require(manifest["format"] == "magic600-look-shot-run" and type(manifest["version"]) is int and manifest["version"] == 1, "format or version differs")
        instant = datetime.strptime(manifest["created"], "%Y-%m-%dT%H:%M:%SZ")
        require(manifest["created"] == instant.strftime("%Y-%m-%dT%H:%M:%SZ") and
                re.fullmatch(re.escape(shot_data["id"] + "-" + instant.strftime("%Y%m%dT%H%M%SZ")) + r"-[0-9a-f]{8}", name), "creation time differs from run")
        keys(manifest["source"], "commit", "clean", "staged")
        require(manifest["run"] == name and manifest["command"] == command and manifest["source"] == identity, "source, command or run differs")
        require(type(manifest["formal"]) is bool and manifest["formal"] == identity["clean"] and type(manifest["source"]["clean"]) is bool, "formal differs from clean")
        s = manifest["shot"]
        keys(s, "id", "path", "sha256", "frames", "frameRate", "heroFrame", "spotFrames", "size", "track")
        require(s == {"id": shot_data["id"], "path": SHOT, "sha256": digest(check.ROOT / SHOT), "frames": frames,
                      "frameRate": 24, "heroFrame": hero, "spotFrames": [0, hero, frames - 1], "size": [width, height], "track": s["track"]}, "shot fields or digest differ")
        require(isinstance(s["track"], str) and re.fullmatch(r"[0-9a-f]{64}", s["track"]), "invalid track digest")
        inputs = manifest["inputs"]
        keys(inputs, "modelId", "turn", "parameters", "shader")
        require(isinstance(inputs["modelId"], str) and inputs["modelId"].strip(), "modelId is empty")
        for key, path in (("turn", TURN), ("parameters", PARAMETERS), ("shader", SHADER)):
            keys(inputs[key], "path", "sha256")
            require(inputs[key] == {"path": path, "sha256": digest(check.ROOT / path)}, key + " input digest differs")
        keys(manifest["tools"], "godot", "dotnet", "driver", "adapter")
        require(all(isinstance(v, str) and v.strip() for v in manifest["tools"].values()), "empty tool identity")
        keys(manifest["capture"], "waits", "spotWaits")
        require(manifest["capture"] == {"waits": 3, "spotWaits": 8}, "capture waits differ")
        require(isinstance(manifest["presets"], list) and len(manifest["presets"]) == len(stems), "preset count differs")
        validate_png = runpy.run_path(str(check.ROOT / check.LAB / "lane_ll3.py"))["validate_png"]
        for stem, p in zip(stems, manifest["presets"]):
            keys(p, "stem", "path", "sha256", "name", "framesRendered", "still", "clip", "spot")
            path = f"{PRESETS}/{stem}.json"
            require(p["stem"] == stem and p["path"] == path and p["sha256"] == digest(check.ROOT / path), "preset identity or digest differs")
            require(isinstance(p["name"], str) and p["framesRendered"] == frames and p["clip"] is None and p["spot"] == "identical", "preset fields differ")
            keys(p["still"], "file", "sha256")
            still = local(f"{RUNS}/{name}/{stem}.png")
            require(p["still"] == {"file": stem + ".png", "sha256": digest(still)}, "still file or digest differs")
            validate_png(still, (width, height))
        require(manifest["clips"] == CLIPS, "clips note differs")
        names = [stem + ".png" for stem in stems]
        files(folder, [*names, "manifest.json"])
        mirror = local(f"{GALLERY}/{name}")
        if identity["clean"]:
            files(mirror, names)
            require(all((mirror / name).read_bytes() == (folder / name).read_bytes() for name in names), "gallery mirror differs")
        else:
            require(not mirror.exists(), "draft must not have a gallery mirror")
        return name
    except (OSError, ValueError, KeyError, TypeError, AttributeError) as exc:
        raise check.CheckFailure("shot: " + str(exc)) from exc
