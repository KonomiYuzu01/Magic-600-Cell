"""Stage 18 (tail completion): a whole-cap step X.W.V'.W^-1.V'^-1 at any pose table, W = (grip, item) one letter.

E2's builder (stage 12, scripts/round22/e2_b06_t1.py) with the grip and the unit as arguments. X is a SAFE-ALL word at
Z given in a preparation directory (x-word.json, its end table zx.json and free.json, the other off pieces inside the
cap that W turns and W^-1 returns); an empty X means the unprepared step. W = (grip, item) must land the unit at Z.X;
V' vacates Bad(W^-1) at Z1 = Z.X.W, a literal word from tclose2.py, else an R22B-10 certificate built and checked at
Z1 and expanded into one literal word (replay_slp.vacancy_slp and vacating_program). The whole program is applied
from Z by slp_apply.py --unit, which checks the strict' contract; the items are tried in the given order up to the
first strict' one. One JSON line per item goes to <work dir>/seq.jsonl. Research only, headless; exact geometry.
Usage (from the repository root): python3 -I -B <this script> <Z table> <prep dir> <new work dir> <grip> <unit file>
<item> [<item> ...] [--seeds atlas] [--literal-limit N]
"""
import argparse
import hashlib
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path('research/jumbling/restoration/12-low-height/scripts/round22').resolve()))
import replay_slp as RS  # noqa: E402

S, TH, GM = RS.S, RS.TH, RS.GM
J = Path('research/jumbling')


def off_at(table):
    """The pieces off the lattice in a pose table (a fresh simulator)."""
    if str(J / 'theory') not in sys.path:
        sys.path[:0] = [str(J / 'theory'), str(J), str(J / 'fixtures')]
    import buffer as B
    R, _ = B.O.start(str(table))
    return {int(p) for p in B.off_pieces(R.st)}


