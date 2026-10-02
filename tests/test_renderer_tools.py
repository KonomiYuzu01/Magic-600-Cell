"""Headless renderer asset and packet checks, using isolated copies and synthetic packets."""
from __future__ import annotations

import contextlib
import hashlib
import io
import json
import math
import os
import shutil
import struct
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / 'tools/perf'))
import check_renderer_assets as assets  # noqa: E402
import check_renderer_packets as packets  # noqa: E402

REAL_PACKETS = ROOT / 'docs/progress/1.0/packets/renderer'
HEADINGS = [line for line in (ROOT / 'templates/problem-packet.md').read_text(encoding='utf-8').splitlines()
            if line.startswith('## ')]
CONTRACT = ('```implement-contract\n'
            '{"allowed_files": ["work/experiments/renderer-test/*"], '
            '"acceptance_check": ["python", "work/experiments/renderer-test/check_project.py"], '
            '"stop_condition": "the static check passes"}\n```')


def capture(function, argv):
    output = io.StringIO()
    with contextlib.redirect_stdout(output):
        status = function(argv)
    return status, output.getvalue().splitlines()


class TemporaryFiles(unittest.TestCase):
    def setUp(self):
        if os.name == 'nt':
            # As in test_renderer_gate: inherit the workspace ACL on Windows.
            mkdir = os.mkdir
            with mock.patch.object(os, 'mkdir', side_effect=lambda path, mode: mkdir(path)):
                temporary = tempfile.TemporaryDirectory(prefix='.renderer-tools-test-', dir=ROOT)
        else:
            temporary = tempfile.TemporaryDirectory(prefix='.renderer-tools-test-', dir=ROOT)
        self.addCleanup(temporary.cleanup)
        self.base = Path(temporary.name)


class AssetTests(TemporaryFiles):
    def setUp(self):
        super().setUp()
        self.directory = self.base / 'assets'
        self.directory.mkdir()
        for name in ('manifest.json', 'mesh_vertices.f32', 'mesh_sticker.u32', 'mesh.json',
                     'cell_frames.f32', 'slot_piece.u32'):
            shutil.copyfile(ROOT / 'assets' / name, self.directory / name)

    def write_asset(self, name, data, refresh_digest=True):
        (self.directory / name).write_bytes(data)
        if refresh_digest:
            path = self.directory / 'manifest.json'
            manifest = json.loads(path.read_text(encoding='utf-8'))
            manifest['files'][f'assets/{name}'] = hashlib.sha256(data).hexdigest()
            path.write_text(json.dumps(manifest), encoding='utf-8')

    def assert_problem(self, name, reason):
        with mock.patch.object(assets, 'ASSETS', self.directory):
            status, lines = capture(assets.main, [])
        self.assertEqual(status, 1)
        self.assertTrue(any(f'assets/{name}:' in line and reason in line for line in lines), lines)
        self.assertTrue(all(line.startswith('assets/') for line in lines), lines)

    def test_real_assets_pass(self):
        status, lines = capture(assets.main, [])
        self.assertEqual(status, 0)
        self.assertEqual(lines, ['renderer assets: ok (600 cells x 433 stickers = 259,800 slots; 30,480 base vertices)'])
        with mock.patch.object(assets, 'ASSETS', self.directory):
            self.assertEqual(capture(assets.main, []), (status, lines))

    def test_changed_byte_fails_digest(self):
        data = bytearray((self.directory / 'slot_piece.u32').read_bytes())
        data[-1] ^= 1
        self.write_asset('slot_piece.u32', data, refresh_digest=False)
        self.assert_problem('slot_piece.u32', 'SHA-256 digest mismatch')

    def test_digest_uses_bytes_as_stored(self):
        original = (self.directory / 'mesh.json').read_bytes()
        self.write_asset('mesh.json', original + b'\n')
        # A digest helper that normalizes CRLF would incorrectly accept this file.
        self.write_asset('mesh.json', original + b'\r\n', refresh_digest=False)
        self.assert_problem('mesh.json', 'SHA-256 digest mismatch')

    def test_truncated_binary_files_fail_size(self):
        for name, size in (('mesh_vertices.f32', 487680), ('mesh_sticker.u32', 121920),
                           ('cell_frames.f32', 38400), ('slot_piece.u32', 1039200)):
            with self.subTest(name=name):
                original = (self.directory / name).read_bytes()
                self.write_asset(name, original[:-1])
                self.assert_problem(name, f'size must be {size:,} bytes')
                self.write_asset(name, original)

    def test_sticker_index_outside_range_fails(self):
        original = (self.directory / 'mesh_sticker.u32').read_bytes()
        self.write_asset('mesh_sticker.u32', struct.pack('<I', 433) + original[4:])
        self.assert_problem('mesh_sticker.u32', 'sticker index outside 0..432')

    def test_sticker_indices_must_match_offsets(self):
        original = (self.directory / 'mesh_sticker.u32').read_bytes()
        self.write_asset('mesh_sticker.u32', struct.pack('<I', 1) + original[4:])
        self.assert_problem('mesh_sticker.u32', 'do not match mesh.json offsets')

    def test_mesh_counts_and_offsets_are_fixed(self):
        original = json.loads((self.directory / 'mesh.json').read_bytes())
        cases = [('non-increasing', lambda mesh: mesh['offsets'].__setitem__(1, 0), 'offsets'),
                 ('missing-offset', lambda mesh: mesh['offsets'].pop(), 'offsets'),
                 ('wrong-start', lambda mesh: mesh['offsets'].__setitem__(0, -1), 'offsets'),
                 ('wrong-end', lambda mesh: mesh['offsets'].__setitem__(-1, 30479), 'offsets'),
                 ('bad-offset-type', lambda mesh: mesh['offsets'].__setitem__(1, '18'), 'offsets')]
        cases += [(name, lambda mesh, field=name: mesh.update({field: original[field] - 1}), name)
                  for name in ('base_vertices', 'base_stickers', 'cells', 'slots')]
        for name, edit, reason in cases:
            with self.subTest(name=name):
                mesh = json.loads(json.dumps(original))
                edit(mesh)
                self.write_asset('mesh.json', json.dumps(mesh).encode('utf-8'))
                self.assert_problem('mesh.json', reason)

    def test_non_finite_floats_fail(self):
        for name in ('mesh_vertices.f32', 'cell_frames.f32'):
            original = (self.directory / name).read_bytes()
            for value in (math.nan, math.inf, -math.inf):
                with self.subTest(name=name, value=value):
                    self.write_asset(name, struct.pack('<f', value) + original[4:])
                    self.assert_problem(name, 'non-finite float32')
            self.write_asset(name, original)


