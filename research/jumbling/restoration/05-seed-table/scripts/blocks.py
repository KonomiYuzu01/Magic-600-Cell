"""Glued-block certificate (Lemma G) from an exact cap-set file of capsets.py (menu I_a).

For the off pieces O of a state: a grip is blocked when some off piece straddles its cap (no twist there is
admissible while that piece stays). Mov(p) = the unblocked grips whose closed cap contains p. Pieces with identical
Mov sets form a block; within a block every unblocked grip that contains one piece contains all of them, which is the
hypothesis of Lemma G. A block of size >= 2 cannot lose any piece to a word that keeps every other off piece fixed,
and a piece with empty Mov cannot move at all before some other off piece moves.
Checks done here (pure set arithmetic on the exact cap sets): the Lemma G hypothesis for every block, and the
partition. Optional: for each k-seed (kseeds.py rows, conjugator m = identity) the off pieces outside the block that
meet the caps of its grips.
usage: blocks.py <capsets json> <out json> [<kseeds json> <comma-separated block pieces>]"""
import json
import sys
from collections import Counter, defaultdict

cap = json.load(open(sys.argv[1]))
rows = {r['piece']: r for r in cap['pieces']}
blocked = sorted({c for r in rows.values() for c in r['straddle']})
bset = set(blocked)
mov = {p: tuple(sorted(set(r['in']) - bset)) for p, r in rows.items()}
groups = defaultdict(list)
for p, m in mov.items():
    groups[m].append(p)
blocks = []
for m, ps in sorted(groups.items(), key=lambda kv: -len(kv[1])):
    # Lemma G hypothesis: every unblocked grip containing a piece of the block contains all of it
    hyp = all(set(m) <= set(rows[p]['in']) for p in ps) and all(set(rows[p]['in']) - bset == set(m) for p in ps)
    blocks.append({'size': len(ps), 'movers': list(m), 'glued': len(ps) >= 2 and len(m) > 0,
                   'frozen': len(m) == 0, 'lemma_G_hypothesis': hyp, 'pieces': sorted(ps)})
assert sum(b['size'] for b in blocks) == len(rows)
out = {'table_digest': cap['table_digest'], 'off': len(rows), 'blocked_grips': blocked,
       'block_sizes': Counter(b['size'] for b in blocks),
       'pieces_in_glued_blocks': sum(b['size'] for b in blocks if b['glued']),
       'pieces_frozen': sum(b['size'] for b in blocks if b['frozen']),
       'free_singletons': sum(1 for b in blocks if b['size'] == 1 and not b['frozen']),
       'lemma_G_hypothesis_all': all(b['lemma_G_hypothesis'] for b in blocks), 'blocks': blocks}
if len(sys.argv) > 4:
    blk = {int(x) for x in sys.argv[4].split(',')}
    out['kseed_conflicts'] = []
    for ks in json.load(open(sys.argv[3])):
        g = set(ks['grips'])
        hit = {p: sorted((set(r['in']) | set(r['straddle'])) & g) for p, r in rows.items() if p not in blk}
        hit = {p: v for p, v in hit.items() if v}
        out['kseed_conflicts'].append({'kseed': ks['name'], 'grips': sorted(g), 'other_pieces_meeting_caps': len(hit),
                                       'grips_met': sorted({c for v in hit.values() for c in v})})
out['block_sizes'] = {str(k): v for k, v in sorted(out['block_sizes'].items())}
json.dump(out, open(sys.argv[2], 'w'), indent=1)
print(json.dumps({k: v for k, v in out.items() if k != 'blocks'}))
for b in blocks[:6]:
    print({k: v for k, v in b.items() if k != 'pieces'})
