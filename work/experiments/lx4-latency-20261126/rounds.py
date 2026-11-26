"""Alternating rounds of LX-4's headless endpoint timing on several builds (standard library only).

    python -B work/experiments/lx4-latency-20261126/rounds.py --fixtures DIR --rounds N --samples N \
        --targets S,M,L --frame-ms F NAME=DRIVER [NAME=DRIVER ...]

Each DRIVER is an `endpoint_timing` executable (renderer/bridge/lx4/examples/endpoint_timing.rs) built from one tree.
A round runs every driver once, in the order given, so that a change of machine state (load, power) falls on every
build alike. Each run drops its first round of samples as a warm-up. The table gives, per size, edit and endpoint,
the median over rounds of each run's p50 and p95, in milliseconds. It is headless timing of the language work the
shell waits for, not a display latency, and not Windows/DirectX, input or on-screen evidence.
"""
from __future__ import annotations

import argparse
import re
import statistics
import subprocess

ROW = re.compile(r'(\S+)\s+(\S+)\s+(\S+)\s+([\d.]+)\s+([\d.]+)\s+([\d.]+)')


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument('--fixtures', required=True)
    parser.add_argument('--rounds', type=int, default=6)
    parser.add_argument('--samples', default='8')
    parser.add_argument('--targets', default='S,M,L')
    parser.add_argument('--frame-ms', default='1')
    parser.add_argument('drivers', nargs='+', help='NAME=PATH of an endpoint_timing executable')
    args = parser.parse_args()
    drivers = [d.split('=', 1) for d in args.drivers]
    cells: dict[tuple[str, str, str], dict[str, list[tuple[float, float]]]] = {}
    for _ in range(args.rounds):
        for name, path in drivers:
            command = [path, args.fixtures, '--samples', args.samples, '--targets', args.targets,
                       '--frame-ms', args.frame_ms]
            out = subprocess.run(command, capture_output=True, text=True, check=True).stdout
            for line in out.splitlines():
                m = ROW.match(line)
                if m and m.group(1) in ('S', 'M', 'L') and m.group(3) not in ('echo', 'stale'):
                    cells.setdefault(m.groups()[:3], {}).setdefault(name, []).append(
                        (float(m.group(4)), float(m.group(5))))
    print(f'{args.rounds} rounds x {int(args.samples) - 1} samples, frame {args.frame_ms} ms; '
          "median over rounds of each round's p50 / p95")
    print(f"{'':2} {'edit':20} {'endpoint':12} " + ' '.join(f'{name:>15}' for name, _ in drivers))
    for key in sorted(cells):
        row = []
        for name, _ in drivers:
            runs = cells[key].get(name)
            row.append(f'{statistics.median(r[0] for r in runs):6.1f} /{statistics.median(r[1] for r in runs):6.1f}'
                       if runs else f"{'-':>15}")
        print(f'{key[0]:2} {key[1]:20} {key[2]:12} ' + ' '.join(f'{c:>15}' for c in row))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
