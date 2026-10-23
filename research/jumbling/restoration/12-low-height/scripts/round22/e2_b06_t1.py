# Theory round 22, experiment E2: b06 (with b05) by X.W.V'.W^-1.V'^-1. X is SAFE-ALL at Z (e2_b06_prep1.py, then
# e2_b06_prep2.py), W = (91, i) lands b06 at Z.X and also turns the other off pieces inside cap 91 (free for tclose2:
# W^-1 returns them), and V' vacates Bad(W^-1) at Z1 = Z.X.W (a literal word from tclose2, else an R22B-10
# certificate built and checked at Z1 by replay_slp.vacancy_slp). The whole program is applied from Z by slp_apply.py
# --unit b06, which checks the strict' contract; the items are tried in the given order up to the first strict' one.
# One JSON line per item goes to <work dir>/seq.jsonl. Research only, headless; exact geometry.
# Usage (from the repository root): python3 -I -B research/jumbling/restoration/12-low-height/scripts/round22/e2_b06_t1.py
# <Z table> <prep dir> <new work dir> <item> [<item> ...] [--seeds atlas] [--literal-limit N]
import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path('research/jumbling/restoration/12-low-height/scripts/round22').resolve()))
import replay_slp as RS  # noqa: E402

S, TH, GM = RS.S, RS.TH, RS.GM
UNIT = Path('research/jumbling/restoration/12-low-height/runs/round22/line-a/units/eb5-k40-k00-b06.json')
J = Path('research/jumbling')


def off_at(table):
    """The pieces off the lattice in a pose table (a fresh simulator)."""
    if str(J / 'theory') not in sys.path:
        sys.path[:0] = [str(J / 'theory'), str(J), str(J / 'fixtures')]
    import buffer as B
    R, _ = B.O.start(str(table))
    return {int(p) for p in B.off_pieces(R.st)}


def main(argv=None):
    ap = argparse.ArgumentParser(description='Replay the prepared b06 T1-prime step.')
    ap.add_argument('start', type=Path)
    ap.add_argument('prep_dir', type=Path)
    ap.add_argument('work_dir', type=Path)
    ap.add_argument('items', type=int, nargs='+')
    RS.literal_options(ap)
    args = ap.parse_args(argv)
    if args.literal_limit < 0:
        ap.error('--literal-limit must be nonnegative')
    Z, P, wd = args.start, args.prep_dir, args.work_dir
    items = args.items
    wd.mkdir(parents=True, exist_ok=False)
    X, ZX, free = P / 'x-word.json', P / 'zx.json', P / 'free.json'
    z, zx = RS.digest(Z), RS.digest(ZX)
    assert json.loads(X.read_text())['input_digest'] == z and json.loads(X.read_text())['output_digest'] == zx
    nfree = len(json.loads(free.read_text())['pieces'])
    with open(wd / 'seq.jsonl', 'w') as out:
        for i in items:
            tag = f'b06-{i}'
            row = {'unit': 'b06', 'item': i, 'start_digest': z, 'zx_digest': zx, 'X': len(json.loads(X.read_text())['word'])}
            wf = wd / f'{tag}-W.json'
            wf.write_text(json.dumps({'word': [[91, i]], 'input_digest': zx}) + '\n')
            RS.run([S / 'round22/stepchk.py', ZX, UNIT, wf, wd / f'{tag}-W-check.json'], wd / f'{tag}-W-check.log')
            if not (wd / f'{tag}-W-check.json').exists():
                RS.emit(out, dict(row, how='none', reason='W is not admissible at Z.X'))
                continue
            chk = json.loads((wd / f'{tag}-W-check.json').read_text())
            row.update(Q=chk['collateral'], others_exact_after_W=chk['others_exact'])
            if chk['unit_on_lattice'] != chk['unit_size'] or chk['others_exact'] + chk['others_landed'] + nfree < chk['others']:
                RS.emit(out, dict(row, how='none', reason='W does not land b06 with the other off pieces exact, landed or free'))
                continue
            wf.write_text(json.dumps({'word': [[91, i]], 'input_digest': zx, 'output_digest': chk['end_digest']}) + '\n')
            RS.run([S / 'round22/prep_t1prime.py', ZX, wf, tag, wd], wd / f'{tag}-prep.log')
            z1t, qf, winv = wd / f'{tag}-z1-table.json', wd / f'{tag}-Q.json', wd / f'{tag}-winv.json'
            if not (z1t.exists() and qf.exists() and winv.exists()):
                RS.emit(out, dict(row, how='none', reason='prep_t1prime.py failed'))
                continue
            zt1 = RS.digest(z1t)
            # tclose2 reads the free pieces at Z1, so those that W lands (b05 for some items) leave the list
            fz1 = wd / f'{tag}-free.json'
            fz1.write_text(json.dumps({'pieces': sorted(set(json.loads(free.read_text())['pieces']) & off_at(z1t))}) + '\n')
            c2 = wd / f'{tag}-c2'
            RS.run([GM / 'gm/tclose2.py', TH, GM / 'echain', z1t, qf, winv, c2, '--free', fz1, '--greedy-seconds', 60],
                   wd / f'{tag}-c2.log', py=RS.PY[:2])
            if (c2 / 'word-full.json').exists():
                row.update(V='word')
                prog = {'segments': [{'word_file': str(X)}, {'word_file': str(wf)}, {'word_file': str(c2 / 'word-full.json')}]}
            else:
                if not (c2 / 'pre.json').exists():
                    RS.emit(out, dict(row, how='none', reason='tclose2.py wrote no premises'))
                    continue
                g = json.loads((c2 / 'pre.json').read_text())['ghost']
                row.update(ghost={k: v for k, v in g.items() if not isinstance(v, list)})
                if g['T_on_lattice_at_end'] != g['T_size'] or not g['protected_exact_at_end'] or not g['free_alive']:
                    RS.emit(out, dict(row, how='none', reason='ghost premises of W^-1 fail at Z1'))
                    continue
                import hashlib
                t = {'T': g['badW'], 'T_digest': hashlib.sha256(json.dumps(g['badW']).encode()).hexdigest()}
                cert = RS.vacancy_slp(f'{tag}-v', wd, z1t, zt1, t, row, seeds=args.seeds)
                if cert is None:
                    RS.emit(out, row)
                    continue
                row.update(V='certificate')
                prog = RS.vacating_program(cert, zt1, {'core_file': str(winv)}, tag, wd, row,
                                           prefix=[{'word_file': str(X)}, {'word_file': str(wf)}],
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
                RS.emit(out, dict(row, how='x-t1prime', letters=letters if args.literal_limit > 0 else str(letters),
                                  off_before=summary['off_before'],
                                  off_after=summary['off_after'], strict_prime=True, end_digest=summary['end_digest'],
                                  end_table=end.name))
                break
            keys = ('unit_on_lattice', 'collateral', 'others', 'others_exact', 'others_landed', 'error')
            RS.emit(out, dict(row, how='none', reason='not strict prime' if rc == 0 else 'slp_apply.py failed',
                              **{k: RS.safe_error(summary[k]) if k == 'error' else summary[k] for k in keys if k in summary}))


if __name__ == '__main__':
    main()
