"""Re-derive every frozen table and require independent computations to agree."""
from fractions import Fraction as Q
from hashlib import sha256
import json
from math import gcd
from pathlib import Path

import b_oracle
import certify
import cyclotomic as cy
import intervals as iv
import option_b as b
import oracle
from q5 import matrix_record

HERE = Path(__file__).resolve().parent


def json_bytes(value):
    return (json.dumps(value, indent=2, sort_keys=True) + '\n').encode('ascii')


def row_key(row):
    return ','.join(map(str, row))


def tsv(header, rows):
    return ('\t'.join(header) + '\n' + ''.join('\t'.join(map(str, row)) + '\n' for row in rows)).encode('ascii')


def case_tables(inputs):
    degrees = iv.binary64(inputs['degree_bits'])
    radians = iv.binary64(inputs['radian_bits'])
    assert degrees == b_oracle.decode_float_bits(inputs['degree_bits'])
    assert radians == b_oracle.decode_float_bits(inputs['radian_bits'])
    # Capture is separate from proof. Verify the stored decimal->binary64 and
    # radians multiplication roundings by exact adjacent-value midpoints.
    for source, bits in ((Q(inputs['degree_literal']), inputs['degree_bits']),
                         (degrees * iv.binary64(inputs['radians_per_degree_bits']), inputs['radian_bits'])):
        u = int(bits, 16)
        lo, mid, hi = (iv.binary64(f'{v:016x}') for v in (u - 1, u, u + 1))
        assert (lo + mid) / 2 < source < (mid + hi) / 2
    primary = certify.case_certificate(degrees, radians)
    independent = oracle.case_certificate(degrees, radians)
    assert primary == independent, 'rational certificate routes disagree'
    rows, largest = [], 0
    for axis in inputs['axes']:
        winner = (1, *axis, 'C')
        second_bits = oracle.verify_case_global(degrees, radians, axis, winner)
        for semantics, angle in (('exact_degrees', None), ('binary64_radians', radians)):
            best, ties, bits = certify.choose(degrees, axis, angle)
            assert best == winner and ties == [winner]
            largest = max(largest, bits, second_bits)
            rows.append([','.join(map(str, axis)), semantics, row_key(best), 432, bits, second_bits])
    primary['degree_dyadic'] = str(degrees)
    primary['radian_dyadic'] = str(radians)
    return {'case.json': json_bytes(primary),
            'case_axes.tsv': tsv(['axis', 'semantics', 'winner', 'candidates', 'primary_bits', 'oracle_bits'], rows)}, largest


