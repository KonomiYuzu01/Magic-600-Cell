"""Audit 'shrink': compare the recomputed level records with shrink-237-13.jsonl and the report's level table.

Usage: python3 -I -B compare.py <out json>
"""
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
S = Path('<work>')
A = S / 'v26/B4/audit/shrink'
TH = S / 'wt-e3/research/jumbling/theory'
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))

import numpy as np  # noqa: E402

import blocks as BL  # noqa: E402

O, CL = BL.O, BL.CL
kp, reg = CL.ctx.kplus, CL.ctx.regions

REPORT = {  # report level table and text
    'P_length': [1, 3, 7, 15, 31], 'parked': [3097, 134, 80, 14, 10], 'T': [3097, 134, 80, 14, 10],
    'BH': [3094, 131, 77, 11, 7], 'min_bh_in_cap': [332, 62, 11, 4, 0], 'F_count': [0, 0, 0, 0, 55],
    'x': [[91, 6], [154, 7], [133, 1], [183, 6], None], 'T_straddle': [0, 0, 0, 0, 0],
}


def main():
    out = Path(sys.argv[1])
    lv = [json.loads((A / f'level{k}.json').read_text()) for k in range(5)]
    gh = {}
    for f in ('ghost012.json', 'ghost34.json'):
        gh.update({int(k): v for k, v in json.loads((A / f).read_text()).items()})
    js = [json.loads(line) for line in (S / 'v26/B4/shrink-237-13.jsonl').read_text().splitlines() if line.strip()]
    R, M = O.start(str(S / 'v26/B4/E2-table.json'))
    st = R.st
    K0 = st._kpose()[st.pose_id]
    NP = len(K0)
    lat = np.flatnonzero(K0 >= 0)
    ch0 = np.full(NP, -1, np.int64)
    ch0[lat] = reg.piece_of[reg.orbit_of[lat], kp.compose_vec(K0[lat], reg.transport[lat])]
    occ0 = np.zeros(NP, bool)
    occ0[ch0[lat]] = True
    res = {'levels': [], 'mismatches': []}

    def chk(name, got, want, k):
        ok = got == want
        if not ok:
            res['mismatches'].append({'level': k, 'item': name, 'got': got, 'want': want})
        return ok

    for k in range(5):
        r, j, g = lv[k], js[k], gh[k]
        row = {'level': k}
        row['P_len'] = len(r['P'])
        chk('P_length/report', len(r['P']), REPORT['P_length'][k], k)
        chk('P/jsonl', r['P'], j['P'], k)
        chk('after_P_digest/jsonl', r['after_P_digest'], j['after_P_digest'], k)
        chk('P_all_admissible', r['P_all_admissible'], True, k)
        chk('undo_back_to_E2', r['undo_back_to_E2'], True, k)
        for key in ('parked', 'T', 'BH'):
            chk(f'{key}/report', r[key], REPORT[key][k], k)
            chk(f'{key}/jsonl', r[key], j[key], k)
        chk('holes/jsonl', r['holes'], j['holes'], k)
        chk('holes=T', r['holes'], r['T'], k)
        chk('bprime_on', r['bprime_on'], 3, k)
        chk('bprime_on/jsonl', r['bprime_on'], j['bprime_on'], k)
        chk('T_straddle/report', r['T_straddle'], REPORT['T_straddle'][k], k)
        chk('T_straddle/jsonl', r['T_straddle'], j['T_straddle'], k)
        chk('L/jsonl', r['L'], j['L'], k)
        chk('L_in_T', r['L_in_T'], 3, k)
        chk('GH/jsonl', r['GH'], j['GH'], k)
        chk('GH=E2 holes', r['GH'], r['E2_holes'], k)
        chk('T_occupied_not_L', r['T_occupied_not_L'], 0, k)
        chk('alphabet_holes_match', r['alphabet_holes_match'], True, k)
        chk('chamber_table_matches_signature_images', r['chamber_table_matches_signature_images'], True, k)
        for key in ('safe_grips', 'letters', 'grips_holding_L', 'candidates', 'min_bh_in_cap'):
            chk(f'{key}/jsonl', r[key], j[key], k)
        chk('min_bh_in_cap/report', r['min_bh_in_cap'], REPORT['min_bh_in_cap'][k], k)
        chk('F_letters/jsonl', r['F_letters'], j['F_letters'], k)
        chk('F_count/report', len(r['F_letters']), REPORT['F_count'][k], k)
        # independent ghost against ghostlib
        chk('T_ids=own ghost Bad(P^-1)', r['T_ids'], g['Bad_Pinv'], k)
        chk('own ghost dead under P^-1', len(g['dead_Pinv']), 0, k)
        # Bad(P) at E2: parked pieces are exactly the lattice pieces of E2 in Bad(P) chambers; E2 holes not in Bad(P)
        badP = np.array(g['Bad_P'], np.int64)
        parked_ch = np.sort(ch0[np.array(r['parked_ids'], np.int64)])
        chk('parked chambers = Bad(P) cap occupied(E2)', parked_ch.tolist(), np.sort(badP[occ0[badP]]).tolist(), k)
        chk('E2 holes outside Bad(P)', int((~occ0[badP]).sum()), 0, k)
        chk('|Bad(P)|=|Bad(P^-1)|', len(g['Bad_P']), len(g['Bad_Pinv']), k)
        # best list against jsonl
        mine = [{'letter': b['letter'], 'cap_size': b['cap'], 'L_in_cap': b['L_in_cap'], 'bh_in_cap': b['bh_in_cap'],
                 'bh_out': b['bh_out'], 'L_out': b['L_out'], 'enter_pieces': b['enter'],
                 'kill_violations': b['kill_viol'], 'F': b['F']} for b in r['best'][:len(j['best'])]]
        chk('best/jsonl', mine, j['best'], k)
        # all candidates: min bad holes over caps holding ALL of L (header reading) vs any chamber of L
        allL = [c for c in r['all_candidates'] if c['L_in_cap'] == 3]
        row['min_bh_cap_holding_all_L'] = min((c['bh_in_cap'] for c in allL), default=None)
        row['grips_holding_all_L'] = r['grips_holding_all_L']
        row['ok_count'] = r['ok_count']
        if k < 4:
            chk('x first/report', r['x'], REPORT['x'][k], k)
            chk('x is SAFE-ALL letter', r['x_is_safe_letter'], True, k)
            chk('x is least by (enter, bh_out, grip, index)', r['x_is_first'], True, k)
            xr = r['x_row']
            chk('x L_in_cap', xr['L_in_cap'], 3, k)
            chk('x L_out', xr['L_out'], 3, k)
            chk('x kill_viol', xr['kill_viol'], 0, k)
            chk('x enter/jsonl', xr['enter'], j['best'][0]['enter_pieces'], k)
            chk('x bh_out/jsonl', xr['bh_out'], j['best'][0]['bh_out'], k)
            chk('Pnext admissible', r['Pnext_all_admissible'], True, k)
            chk('Pnext bprime_on', r['Pnext_bprime_on'], 3, k)
            chk('step set = alphabet entering set', r['step_set_equal_alpha'], True, k)
            chk('step set = simulator set in T after x', r['step_set_equal_sim'], True, k)
            chk('step count = 3 + bh_out', r['step_count_3_plus_bh_out'], True, k)
            chk('Pnext parked = next level parked', r['Pnext_parked_ids'], lv[k + 1]['parked_ids'], k)
            chk('Pnext digest = next after_P digest', r['Pnext_digest'], lv[k + 1]['after_P_digest'], k)
            chk('x good_holes_in', xr['good_holes_in'], 0, k)
            chk('after x B prime outside T', r['after_x_Bprime_outside_T'], True, k)
            chk('after x parked unchanged', r['after_x_parked_unchanged'], True, k)
            row.update({'x': r['x'], 'x_enter': xr['enter'], 'x_bh_out': xr['bh_out'], 'x_bh_in_cap': xr['bh_in_cap'],
                        'x_ties_first_keys': r['x_ties_first_keys'], 'enter_ids': r['enter_alpha_ids'],
                        'Pnext_parked': r['Pnext_parked']})
        else:
            chk('F grips', r['F_grips'], [162, 183, 260, 290, 302], k)
            chk('F bh_in_cap all zero', r['F_bh_in_cap'], [0], k)
            chk('F least by (grip,index)', min(tuple(x) for x in r['F_letters']), (162, 1), k)
            chk('F least by greedy key', r['best'][0]['letter'], [162, 1], k)
            fl = [c for c in r['all_candidates'] if c['F']]
            chk('F all L_out 3, enter 3', sorted({(c['L_out'], c['enter']) for c in fl}), [(3, 3)], k)
            row.update({'F_count': len(r['F_letters']), 'F_grips': r['F_grips'], 'F_bh_in_cap': r['F_bh_in_cap'],
                        'W_parked_ids': r['W_parked_ids'], 'W_bprime_on': r['W_bprime_on'],
                        'afterF_on_K_in_T_ids': r['afterF_on_K_in_T_ids']})
        row.update({key: r[key] for key in ('parked', 'T', 'T_straddle', 'holes', 'BH', 'GH', 'L', 'min_bh_in_cap',
                                            'min_bh_grips', 'safe_grips', 'letters', 'candidates')})
        row['F_count'] = len(r['F_letters'])
        res['levels'].append(row)
    sg = json.loads((A / 'single.json').read_text())
    res['single'] = {key: sg[key] for key in ('grips_any_in', 'grips_all_in', 'grips_straddled_none_in',
                                               'grips_straddled_total', 'letters_screened', 'n_hits',
                                               'hits_equal_claim', 'hits_equal_singleP_json', 'digest_restored')}
    res['single']['parked_values'] = sorted({a['parked'] for a in sg['applied']})
    res['single']['parked_equals_lattice_inside_cut'] = all(a['parked_equals_lattice_inside_cut'] for a in sg['applied'])
    res['single']['bprime_on_values'] = sorted({a['bprime_on'] for a in sg['applied']})
    res['single']['em_in_K_letters_total'] = len(sg['letters_em_in_K_any'])
    out.write_text(json.dumps(res, indent=1) + '\n')
    print('mismatches:', json.dumps(res['mismatches'], indent=1))
    for row in res['levels']:
        print({x: y for x, y in row.items() if x not in ('enter_ids', 'L', 'GH')})
    print(res['single'])


if __name__ == '__main__':
    main()
