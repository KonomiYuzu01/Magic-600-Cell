"""CPU-only LL3 validation and prepared gallery fixtures; no Godot window."""
from pathlib import Path
import contextlib
import io
import json
import os
import runpy
import shutil
import struct
import subprocess
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch
from uuid import uuid4
import zlib

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT))
lane = SimpleNamespace(**runpy.run_path(str(ROOT / "tools/looklab/lane_ll3.py")))
check = SimpleNamespace(**runpy.run_path(str(ROOT / "tools/looklab/check.py")))


def png(width=3, height=2, pixels=None):
    def chunk(kind, body):
        return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body))
    rows = (b"\0" + bytes([120, 130, 140, 255]) * width) * height if pixels is None else pixels
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b""))


class CaptureTests(unittest.TestCase):
    def setUp(self):
        self.folder = Path(tempfile.gettempdir()) / ("looklab-ll3-test-" + uuid4().hex)
        self.folder.mkdir()

    def tearDown(self):
        if not self.folder.resolve().is_relative_to(Path(tempfile.gettempdir()).resolve()):
            raise RuntimeError("temporary test folder escaped its root")
        shutil.rmtree(self.folder)

    def write_png(self, data):
        path = self.folder / "fixture.png"
        path.write_bytes(data)
        return path

    def test_png_native_dimensions_and_valid_pixels(self):
        self.assertEqual(lane.validate_png(self.write_png(png()), (3, 2)), (3, 2))
        with self.assertRaisesRegex(ValueError, "size differs"):
            lane.validate_png(self.folder / "fixture.png", (4, 2))

    def test_png_corrupt_crc_signature_truncation_and_missing_end(self):
        data = bytearray(png()); data[36] ^= 1
        for bad in (bytes(data), b"not a png", png()[:-1], png()[:-12], png() + b"extra"):
            with self.assertRaises(ValueError):
                lane.validate_png(self.write_png(bad), (3, 2))

    def test_png_decoded_row_bounds_and_filter(self):
        for pixels in (b"\0", bytes([5]) + bytes(12) + bytes(13), bytes(27)):
            with self.assertRaisesRegex(ValueError, "row or decoded"):
                lane.validate_png(self.write_png(png(pixels=pixels)), (3, 2))

    def samples(self, frames=24, width=320, height=240):
        return [(i, i / 24, width, height) for i in range(frames)]

    def test_clip_one_and_ten_seconds_fixed_rate_and_even_padding(self):
        self.assertEqual(lane.validate_clip_samples(self.samples(), 320, 240, 24), 1)
        self.assertEqual(lane.validate_clip_samples(self.samples(240), 320, 240, 240), 10)
        self.assertEqual(lane.validate_clip_samples(self.samples(width=322, height=242), 321, 241, 24), 1)

    def test_clip_invalid_duration_count_rate_and_dimensions(self):
        cases = [(self.samples(241), 241), (self.samples(23), 24), ([], 0)]
        for index in (0, 10):
            values = self.samples(); values[index] = (index, index / 25, 320, 240)
            if index == 0:
                values[index] = (index, float("nan"), 320, 240)
            cases.append((values, 24))
        cases.extend([(self.samples(width=318), 24), (list(reversed(self.samples())), 24)])
        for values, frames in cases:
            with self.assertRaises(ValueError):
                lane.validate_clip_samples(values, 320, 240, frames)

    def test_ffmpeg_decode_validation_and_frame_rate_refusal(self):
        output = "[showinfo] config in time_base: 1/12288, frame_rate: 24/1\n" + "\n".join(
            f"[showinfo] n: {i} pts: {i * 512} pts_time:{i / 24:.7f} fmt:yuv420p s:320x240" for i in range(24))
        fake = SimpleNamespace(run_process=lambda *a, **k: SimpleNamespace(stdout=output), require_success=lambda run: None)
        self.assertEqual(lane.validate_clip(fake, "clip.mp4", "local-ffmpeg", 320, 240, 24, self.folder, {}), 1)
        output = output.replace("frame_rate: 24/1", "frame_rate: 25/1")
        with self.assertRaisesRegex(ValueError, "frame rate"):
            lane.validate_clip(fake, "clip.mp4", "local-ffmpeg", 320, 240, 24, self.folder, {})

    def prepared_gallery(self):
        captures, gallery = self.folder / lane.CAPTURES, self.folder / lane.GALLERY
        captures.mkdir(parents=True); gallery.mkdir(parents=True)
        for name, data in (("prepared.png", png()), ("prepared.mp4", b"synthetic scanner fixture, not encoded video")):
            (captures / name).write_bytes(data); (gallery / name).write_bytes(data)
        return ["prepared.png", "prepared.mp4"]

    def test_gallery_own_scanner_finds_prepared_image_and_video(self):
        names = self.prepared_gallery()
        self.assertEqual(lane.verify_gallery(self.folder, names, 3, 2), 2)
        from tools.workbench import home
        self.assertIn("work/gallery", home.GALLERY_DIRS)
        self.assertIn(".png", home.IMAGE_EXT); self.assertIn(".mp4", home.VIDEO_EXT)
        self.assertLessEqual(len(lane.gallery_scan(self.folder)), home.GALLERY_ITEMS)

    def test_gallery_missing_wrong_size_and_nonidentical_copy_fail(self):
        names = self.prepared_gallery()
        with self.assertRaisesRegex(ValueError, "size differs"):
            lane.verify_gallery(self.folder, names, 4, 2)
        (self.folder / lane.GALLERY / names[1]).write_bytes(b"different")
        with self.assertRaisesRegex(ValueError, "copy differs"):
            lane.verify_gallery(self.folder, names, 3, 2)
        (self.folder / lane.GALLERY / names[0]).unlink()
        with self.assertRaisesRegex(ValueError, "did not find"):
            lane.verify_gallery(self.folder, names, 3, 2)

    def test_ffmpeg_missing_disables_without_install(self):
        with patch("shutil.which", return_value=None):
            self.assertIsNone(lane.find_ffmpeg({"LOCALAPPDATA": str(self.folder), "PATH": ""}))
            self.assertIsNone(lane.find_ffmpeg({"LOOKLAB_FFMPEG": str(self.folder / "absent.exe")}))
        local = self.folder / "ffmpeg.exe"; local.write_bytes(b"discovery fixture")
        self.assertEqual(lane.find_ffmpeg({"LOOKLAB_FFMPEG": str(local)}), local.resolve())

    def test_headless_hook_requires_results_and_runs_faults(self):
        calls = []
        def godot_run(*args, **kwargs):
            calls.append((args[-1], kwargs))
            if kwargs.get("expected_failure"):
                return SimpleNamespace(stdout="")
            mode = args[-1][-1]
            return SimpleNamespace(stdout=("LOOKLAB_SWIPE_PASS answers=2 records=2" if mode == "swipe-check" else
                "LOOKLAB_COMPARE_PASS counts=2,3,4 samples=45 camera=shared clock=shared") + "\n")
        fake = SimpleNamespace(godot_run=godot_run, CheckFailure=ValueError)
        with contextlib.redirect_stdout(io.StringIO()):
            lane.headless(fake, "local-godot", self.folder, self.folder, {"LOOKLAB_TEMP_ROOT": str(self.folder)}, [])
        self.assertEqual([kw["expected_failure"] for _, kw in calls if "expected_failure" in kw],
                         ["swipe-answer-disabled", "shared-clock-disabled"])
        for output in ("", "LOOKLAB_SWIPE_PASS answers=2 records=2\n" * 2):
            fake.godot_run = lambda *a, **k: SimpleNamespace(stdout=output)
            with self.assertRaisesRegex(ValueError, "missing or repeated"):
                lane.headless(fake, "local-godot", self.folder, self.folder, {"LOOKLAB_TEMP_ROOT": str(self.folder)}, [])

    def test_graphics_hook_routes_validated_still_gallery_and_fault_without_gpu(self):
        root = self.folder / "ll3-capture"; root.mkdir()
        for folder in (lane.CAPTURES, lane.GALLERY):
            (root / folder).mkdir(parents=True); (root / folder / "still.png").write_bytes(png())
        report = {"width": 3, "height": 2, "still": "still.png", "clip": None, "frames": 0}
        calls = []
        def godot_run(*args, **kwargs):
            calls.append(kwargs)
            return SimpleNamespace(stdout="LOOKLAB_CAPTURE_PASS " + json.dumps(report) + "\n")
        fake = SimpleNamespace(godot_run=godot_run, CheckFailure=ValueError, ROOT=ROOT, LAB=Path("tools/looklab"),
            run_process=lambda *a, **k: SimpleNamespace(returncode=0), require_success=lambda run: None)
        # The scanner is proved separately; this only exercises graphics hook routing.
        with patch.dict(lane.graphics.__globals__, find_ffmpeg=lambda: None), contextlib.redirect_stdout(io.StringIO()):
            lane.graphics(fake, "local-godot", self.folder, self.folder, {"LOOKLAB_PYTHON": sys.executable, "LOOKLAB_TEMP_ROOT": str(self.folder)}, [])
        self.assertTrue(all(kw["graphics"] for kw in calls))
        self.assertEqual(calls[-1]["expected_failure"], "capture-handler-disabled")

    def test_local_ffmpeg_synthetic_clip_when_installed(self):
        ffmpeg = lane.find_ffmpeg()
        if ffmpeg is None:
            self.skipTest("local FFmpeg unavailable; no installation attempted")
        path = self.folder / "synthetic.mp4"
        command = [str(ffmpeg), "-nostdin", "-hide_banner", "-loglevel", "error", "-f", "rawvideo", "-pixel_format", "rgba",
                   "-video_size", "2x2", "-framerate", "24", "-i", "pipe:0", "-an", "-c:v", "libx264", "-pix_fmt", "yuv420p",
                   "-frames:v", "24", "-r", "24", "-t", "1", str(path)]
        env = check.scoped_env()
        run = subprocess.run(command, input=bytes([120, 130, 140, 255]) * 4 * 24,
                             stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=30, env=env)
        self.assertEqual(run.returncode, 0, run.stderr.decode(errors="replace"))
        self.assertEqual(lane.validate_clip(check, path, ffmpeg, 2, 2, 24, self.folder, env), 1)


if __name__ == "__main__":
    if sys.argv[1:2] == ["--gallery"]:
        root = Path(sys.argv[2]); width, height = int(sys.argv[3]), int(sys.argv[4])
        print("LOOKLAB_CAPTURE_GALLERY_PASS items=" + str(lane.verify_gallery(root, sys.argv[5:], width, height)))
    else:
        unittest.main()
