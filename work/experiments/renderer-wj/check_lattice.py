"""Separate, failing acceptance-item-2 check; never hides fixed-asset disagreement."""
import sys
sys.dont_write_bytecode = True
from fixture_data import wj
from lattice_wj import controls, check_frame

try:
    ctx = wj.sim.get_context()
    check_frame(ctx)
    controls(ctx)
except ValueError as error:
    print(f'W-J lattice agreement: fail ({error})', flush=True)
    raise SystemExit(1)
