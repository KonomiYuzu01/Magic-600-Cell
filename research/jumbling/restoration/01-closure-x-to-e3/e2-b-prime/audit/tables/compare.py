"""Compare the report's 'Result' and 'Certificate chain' numbers with chk_indep.json / chk_sim.json (computed here)
and with c3/chain-check.json (the producer's record). Also: word-file lengths and the digest chain of the word
files X -> E1 -> E2 -> E3 and of the five parts. Writes compare.json next to this file.
"""
import json
from pathlib import Path

import numpy as np

SP = Path('<work>')
HERE = SP / 'v26/B4/audit/tables'
C3 = SP / 'v26/B4/c3'
mine = json.loads((HERE / 'chk_indep.json').read_text())
simr = json.loads((HERE / 'chk_sim.json').read_text())
cc = json.loads((C3 / 'chain-check.json').read_text())
sets = np.load(HERE / 'chk_indep_sets.npz')

E2SHA = '649576e69ede65a6203dfbebd3b879a68e40250abaf18a0f08ad86084834d893'
E2DIG = '19e404d438fd929b0c6480c35374aa5147b361831806774ff65ca0f4739bc5ff'
E3SHA = '53cec2e37f3b858924c216d45d99f1563f0f1e57d1d8a78fa39946c1c965d889'
E3DIG = 'fb199d9997a240c69ce91b39953d2434939f094f183f566067ca1779fe10c441'

rows = []


def row(claim, report, computed, chain=None):
    ok = report == computed and (chain is None or chain == computed)
    rows.append({'claim': claim, 'report': report, 'computed': computed, 'chain_check': chain, 'agree': ok})


t = mine['tables']
row('E2 sha256 (pkg27 D87/E2-table.json)', E2SHA, mine['sha256']['E2_copies']['pkg27/D87/E2-table.json'],
    cc['segment2_end_table_sha256'])
for k, v in mine['sha256']['E2_copies'].items():
    row(f'E2 copy sha256 {k}', E2SHA, v, cc['segment3_start_table_sha256'])
row('E2 digest (field, recomputed from content)', E2DIG, t['E2']['digest_recomputed'], cc['E2_digest'])
row('E2 digest field matches content', True, t['E2']['digest_field_matches_content'])
row('E2 off pieces = B\'', [16245, 16249, 32088], mine['E2_off_set'], cc['E2_off_set'])
row('E2 off count', 3, mine['off_counts']['E2'], cc['E2_off'])
row('B\' share one exact pose at E2', True, mine['Bprime_one_pose_at_E2'])
row('that pose outside K+ (closure / geometric)', False, mine['Bprime_pose_E2_in_Kplus'])
row('E2 holes', [24905, 89251, 89252], mine['chambers']['E2']['holes'], cc['E2_holes'])
row('E2 holes (simulator region transport)', [24905, 89251, 89252], simr['E2']['holes'])
row('B\' restored at E3 (in K+)', [True, True, True], mine['Bprime_E3_in_Kplus'])
row('B\' on K at E3 (chain-check)', True, all(mine['Bprime_E3_in_Kplus']), cc['Bprime_on_K_at_E3'])
row('B\' exact poses at E3', cc['Bprime_poses_at_E3'], mine['Bprime_poses_E3'])
row('E2 lattice pieces', 177117, mine['E2_E3']['E2_lattice'], cc['E2_lattice_pieces'])
row('E2 lattice on K at E3', 177117, mine['E2_E3']['E2_lattice_on_K_at_E3'])
row('E2 lattice at exact E2 pose at E3', 174026, mine['E2_E3']['E2_lattice_same_exact_pose'],
    cc['E2_lattice_pieces_same_exact_pose'])
row('collateral at end', 0, mine['E2_E3']['collateral_E2_lattice_off_at_E3'], cc['stages'][-1]['collateral_vs_E2'])
row('off at end', 0, mine['off_counts']['E3'], cc['E3_off'])
row('E3 all on K', True, mine['off_counts']['E3'] == 0, cc['E3_all_on_K'])
row('E3 digest (recomputed from content)', E3DIG, t['E3']['digest_recomputed'], cc['E3_digest'])
row('E3 digest field', E3DIG, t['E3']['digest_field'])
row('E3 sha256', E3SHA, mine['sha256']['E3'], cc['E3_table_sha256'])
row('E3 holes', 0, mine['chambers']['E3']['hole_count'], cc['E3_holes'])
row('lattice pieces with changed exact pose', 3091, mine['E2_E3']['lattice_moved'],
    cc['E2_lattice_pieces_with_changed_exact_pose'])
