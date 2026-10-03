"""B4-12 summary contract, using synthetic files only (no application runs)."""
from __future__ import annotations

import contextlib
import csv
import io
import json
import math
import os
import shutil
import socket
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / 'tools/perf'))
try:
    import b412_summary as summary
except ModuleNotFoundError:
    summary = None


class SummaryTests(unittest.TestCase):
    def setUp(self):
        self.assertIsNotNone(summary, 'The B4-12 summary tool must exist')
        if os.name == 'nt':
            # CPython's mode 0700 ACL omits the Windows sandbox's restricted SID.
            # Inherit the writable workspace ACL for these synthetic fixtures.
            mkdir = os.mkdir
            with mock.patch.object(os, 'mkdir', side_effect=lambda path, mode: mkdir(path)):
                temporary = tempfile.TemporaryDirectory(prefix='.b412-test-', dir=ROOT)
        else:
            temporary = tempfile.TemporaryDirectory(prefix='.b412-test-', dir=ROOT)
        self.addCleanup(temporary.cleanup)
        self.base = Path(temporary.name)
        self.sequence = 0

    def write_json(self, path, value):
        path.write_text(json.dumps(value), encoding='utf-8')

    def latency(self, values=None, *, series='m1-active', warmups=(), **overrides):
        self.sequence += 1
        directory = self.base / f'run-{self.sequence}'
        directory.mkdir()
        values = [10.0] * 100 if values is None else list(values)
        run = {
            'format': 'magic600-b412-run-v1', 'run_id': directory.name,
            'series': series, 'attribution': False, 'status': 'valid',
            'invalid_reasons': [], 'qpc_frequency': 1_000_000_000,
            'warmups': len(warmups), 'measured': len(values),
            'build': {'build_identity': 'fixture-build'},
        }
        run.update(overrides)
        self.write_json(directory / 'run.json', run)
        self.write_json(directory / 'environment.json', {})
        samples = []
        for index, value in enumerate(list(warmups) + values):
            samples.append({
                'format': 'magic600-b412-sample-v1', 'series': series,
                'index': index, 'warmup': index < len(warmups), 'ms': value,
                'input_ticks': index * 100_000_000,
                'end_ticks': index * 100_000_000 + round(value * 1_000_000),
            })
        self.write_samples(directory, samples)
        return directory

    def write_samples(self, directory, samples):
        (directory / 'samples.jsonl').write_text(
            ''.join(json.dumps(sample) + '\n' for sample in samples), encoding='utf-8')

    def samples(self, directory):
        return [json.loads(line) for line in
                (directory / 'samples.jsonl').read_text(encoding='utf-8').splitlines()]

    def change_run(self, directory, **updates):
        path = directory / 'run.json'
        run = json.loads(path.read_text(encoding='utf-8'))
        run.update(updates)
        self.write_json(path, run)

    def m3(self, rows=None, *, stop=20_000, columns=None, **overrides):
        settings = {'qpc_frequency': 1000, 'rotation': {'start_qpc': 10_000, 'stop_qpc': stop},
                    'presentmon': {'process_id': 42}}
        settings.update(overrides)
        directory = self.latency(series='m3', **settings)
        (directory / 'samples.jsonl').unlink()
        if rows is None:
            rows = [(42, 'primary', 15_000 + i * 10, 20, 20) for i in range(100)]
        names = columns or ['ProcessID', 'SwapChainAddress', 'CPUStartQPC',
                            'MsBetweenPresents', 'MsBetweenDisplayChange']
        with (directory / 'presentmon.csv').open('w', newline='', encoding='utf-8') as stream:
            writer = csv.writer(stream)
            writer.writerow(names)
            writer.writerows(rows)
        return directory

    def summarize(self, *directories, **kwargs):
        return summary.summarize(directories, **kwargs)

    def unreadable(self, directory, reason):
        result = self.summarize(directory)
        self.assertEqual(result['unreadable'], [{'directory': directory.name, 'reason': reason}])
        self.assertTrue(all(not item['runs'] for item in result['series'].values()))
        self.assertEqual(result['environment'], [])
        self.assertEqual(result['invalid_runs'], [])
        self.assertEqual(result['attribution_runs'], [])

    def cli(self, *directories, extra=()):
        out = self.base / 'summary.json'
        markdown = self.base / 'summary.md'
        completed = subprocess.run(
            [sys.executable, '-B', str(ROOT / 'tools/perf/b412_summary.py'),
             *(str(directory) for directory in directories), '--out', str(out),
             '--markdown', str(markdown), *extra],
            capture_output=True, text=True, encoding='utf-8', check=False)
        return completed, out, markdown

    def test_nearest_rank_uses_integer_ranks(self):
        ranks = {
            1: (1, 1, 1), 2: (1, 2, 2), 100: (50, 95, 99),
            300: (150, 285, 297), 301: (151, 286, 298),
        }
        for n, expected in ranks.items():
            for percentile, rank in zip((50, 95, 99), expected):
                with self.subTest(n=n, percentile=percentile):
                    self.assertEqual(summary.nearest_rank(list(range(n, 0, -1)), percentile), rank)
        for n, rank in [(1, 1), (2, 2), (100, 100), (300, 300),
                        (301, 301), (1000, 999), (1001, 1000), (10000, 9990)]:
            self.assertEqual(summary.nearest_rank(list(range(1, n + 1)), 99.9), rank)
        differences = [n for n in range(1, 10001)
                       if math.ceil(0.95 * n) != (95 * n + 99) // 100]
        if differences:
            for n in differences:
                self.assertEqual(summary.nearest_rank(list(range(1, n + 1)), 95),
                                 (95 * n + 99) // 100)
        else:
            self.assertEqual(differences, [])  # No floating-point counterexample in this range.

    def test_three_runs_all_statistics_and_each_run_target(self):
        first = self.latency(range(1, 101), warmups=(10000, 20000))
        second = self.latency(range(100, 0, -1))
        third = self.latency([1] * 94 + [200] * 6)
        result = self.summarize(first, second, third)
        self.assertEqual(result['format'], 'magic600-b412-summary-v1')
        self.assertEqual(result['method'], 'nearest-rank percentiles; no outlier removal; '
                         'a target is met only if the pooled value and every run meet it')
        active = result['series']['m1-active']
        self.assertEqual(active['runs'][0], {'run_id': first.name, 'n': 100, 'mean': 50.5,
                         'p50': 50, 'p95': 95, 'p99': 99, 'max': 100})
        self.assertEqual(active['runs'][1], {'run_id': second.name, 'n': 100, 'mean': 50.5,
                         'p50': 50, 'p95': 95, 'p99': 99, 'max': 100})
        self.assertEqual(active['runs'][2], {'run_id': third.name, 'n': 100, 'mean': 12.94,
                         'p50': 1, 'p95': 200, 'p99': 200, 'max': 200})
        self.assertEqual(active['pooled'], {'n': 300, 'mean': 37.98, 'p50': 28,
                         'p95': 96, 'p99': 200, 'max': 200})
        self.assertEqual(active['verdict'], 'not-met')  # Pooled p95 meets, one run does not.
        self.assertEqual(active['target'], 'p95 <= 100 ms')
        self.assertTrue(active['formal'])
        self.assertAlmostEqual(active['spread'], 200 / 95 - 1)
        self.assertTrue(active['needs_extra_run'])

    def test_latency_target_boundaries_and_no_data(self):
        directories = [self.latency([100] * 100, series='m1-all'),
                       self.latency([50] * 100, series='m2-bank'),
                       self.latency([49.999] * 100, series='m2-local')]
        result = self.summarize(*directories)
        self.assertEqual(list(result['series']), ['m1-active', 'm1-all', 'm2-bank', 'm2-local', 'm3'])
        self.assertEqual(result['series']['m1-all']['verdict'], 'met')
        self.assertEqual(result['series']['m2-bank']['verdict'], 'not-met')
        self.assertEqual(result['series']['m2-local']['verdict'], 'met')
        self.assertEqual(result['series']['m2-local']['target'], 'p95 < 50 ms')
        empty = result['series']['m1-active']
        self.assertEqual(empty['verdict'], 'no-data')
        self.assertIsNone(empty['pooled'])
        self.assertIsNone(empty['spread'])
        self.assertFalse(empty['needs_extra_run'])
        self.assertFalse(empty['formal'])

    def test_spread_threshold_and_zero_minimum(self):
        for low, high, expected in [(100, 119, False), (100, 120, False),
                                    (100, 121, True), (0, 1, True), (0, 0, True)]:
            with self.subTest(low=low, high=high):
                active = self.summarize(self.latency([low] * 100),
                                        self.latency([high] * 100))['series']['m1-active']
                self.assertEqual(active['needs_extra_run'], expected)
                if low == 0:
                    self.assertIsNone(active['spread'])
                else:
                    self.assertAlmostEqual(active['spread'], high / low - 1)

    def test_formal_requires_three_hundred_sample_runs(self):
        one = self.summarize(self.latency([12.34567] * 100))
        active = one['series']['m1-active']
        self.assertFalse(active['formal'])
        self.assertFalse(active['needs_extra_run'])
        self.assertIsNone(active['spread'])
        self.assertEqual(active['pooled']['p95'], 12.34567)
        markdown = summary.render_markdown(one)
        self.assertIn('baseline, not a formal 3 x 100 result', markdown)
        self.assertIn('12.35', markdown)
        self.assertIn('nearest-rank percentiles', markdown)
        self.assertIn('fixture-build', markdown)
        short = self.summarize(*(self.latency([1, 2]) for _ in range(3)))
        self.assertFalse(short['series']['m1-active']['formal'])
        two = self.summarize(self.latency(), self.latency())
        self.assertFalse(two['series']['m1-active']['formal'])

    def test_repeated_runs_are_refused_not_pooled(self):
        run = self.latency()
        for directories in ((run, run, run), (run, Path(str(run) + '/.'), run.parent / '..' / run.parent.name / run.name)):
            with self.subTest(count=len(directories)):
                with self.assertRaisesRegex(ValueError, 'duplicate-run'):
                    self.summarize(*directories)
        copy = self.base / 'copied'
        shutil.copytree(run, copy)
        with self.assertRaisesRegex(ValueError, 'duplicate-run'):
            self.summarize(run, copy, self.latency())
        completed, out, _ = self.cli(run, copy, self.latency())
        self.assertEqual(completed.returncode, 2)
        self.assertIn('duplicate-run', completed.stderr)
        self.assertFalse(out.exists())
        self.assertTrue(self.summarize(run, self.latency(), self.latency())['series']['m1-active']['formal'])

    def test_embedded_unc_paths_are_redacted(self):
        unc = r'\\archive-canary\private-share\run.csv'
        directory = self.latency(status='invalid', invalid_reasons=['capture failed at ' + unc])
        environment = {'cpu_name': 'cpu at ' + unc, 'power_mode': 'mode //forward-canary/share'}
        self.write_json(directory / 'environment.json', environment)
        result = self.summarize(directory)
        markdown = summary.render_markdown(result)
        for text in (json.dumps(result), str(result), markdown):
            self.assertNotIn('archive-canary', text)
            self.assertNotIn('forward-canary', text)
        self.assertEqual(result['invalid_runs'][0]['reasons'], ['redacted'])
        self.assertEqual(result['redactions'], 3)

    def test_invalid_and_attribution_runs_do_not_count(self):
        good = self.latency([10] * 100)
        invalid = self.latency([999] * 100, status='invalid', invalid_reasons=['fixture-invalid'])
        attribution = self.latency([777] * 100, attribution=True)
        both = self.latency([888] * 100, status='invalid', attribution=True,
                            invalid_reasons=['fixture-both'])
        result = self.summarize(good, invalid, attribution, both)
        self.assertEqual(result['series']['m1-active']['pooled']['mean'], 10)
        self.assertEqual(result['series']['m1-active']['pooled']['n'], 100)
        self.assertEqual(result['invalid_runs'], [
            {'run_id': invalid.name, 'series': 'm1-active', 'reasons': ['fixture-invalid']},
            {'run_id': both.name, 'series': 'm1-active', 'reasons': ['fixture-both']}])
        self.assertEqual(result['attribution_runs'], [{'run_id': attribution.name,
            'series': 'm1-active', 'n': 100, 'mean': 777, 'p50': 777, 'p95': 777,
            'p99': 777, 'max': 777}])
        self.assertEqual(len(result['environment']), 4)

    def test_every_unreadable_reason_code(self):
        missing = self.latency()
        (missing / 'run.json').unlink()
        bad_json = self.latency()
        (bad_json / 'run.json').write_text('{', encoding='utf-8')
        bad_format = self.latency(format='wrong')
        bad_field = self.latency(qpc_frequency=True)
        count = self.latency()
        self.write_samples(count, self.samples(count)[:-1])
        order = self.latency()
        samples = self.samples(order)
        samples[3]['index'] = 4
        self.write_samples(order, samples)
        mismatch = self.latency()
        samples = self.samples(mismatch)
        samples[0]['ms'] += 1
        self.write_samples(mismatch, samples)
        missing_column = self.m3(columns=['ProcessID', 'SwapChainAddress', 'CPUStartQPC'])
        no_rows = self.m3(rows=[(99, 'other', 15_000, 20, 20)])
        cases = [(missing, 'missing-file'), (bad_json, 'bad-json'),
                 (bad_format, 'bad-format'), (bad_field, 'bad-field'),
                 (count, 'sample-count-mismatch'), (order, 'sample-order'),
                 (mismatch, 'sample-ms-mismatch'),
                 (missing_column, 'presentmon-missing-column'), (no_rows, 'presentmon-no-rows')]
        for directory, reason in cases:
            with self.subTest(reason=reason):
                self.unreadable(directory, reason)
        result = self.summarize(*(directory for directory, _ in cases), self.latency())
        self.assertEqual(len(result['unreadable']), 9)
        self.assertEqual(result['series']['m1-active']['pooled']['n'], 100)

    def test_required_input_files_and_json_objects(self):
        for filename in ('run.json', 'samples.jsonl', 'environment.json', 'presentmon.csv'):
            with self.subTest(missing=filename):
                directory = self.m3() if filename == 'presentmon.csv' else self.latency()
                (directory / filename).unlink()
                self.unreadable(directory, 'missing-file')
        for filename in ('run.json', 'environment.json', 'samples.jsonl'):
            for content, reason in [('[]\n', 'bad-field'), ('{\n', 'bad-json'),
                                    ('\xff', 'bad-json')]:
                with self.subTest(file=filename, content=content):
                    directory = self.latency()
                    (directory / filename).write_bytes(content.encode('latin-1'))
                    self.unreadable(directory, reason)

    def test_run_field_validation(self):
        cases = [('run_id', 5), ('series', 'm4'), ('attribution', 1),
                 ('status', 'complete'), ('invalid_reasons', 'reason'),
                 ('invalid_reasons', [1]), ('qpc_frequency', 0),
                 ('qpc_frequency', -1), ('qpc_frequency', 1.5),
                 ('warmups', -1), ('warmups', True), ('measured', -1),
                 ('measured', True), ('measured', 0), ('build', []),
                 ('build', {}), ('build', {'build_identity': 1})]
        for field, value in cases:
            with self.subTest(field=field, value=value):
                directory = self.latency()
                self.change_run(directory, **{field: value})
                self.unreadable(directory, 'bad-field')
        for field in ('run_id', 'series', 'attribution', 'status', 'invalid_reasons',
                      'qpc_frequency', 'warmups', 'measured', 'build'):
            with self.subTest(missing=field):
                directory = self.latency()
                path = directory / 'run.json'
                data = json.loads(path.read_text(encoding='utf-8'))
                del data[field]
                self.write_json(path, data)
                self.unreadable(directory, 'bad-field')

    def test_sample_validation_and_warmup_order(self):
        cases = [('series', 'm1-all', 'bad-field'), ('format', 'wrong', 'bad-format'),
                 ('index', True, 'bad-field'), ('index', -1, 'sample-order'),
                 ('warmup', 1, 'bad-field'), ('ms', True, 'bad-field'),
                 ('ms', -1, 'bad-field'), ('ms', float('nan'), 'bad-field'),
                 ('ms', float('inf'), 'bad-field'), ('input_ticks', 1.5, 'bad-field'),
                 ('end_ticks', True, 'bad-field')]
        for field, value, reason in cases:
            with self.subTest(field=field, value=value):
                directory = self.latency()
                samples = self.samples(directory)
                samples[0][field] = value
                self.write_samples(directory, samples)
                self.unreadable(directory, reason)
        for field in ('format', 'series', 'index', 'warmup', 'ms', 'input_ticks', 'end_ticks'):
            with self.subTest(missing=field):
                directory = self.latency()
                samples = self.samples(directory)
                del samples[0][field]
                self.write_samples(directory, samples)
                self.unreadable(directory, 'bad-format' if field == 'format' else 'bad-field')
        directory = self.latency(warmups=(999,))
        samples = self.samples(directory)
        samples[0]['warmup'], samples[1]['warmup'] = False, True
        self.write_samples(directory, samples)
        self.unreadable(directory, 'sample-order')
        directory = self.latency(warmups=(999,))
        samples = self.samples(directory)
        samples[0]['warmup'] = False
        self.write_samples(directory, samples)
        self.unreadable(directory, 'sample-count-mismatch')

    def test_sample_ms_relative_tolerance_and_zero(self):
        for value, expected in [(100.00005, None), (100.0002, 'sample-ms-mismatch')]:
            directory = self.latency([100] * 100)
            samples = self.samples(directory)
            samples[0]['ms'] = value
            self.write_samples(directory, samples)
            if expected:
                self.unreadable(directory, expected)
            else:
                self.assertEqual(self.summarize(directory)['unreadable'], [])
        directory = self.latency([0] * 100)
        samples = self.samples(directory)
        samples[0]['ms'] = 1e-10
        self.write_samples(directory, samples)
        self.unreadable(directory, 'sample-ms-mismatch')

    def test_sample_conversion_uses_run_frequency_and_rejects_negative_elapsed_time(self):
        directory = self.latency([10] * 100, qpc_frequency=123_000)
        samples = self.samples(directory)
        for sample in samples:
            sample['end_ticks'] = sample['input_ticks'] + 1230
        self.write_samples(directory, samples)
        self.assertEqual(self.summarize(directory)['series']['m1-active']['pooled']['mean'], 10)
        samples[0]['end_ticks'] = samples[0]['input_ticks'] - 1
        self.write_samples(directory, samples)
        self.unreadable(directory, 'sample-ms-mismatch')

    def test_m2_requires_each_run_as_well_as_pooled_target(self):
        result = self.summarize(self.latency([10] * 100, series='m2-bank'),
                                self.latency([10] * 100, series='m2-bank'),
                                self.latency([10] * 94 + [60] * 6, series='m2-bank'))
        bank = result['series']['m2-bank']
        self.assertEqual(bank['pooled']['p95'], 10)
        self.assertEqual(bank['runs'][2]['p95'], 60)
        self.assertEqual(bank['verdict'], 'not-met')

    def test_m3_boundary_uses_run_frequency_and_samples_file_is_ignored(self):
        rows = [(42, 'primary', 10_100 + i, 20, 20) for i in range(100)]
        rows += [(42, 'primary', 10_099, 9999, 9999)]
        directory = self.m3(rows=rows, qpc_frequency=20, stop=10_199, measured=0)
        (directory / 'samples.jsonl').write_text('not-json', encoding='utf-8')
        result = self.summarize(directory)
        self.assertEqual(result['unreadable'], [])
        run = result['series']['m3']['runs'][0]
        self.assertEqual(run['fps'], 50)
        self.assertEqual(run['context']['n'], 100)
        self.assertEqual(run['context']['covered_seconds'], 4.95)

    def test_swap_chain_selection_counts_rows_before_warmup_between_markers(self):
        rows = [(42, 'warm-chain', 10_000 + i, 20, 20) for i in range(200)]
        rows += [(42, 'warm-chain', 15_000 + i, 20, 20) for i in range(99)]
        rows += [(42, 'other-chain', 15_000 + i, 20, 20) for i in range(100)]
        result = self.summarize(self.m3(rows=rows))
        self.assertEqual(result['invalid_runs'][0]['reasons'], ['m3-too-few-frames'])
        self.assertEqual(result['series']['m3']['runs'], [])

    def test_swap_chain_ties_are_deterministic_and_metadata_is_sanitized(self):
        rows = [(42, name, 15_000 + i, interval, 20)
                for name, interval in [('Z:/chain-canary', 20), ('a-chain', 40)] for i in range(100)]
        directory = self.m3(rows=rows)
        run = self.summarize(directory)['series']['m3']['runs'][0]
        # Lexicographic tie break selects the drive-path chain; only the ignored
        # name is published, so test the reversed choice separately for privacy.
        self.assertEqual(run['fps'], 50)
        self.assertEqual(run['swap_chains_ignored'], [{'swap_chain': 'a-chain', 'rows': 100}])
        rows += [(42, 'a-chain', 16_100, 40, 20)]
        result = self.summarize(self.m3(rows=rows))
        run = result['series']['m3']['runs'][0]
        self.assertEqual(run['fps'], 25)
        self.assertEqual(run['swap_chains_ignored'], [{'swap_chain': 'redacted', 'rows': 100}])
        self.assertEqual(result['redactions'], 1)

    def test_presentmon_filter_warmup_first_hundred_and_context(self):
        rows = [(42, 'primary', 15_000 + i * 10, i + 1, '' if i == 10 else 20)
                for i in range(100)]
        rows += [(42, 'primary', 14_999, 9999, 9999),
                 (42, 'primary', 16_000, 200, 'not-a-number'),
                 (42, 'primary', 75_000, 300, 40),
                 (42, 'primary', 80_000, 400, 80),
                 (42, 'primary', 80_001, 'outside-markers', '')]
        rows += [(42, 'secondary', 15_000 + i * 10, 1, 1) for i in range(25)]
        rows += [(42, 'secondary', 90_000 + i, 1, 1) for i in range(500)]
        rows += [(99, 'foreign', 'not-qpc', 'not-an-interval', '') for _ in range(500)]
        directory = self.m3(rows=list(reversed(rows)), stop=80_000)
        result = self.summarize(directory)
        self.assertEqual(result['unreadable'], [])
        m3 = result['series']['m3']
        run = m3['runs'][0]
        self.assertAlmostEqual(run['fps'], 1000 / 50.5)
        self.assertEqual(run['m3b_p99_ms'], 99)
        self.assertEqual(run['swap_chains_ignored'], [{'swap_chain': 'secondary', 'rows': 25}])
        self.assertEqual(m3['pooled']['n'], 100)
        self.assertAlmostEqual(m3['pooled']['fps'], 1000 / 50.5)
        self.assertEqual(m3['pooled']['m3b_p99_ms'], 99)
        self.assertEqual(m3['verdict'], 'not-met')
        self.assertEqual(m3['m3b_verdict'], 'report')
        self.assertEqual(m3['target'], 'fps >= 30')
        context = run['context']
        self.assertEqual(context['n'], 102)
        self.assertEqual(context['covered_seconds'], 60)
        self.assertAlmostEqual(context['mean_fps'], 1000 / (5550 / 102))
        self.assertEqual([context[key] for key in ('p50', 'p95', 'p99', 'max')], [51, 97, 200, 300])
        self.assertEqual(context['display_missing'], 2)
        display = context['display']
        self.assertEqual(display['n'], 100)
        self.assertEqual(display['covered_seconds'], 60)
        self.assertAlmostEqual(display['mean_fps'], 1000 / 20.2)
        self.assertEqual([display[key] for key in ('p50', 'p95', 'p99', 'max')], [20, 20, 20, 40])

    def test_context_is_clipped_to_stop_and_display_is_optional(self):
        rows = [(42, 'primary', 15_000 + i * 10, 20) for i in range(100)]
        rows += [(42, 'primary', 17_000, 40), (42, 'primary', 17_001, 999)]
        directory = self.m3(rows=rows, stop=17_000, columns=[
            'ProcessID', 'SwapChainAddress', 'CPUStartQPC', 'MsBetweenPresents'])
        context = self.summarize(directory)['series']['m3']['runs'][0]['context']
        self.assertEqual(context['n'], 101)
        self.assertEqual(context['covered_seconds'], 2)
        self.assertEqual(context['max'], 40)
        self.assertNotIn('display', context)
        self.assertNotIn('display_missing', context)

    def test_missing_display_values_are_counted_without_zero_samples(self):
        rows = [(42, 'primary', 15_000 + i * 10, 20, value)
                for i, value in enumerate(['', 'junk', 'NaN', 'inf'] * 25)]
        context = self.summarize(self.m3(rows=rows))['series']['m3']['runs'][0]['context']
        self.assertEqual(context['display_missing'], 100)
        self.assertEqual(context['display']['n'], 0)
        self.assertIsNone(context['display']['mean_fps'])
        for key in ('p50', 'p95', 'p99', 'max'):
            self.assertIsNone(context['display'][key])

    def test_m3_too_few_frames_is_invalid_not_unreadable(self):
        rows = [(42, 'primary', 15_000 + i * 10, 20, 20) for i in range(99)]
        rows += [(42, 'primary', 14_999, 20, 20)]
        directory = self.m3(rows=rows)
        result = self.summarize(directory)
        self.assertEqual(result['unreadable'], [])
        self.assertEqual(result['invalid_runs'], [{'run_id': directory.name,
                         'series': 'm3', 'reasons': ['m3-too-few-frames']}])
        self.assertEqual(result['series']['m3']['verdict'], 'no-data')
        self.assertEqual(result['series']['m3']['m3b_verdict'], 'no-data')
        invalid = self.m3(rows=rows, status='invalid', invalid_reasons=['from-harness'],
                          attribution=True)
        result = self.summarize(invalid)
        self.assertEqual(result['invalid_runs'][0]['reasons'], ['from-harness', 'm3-too-few-frames'])
        self.assertEqual(result['attribution_runs'], [])

    def test_m3_pooled_each_run_target_formal_and_spread(self):
        directories = [self.m3(rows=[(42, 'primary', 15_000 + i, interval, 20)
                                    for i in range(100)]) for interval in (20, 20, 40)]
        m3 = self.summarize(*directories)['series']['m3']
        self.assertEqual([run['fps'] for run in m3['runs']], [50, 50, 25])
        self.assertEqual(m3['pooled'], {'n': 300, 'fps': 37.5, 'm3b_p99_ms': 40})
        self.assertEqual(m3['verdict'], 'not-met')  # Pooled fps meets, one run does not.
        self.assertTrue(m3['formal'])
        self.assertEqual(m3['spread'], 1)
        self.assertTrue(m3['needs_extra_run'])
        met = self.summarize(directories[0])['series']['m3']
        self.assertEqual(met['verdict'], 'met')
        self.assertFalse(met['formal'])
        for interval, flag in [(100 / 1.19, False), (100 / 1.21, True)]:
            low = self.m3(rows=[(42, 'primary', 15_000 + i, 100, 20) for i in range(100)])
            high = self.m3(rows=[(42, 'primary', 15_000 + i, interval, 20) for i in range(100)])
            self.assertEqual(self.summarize(low, high)['series']['m3']['needs_extra_run'], flag)

    def test_m3_attribution_statistics_and_invalid_exclusion(self):
        good = self.m3()
        attribution = self.m3(attribution=True)
        invalid = self.m3(status='invalid', invalid_reasons=['bad-setup'])
        result = self.summarize(good, attribution, invalid)
        self.assertEqual(result['series']['m3']['pooled']['n'], 100)
        self.assertEqual(result['attribution_runs'][0]['series'], 'm3')
        self.assertEqual(result['attribution_runs'][0]['fps'], 50)
        self.assertEqual(result['attribution_runs'][0]['context']['n'], 100)
        self.assertEqual(result['invalid_runs'][0]['reasons'], ['bad-setup'])

    def test_presentmon_column_override_file_and_partial_mapping(self):
        names = ['pid', 'chain', 'qpc', 'interval', 'display']
        directory = self.m3(columns=names)
        mapping = dict(zip(('process', 'swap_chain', 'time', 'frame_interval', 'display_interval'), names))
        override = self.base / 'columns.json'
        self.write_json(override, mapping)
        completed, out, _ = self.cli(directory, extra=('--presentmon-columns', str(override)))
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertEqual(json.loads(out.read_text(encoding='utf-8'))['series']['m3']['pooled']['fps'], 50)
        partial = self.m3(columns=['pid', 'SwapChainAddress', 'CPUStartQPC',
                                   'MsBetweenPresents', 'MsBetweenDisplayChange'])
        self.assertEqual(self.summarize(partial, columns={'process': 'pid'})['unreadable'], [])

    def test_presentmon_fields_and_required_columns_are_validated(self):
        for field, value in [('rotation', {}), ('rotation', []),
                             ('rotation', {'start_qpc': True, 'stop_qpc': 20_000}),
                             ('rotation', {'start_qpc': 10_000, 'stop_qpc': 1}),
                             ('presentmon', {}), ('presentmon', {'process_id': True}),
                             ('presentmon', {'process_id': 42.5})]:
            with self.subTest(field=field, value=value):
                self.unreadable(self.m3(**{field: value}), 'bad-field')
        names = ['ProcessID', 'SwapChainAddress', 'CPUStartQPC',
                 'MsBetweenPresents', 'MsBetweenDisplayChange']
        for index in range(4):
            with self.subTest(column=names[index]):
                modified = names.copy()
                modified[index] = 'wrong'
                self.unreadable(self.m3(columns=modified), 'presentmon-missing-column')
        for index, value in [(0, 'not-pid'), (1, ''), (2, '15000.0'), (2, 'junk'),
                             (3, ''), (3, 'NaN'), (3, 'inf'), (3, '-1')]:
            with self.subTest(index=index, value=value):
                rows = [(42, 'primary', 15_000 + i, 20, 20) for i in range(100)]
                bad = list(rows[0])
                bad[index] = value
                rows[0] = bad
                self.unreadable(self.m3(rows=rows), 'bad-field')
        self.unreadable(self.m3(rows=[(42, 'primary', 90_000, 20, 20)]), 'presentmon-no-rows')
        self.unreadable(self.m3(rows=[(42, 'primary', 15_000 + i, 0, 20)
                                     for i in range(100)]), 'bad-field')

    def test_public_environment_allowlist_and_sanitizer_canaries(self):
        windows = r'C:\private-canary\file'
        posix = '/home/posix-canary/file'
        unc = r'\\server-canary\share'
        private = 'non-public-canary'
        names = {'USERNAME': 'SensitiveUser', 'USER': 'SensitiveUnixUser',
                 'COMPUTERNAME': 'SensitiveComputer', 'HOSTNAME': 'SensitiveHost'}
        host = 'SensitiveSocketHost'
        directory = self.latency(run_id='SensitiveUser-SensitiveSocketHost')
        renamed = directory.with_name('SensitiveUser-SensitiveSocketHost')
        directory.rename(renamed)
        environment = {
            'build_identity': windows, 'executable_sha256': posix, 'package_sha256': unc,
            'gpus': [{'name': 'Fixture GPU', 'driver_version': 'prefixSENSITIVEUSERsuffix',
                      'secret': private}], 'presenting_adapter': host.upper(),
            'cpu_name': 'Fixture CPU', 'ram_bytes': 1024,
            'display': {'width': 1920, 'height': 1080, 'refresh_hz': 60, 'dpi': 96, 'secret': private},
            'backbuffer': {'width': 1920, 'height': 1080, 'secret': private},
            'presentation_interval': 1, 'power_source': 'AC',
            'power_mode': 'prefixSensitiveUnixUsersuffix', 'gpu_processes_at_start': 0,
            'declared': {'vendor_mode': 'SensitiveComputer', 'frame_generation': False,
                         'driver_vsync': True, 'overlays': ['SensitiveHost', 'clean'], 'secret': private},
            'non_public_key': private,
        }
        self.write_json(renamed / 'environment.json', environment)
        missing = self.base / 'SensitiveComputer-SensitiveHost'
        missing.mkdir()
        with mock.patch.dict(os.environ, names), mock.patch.object(socket, 'gethostname', return_value=host):
            result = self.summarize(renamed, missing)
        out = self.base / 'sanitized.json'
        out.write_text(json.dumps(result), encoding='utf-8')
        markdown = self.base / 'sanitized.md'
        markdown.write_text(summary.render_markdown(result), encoding='utf-8')
        canaries = [windows, posix, unc, private, 'non_public_key', str(self.base),
                    *names.values(), host]
        for output in (out, markdown):
            text = output.read_text(encoding='utf-8')
            # Decode JSON so escaping cannot disguise an absolute-path leak.
            for canary in canaries:
                self.assertNotIn(canary.casefold(), text.casefold())
                if output == out:
                    self.assertNotIn(canary.casefold(), str(json.loads(text)).casefold())
        self.assertEqual(result['redactions'], 11)
        copied = result['environment'][0]
        self.assertEqual(set(copied), (set(environment) - {'non_public_key'}) | {'run_id'})
        self.assertEqual(copied['gpus'], [{'name': 'Fixture GPU', 'driver_version': 'redacted'}])
        self.assertEqual(set(copied['display']), {'width', 'height', 'refresh_hz', 'dpi'})
        self.assertEqual(set(copied['backbuffer']), {'width', 'height'})
        self.assertEqual(set(copied['declared']), {'vendor_mode', 'frame_generation', 'driver_vsync', 'overlays'})
        self.assertEqual(copied['declared']['overlays'], ['redacted', 'clean'])
        self.assertEqual(result['unreadable'][0]['directory'], 'redacted')

    def test_actual_user_and_host_canaries_are_sanitized_in_cli(self):
        canaries = [os.environ.get(key, '') for key in ('USERNAME', 'USER', 'COMPUTERNAME', 'HOSTNAME')]
        canaries.append(socket.gethostname())
        canaries = [value for value in canaries if len(value) >= 3]
        directory = self.latency()
        self.write_json(directory / 'environment.json', {'gpus': [
            {'name': 'prefix' + value.upper() + 'suffix', 'driver_version': 'fixture'} for value in canaries]})
        completed, out, markdown = self.cli(directory)
        self.assertEqual(completed.returncode, 0, completed.stderr)
        result = json.loads(out.read_text(encoding='utf-8'))
        self.assertEqual(result['redactions'], len(canaries))
        for output in (out, markdown):
            text = output.read_text(encoding='utf-8').casefold()
            for canary in canaries:
                self.assertNotIn(canary.casefold(), text)

    def test_sanitizer_also_covers_metadata_and_word_boundaries(self):
        directory = self.latency(status='invalid', run_id=r'Z:/metadata-canary/run',
                                 build={'build_identity': '/Users/build-canary'},
                                 invalid_reasons=[r'\\reason-canary\share', 'path=/tmp/reason-canary'])
        result = self.summarize(directory)
        self.assertEqual(result['invalid_runs'][0]['run_id'], 'redacted')
        self.assertEqual(result['invalid_runs'][0]['reasons'], ['redacted', 'redacted'])
        self.assertEqual(result['series']['m1-active']['build_identity'], 'redacted')
        self.assertEqual(result['redactions'], 5)
        directory = self.latency()
        self.write_json(directory / 'environment.json', {'cpu_name': 'vendor/model',
                                                        'power_source': 'ab'})
        with mock.patch.dict(os.environ, {'USERNAME': 'ab', 'USER': 'xy', 'COMPUTERNAME': '', 'HOSTNAME': ''}), \
                mock.patch.object(socket, 'gethostname', return_value='zz'):
            result = self.summarize(directory)
        self.assertEqual(result['environment'][0]['cpu_name'], 'vendor/model')
        self.assertEqual(result['environment'][0]['power_source'], 'ab')
        self.assertEqual(result['redactions'], 0)

    def test_malformed_public_environment_cannot_smuggle_private_fields(self):
        for environment in ({'gpus': {}}, {'gpus': [1]}, {'display': []},
                            {'backbuffer': 'path'}, {'declared': []},
                            {'gpu_processes_at_start': True},
                            {'cpu_name': {'private': 'canary'}}):
            with self.subTest(environment=environment):
                directory = self.latency()
                self.write_json(directory / 'environment.json', environment)
                self.unreadable(directory, 'bad-field')

    def test_optional_renderer_controls_are_published(self):
        directory = self.latency()
        environment = {'msaa': 4, 'vsync': False, 'tearing': True, 'warp': False,
                       'declared': {'upscaling': False}}
        self.write_json(directory / 'environment.json', environment)
        self.assertEqual(self.summarize(directory)['environment'], [{'run_id': directory.name, **environment}])
        self.assertEqual(summary.public_environment({}), {})

    def test_markdown_table_order_extra_runs_rounding_and_notes(self):
        runs = [self.latency([12.34567] * 100) for _ in range(4)]
        runs += [self.latency([25.5555] * 100, series='m2-local'), self.m3(),
                 self.latency(status='invalid', invalid_reasons=['fixture-invalid'])]
        missing = self.base / 'missing-run'
        result = self.summarize(*runs, missing)
        markdown = summary.render_markdown(result)
        lines = [line for line in markdown.splitlines() if line.startswith('|')]
        self.assertIn('Run 4 p95', lines[0])
        self.assertIn('Pooled p95 (n=…)', lines[0])
        expected = ['M1 instant turn (active orbit)', 'M1 instant turn (all pieces)',
                    'M2 bank navigation', 'M2 Local-centre navigation',
                    'M3 full-detail fps (mean)', 'M3b frame time p99']
        self.assertEqual([line.split('|')[1].strip() for line in lines[2:]], expected)
        self.assertIn('12.35 (n=400)', lines[2])
        self.assertIn('–', lines[3])
        self.assertIn('no-data', lines[3])
        self.assertIn('50.0', lines[6])
        self.assertIn('20.00', lines[7])
        self.assertIn('report', lines[7])
        self.assertIn('m1-active: 4', markdown)
        self.assertIn('formal 3 x 100 result', markdown)
        self.assertIn('baseline, not a formal 3 x 100 result', markdown)
        self.assertIn('fixture-invalid', markdown)
        self.assertIn('missing-run: missing-file', markdown)
        self.assertIn('fixture-build', markdown)
        self.assertAlmostEqual(result['series']['m1-active']['pooled']['mean'], 12.34567)

    def test_markdown_escapes_untrusted_table_characters(self):
        directory = self.latency(build={'build_identity': 'fixture|build\nnext'})
        invalid = self.latency(status='invalid', run_id='run|id\nnext',
                               invalid_reasons=['reason|text\nnext'],
                               build={'build_identity': 'fixture|build\nnext'})
        markdown = summary.render_markdown(self.summarize(directory, invalid))
        self.assertIn('fixture\\|build next', markdown)
        self.assertIn('run\\|id next', markdown)
        self.assertIn('reason\\|text next', markdown)

    def test_cli_exit_codes_and_read_only_inputs(self):
        directory = self.latency([101] * 100)
        original = {path: path.read_bytes() for path in directory.iterdir()}
        completed, out, markdown = self.cli(directory)
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertTrue(markdown.is_file())
        self.assertEqual(json.loads(out.read_text(encoding='utf-8'))['series']['m1-active']['verdict'], 'not-met')
        self.assertEqual({path: path.read_bytes() for path in directory.iterdir()}, original)
        before = out.read_bytes(), markdown.read_bytes()
        completed, _, _ = self.cli(directory)
        self.assertEqual(completed.returncode, 2)
        self.assertEqual((out.read_bytes(), markdown.read_bytes()), before)

    def test_existing_markdown_prevents_any_output_creation(self):
        directory = self.latency()
        markdown = self.base / 'summary.md'
        markdown.write_text('keep', encoding='utf-8')
        completed, out, _ = self.cli(directory)
        self.assertEqual(completed.returncode, 2)
        self.assertFalse(out.exists())
        self.assertEqual(markdown.read_text(encoding='utf-8'), 'keep')

    def test_no_readable_directory_exit_one_but_readable_invalid_exit_zero(self):
        completed, out, markdown = self.cli(self.base / 'missing')
        self.assertEqual(completed.returncode, 1)
        self.assertFalse(out.exists())
        self.assertFalse(markdown.exists())
        completed, out, _ = self.cli(self.latency(status='invalid', invalid_reasons=['invalid']))
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertEqual(json.loads(out.read_text(encoding='utf-8'))['series']['m1-active']['verdict'], 'no-data')

    def test_mixed_builds_include_invalid_and_attribution_but_are_per_series(self):
        first = self.latency()
        other = self.latency(attribution=True, build={'build_identity': 'different-build'})
        completed, out, _ = self.cli(first, other)
        self.assertEqual(completed.returncode, 2)
        self.assertIn('mixed-builds', completed.stderr)
        self.assertFalse(out.exists())
        self.change_run(other, attribution=False, status='invalid', invalid_reasons=['invalid'])
        completed, out, _ = self.cli(first, other)
        self.assertEqual(completed.returncode, 2)
        self.assertFalse(out.exists())
        self.change_run(other, series='m1-all', status='valid')
        samples = self.samples(other)
        for sample in samples:
            sample['series'] = 'm1-all'
        self.write_samples(other, samples)
        completed, out, _ = self.cli(first, other)
        self.assertEqual(completed.returncode, 0, completed.stderr)
        result = json.loads(out.read_text(encoding='utf-8'))
        self.assertEqual(result['series']['m1-all']['build_identity'], 'different-build')

    def test_cli_usage_bad_mapping_and_optional_markdown(self):
        directory = self.latency()
        cases = [[], [str(directory)], ['--out', str(self.base / 'unused')],
                 [str(directory), '--out', str(self.base / 'same'), '--markdown', str(self.base / 'same')]]
        for arguments in cases:
            with self.subTest(arguments=arguments):
                completed = subprocess.run([sys.executable, '-B', str(ROOT / 'tools/perf/b412_summary.py'),
                                            *arguments], capture_output=True, check=False)
                self.assertEqual(completed.returncode, 2)
        override = self.base / 'bad-columns.json'
        for contents in ('{', '[]', '{"unknown": "value"}', '{"time": 1}', '{"time": ""}'):
            override.write_text(contents, encoding='utf-8')
            completed, out, _ = self.cli(directory, extra=('--presentmon-columns', str(override)))
            self.assertEqual(completed.returncode, 2)
            self.assertFalse(out.exists())
        completed, out, _ = self.cli(directory, extra=('--presentmon-columns', str(self.base / 'missing-map')))
        self.assertEqual(completed.returncode, 2)
        self.assertFalse(out.exists())
        completed = subprocess.run([sys.executable, '-B', str(ROOT / 'tools/perf/b412_summary.py'),
                                    str(directory), '--out', str(out)], capture_output=True, check=False)
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertTrue(out.is_file())
        self.assertFalse((self.base / 'summary.md').exists())

    def test_ignored_run_and_sample_fields_do_not_appear(self):
        directory = self.latency(private_key='private-canary')
        samples = self.samples(directory)
        samples[0]['private_key'] = 'private-canary'
        self.write_samples(directory, samples)
        result = self.summarize(directory)
        self.assertNotIn('private-canary', json.dumps(result))
        self.assertNotIn('private_key', json.dumps(result))

    def test_main_returns_success_without_stdout_payload(self):
        directory = self.latency()
        stdout, stderr = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            code = summary.main([str(directory), '--out', str(self.base / 'out.json')])
        self.assertEqual(code, 0)
        self.assertEqual(stdout.getvalue(), '')
        self.assertEqual(stderr.getvalue(), '')


if __name__ == '__main__':
    unittest.main()