class PacketTests(TemporaryFiles):
    def setUp(self):
        super().setUp()
        self.directory = self.base / 'packets'
        self.directory.mkdir()
        self.paths, self.texts = {}, {}
        for number in range(6):
            path = self.directory / f'E-2.4-{number:02d}-test.md'
            run = '--kind review --model gpt-6-astra --effort max'
            if number in (1, 2, 3):
                run = '--kind implement --model gpt-6.1-sol --effort max'
            elif number == 4:
                run = '--kind review --model gpt-6-astra --effort ultra --gate day7-go-no-go'
            real = next(REAL_PACKETS.glob(f'E-2.4-{number:02d}-*.md')).read_text(encoding='utf-8')
            # In particular, readiness returns a checklist in a README path, implementations
            # return README.md, and reviews return the schema. Keep their real section 7 text.
            section7 = real.split(HEADINGS[-1], 1)[1].strip()
            sections = ['renderer_gate.py: all 259,800 slots, an average of at least 30 fps, p99 at most 33.3 ms, '
                        'three valid cold runs of 180 s.\n'
                        '- NVIDIA features off by default.',
                        'Synthetic packet for a headless check.', 'CPython 3.11, standard library only.',
                        'The shared assets and renderer_gate.py.', 'None.',
                        CONTRACT if number in (1, 2, 3) else 'Read only.', section7]
            text = f'# Packet E-2.4-{number:02d}\n\n{run}\n\n' + '\n\n'.join(
                f'{heading}\n{body}' for heading, body in zip(HEADINGS, sections)) + '\n'
            path.write_text(text, encoding='utf-8')
            self.paths[number], self.texts[number] = path, text
        (self.directory / 'README.md').write_text(
            'All 259,800 slots; an average of at least 30 fps; p99 at most 33.3 ms; PresentMon 2.6.0.0; peak about 7 GB; '
            'three valid cold runs of 180 s after 10 s warmup; NVIDIA features off by default.\n', encoding='utf-8')

    def assert_problem(self, filename, reason):
        status, lines = capture(packets.main, [str(self.directory)])
        self.assertEqual(status, 1)
        self.assertEqual(len(lines), 1, lines)
        self.assertIn(filename, lines[0])
        self.assertIn(reason, lines[0])

    def test_complete_synthetic_set_passes(self):
        with mock.patch.object(packets.codex_review, 'parse_contract', wraps=packets.codex_review.parse_contract) as parse:
            self.assertEqual(capture(packets.main, [str(self.directory)]), (0, ['renderer packets: ok (6 packets)']))
        self.assertEqual(parse.call_count, 3)
        self.assertEqual([call.args[0] for call in parse.call_args_list], [self.texts[number] for number in (1, 2, 3)])

    def test_real_repository_packets_pass(self):
        self.assertEqual(capture(packets.main, []), (0, ['renderer packets: ok (6 packets)']))

    def test_seven_headings_are_required_in_order(self):
        original = self.texts[1]
        swapped = (original.replace(HEADINGS[1], '## Temporary heading')
                   .replace(HEADINGS[2], HEADINGS[1]).replace('## Temporary heading', HEADINGS[2]))
        cases = (original.replace(HEADINGS[1], '## Reproduction'),
                 swapped,
                 original.replace(HEADINGS[0], HEADINGS[0] + '\n' + HEADINGS[0]))
        for text in cases:
            with self.subTest(text=text[:120]):
                self.paths[1].write_text(text, encoding='utf-8')
                self.assert_problem(self.paths[1].name, 'seven section headings in order')

    def test_contract_rules_are_enforced(self):
        original = self.texts[1]
        cases = [('two-blocks', original + '\n' + CONTRACT, 'exactly one'),
                 ('invalid-json', original.replace('"allowed_files"', 'allowed_files'), 'not JSON'),
                 ('outside-renderer', original.replace('work/experiments/renderer-test/*', 'tests/*'), 'allowed_files'),
                 ('unsafe-glob', original.replace('work/experiments/renderer-test/*', '../outside/*'), 'unsafe')]
        for name, text, reason in cases:
            with self.subTest(name=name):
                self.paths[1].write_text(text, encoding='utf-8')
                self.assert_problem(self.paths[1].name, reason)
        self.paths[1].write_text(original, encoding='utf-8')
        self.paths[5].write_text(self.texts[5] + '\n' + CONTRACT, encoding='utf-8')
        self.assert_problem(self.paths[5].name, 'only allowed with --kind implement')

    def test_gate_flags_are_required(self):
        original = self.texts[4]
        for text in (original + '\n--speed fast\n', original.replace('--effort ultra', '--effort max'),
                     original.replace('--model gpt-6-astra', '--model gpt-6.1-sol')):
            with self.subTest(text=text[:160]):
                self.paths[4].write_text(text, encoding='utf-8')
                self.assert_problem(self.paths[4].name, '--gate requires')

    def test_readme_requires_shared_gate_values(self):
        path = self.directory / 'README.md'
        original = path.read_text(encoding='utf-8')
        for value in ('259,800', '30 fps', '33.3 ms', 'PresentMon 2.6.0.0', '7 GB', '180 s', '10 s', 'off by default'):
            with self.subTest(value=value):
                path.write_text(original.replace(value, ''), encoding='utf-8')
                self.assert_problem('README.md', value)

    def assert_gate_problem(self, filename, reason):
        status, lines = capture(packets.main, [str(self.directory)])
        self.assertEqual(status, 1)
        self.assertTrue(all(filename in line for line in lines), lines)
        self.assertTrue(any(reason in line for line in lines), lines)

    def test_gate_packet_requires_fixed_gate_numbers(self):
        for before, after, reason in (('33.3 ms', '40 ms', '40 ms differs'), ('30 fps', '20 fps', '20 fps differs'),
                                      ('259,800', '250,000', 'missing gate text: 259,800')):
            with self.subTest(before=before):
                self.paths[4].write_text(self.texts[4].replace(before, after), encoding='utf-8')
                self.assert_gate_problem(self.paths[4].name, reason)

    def test_weakened_gate_text_fails_beside_the_correct_text(self):
        # Mutations a reviewer found to pass the first version of the checker.
        readme = self.directory / 'README.md'
        readme_text = readme.read_text(encoding='utf-8')
        cases = (
            (5, self.texts[5].replace('renderer_gate.py:', 'renderer_gate.py: 20 fps, p99 40 ms;'), '20 fps differs'),
            (4, self.texts[4].replace('33.3 ms', '133.3 ms'), '133.3 ms differs'),
            (4, self.texts[4] + 'One valid cold run is enough.\n', 'One cold runs differs'),
            (4, self.texts[4].replace('at most 33.3 ms', 'at least 33.3 ms'), 'must read "at most 33.3 ms"'),
            (5, self.texts[5].replace('at most 33.3 ms', 'at least 33.3 ms'), 'must read "at most 33.3 ms"'),
            (4, self.texts[4].replace('at least 30 fps', 'at most 30 fps'), 'must read "at least 30 fps"'),
        )
        for number, text, reason in cases:
            with self.subTest(reason=reason):
                self.paths[number].write_text(text, encoding='utf-8')
                self.assert_gate_problem(self.paths[number].name, reason)
                self.paths[number].write_text(self.texts[number], encoding='utf-8')
        readme.write_text(readme_text.replace('33.3 ms', '133.3 ms'), encoding='utf-8')
        self.assert_gate_problem('README.md', '133.3 ms differs')
        readme.write_text(readme_text.replace('at most 33.3 ms', 'at least 33.3 ms'), encoding='utf-8')
        self.assert_gate_problem('README.md', 'must read "at most 33.3 ms"')

    def test_checker_writes_no_file(self):
        # A fresh interpreter whose audit hook refuses every write-capable open or file creation.
        code = (
            'import sys\n'
            'def hook(event, args):\n'
            '    if event == "open" and len(args) > 1 and isinstance(args[1], str) and any(c in args[1] for c in "wax+"):\n'
            '        raise PermissionError(f"write attempt: {args[0]}")\n'
            '    if event == "open" and len(args) > 2 and isinstance(args[2], int) and args[2] & 0o3103:\n'
            '        raise PermissionError(f"write attempt: {args[0]}")\n'
            '    if event in ("os.mkdir", "os.remove", "os.rename", "os.unlink"):\n'
            '        raise PermissionError(f"write attempt: {event} {args[0]}")\n'
            'sys.addaudithook(hook)\n'
            'import runpy\n'
            'sys.argv = [%r]\n'
            'runpy.run_path(sys.argv[0], run_name="__main__")\n' % str(ROOT / 'tools/perf/check_renderer_packets.py'))
        env = {k: v for k, v in os.environ.items() if k not in ('TMPDIR', 'TEMP', 'TMP')}
        result = subprocess.run([sys.executable, '-B', '-c', code], capture_output=True, text=True, env=env, cwd=ROOT)
        self.assertEqual(result.returncode, 0, result.stderr[-2000:])
        self.assertIn('renderer packets: ok', result.stdout)

    def test_nvidia_restriction_is_required(self):
        for number in range(1, 6):
            with self.subTest(number=number):
                self.paths[number].write_text(self.texts[number].replace('- NVIDIA features off by default.\n', ''),
                                              encoding='utf-8')
                self.assert_problem(self.paths[number].name, 'NVIDIA restriction')
                self.paths[number].write_text(self.texts[number], encoding='utf-8')

    def test_judge_constants_agree_with_packet_readme(self):
        for name, wrong in (('FPS_MIN', 31), ('P99_MAX_MS', 40), ('INTERVAL_S', 60), ('WARMUP_S', 0), ('RUNS_MIN', 2)):
            with self.subTest(name=name), mock.patch.object(packets.renderer_gate, name, wrong):
                self.assert_problem('renderer_gate.py', name)

    def test_missing_packet_is_a_problem(self):
        self.paths[3].unlink()
        self.assert_problem('E-2.4-03-', 'expected exactly one packet')

    def test_duplicate_packet_number_is_a_problem(self):
        (self.directory / 'E-2.4-03-duplicate.md').write_text(self.texts[3], encoding='utf-8')
        self.assert_problem('E-2.4-03-duplicate.md', 'expected exactly one packet')

    def test_missing_directory_is_a_problem(self):
        missing = self.base / 'missing'
        status, lines = capture(packets.main, [str(missing)])
        self.assertEqual(status, 1)
        self.assertEqual(len(lines), 1, lines)
        self.assertIn('missing', lines[0])
        self.assertIn('missing directory', lines[0])

    def test_section_seven_requires_a_return_format(self):
        for number in (0, 1, 4):
            with self.subTest(number=number):
                prefix = self.texts[number].split(HEADINGS[-1], 1)[0]
                self.paths[number].write_text(prefix + HEADINGS[-1] + '\nA result.\n', encoding='utf-8')
                self.assert_problem(self.paths[number].name, 'section 7')
                self.paths[number].write_text(self.texts[number], encoding='utf-8')


if __name__ == '__main__':
    unittest.main(verbosity=2)
