"""Compare the I_a menu that O.start takes from the W-J fixture record with the menu built from the theory code alone
(restore.menu -> wj.make_menu -> groups.build): name, identity hash, and every item (label and exact matrix) in order.

Usage: python3 -I -B menu_compare.py <out json>
"""
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
SP = Path('<work>')
TH = SP / 'wt-e3' / 'research' / 'jumbling' / 'theory'
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))

import restore  # noqa: E402
import wj  # noqa: E402


def main():
    _, Mf = wj.load(restore.ctx, 'I_a')     # the menu record O.start uses (seams._fixture_state)
    Mm = restore.menu('I_a')                # built from the theory code, no fixture file
    same_items = len(Mf.items) == len(Mm.items) and all(a[0] == b[0] and a[1] == b[1] for a, b in zip(Mf.items, Mm.items))
    res = {'fixture_menu': {'name': Mf.name, 'items': len(Mf.items), 'identity': Mf.identity},
           'built_menu': {'name': Mm.name, 'items': len(Mm.items), 'identity': Mm.identity},
           'same_name': Mf.name == Mm.name, 'same_identity': Mf.identity == Mm.identity,
           'same_items_in_order': same_items}
    Path(sys.argv[1]).write_text(json.dumps(res, indent=1) + '\n')
    print(json.dumps(res))


if __name__ == '__main__':
    main()
