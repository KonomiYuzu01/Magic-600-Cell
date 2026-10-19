# Expand a checked R22B-10 DAG literally, after an exact length preflight.
# Research/headless only. From the repository root:
#   python3 -I -B .../expand_slp.py <certificate[.gz]> <word.json[.gz]> \
#       [--node V|ID] [--inverse] [--limit 10000000]
# No cancellation, reduction, recursion or chamber-identity filtering occurs.
import argparse
import gzip
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import slp_apply as A


def expansion(cert, root, lengths, inverse_items, inverse=False):
    """An explicit stack; a power uses one repeat frame, even for huge exponents."""
    nodes, todo = cert['nodes'], [(root, inverse, 1)]
    while todo:
        ident, backwards, repeat = todo.pop()
        if not repeat or not lengths[ident]:
            continue
        if repeat > 1:
            todo.append((ident, backwards, repeat - 1))
        node = nodes[ident]
        op = node['op']
        if op == 'letter':
            yield node['grip'], inverse_items[node['item']] if backwards else node['item']
        elif op == 'cat':
            refs = node['args'] if backwards else reversed(node['args'])
            todo.extend((child, backwards, 1) for child in refs)
        elif op == 'inv':
            todo.append((node['arg'], not backwards, 1))
        else:
            todo.append((node['arg'], backwards, node['exp']))


def main(argv=None):
    ap = argparse.ArgumentParser(description='Stream a literal R22B-10 word after exact length checks.')
    ap.add_argument('certificate')
    ap.add_argument('out_word')
    ap.add_argument('--node', default='V')
    ap.add_argument('--inverse', action='store_true')
    ap.add_argument('--limit', type=int, default=10000000)
    args = ap.parse_args(argv)
    if args.limit < 0:
        ap.error('--limit must be nonnegative')
    try:
        cert = A.read_json(args.certificate)
        root = 'V' if args.node == 'V' else int(args.node)
        root, _, lengths = A.checked_dag(cert, root)
        A.require(all('length' in node for node in cert['nodes']), 'node is missing its expanded length')
        A.require('V_length' in cert, 'certificate is missing V_length')
        length = lengths[root]
        if length > (args.limit or 10000000):
            A.print_row({'over_limit': True, 'length': str(length)})
            return 3
        A.require(isinstance(cert.get('state_digest'), str), 'certificate is missing state_digest')
        # Load the retained menu only after the length preflight; rejecting a
        # Jordan certificate needs no geometry or simulator initialization.
        B, _ = A.modules()
        M = B.restore.menu('I_a')
        inverse_items = [A.inverse_word(M, [[0, i]])[0][1] for i in range(len(M.items))]
        for ident, node in enumerate(cert['nodes']):
            if node['op'] == 'letter':
                A.word_pairs([[node['grip'], node['item']]], M)
        opener = gzip.open if args.out_word.endswith('.gz') else open
        with opener(args.out_word, 'wb') as output:
            output.write(b'{"word": ')
            count, digest = A.stream_word(expansion(cert, root, lengths, inverse_items, args.inverse), output)
            A.require(count == length, 'emitted length differs from exact DAG length')
            output.write(b', "input_digest": ' + json.dumps(cert['state_digest']).encode('utf-8') + b'}\n')
        A.print_row({'letters': count, 'word_sha256': digest})
        return 0
    except (ValueError, AssertionError, KeyError, TypeError, IndexError, OSError, OverflowError) as error:
        A.print_row({'error': str(error)})
        return 1


if __name__ == '__main__':
    sys.exit(main())
