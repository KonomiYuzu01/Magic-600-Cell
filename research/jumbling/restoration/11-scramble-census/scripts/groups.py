# Mov groups and K+ shape classes of a census residue, from the files run_fx.sh writes with stage 05 blocks.py and
# stage 06 census.py: the groups, those with more than one pose and their pieces, the frozen pieces, the distinct shape
# keys (census.py gives a group with several poses one key made from the keys of its parts), and the groups and pieces
# whose key starts with the shape of a stored template, as fxrun.py selects them. Run from the repository root:
#   python3 -I -B groups.py <blocks json> <census json> <templates json>
import sys, json
from collections import Counter

blocks = json.load(open(sys.argv[1]))
census = json.load(open(sys.argv[2]))
(name, st), = census['states'].items()
rows = st['blocks']
assert st['table_digest'] == blocks['table_digest'] and len(rows) == len(blocks['blocks'])
assert all(r['size'] == len(b['pieces']) for r, b in zip(rows, blocks['blocks']))
off = blocks['off']
assert sum(r['size'] for r in rows) == off
multi = [r for r in rows if r['poses'] > 1]
single = [r for r in rows if r['poses'] == 1]
keys = Counter(r['shape'] for r in rows)
assert keys == Counter(census['shape_multiplicity'])
tmpl = {}
for p in sorted({t['shape'] for t in json.load(open(sys.argv[3]))}):
    g = [r for r in rows if r['shape'].startswith(p)]
    tmpl[p] = {'groups': len(g), 'sizes': sorted({r['size'] for r in g}), 'pieces': sum(r['size'] for r in g),
               'single_pose': all(r['poses'] == 1 for r in g)}
pieces = sum(v['pieces'] for v in tmpl.values())
print(json.dumps({'state': name, 'table_digest': st['table_digest'], 'off': off, 'groups': len(rows),
                  'groups_with_several_poses': len(multi), 'pieces_in_them': sum(r['size'] for r in multi),
                  'frozen_pieces': sum(r['size'] for r in rows if r['frozen']),
                  'distinct_shape_keys': len(keys), 'distinct_keys_of_single_pose_groups': len({r['shape'] for r in single}),
                  'template_shapes': tmpl, 'pieces_in_template_shapes': pieces,
                  'share_of_off': round(pieces / off, 4)}))
