"""Produce the C-02 fixture once in a Linux cloud session, on the CPU.

Needs Python 3.12 or later. From a clean checkout in that Linux session (never
on the owner's PC):
  python3 -m venv /tmp/tastelab-reference
  /tmp/tastelab-reference/bin/python -m pip install torch torchvision --index-url https://download.pytorch.org/whl/cpu
  /tmp/tastelab-reference/bin/python -m pip install -r tools/python/tastelab-reference.txt
  PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=tools /tmp/tastelab-reference/bin/python -m tastelab.openclip_reference

Use --out PATH for another output file. No owner images, data, or captions are
read. The model downloads use only each lock entry's HTTPS network_hosts:
huggingface.co (both files), and us.aws.cdn.hf.co (weights redirects). Package
installation also needs download.pytorch.org, pypi.org and files.pythonhosted.org.
Proxies come from the environment, as a cloud session may require one. If the
session cannot reach huggingface.co, place both pinned files at their lock
`dest` paths under tools/.models/ first: an existing file is verified, never
downloaded again. Run time has not been measured; no performance claim is made.

OpenCLIP 3.3.0 is expected to accept a local safetensors pretrained path,
force_quick_gelu=False, precision='fp32', and device='cpu'. Its evaluation
Compose is expected to contain one Resize, CenterCrop and Normalize, and its
bundled ViT-B-16 tokenizer must produce 77 tokens. These assumptions are checked
during the cloud run; importing this module or checking versions needs no torch.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.metadata
import platform
import re
import sys
import tempfile
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
from uuid import uuid4

from tastelab import common

REQUIREMENTS = common.ROOT / "tools/python/tastelab.txt"
SHARED_PACKAGES = ("pillow", "numpy", "scipy", "ftfy", "regex")


def check_environment(versions, requirements):
    """Pure pin check: mappings and text only; no package imports or downloads."""
    expected = {}
    for name in SHARED_PACKAGES:
        matches = re.findall(r"^" + name + r"==([^\s\\]+)", requirements, flags=re.MULTILINE | re.IGNORECASE)
        if len(matches) != 1:
            raise common.Refused(f"missing or duplicate requirement pin: {name}")
        expected[name] = matches[0]
    expected["open_clip_torch"] = "3.3.0"
    for name, version in expected.items():
        if versions.get(name) != version:
            raise common.Refused(f"environment: {name} must equal {version}")
    return expected


def _check_url(url, hosts):
    try:
        parsed = urllib.parse.urlsplit(url)
        valid = (parsed.scheme == "https" and parsed.hostname in hosts and parsed.port in (None, 443)
                 and parsed.username is None and parsed.password is None)
    except ValueError:
        valid = False
    if not valid:
        raise common.Refused("model download: URL is outside the entry's HTTPS network_hosts")


class AllowedRedirect(urllib.request.HTTPRedirectHandler):
    def __init__(self, hosts):
        self.hosts = hosts

    def redirect_request(self, req, fp, code, msg, headers, newurl):
        _check_url(newurl, self.hosts)
        return super().redirect_request(req, fp, code, msg, headers, newurl)


def download_verified(pin, destination, *, opener=None):
    """Stream to a fresh partial file, verify, then rename; bad existing files refuse."""
    from tastelab import embed

    destination = common.check_input(destination, kind="model download")
    models = (common.ROOT / "tools/.models").resolve()
    temporary = Path(tempfile.gettempdir()).resolve()
    if not (destination.is_relative_to(models) or destination.is_relative_to(temporary)):
        raise common.Refused("model download: destination must be inside tools/.models/ or a temporary folder")
    hosts = pin["network_hosts"]
    _check_url(pin["url"], hosts)
    if destination.exists():
        embed.verify_file(destination, pin)
        return destination
    if opener is None:
        # Environment proxies only; no credentials, cookies or authentication handlers.
        opener = urllib.request.build_opener(AllowedRedirect(hosts))
    destination.parent.mkdir(parents=True, exist_ok=True)
    partial = destination.with_name(destination.name + ".part-" + uuid4().hex)
    try:
        request = urllib.request.Request(pin["url"], headers={"User-Agent": common.USER_AGENT})
        with opener.open(request, timeout=60) as response:
            _check_url(response.geturl(), hosts)
            with partial.open("xb") as output:
                size = 0
                while data := response.read(1024 * 1024):
                    size += len(data)
                    if size > pin["size"]:
                        raise common.Refused("model download: size exceeds the pinned file")
                    output.write(data)
        embed.verify_file(partial, pin)
        partial.replace(destination)
    finally:
        partial.unlink(missing_ok=True)
    return destination


class OpenClipAdapter:
    def __init__(self, weights, pins):
        # These imports happen only after the version/platform and file checks.
        import numpy as np
        import open_clip
        import torch
        from tastelab import clipref

        self.np, self.torch = np, torch
        self.pins = clipref.reference_pins(pins)
        self.model, _, self.transform = open_clip.create_model_and_transforms(
            "ViT-B-16", pretrained=str(weights), force_quick_gelu=False, precision="fp32", device="cpu")
        self.model.to(device="cpu", dtype=torch.float32).eval()
        modules = list(self.model.modules())
        if (not any(isinstance(m, torch.nn.GELU) for m in modules)
                or any(type(m).__name__ == "QuickGELU" or (isinstance(m, torch.nn.GELU) and m.approximate != "none") for m in modules)):
            raise common.Refused("upstream model must use standard, exact GELU")
        # Deliberately use OpenCLIP's vocabulary, independently of merges.txt.
        self.tokenizer = open_clip.get_tokenizer("ViT-B-16")

    def tokenize(self, texts):
        return self.tokenizer(texts).cpu().numpy().astype(self.np.int64)

    def preprocess(self, image):
        with self.torch.no_grad():
            return self.transform(image).cpu().numpy().astype(self.np.float32)

    def encode_images(self, pixels):
        with self.torch.no_grad():
            value = self.torch.from_numpy(self.np.ascontiguousarray(pixels, dtype=self.np.float32))
            return self.model.encode_image(value, normalize=False).cpu().numpy().astype(self.np.float32)

    def encode_texts(self, tokens):
        with self.torch.no_grad():
            value = self.torch.from_numpy(self.np.ascontiguousarray(tokens, dtype=self.np.int64))
            return self.model.encode_text(value, normalize=False).cpu().numpy().astype(self.np.float32)

    def producer(self, versions):
        from tastelab import clipref
        from torchvision.transforms import CenterCrop, Normalize, Resize

        def one(cls):
            matches = [t for t in self.transform.transforms if isinstance(t, cls)]
            if len(matches) != 1:
                raise common.Refused("unexpected upstream evaluation transform")
            return matches[0]

        resize, crop, norm = one(Resize), one(CenterCrop), one(Normalize)
        resize_size = [resize.size] if isinstance(resize.size, int) else list(resize.size)
        crop_size = [crop.size, crop.size] if isinstance(crop.size, int) else list(crop.size)
        return {"python": platform.python_version(), "platform": platform.system() + " " + platform.machine(),
                "torch": str(self.torch.__version__), "torchvision": importlib.metadata.version("torchvision"),
                "open_clip": versions["open_clip_torch"], "pillow": versions["pillow"], "numpy": versions["numpy"],
                "transform": {"resize_size": resize_size, "interpolation": resize.interpolation.value,
                              "crop_size": crop_size, "mean": clipref.encode_floats(norm.mean), "std": clipref.encode_floats(norm.std)}}


# URLs (a signed redirect) and absolute paths never reach the printed reason.
_SENSITIVE = re.compile(r"[a-z][a-z0-9+.-]*://\S+|[a-z]:[\\/]\S*|(?<![\w.~-])/\S+", re.IGNORECASE)


def describe(exc):
    """A diagnosable reason: the exception type and its message, with URLs and absolute paths replaced."""
    if isinstance(exc, urllib.error.HTTPError):
        text = f"HTTPError {exc.code}"
    elif isinstance(exc, urllib.error.URLError):
        reason = exc.reason
        detail = f"errno={reason.errno} {reason.strerror or ''}" if isinstance(reason, OSError) else str(reason)
        text = f"URLError {type(reason).__name__}: {detail}"
    elif isinstance(exc, OSError):
        text = f"{type(exc).__name__} errno={exc.errno} {exc.strerror or ''}"
    else:
        text = str(exc) if isinstance(exc, common.Refused) else f"{type(exc).__name__}: {exc}"
    return _SENSITIVE.sub("<redacted>", text)[:300].rstrip()


def main(argv=None):
    parser = argparse.ArgumentParser(description="Create the synthetic C-02 upstream reference on Linux CPU.")
    parser.add_argument("--out", type=Path, default=common.ROOT / "tests/fixtures/tastelab/openclip-reference.json")
    args = parser.parse_args(argv)
    if sys.version_info < (3, 12):
        print("refused: Python 3.12 or later is required", file=sys.stderr)
        return 2
    try:
        versions = {}
        for name in (*SHARED_PACKAGES, "open_clip_torch"):
            try:
                versions[name] = importlib.metadata.version(name)
            except importlib.metadata.PackageNotFoundError:
                versions[name] = None
        check_environment(versions, REQUIREMENTS.read_text(encoding="utf-8"))
        if platform.system() != "Linux":
            raise common.Refused("the upstream reference runs only in the Linux cloud session")
        from tastelab import clipref, embed

        if clipref.cases_digest(clipref.cases()) != clipref.CASES_DIGEST:
            raise common.Refused("synthetic cases differ from the committed digest")
        pins = embed.model_pins()
        files = {name: download_verified(pin, common.ROOT / pin["dest"]) for name, pin in pins.items()}
        upstream = OpenClipAdapter(files["clip-vit-b-16"], pins)
        fixture = clipref.build_fixture(upstream, upstream.producer(versions))
        raw = clipref.fixture_bytes(fixture)
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_bytes(raw)
        clipref.read_fixture(args.out)
        written = args.out.read_bytes()
        print(f"fixture sha256={hashlib.sha256(written).hexdigest()} bytes={len(written)}")
        return 0
    except (common.Refused, ValueError, OSError, AttributeError, ImportError, RuntimeError) as exc:
        print("refused: " + describe(exc), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
