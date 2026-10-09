"""LL3 lane hooks and capture checks; no imports write bytecode in the checkout."""
from pathlib import Path
import json
import math
import os
import re
import shutil
import struct
import time
from types import SimpleNamespace
import zlib

FRAME_RATE = 24
MAX_SECONDS = 10
CAPTURES = Path("work/loop-memory/looklab/captures")
GALLERY = Path("work/gallery/looklab")


def headless(check, godot, stage, temporary, env, common):
    # Every executable lane goes through child_env(), which always declares the guarded root.
    if "LOOKLAB_TEMP_ROOT" not in env:
        raise RuntimeError("LL3 lane needs the harness temporary root")
    check = SimpleNamespace(**check) if isinstance(check, dict) else check
    for mode, marker, fault in (
        ("swipe-check", "LOOKLAB_SWIPE_PASS answers=2 records=2", "swipe-answer-disabled"),
        ("compare-check", "LOOKLAB_COMPARE_PASS counts=2,3,4 samples=45 camera=shared clock=shared", "shared-clock-disabled"),
    ):
        run = check.godot_run(godot, stage, temporary, env, ["--headless", *common, mode])
        if run.stdout.splitlines().count(marker) != 1:
            raise check.CheckFailure("LL3 result missing or repeated: " + mode)
        check.godot_run(godot, stage, temporary, env, ["--headless", *common, mode, fault], expected_failure=fault)
    print("looklab LL3: swipe and compare checks and both disabled-handler faults passed", flush=True)


def find_ffmpeg(parent=None):
    parent = os.environ if parent is None else parent
    if parent.get("LOOKLAB_FFMPEG"):
        path = Path(parent["LOOKLAB_FFMPEG"])
        return path.resolve() if path.is_file() else None
    found = shutil.which("ffmpeg", path=parent.get("PATH", ""))
    if found:
        return Path(found).resolve()
    packages = Path(parent.get("LOCALAPPDATA", "")) / "Microsoft/WinGet/Packages"
    matches = sorted(p for folder in packages.glob("Gyan.FFmpeg_*") for p in folder.rglob("ffmpeg.exe") if p.is_file())
    return matches[0].resolve() if matches else None


def validate_png(path, expected):
    """Check chunks, CRCs, decoded row bounds and filter bytes for captured RGB(A) PNGs."""
    data = Path(path).read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError("capture: invalid PNG signature")
    pos, chunks, compressed, dimensions, channels = 8, [], bytearray(), None, 0
    while pos < len(data):
        if pos + 12 > len(data):
            raise ValueError("capture: truncated PNG chunk")
        length = int.from_bytes(data[pos:pos + 4], "big")
        kind, body = data[pos + 4:pos + 8], data[pos + 8:pos + 8 + length]
        end = pos + 12 + length
        if end > len(data) or zlib.crc32(kind + body) != int.from_bytes(data[end - 4:end], "big"):
            raise ValueError("capture: PNG chunk length or CRC is invalid")
        if not chunks and kind != b"IHDR":
            raise ValueError("capture: PNG must start with IHDR")
        if kind == b"IHDR":
            if chunks or length != 13:
                raise ValueError("capture: invalid PNG IHDR")
            w, h, bits, colour, compression, filtering, interlace = struct.unpack(">IIBBBBB", body)
            dimensions = (w, h)
            if dimensions != tuple(expected) or w <= 0 or h <= 0 or w * h > 40_000_000:
                raise ValueError("capture: PNG size differs from the viewport or exceeds the gallery bound")
            if bits != 8 or colour not in (2, 6) or (compression, filtering, interlace) != (0, 0, 0):
                raise ValueError("capture: expected a non-interlaced 8-bit RGB(A) PNG")
            channels = 3 if colour == 2 else 4
        elif kind == b"IDAT":
            if b"IDAT" in chunks and chunks[-1] != b"IDAT":
                raise ValueError("capture: PNG IDAT chunks must be consecutive")
            compressed.extend(body)
        elif kind == b"IEND":
            if length or end != len(data) or not compressed:
                raise ValueError("capture: invalid PNG end")
            chunks.append(kind)
            break
        elif not kind[0] & 32:
            raise ValueError("capture: unsupported critical PNG chunk")
        chunks.append(kind)
        pos = end
    if not chunks or chunks[-1] != b"IEND" or dimensions is None:
        raise ValueError("capture: missing PNG end")
    stride = dimensions[0] * channels + 1
    decoder = zlib.decompressobj()
    try:
        pixels = decoder.decompress(compressed, stride * dimensions[1] + 1)
    except zlib.error as exc:
        raise ValueError("capture: invalid PNG pixels") from exc
    if not decoder.eof or decoder.unused_data or len(pixels) != stride * dimensions[1] or any(pixels[i] > 4 for i in range(0, len(pixels), stride)):
        raise ValueError("capture: invalid PNG row or decoded pixel bounds")
    return dimensions