def angle_tables():
    orders, angles = cy.exceptional_orders(), cy.exceptional_angles()
    assert (orders, angles) == oracle.other_exceptional_angles()
    assert all(len(cy.phi_polynomial(n)) - 1 == sum(gcd(j, n) == 1 for j in range(1, n + 1)) for n in orders)
    rows, largest = [], 0
    for turns in angles:
        best, ties, bits = certify.choose(360 * turns)
        second, other_ties, other_bits = oracle.brute_exact(turns)
        assert (best, ties) == (second, other_ties), ('exceptional-angle mismatch', str(turns))
        largest = max(largest, bits, other_bits)
        rows.append([str(360 * turns), turns.numerator, turns.denominator, row_key(best),
                     ';'.join(map(row_key, ties)), 458 if turns == Q(1, 2) else 432, bits, other_bits])
    # The named tie is global; all remaining candidates are strictly farther.
    left, right = (2, 1, 0, 0, 'C'), (3, 1, 0, 0, 'C')
    delta = certify.subtract(certify.coeff(left), certify.coeff(right))
    assert delta == (Q(0), Q(-1, 10), Q(1, 10))
    assert not any(cy.exact_linear(delta, Q(1, 8)))
    assert certify.choose(Q(45))[:2] == (left, [left, right])
    probes = []
    for bits in (16, 32, 64, 128, 256):
        first = iv.linear(delta, *iv.degree_trig(Q(45), bits))
        s, c = oracle.angle_trig(Q(1, 16), bits)
        x = oracle.factor(oracle.square(oracle.plus(oracle.factor(c, 2), s)), Q(1, 5))
        y = oracle.factor(oracle.square(oracle.plus(oracle.factor(c, 3), s)), Q(1, 10))
        second = oracle.plus(x, oracle.factor(y, -1))
        assert first[0] < 0 < first[1] and second[0] < 0 < second[1]
        probes.append({'bits': bits, 'coefficient_route': iv.text(first),
                       'quaternion_route': iv.text(second), 'exact_difference': '0'})
    # Distinct rotations with identical scores at every angle: coefficient-zero branch.
    universal = certify.subtract(certify.coeff((1, 0, 1, 0, 'C')),
                                 certify.coeff((1, 0, -1, 0, 'C')))
    assert universal == (0, 0, 0)
    f = cy.Field(4)
    assert f.mul(f.power(1), f.power(1)) == f.scale(f.one, -1)
    radical_cases = [
        # Independently evaluate these with sin(45)=sqrt(2)/2 or sin(90)=1.
        (Q(1, 8), (1, 0), (0, 0), (0, 0), (-1, 0), Q(2), True),
        (Q(1, 4), (1, 0), (0, 0), (0, 0), (1, 0), Q(1), False),
        (Q(1, 4), (1, 0), (0, 0), (0, 0), (-1, 0), Q(7), False),
        (Q(1, 4), (0, 1), (0, 0), (0, 0), (-1, 0), Q(5), True),
    ]
    radical_rows = []
    for *args, expected in radical_cases:
        actual = cy.radical_zero(*args)
        assert actual == expected
        radical_rows.append({'turns': str(args[0]), 'coefficients': args[1:5],
                             'q': str(args[5]), 'zero': actual})
    summary = {'orders': orders, 'angles_modulo_360': len(angles), 'fixed_field_conductor': 1440,
               'fixed_field_degree': len(cy.phi_polynomial(1440)) - 1,
               'degree_upper_bound': 16, 'largest_deciding_bits': largest,
               'identically_zero_coefficients': list(map(str, universal)), 'radical_guard_cases': radical_rows}
    return {'exceptional.tsv': tsv(['degrees', 'turn_numerator', 'root_order', 'winner', 'all_minimisers',
                                    'candidates', 'primary_bits', 'oracle_bits'], rows),
            'argument.json': json_bytes(summary), 'tie_refinement.json': json_bytes(probes)}, largest


def refused(call):
    try:
        call()
    except ValueError:
        return True
    return False


