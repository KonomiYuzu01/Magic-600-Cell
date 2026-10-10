"""Owner-run export of the three W-J fixtures (one J1 replay per fixture)."""
import argparse
import sys
from pathlib import Path

sys.dont_write_bytecode = True
try:
    import numpy
except ImportError:
    raise SystemExit('W-J: NumPy is required; use the pinned engine Python')
from fixture_data import replay, write_exports, wj
from lattice_wj import check_frame, controls


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--out', type=Path, required=True)
    ap.add_argument('--menu', choices=wj.MENUS, nargs='+', default=list(wj.MENUS))
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    ctx = wj.sim.get_context()
    check_frame(ctx)
    controls(ctx, args.out, require_agreement=False)
    for name in args.menu:
        fixture, snapshots, moving = replay(ctx, name)
        write_exports(args.out, fixture, snapshots, moving)
    print('W-J exports: pass', flush=True)


if __name__ == '__main__':
    main()