def validate_clip_samples(samples, width, height, frames):
    if not 0 < frames <= MAX_SECONDS * FRAME_RATE or len(samples) != frames:
        raise ValueError("capture: clip frame count exceeds its bound or differs from the request")
    padded = (width + width % 2, height + height % 2)
    for i, (number, seconds, w, h) in enumerate(samples):
        if number != i or not math.isfinite(seconds) or abs(seconds - i / FRAME_RATE) > 1e-5 or (w, h) != padded:
            raise ValueError("capture: clip must have fixed 24 fps timing and viewport dimensions (padded to even pixels)")
    return frames / FRAME_RATE


def validate_clip(check, path, ffmpeg, width, height, frames, cwd, env):
    # FFmpeg itself decodes every frame. No separately installed ffprobe is needed.
    run = check.run_process([str(ffmpeg), "-nostdin", "-hide_banner", "-i", str(path),
                             "-map", "0:v:0", "-vf", "showinfo", "-f", "null", "-"],
                            cwd=cwd, env=env, timeout=60)
    check.require_success(run)
    rates = re.findall(r"config in time_base:.*?frame_rate:\s*(\d+)/(\d+)", run.stdout)
    if not rates or any(int(den) == 0 or int(num) / int(den) != FRAME_RATE for num, den in rates):
        raise ValueError("capture: encoded clip frame rate is not 24 fps")
    samples = [(int(n), float(t), int(w), int(h)) for n, t, w, h in re.findall(
        r"\bn:\s*(\d+)\s+pts:\s*-?\d+\s+pts_time:([-+\d.eE]+).*?\bs:(\d+)x(\d+)", run.stdout)]
    return validate_clip_samples(samples, width, height, frames)


def gallery_scan(root):
    # Import the workbench's implementation; do not duplicate its folder scanner.
    from tools.workbench.home import GalleryScanner
    return GalleryScanner().scan(Path(root), [Path(root)], [], [], [], time.time(), force=True)


def verify_gallery(root, names, width, height):
    items = {item["rel"]: item for item in gallery_scan(root)}
    for name in names:
        item = items.get((GALLERY / name).as_posix())
        expected = "image" if name.endswith(".png") else "video"
        if item is None or item["type"] != expected or "gallery" not in item["origins"]:
            raise ValueError("capture: gallery's own scanner did not find " + name)
        source, mirror = Path(root) / CAPTURES / name, Path(root) / GALLERY / name
        if source.read_bytes() != mirror.read_bytes():
            raise ValueError("capture: gallery copy differs from the capture")
        if expected == "image" and item["pixels"] != [width, height]:
            raise ValueError("capture: gallery PNG size differs from the viewport")
    return len(names)


def graphics(check, godot, stage, temporary, env, common):
    if "LOOKLAB_TEMP_ROOT" not in env:
        raise RuntimeError("LL3 lane needs the harness temporary root")
    check = SimpleNamespace(**check) if isinstance(check, dict) else check
    env = dict(env)
    ffmpeg = find_ffmpeg()
    # Resolve the installed encoder before Godot's AppData redirection.
    env["LOOKLAB_FFMPEG"] = str(ffmpeg) if ffmpeg else str(temporary / "ffmpeg-unavailable.exe")
    run = check.godot_run(godot, stage, temporary, env, [*common, "capture-check"], graphics=True)
    lines = [line.removeprefix("LOOKLAB_CAPTURE_PASS ") for line in run.stdout.splitlines() if line.startswith("LOOKLAB_CAPTURE_PASS ")]
    if len(lines) != 1:
        raise check.CheckFailure("capture: result missing or repeated")
    report = json.loads(lines[0])
    root = temporary / "ll3-capture"
    width, height = report["width"], report["height"]
    still, clip = report["still"], report["clip"]
    if not isinstance(still, str) or Path(still).name != still or not still.endswith(".png"):
        raise check.CheckFailure("capture: invalid still filename")
    try:
        validate_png(root / CAPTURES / still, (width, height))
        names = [still]
        if ffmpeg:
            if not isinstance(clip, str) or Path(clip).name != clip or not clip.endswith(".mp4") or report["frames"] != FRAME_RATE:
                raise ValueError("capture: one-second clip result missing or unbounded")
            validate_clip(check, root / CAPTURES / clip, ffmpeg, width, height, FRAME_RATE, stage, env)
            names.append(clip)
        elif clip is not None or report["frames"] != 0:
            raise ValueError("capture: clip must be disabled without local FFmpeg")
        # -B in the isolated child also protects the workbench imports from bytecode writes.
        command = [env["LOOKLAB_PYTHON"], "-B", str(check.ROOT / check.LAB / "tests/ll3_capture_test.py"),
                   "--gallery", str(root), str(width), str(height), *names]
        check.require_success(check.run_process(command, cwd=check.ROOT, env=env, timeout=60))
    except (OSError, ValueError, KeyError, TypeError) as exc:
        raise check.CheckFailure(str(exc)) from exc
    check.godot_run(godot, stage, temporary, env, [*common, "capture-check", "capture-handler-disabled"],
                    graphics=True, expected_failure="capture-handler-disabled")
    print("looklab LL3: native PNG, gallery scan and disabled capture handler passed; " +
          ("one-second 24 fps clip passed" if ffmpeg else "clip disabled (local FFmpeg unavailable)"), flush=True)