def b_tables(inputs, record):
    menu = b.Menu(record)
    identities, quats = b_oracle.menu_data(record)
    assert identities == (menu.set_identity, menu.order_identity)
    assert list(identities) == inputs['menu_identities']
    self_rows = []
    for index, matrix in enumerate(menu.matrices):
        assert matrix == b_oracle.quat_matrix(quats[index])
        first, second = menu.choose(matrix), b_oracle.select(quats[index], quats)
        assert first == second and first[:2] == (index, [index]) and first[2] == 0
        wire = b.menu_letter(menu, index)
        assert menu.request(wire) == b_oracle.request(wire, identities, quats) == first
        assert refused(lambda: menu.request(wire, drag=True))
        assert refused(lambda: b_oracle.request(wire, identities, quats, drag=True))
        self_rows.append([index, index, '0,0,1', sha256(wire).hexdigest()])
    rows = []
    for case in inputs['b_requests']:
        data = case['wire'].encode('ascii')
        first, second = menu.request(data, drag=True), b_oracle.request(data, identities, quats, drag=True)
        assert first == second
        binding = menu.bound_request(data, drag=True)
        independent_binding = {'menu_identity': identities[0], 'menu_order': identities[1],
            'index': second[0], 'ties': second[1], 'distance_squared_Q5': second[2].record(),
            'request_sha256': sha256(data).hexdigest(), 'operation': 'drag'}
        assert binding == independent_binding
        assert menu.bound_request(bytes(bytearray(data)), drag=True) == binding
        assert menu.request(bytes(bytearray(data)), drag=True) == first
        assert b_oracle.request(bytes(bytearray(data)), identities, quats, drag=True) == second
        reject = refused(lambda: menu.request(data))
        assert reject == refused(lambda: b_oracle.request(data, identities, quats))
        direct, _ = b.free(data)
        assert (direct, None) == b_oracle.free(data)
        quantised, key = b.free(data, quantise=True)
        assert (quantised, key) == b_oracle.free(data, quantise=True)
        rows.append({'name': case['name'], 'wire_sha256': sha256(data).hexdigest(),
                     'bound_drag_choice': binding,
                     'direct_menu': 'refused' if reject else first[0], 'drag_index': first[0],
                     'drag_ties': first[1], 'distance_squared': first[2].record(),
                     'free_direct_matrix': matrix_record(direct),
                     'free_quantised_matrix': matrix_record(quantised), 'quantised_key': list(key),
                     'free_variants_differ': direct != quantised})
    assert any(r['free_variants_differ'] for r in rows)
    assert any(len(r['drag_ties']) > 1 for r in rows)
    # Reverse the menu: set identity stays, order identity and tie-selected rotation change.
    swapped = dict(record, base=list(reversed(record['base'])))
    reverse = b.Menu(swapped)
    reversed_ids, reversed_quats = b_oracle.menu_data(swapped)
    assert reversed_ids == (reverse.set_identity, reverse.order_identity)
    assert reverse.set_identity == menu.set_identity and reverse.order_identity != menu.order_identity
    tie_wire = inputs['reorder_request'].encode('ascii')
    original, reordered = menu.request(tie_wire, True), reverse.request(tie_wire, True)
    assert original == b_oracle.request(tie_wire, identities, quats, True)
    assert reordered == b_oracle.request(tie_wire, reversed_ids, reversed_quats, True)
    assert menu.matrices[original[0]] != reverse.matrices[reordered[0]]
    bound = b.menu_letter(menu, original[0])
    assert refused(lambda: reverse.request(bound))
    assert refused(lambda: b_oracle.request(bound, reversed_ids, reversed_quats))
    reorder = {'set_identity': menu.set_identity, 'order_identity': menu.order_identity,
               'reordered_order_identity': reverse.order_identity,
               'original_index': original[0], 'original_ties': original[1],
               'reordered_index': reordered[0], 'reordered_ties': reordered[1],
               'reordered_original_index': 59 - reordered[0], 'old_binding_refused': True}
    validation = []
    for case in inputs['invalid_wire']:
        data = case['wire'].encode('ascii')
        assert refused(lambda: b.decode(data, menu)), case['name']
        assert refused(lambda: b_oracle.parse(data, identities)), case['name']
        validation.append([case['name'], 'refused', 'refused'])
    for name, value in [('numerator_bound', str(2 ** 1024) + ':0'), ('exponent_bound', '1:1075'),
                        ('byte_bound', '1' * 2049)]:
        data = ('LX9B1|C|' + value + '|0:0|0:0\n').encode('ascii')
        assert refused(lambda: b.decode(data)) and refused(lambda: b_oracle.parse(data))
        validation.append([name, 'refused', 'refused'])
    for case in inputs['binary64_ingress']:
        token = case['bits']
        if case['valid']:
            v = iv.binary64(token)
            assert v == b_oracle.decode_float_bits(token)
            wire = b.from_binary64('C', [token, '0000000000000000', '0000000000000000'])
            kind, vals = b.decode(wire)
            other_kind, quat = b_oracle.parse(wire)
            assert kind == other_kind == 'C' and vals[0] == v and quat[1] == v
            if not v:
                assert wire == b'LX9B1|C|0:0|0:0|0:0\n'
            validation.append([case['name'], 'accepted:' + str(v), 'accepted:' + str(v)])
        else:
            assert refused(lambda: iv.binary64(token)) and refused(lambda: b_oracle.decode_float_bits(token))
            validation.append([case['name'], 'refused', 'refused'])
    # Canonical half-turn scale/sign and exact dyadic coordinate encoding.
    assert b.encode('H', [Q(-1, 4), Q(-1, 2), Q(-3, 4)]) == b.encode('H', [1, 2, 3])
    assert b.encode('C', [Q(6, 16), 0, 0]) == b'LX9B1|C|3:3|0:0|0:0\n'
    return {'option_b.json': json_bytes(rows), 'menu_order.json': json_bytes(reorder),
            'menu_self.tsv': tsv(['index', 'selected', 'distance_squared_Q5', 'wire_sha256'], self_rows),
            'validation.tsv': tsv(['case', 'primary', 'oracle'], validation)}


def produce():
    inputs = json.loads((HERE / 'INPUTS.json').read_bytes())
    record = json.loads((HERE / 'MENU.json').read_bytes())
    outputs, case_bits = case_tables(inputs)
    angle_outputs, angle_bits = angle_tables()
    outputs.update(angle_outputs)
    outputs.update(b_tables(inputs, record))
    outputs['summary.json'] = json_bytes({'case_axes': 6, 'case_semantics': 2, 'exceptional_angles': 152,
        'menu_self_selections': 60, 'largest_deciding_refinement_bits': max(case_bits, angle_bits),
        'largest_nondeciding_tie_probe_bits': 256, 'decision': 'open', 'decider': 'owner'})
    return outputs
