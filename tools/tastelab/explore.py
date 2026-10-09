"""Open the taste map in the marimo editor with marimo's own files kept private (tastelab environment).

  python tools/tastelab/explore.py [--data DIR]

marimo stores session caches (every cell output) and exports in a __marimo__
folder beside a notebook, so editing tools/tastelab/tastemap.py directly would
copy private report content, notes and thumbnails, into the source tree. While
PYTHONPYCACHEPREFIX is set, marimo mirrors that folder under the prefix instead;
this launcher points the prefix at <data>/marimo/, and outside script mode the
notebook shows nothing private unless the prefix lies inside the data folder
(mapping.cache_contained). The editor's online update check is switched off.
"""
from __future__ import annotations

import argparse
import os
import subprocess
import sys
from pathlib import Path

if not __package__:
    sys.path[0] = str(Path(__file__).resolve().parents[1])

from tastelab import common

NOTEBOOK = common.PACKAGE / "tastemap.py"
CACHE_FOLDER = "marimo"


def command(root: Path) -> tuple[list[str], dict]:
    """The editor's command line and environment for the data folder `root`."""
    argv = [sys.executable, "-m", "marimo", "edit", "--skip-update-check", str(NOTEBOOK), "--", "--data", str(root)]
    env = dict(os.environ, PYTHONPYCACHEPREFIX=str(root / CACHE_FOLDER), MARIMO_SKIP_UPDATE_CHECK="1")
    return argv, env


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description="Open the private taste map in the marimo editor.")
    parser.add_argument("--data", help="private Taste Lab data folder")
    args = parser.parse_args(argv)
    root = common.data_root(args.data)
    cmd, env = command(root)
    (root / CACHE_FOLDER).mkdir(parents=True, exist_ok=True)
    return subprocess.call(cmd, env=env)


if __name__ == "__main__":
    sys.exit(main())
