"""No-GPU acceptance: asset/frame identities, J1 snapshots, immutable references."""
from __future__ import annotations

import argparse
import shutil
import sys
import tempfile
import uuid
from pathlib import Path

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def main():
    sys.path.insert(0, str(ROOT / 'tools/perf'))
    import check_renderer_assets
    problems = check_renderer_assets.check()
    if problems:
        raise ValueError('; '.join(problems))
    print(check_renderer_assets.SUCCESS, flush=True)
    try:
        import numpy
    except ImportError:
        print('W-J: NumPy is required; use the pinned engine Python', flush=True)
        return 1
    from fixture_data import replay, write_exports, wj
    from lattice_wj import check_frame
    from reference_wj import build_reference
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--menu', choices=wj.MENUS, nargs='+', default=list(wj.MENUS))
    args = ap.parse_args()
    # Plain mkdir is required by the restricted Windows token; no mkdtemp.
    directory = Path(tempfile.gettempdir()) / ('magic600-wj-' + uuid.uuid4().hex)
    directory.mkdir()
    try:
        ctx = wj.sim.get_context()
        check_frame(ctx)
        for name in args.menu:
            fixture, snapshots, moving = replay(ctx, name)
            write_exports(directory, fixture, snapshots, moving)
            files = build_reference(directory, name)
            recomputed = directory / ('reference-' + name)
            recomputed.mkdir()
            for file, raw in files.items():
                (recomputed / file).write_bytes(raw)
                if (HERE / file).read_bytes() != (recomputed / file).read_bytes():
                    raise ValueError(f'{file}: committed reference differs byte for byte')
            print(f'{name}: six altered-fixture controls refused; {len(files)} reference files match byte for byte', flush=True)
        print('W-J acceptance: pass (source/fixture evidence; GPU checks and measurements remain owner-run)', flush=True)
        return 0
    finally:
        # This unique directory is the only target, including on failure.
        if directory.resolve().parent == Path(tempfile.gettempdir()).resolve():
            shutil.rmtree(directory)


if __name__ == '__main__':
    try:
        raise SystemExit(main())
    except (OSError, ValueError, AssertionError, KeyError) as error:
        print(f'W-J acceptance: fail ({error})', flush=True)
        raise SystemExit(1)
