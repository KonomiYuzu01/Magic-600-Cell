"""Acceptance-item-2 check: SPEC anchors must agree with the solved/retained lattice."""
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