row('their ids equal chain-check ids', True,
    sets['moved3'].tolist() == cc['E2_lattice_pieces_with_changed_exact_pose_ids'])
row('of them on K at E3', True, mine['E2_E3']['lattice_moved_all_on_K_at_E3'])
row('of them change chamber', 3086, mine['E2_E3']['lattice_moved_changed_chamber'], cc['of_them_changed_chamber'])
row('E2->E3 changed', 3094, mine['changed_counts']['E2_to_E3'], cc['E2_to_E3_changed'])
row('E2->E3 changed = B\' + lattice moved', True, mine['E2_E3']['changed_minus_Bprime_equals_lattice_moved']
    and mine['E2_E3']['changed_includes_Bprime'])
row('E2->E3 changed all on K at E3', True, mine['E2_E3']['changed_all_on_K_at_E3'])
row('E1->E3 changed', 3151, mine['changed_counts']['E1_to_E3'], cc['E1_to_E3_changed'])
row('X->E3 changed', 3164, mine['changed_counts']['X_to_E3'], cc['X_to_E3_changed'])
row('87-block (E1 off minus B\') among the 3,091', 30, mine['blocks']['block87_in_lattice_moved'])
row('17-block (X off minus E1 off) among the 3,091', 4, mine['blocks']['block17_in_lattice_moved'])
row('E1->E3 = 90 + 3091 - 30', 3151, mine['decomposition']['E1_to_E3_formula'])
row('X->E3 = 107 + 3091 - 34', 3164, mine['decomposition']['X_to_E3_formula'])
row('E1->E3 set = E1 off U lattice moved', True, mine['decomposition']['E1_to_E3_equals_E1off_union_moved3'])
row('X->E3 set = X off U lattice moved', True, mine['decomposition']['X_to_E3_equals_Xoff_union_moved3'])
row('off counts X, E1, E2, E3', [107, 90, 3, 0], [mine['off_counts'][k] for k in ('X', 'E1', 'E2', 'E3')],
    [cc['X_off'], cc['E1_off'], cc['E2_off'], cc['E3_off']])
row('X digest (recomputed)', cc['X_digest'], t['X']['digest_recomputed'])
row('E1 digest (recomputed)', cc['E1_digest'], t['E1']['digest_recomputed'])
row('simulator off sets equal independent ones', [True] * 4, [simr[k]['off_equals_independent'] for k in ('X', 'E1', 'E2', 'E3')])
row('simulator chambers equal independent ones (E2, E3)', [True, True],
    [simr['E2']['chambers_equal_independent'], simr['E3']['chambers_equal_independent']])
row('simulator K+ = independent closure (7,200)', True, simr['sim_Kplus_equals_independent_closure'])

# word files
w1 = json.loads((SP / 'pkg26/Claude-E-closure-X1-20261012/D17/word-full.json').read_text())
w2 = json.loads((SP / 'pkg27/Claude-E-closure-E1E2-20261012/D87/word-full.json').read_text())
w3 = json.loads((C3 / 'word-full.json').read_text())
row('segment word lengths', [1112, 1210, 79], [len(w1['word']), len(w2['word']), len(w3['word'])])
row('word digest chain X->E1->E2->E3',
    [t['X']['digest_recomputed'], t['E1']['digest_recomputed'], t['E1']['digest_recomputed'], E2DIG, E2DIG, E3DIG],
    [w1['input_digest'], w1['output_digest'], w2['input_digest'], w2['output_digest'], w3['input_digest'],
     w3['output_digest']])
parts = [json.loads((C3 / 'parts' / f'{n}.json').read_text()) for n in ('V', 'P', 'Q', 'Pinv', 'Vinv')]
row('part lengths V, P, Q(F), Pinv, Vinv', [8, 31, 1, 31, 8], [len(p['word']) for p in parts])
row('parts concatenate to word-full', True, sum((p['word'] for p in parts), []) == w3['word'])
chain_ok = parts[0]['input_digest'] == E2DIG and parts[-1]['output_digest'] == E3DIG and all(
    parts[i]['output_digest'] == parts[i + 1]['input_digest'] for i in range(4))
row('part digests chain E2 -> E3', True, chain_ok)
row('F = (162,1)', [[162, 1]], parts[2]['word'])

out = {'rows': rows, 'all_agree': all(r['agree'] for r in rows), 'disagreements': [r for r in rows if not r['agree']]}
(HERE / 'compare.json').write_text(json.dumps(out, indent=1) + '\n')
for r in rows:
    print(('OK  ' if r['agree'] else 'DIFF'), r['claim'], '|', json.dumps(r['computed'])[:100])
print('all_agree', out['all_agree'])
