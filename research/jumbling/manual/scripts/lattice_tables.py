# Prints the three tables of chapter 4, section 4.7, from repository data, and checks the facts the chapter states
# about them:
# - 4.7.1 the 35 orbits: counts from card 1, ranks from the theory report, places and seeds from round 7
#   (09-general-method/data/realign.json.gz);
# - 4.7.2 the correction library (09-general-method/data/library-merged.json.gz) and what each macro changes;
# - 4.7.3 for every bit of nu that can be 1, the library macros whose nu adds up to that bit alone (elimination over
#   F2; the library is a basis, so the combination is unique).
# Run from the repository root:  python3 -I -B research/jumbling/manual/scripts/lattice_tables.py
import gzip
import json
import sys
from pathlib import Path

REPO = Path.cwd()
GM = REPO / 'research/jumbling/restoration/09-general-method'
sys.path.insert(0, str(GM / 'scripts/lattice/scripts'))
from nucoords import C2F, vec  # noqa: E402

# research/Full_600cell_Puzzle_Theory.tex, the census of the 35 moving orbits
RANK = [1, 2, 3, 3, 4, 4, 4, 5, 5, 6, 7, 7, 8, 8, 8, 9, 9, 9, 10, 10, 10, 11, 11, 12, 13, 13, 14, 14, 14, 15, 15,
        16, 17, 18, 19]
# theory/cards/card-1-retained-group.md, stickers per piece k_o
STICKERS = [2, 1, 1, 2, 1, 1, 5, 1, 2, 2, 1, 2, 1, 1, 2, 1, 2, 5, 1, 1, 2, 1, 2, 2, 1, 2, 1, 1, 5, 1, 2, 1, 2, 1, 20]
# stage 04, Theorems A and B5: the coordinates that are 0 on every reachable lattice configuration
ZERO = {'b0', 'b1', 'b2', 'b3', 'b5', 'b6', 'f0', 'f3', 'f6'}
FRAMES = {'1': '1', 'C2': 'C2 (2)', 'C5': 'C5 (5)', 'D5': 'D5 (10)', 'A5': 'A5 (60)'}
NAMES = [f'b{o}' for o in range(35)] + [f'f{o}' for o in C2F]


def orbit_table():
    R = json.load(gzip.open(GM / 'data/realign.json.gz'))
    place = {o: i + 1 for i, o in enumerate(R['cascade_order'])}
    print('| Orbit | Rank | Stickers | Pieces | Frames | ν coordinates | Place | Seed (twists) |')
    print('| --- | ---: | ---: | ---: | --- | --- | ---: | --- |')
    total = 0
    for d in R['orbits']:
        o = d['orbit']
        co = [f'b{o}'] + ([f'f{o}'] if o in C2F else []) + ([f'C5_{o}'] if o in (17, 28) else [])
        co = ', '.join(f'{c} = 0' if c in ZERO else c for c in co)
        kind = 'kept' if not d['replaced'] else ('pure' if d['choice'].get('pure') else 'prefix')
        total += d['s']
        print(f"| O{o:02d} | {RANK[o]} | {STICKERS[o]} | {d['N']:,} | {FRAMES[str(d['H'])]} | {co} | {place[o]} "
              f"| {d['s']:,} {kind} |")
    assert sum(d['N'] for d in R['orbits']) == 176520
    assert 600 + sum(STICKERS[d['orbit']] * d['N'] for d in R['orbits']) == 259800
    assert total == 7024, total
    print()


def coords(m):
    b2, b5 = vec(m['nu'])
    return [NAMES[j] for j in range(43) if b2[j]], b5


def runs(cs):
    """Sign bits in runs of three or more as 'b16 to b18'."""
    out, run = [], []

    def flush():
        out.extend([f'b{run[0]} to b{run[-1]}'] if len(run) >= 3 else [f'b{x}' for x in run])
        run.clear()
    for c in cs:
        if c[0] == 'b':
            if run and int(c[1:]) != run[-1] + 1:
                flush()
            run.append(int(c[1:]))
        else:
            flush()
            out.append(c)
    flush()
    return ', '.join(out)


def library_tables():
    L = json.load(gzip.open(GM / 'data/library-merged.json.gz'))
    macros = L['macros']
    length = {m['id']: m['length'] for m in macros}
    print('| Id | Family | Twists | Changes |')
    print('| ---: | --- | ---: | --- |')
    bits, fives = [], []
    for m in macros:
        cs, b5 = coords(m)
        assert not (cs and any(b5)), m['id']                     # no macro mixes the two parts
        (bits if cs else fives).append(m['id'])
        change = runs(cs) if cs else ', '.join(f'C5_{o} by {b5[i]}' for i, o in enumerate((17, 28)) if b5[i])
        print(f"| {m['id']} | {m['family']} | {m['length']:,} | {change} |")
    print()
    # elimination over F2, keeping every basis row fully reduced, with its combination of library ids
    basis = []
    for k, i in enumerate(bits):
        v = sum(1 << j for j, x in enumerate(vec(macros[i]['nu'])[0]) if x)
        comb = 1 << k
        for p, bv, bc in basis:
            if v >> p & 1:
                v, comb = v ^ bv, comb ^ bc
        assert v, f'macro {i} is dependent'
        p = v.bit_length() - 1
        basis = [(q, bv ^ v, bc ^ comb) if bv >> p & 1 else (q, bv, bc) for q, bv, bc in basis] + [(p, v, comb)]
    assert len(basis) == 34
    free = {NAMES[p] for p, _, _ in basis}
    assert free == set(NAMES) - ZERO, sorted(set(NAMES) - ZERO - free)
    rows = {}
    for p, bv, bc in basis:
        assert bv == 1 << p
        rows[NAMES[p]] = [bits[k] for k in range(len(bits)) if bc >> k & 1]
    print('| Bit | Corrections | Twists |')
    print('| --- | --- | ---: |')
    for nm in NAMES:
        if nm in rows:
            print(f"| {nm} | {', '.join(map(str, rows[nm]))} | {sum(length[i] for i in rows[nm]):,} |")
    assert len(fives) == 2 and sum(length[i] for i in bits) + 2 * sum(length[i] for i in fives) == 25852
    print()
    print(f'checks: rank {len(basis)} over F2, C5 macros {fives}, bound '
          f'{sum(length[i] for i in bits)} + 2 x {sum(length[i] for i in fives)} = 25,852')


orbit_table()
library_tables()