def main(argv=None):
    ap = argparse.ArgumentParser(description='A whole-cap T1-prime step at a pose table.')
    ap.add_argument('start', type=Path)
    ap.add_argument('prep_dir', type=Path)
    ap.add_argument('work_dir', type=Path)
    ap.add_argument('grip', type=int)
    ap.add_argument('unit', type=Path)
    ap.add_argument('items', type=int, nargs='+')
    RS.literal_options(ap)
    args = ap.parse_args(argv)
    if args.literal_limit < 0:
        ap.error('--literal-limit must be nonnegative')
    Z, P, wd, c, UNIT = args.start, args.prep_dir, args.work_dir, args.grip, args.unit
    wd.mkdir(parents=True, exist_ok=False)
    X, ZX, free = P / 'x-word.json', P / 'zx.json', P / 'free.json'
    z, zx = RS.digest(Z), RS.digest(ZX)
    xw = json.loads(X.read_text())
    assert xw['input_digest'] == z and xw['output_digest'] == zx
    nfree = len(json.loads(free.read_text())['pieces'])
    with open(wd / 'seq.jsonl', 'w') as out:
        for i in args.items:
            tag = f'g{c}-{i}'
            row = {'grip': c, 'item': i, 'start_digest': z, 'zx_digest': zx, 'X': len(xw['word'])}
            wf = wd / f'{tag}-W.json'
            wf.write_text(json.dumps({'word': [[c, i]], 'input_digest': zx}) + '\n')
            RS.run([S / 'round22/stepchk.py', ZX, UNIT, wf, wd / f'{tag}-W-check.json'], wd / f'{tag}-W-check.log')
            if not (wd / f'{tag}-W-check.json').exists():
                RS.emit(out, dict(row, how='none', reason='W is not admissible at Z.X'))
                continue
            chk = json.loads((wd / f'{tag}-W-check.json').read_text())
            row.update(Q=chk['collateral'], others_exact_after_W=chk['others_exact'])
            if chk['unit_on_lattice'] != chk['unit_size'] or chk['others_exact'] + chk['others_landed'] + nfree < chk['others']:
                RS.emit(out, dict(row, how='none', reason='W does not land the unit with the other off pieces exact, landed or free'))
                continue
            wf.write_text(json.dumps({'word': [[c, i]], 'input_digest': zx, 'output_digest': chk['end_digest']}) + '\n')
            RS.run([S / 'round22/prep_t1prime.py', ZX, wf, tag, wd], wd / f'{tag}-prep.log')
            z1t, qf, winv = wd / f'{tag}-z1-table.json', wd / f'{tag}-Q.json', wd / f'{tag}-winv.json'
            if not (z1t.exists() and qf.exists() and winv.exists()):
                RS.emit(out, dict(row, how='none', reason='prep_t1prime.py failed'))
                continue
            zt1 = RS.digest(z1t)
            fz1 = wd / f'{tag}-free.json'
            fz1.write_text(json.dumps({'pieces': sorted(set(json.loads(free.read_text())['pieces']) & off_at(z1t))}) + '\n')
            prefix = ([{'word_file': str(X)}] if xw['word'] else []) + [{'word_file': str(wf)}]
            c2 = wd / f'{tag}-c2'
            RS.run([GM / 'gm/tclose2.py', TH, GM / 'echain', z1t, qf, winv, c2, '--free', fz1, '--greedy-seconds', 60],
                   wd / f'{tag}-c2.log', py=RS.PY[:2])
            if (c2 / 'word-full.json').exists():
                row.update(V='word')
                prog = {'segments': [*prefix, {'word_file': str(c2 / 'word-full.json')}]}
            else:
                if not (c2 / 'pre.json').exists():
                    RS.emit(out, dict(row, how='none', reason='tclose2.py wrote no premises'))
                    continue
                g = json.loads((c2 / 'pre.json').read_text())['ghost']
                row.update(ghost={k: v for k, v in g.items() if not isinstance(v, list)})
                if g['T_on_lattice_at_end'] != g['T_size'] or not g['protected_exact_at_end'] or not g['free_alive']:
                    RS.emit(out, dict(row, how='none', reason='ghost premises of W^-1 fail at Z1'))
                    continue
                t = {'T': g['badW'], 'T_digest': hashlib.sha256(json.dumps(g['badW']).encode()).hexdigest()}
                cert = RS.vacancy_slp(f'{tag}-v', wd, z1t, zt1, t, row, seeds=args.seeds)
                if cert is None:
                    RS.emit(out, row)
                    continue
                row.update(V='certificate')
                prog = RS.vacating_program(cert, zt1, {'core_file': str(winv)}, tag, wd, row, prefix=prefix,
                                           literal_limit=args.literal_limit)
                if prog is None:
                    RS.emit(out, row)
                    continue
            pf = wd / f'{tag}-program.json'
            pf.write_text(json.dumps(prog) + '\n')
            end = wd / f'z-{tag}.json'
            rc = RS.run([S / 'slp_apply.py', 'table', Z, pf, end, '--unit', UNIT], wd / f'{tag}-apply.log')
            rows = RS.json_lines(wd / f'{tag}-apply.log')
            summary = rows[-1] if rows else {}
            if rc == 0 and summary.get('strict_prime'):
                letters = sum(int(r['length']) for r in rows if 'segment' in r)
                RS.emit(out, dict(row, how='x-t1prime' if xw['word'] else 't1prime-cap',
                                  letters=letters if args.literal_limit > 0 else str(letters),
                                  off_before=summary['off_before'], off_after=summary['off_after'], strict_prime=True,
                                  end_digest=summary['end_digest'], end_table=end.name))
                break
            keys = ('unit_on_lattice', 'collateral', 'others', 'others_exact', 'others_landed', 'error')
            RS.emit(out, dict(row, how='none', reason='not strict prime' if rc == 0 else 'slp_apply.py failed',
                              **{k: RS.safe_error(summary[k]) if k == 'error' else summary[k] for k in keys if k in summary}))


if __name__ == '__main__':
    main()
