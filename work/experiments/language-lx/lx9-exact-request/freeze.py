"""Explicit production command. Acceptance never invokes this writer.

Captures only the repository's public menu and the named scalar inputs, then
writes tables only after both arithmetic routes agree. Does not run an engine.
"""
import sys

sys.dont_write_bytecode = True

from hashlib import sha256
import json
import math
from pathlib import Path
import struct

from fixtures import json_bytes, produce
from check import BUILD, remove_build
import grid_probe

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]


def capture():
    record = json.loads((ROOT / 'research/jumbling/fixtures/wj-I_a.json').read_bytes())['menu']
    # One menu element per line keeps the captured exact matrices inspectable.
    metadata = json.dumps({k: v for k, v in record.items() if k != 'base'}, sort_keys=True)
    menu_text = metadata[:-1] + ', "base": [\n' + ',\n'.join(
        json.dumps(item, sort_keys=True, separators=(',', ':')) for item in record['base']) + '\n]}\n'
    (HERE / 'MENU.json').write_bytes(menu_text.encode('ascii'))
    wire = lambda kind, *tokens: 'LX9B1|' + kind + '|' + '|'.join(tokens) + '\n'
    cases = [
        ('identity', 'C', '0:0', '0:0', '0:0'),
        ('dyadic_nonmember', 'C', '3:3', '0:0', '0:0'),
        ('menu_third_turn', 'C', '1:0', '1:0', '1:0'),
        ('quarter_tie_x', 'C', '1:0', '0:0', '0:0'),
        ('quarter_tie_y', 'C', '0:0', '1:0', '0:0'),
        ('half_x', 'H', '1:0', '0:0', '0:0'),
        ('half_y', 'H', '0:0', '1:0', '0:0'),
        ('half_mixed', 'H', '1:0', '2:0', '3:0'),
        ('outside_half_window', 'C', '64:0', '0:0', '0:0'),
        ('inside_half_window', 'C', '128:0', '0:0', '0:0'),
        ('negative_mixed', 'C', '-3:2', '1:1', '-1:3'),
    ]
    invalid = [
        ('signed_zero', wire('C', '-0:0', '0:0', '0:0')),
        ('nonreduced', wire('C', '2:1', '0:0', '0:0')),
        ('zero_exponent', wire('C', '0:1', '0:0', '0:0')),
        ('negative_exponent', wire('C', '1:-1', '0:0', '0:0')),
        ('leading_zero', wire('C', '01:0', '0:0', '0:0')),
        ('plus_sign', wire('C', '+1:0', '0:0', '0:0')),
        ('nan', wire('C', 'NaN', '0:0', '0:0')),
        ('infinity', wire('C', 'Infinity', '0:0', '0:0')),
        ('half_zero', wire('H', '0:0', '0:0', '0:0')),
        ('half_scale', wire('H', '2:0', '0:0', '0:0')),
        ('half_sign', wire('H', '-1:0', '0:0', '0:0')),
        ('half_fraction', wire('H', '1:1', '0:0', '0:0')),
        ('unknown_branch', wire('X', '1:0', '0:0', '0:0')),
        ('missing_newline', wire('C', '1:0', '0:0', '0:0')[:-1]),
        ('extra_field', wire('C', '1:0', '0:0', '0:0', '0:0')),
        ('wrong_menu', wire('M', '0' * 64, '0' * 64, '0')),
    ]
    literal = '71.56505117707799'
    degree = float(literal)
    bits = lambda x: struct.pack('>d', x).hex()
    inputs = {'degree_literal': literal, 'degree_bits': bits(degree),
              'radian_bits': bits(math.radians(math.remainder(degree, 360.0))),
              'radians_per_degree_bits': bits(math.radians(1.0)),
              'axes': [[1,0,0], [-1,0,0], [0,1,0], [0,-1,0], [0,0,1], [0,0,-1]],
              'menu_source': 'research/jumbling/fixtures/wj-I_a.json:menu',
              'menu_identities': [record['identity'], '5a0429d2946fbf58b495d8f300e24a3fa164b1d9518ce4a5c3f4ef0a40625c3c'],
              'b_requests': [{'name': name, 'wire': wire(kind, *tokens)} for name, kind, *tokens in cases],
              'reorder_request': wire('C', '1:0', '0:0', '0:0'),
              'invalid_wire': [{'name': name, 'wire': data} for name, data in invalid],
              'binary64_ingress': [
                  {'name': name, 'bits': value, 'valid': valid} for name, value, valid in (
                      ('positive_zero', '0000000000000000', True), ('negative_zero', '8000000000000000', True),
                      ('smallest_subnormal', '0000000000000001', True), ('largest_finite', '7fefffffffffffff', True),
                      ('negative_finite', 'bff8000000000000', True), ('positive_inf', '7ff0000000000000', False),
                      ('negative_inf', 'fff0000000000000', False), ('quiet_nan', '7ff8000000000000', False),
                      ('signaling_nan', '7ff0000000000001', False), ('malformed_bits', '0' * 15, False))]}
    cases, _ = grid_probe.requests()
    inputs['default_grid_requests'] = [{'name': name, 'target': list(t), 'half': half} for name, t, half in cases]
    (HERE / 'INPUTS.json').write_bytes(json_bytes(inputs))


def pin(outputs):
    sources = {p.name: sha256(p.read_bytes()).hexdigest() for p in sorted(HERE.glob('*.py'))}
    sources.update({p.name: sha256(p.read_bytes()).hexdigest() for p in sorted(HERE.glob('*.rs'))})
    for name in ('CARD.md', 'PROOF.md', 'REQUEST.md'):
        sources[name] = sha256((HERE / name).read_bytes()).hexdigest()
    (HERE / 'CANDIDATE.json').write_bytes(json_bytes(sources))
    files = ['CANDIDATE.json', 'INPUTS.json', 'MENU.json', *sorted(outputs)]
    manifest = {name: sha256((HERE / name).read_bytes()).hexdigest() for name in files}
    (HERE / 'MANIFEST.json').write_bytes(json_bytes(manifest))
    print('CANDIDATE.json', sha256((HERE / 'CANDIDATE.json').read_bytes()).hexdigest())
    print('MANIFEST.json', sha256((HERE / 'MANIFEST.json').read_bytes()).hexdigest())


if __name__ == '__main__':
    try:
        remove_build()
        capture()
        outputs = produce()
        outputs.update(grid_probe.run(BUILD))
        for name, data in outputs.items():
            (HERE / name).write_bytes(data)
        pin(outputs)
        print('Frozen', len(outputs), 'tables; all independent checks passed.')
    finally:
        remove_build()
