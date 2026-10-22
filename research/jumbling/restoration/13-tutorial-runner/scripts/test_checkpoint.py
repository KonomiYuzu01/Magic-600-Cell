"""P6 checkpoint tests: isolated Git repositories, no remotes and --no-push.

All fixtures and caches are temporary. No real scramble is generated here.
"""
import copy
import hashlib
import io
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
for name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[name] = '1'
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import checkpoint as C  # noqa: E402
import test_runner as T  # noqa: E402; existing fake mechanics and literal fixtures
R, api = T.R, T.api


class CheckpointTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix='tutorial-checkpoint-test-'))
        self.addCleanup(shutil.rmtree, self.tmp)
        self.repo = self.tmp / 'repo'
        self.repo.mkdir()
        self.git('init', '-b', 'packet-test')
        self.git('config', 'user.name', 'Checkpoint Fixture')
        self.git('config', 'user.email', 'checkpoint@example.invalid')
        # Do not inherit personal Git hooks, signing, identity or index settings.
        self.git('config', 'core.hooksPath', str(self.tmp / 'no-hooks'))
        self.git('config', 'commit.gpgsign', 'false')
        environment = patch.dict(os.environ, JUMBLING_HOME_CACHE=str(self.tmp / 'cache/home_centroids.npy'))
        environment.start()
        self.addCleanup(environment.stop)

    def git(self, *args):
        return subprocess.check_output(['git', *args], cwd=self.repo, stderr=subprocess.DEVNULL)

    def args(self, name='sample', **changes):
        args = R.parse_args(['--scramble', '1', '7', '--out', str(self.repo / 'runs' / name),
                             '--cache', str(self.tmp / 'cache'), '--bulk-steps', '0',
                             '--fallback-max-length', '0', '--no-lattice'])
        for key, value in changes.items():
            setattr(args, key, value)
        return args

    def runner(self, args, script=None, start=None):
        script = copy.deepcopy(script or {})
        if args.resume:
            saved = R.read_json(args.out / 'run.json')
            for name in script:
                count = sum((s['subphase'] or s['phase']) == name for s in saved['steps'])
                script[name] = script[name][count:]
        modules = T.plugins(height=T.Plugin('height', {k: v for k, v in script.items() if k != 'high'}),
                            high=T.Plugin('high', {'high': script.get('high', [])}))
        return T.StubRunner(args, modules, start)

    def run_stub(self, runner):
        with T.bulk_finds_nothing(runner):
            return runner.run()

    def interrupt(self, runner, step):
        save = runner.save

        def crash():
            save()
            if len(runner.record['steps']) == step:
                raise T.PowerLoss()

        with patch.object(runner, 'save', side_effect=crash), patch.object(runner, 'finish'):
            with self.assertRaises(T.PowerLoss):
                self.run_stub(runner)

    def fresh_checkout(self, label):
        """Exactly the committed tree a fresh clone has, without adding a remote or using transports."""
        fresh = self.tmp / ('fresh-' + label)
        fresh.mkdir()
        raw = self.git('archive', '--format=tar', 'HEAD')
        with tarfile.open(fileobj=io.BytesIO(raw)) as archive:
            archive.extractall(fresh, filter='data')
        return fresh

    def test_stages_only_complete_snapshot_and_skips_unchanged(self):
        args = self.args()
        runner = self.runner(args)
        self.interrupt(runner, 1)
        for name in ('untracked.txt', 'cache/junk', 'logs/junk', 'run.json.tmp'):
            path = args.out / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text('do not checkpoint')
        (self.repo / '.gitignore').write_text('*\n')
        cp = C.Checkpointer(args.out, no_push=True, trailers=['Evidence: fixture', 'Packet: P6'])
        staged = []
        git = cp.git

        def audit(*arguments, **kwargs):
            if arguments[0] == 'commit':
                staged.extend(os.fsdecode(p) for p in git('diff', '--cached', '--name-only', '-z').stdout.split(b'\0') if p)
            return git(*arguments, **kwargs)

        with patch.object(cp, 'git', side_effect=audit):
            self.assertTrue(cp.round())
        record = R.read_json(args.out / 'run.json')
        names = {'run.json', 'start-table.json'} | {name for _, name, _ in C.references(record)}
        expected = {cp.relative + '/' + name for name in names}
        self.assertEqual(set(staged), expected)
        self.assertEqual(set(os.fsdecode(p) for p in self.git('ls-tree', '-r', '--name-only', '-z', 'HEAD').split(b'\0') if p), expected)
        self.assertEqual(self.git('remote'), b'')
        body = self.git('log', '-1', '--format=%B').decode()
        self.assertIn('Checkpoint sample: step 1', body)
        self.assertIn('Evidence: fixture\nPacket: P6', body)
        head = self.git('rev-parse', 'HEAD')
        self.assertFalse(cp.round())
        self.assertFalse(C.Checkpointer(args.out, no_push=True).round())
        self.assertEqual(self.git('rev-parse', 'HEAD'), head)
        # A superseding snapshot also removes stale files from its staged directory.
        junk = cp.checkpoint / 'obsolete.txt'
        junk.write_text('old checkpoint artifact')
        self.git('add', '-f', str(junk))
        self.git('commit', '-m', 'Fixture obsolete artifact')
        record['totals']['seconds'] += 1
        R.atomic_json(args.out / 'run.json', record)
        self.assertTrue(cp.round())
        self.assertFalse(junk.exists())
        self.assertEqual(self.git('status', '--porcelain', '--untracked-files=no'), b'')

    def test_refuses_protected_detached_outside_and_unrelated_index(self):
        args = self.args()
        self.runner(args).initialize()
        self.git('commit', '--allow-empty', '-m', 'Fixture root')
        for name in ('main', 'claude/cowork-chat-memories-0j732y'):
            self.git('checkout', '-b', name)
            with self.assertRaisesRegex(C.CheckpointError, 'protected'):
                C.Checkpointer(args.out, no_push=True)
        self.git('checkout', '--detach')
        with self.assertRaisesRegex(C.CheckpointError, 'detached'):
            C.Checkpointer(args.out, no_push=True)
        self.git('checkout', 'packet-test')
        with self.assertRaises(C.CheckpointError):
            C.Checkpointer(self.tmp / 'outside', no_push=True)
        unrelated = self.repo / 'unrelated.txt'
        unrelated.write_text('retain staged content')
        self.git('add', 'unrelated.txt')
        before = self.git('diff', '--cached', '--binary')
        with self.assertRaisesRegex(C.CheckpointError, 'outside'):
            C.Checkpointer(args.out, no_push=True)
        self.assertEqual(self.git('diff', '--cached', '--binary'), before)
        self.assertFalse(args.out.with_name(args.out.name + '-checkpoint').exists())

    def test_restored_checkpoints_resume_bulk_height_high_and_tail(self):
        scenarios = (
            ('bulk', T.FakeState({1, 2}), {}, 0),
            ('height', T.FakeState({1, 2, 3}, {1: 6, 2: 4}), T.ResumeTests.SCRIPT, 1),
            ('high', T.FakeState({1, 2}, {1: 6}),
             {'high': [('r22d25/F1', [[1, 4]]), ('r22d25/F2', [[1, 2]])]}, 1),
            ('tail', T.FakeState({1, 2}), {}, 1))
        for name, state, script, step in scenarios:
            with self.subTest(phase=name):
                original = self.run_stub(self.runner(self.args('whole-' + name), script, state))
                args = self.args(name)
                interrupted = self.runner(args, script, state)
                self.interrupt(interrupted, step)
                cp = C.Checkpointer(args.out, no_push=True)
                self.assertTrue(cp.round())
                fresh = self.fresh_checkout(name)
                restored = fresh / args.out.relative_to(self.repo)
                self.assertFalse(restored.exists())
                C.restore(restored)
                args.out, args.resume = restored, True
                result = self.run_stub(self.runner(args, script, state))
                self.assertEqual(T.without_times(original), T.without_times(result))
                self.assertEqual(result['resumes'][0]['at_step'], step)
                self.assertEqual(T.replay(self, restored, result).digest(), original['steps'][-1]['after']['digest'])

    def test_tail_probe_and_finished_lattice_dependencies_survive(self):
        args = self.args('probe', mode='acceptance', fallback_max_length=1)
        runner = self.runner(args)
        runner.fail = {2}
        runner.fallback = lambda unit, work: runner.make_candidate(runner.table, unit, work / 'fallback', 'fallback')
        with patch.object(R, 'acceptance_config', side_effect=T.any_acceptance_config):
            record = self.run_stub(runner)
        self.assertTrue(record['stuck']['probe'])
        self.assertEqual(len(record['steps']), 1)
        cp = C.Checkpointer(args.out, no_push=True)
        self.assertTrue(cp.round())
        fresh = self.fresh_checkout('probe')
        restored = fresh / 'runs/probe'
        C.restore(restored)
        self.assertEqual(R.read_json(restored / 'run.json'), record)
        self.assertTrue((restored / 'units/00000.json').is_file())
        self.assertTrue((restored / 'probe/unit.json').is_file())
        self.assertTrue((restored / 'probe/word.json').is_file())

        args = self.args('lattice', no_lattice=False)
        runner = T.StubRunner(args, T.plugins(lattice=T.LatticePlay([('lattice/orbit-0', [[0, 0]], None)])),
                              T.FakeState((), turn=1))
        record = runner.run()
        cp = C.Checkpointer(args.out, no_push=True)
        self.assertTrue(cp.round())
        fresh = self.fresh_checkout('lattice')
        restored = fresh / 'runs/lattice'
        C.restore(restored)
        self.assertEqual(R.read_json(restored / 'run.json'), record)
        C.verify_copy(restored, record)
        self.assertTrue((restored / 'lattice/report.json').is_file())
        self.assertTrue((restored / record['steps'][-1]['table']).is_file())

    def test_real_provenance_needs_no_run_local_repository_files(self):
        m = R.Mechanics(self.tmp / 'cache')
        source = self.tmp / 'source-table.json'
        m.export(m.sim.State(m.restore.ctx, menu=m.menu), source)
        args = R.parse_args(['--table', str(source), '--out', str(self.repo / 'runs/provenance'),
                             '--cache', str(self.tmp / 'cache'), '--mode', 'acceptance'])
        record = R.Runner(args, m).run()
        self.assertTrue(record['config']['acceptance_config']['file'])
        self.assertTrue(record['config']['kernel']['d19']['file'])
        before = C.checker.check_run(args.out, lambda row: None)
        self.assertTrue(before['pass'], before)
        cp = C.Checkpointer(args.out, no_push=True)
        with patch.object(C, 'copy_round', wraps=C.copy_round) as copies:
            self.assertTrue(cp.round())
            self.assertEqual(copies.call_count, 1)
        fresh = self.fresh_checkout('provenance')
        restored = fresh / 'runs/provenance'
        C.restore(restored)
        self.assertFalse((restored / 'research').exists())
        after = C.checker.check_run(restored, lambda row: None)
        self.assertEqual(before, after)

    def test_real_checkpoint_restores_resumes_and_keeps_checker_verdict(self):
        m = R.Mechanics(self.tmp / 'cache')
        source = self.tmp / 'source-table.json'
        state = m.sim.State(m.restore.ctx, menu=m.menu)
        T.play_real(m, state, T.WORD36)
        m.export(state, source)

        def arguments(name):
            return R.parse_args(['--table', str(source), '--out', str(self.repo / 'runs' / name),
                                 '--cache', str(self.tmp / 'cache'), '--bulk-steps', '0',
                                 '--tail-rules', '', '--fallback-max-length', '0'])

        args = arguments('real')
        original_args = arguments('whole-real')
        original = self.run_stub(R.Runner(original_args, m))
        cut = R.Runner(args, m)
        self.interrupt(cut, 2)
        cp = C.Checkpointer(args.out, no_push=True)
        self.assertTrue(cp.round())
        fresh = self.fresh_checkout('real-resume')
        restored = fresh / 'runs/real'
        shutil.rmtree(args.out)
        source.unlink()
        C.restore(restored)
        args.out, args.table, args.resume = restored, restored / 'start-table.json', True
        result = self.run_stub(R.Runner(args, m))
        self.assertEqual(T.without_times(original), T.without_times(result))
        before = C.checker.check_run(original_args.out, lambda row: None)
        after = C.checker.check_run(restored, lambda row: None)
        self.assertTrue(before['pass'], before)
        self.assertEqual(before, after)

    def test_consistency_race_retries_and_corruption_preserves_previous_copy(self):
        args = self.args()
        runner = self.runner(args)
        self.interrupt(runner, 1)
        cp = C.Checkpointer(args.out, no_push=True)
        verify = C.verify_copy
        calls = []

        def barrier(base, record):
            calls.append(record['totals']['seconds'])
            if len(calls) == 1:
                changed = R.read_json(args.out / 'run.json')
                changed['totals']['seconds'] += 10
                R.atomic_json(args.out / 'run.json', changed)
            verify(base, record)

        with patch.object(C, 'verify_copy', side_effect=barrier):
            self.assertTrue(cp.round())
        self.assertEqual(len(calls), 2)
        self.assertEqual((cp.checkpoint / 'run.json').read_bytes(), (args.out / 'run.json').read_bytes())
        previous = self.git('rev-parse', 'HEAD')
        record = R.read_json(args.out / 'run.json')
        record['totals']['seconds'] += 1
        R.atomic_json(args.out / 'run.json', record)
        for name in (record['steps'][0]['word'], record['steps'][0]['tree']['file'], 'tables/bulk-end.json',
                     record['steps'][0]['table']):
            with self.subTest(corrupt=name):
                path = args.out / name
                saved = path.read_bytes()
                path.write_bytes(b'corrupted')
                with patch.object(C, 'copy_round', wraps=C.copy_round) as copies, patch('sys.stderr', new=io.StringIO()):
                    self.assertFalse(cp.round())
                    self.assertEqual(copies.call_count, 5)
                self.assertEqual(self.git('rev-parse', 'HEAD'), previous)
                path.write_bytes(saved)
        self.assertTrue(cp.round())

    def test_restore_refuses_nonempty_and_checkpoint_paths_cannot_escape(self):
        args = self.args()
        runner = self.runner(args)
        runner.initialize()
        cp = C.Checkpointer(args.out, no_push=True)
        self.assertTrue(cp.round())
        before = (args.out / 'run.json').read_bytes()
        with self.assertRaisesRegex(C.CheckpointError, 'empty'):
            C.restore(args.out)
        self.assertEqual((args.out / 'run.json').read_bytes(), before)
        record = R.read_json(args.out / 'run.json')
        record['start']['file'] = '../outside.json'
        R.atomic_json(args.out / 'run.json', record)
        with patch('sys.stderr', new=io.StringIO()):
            self.assertFalse(cp.round())

    def test_final_round_after_pid_exits_and_network_retry_schedule(self):
        args = self.args()
        self.runner(args).initialize()
        with patch.object(C, 'alive', return_value=False):
            self.assertEqual(C.main(['--run', str(args.out), '--pid', '1', '--no-push']), 0)
        self.assertIn('Checkpoint sample: step 0', self.git('log', '-1', '--format=%s').decode())
        cp = C.Checkpointer(args.out, no_push=True)
        cp.no_push, cp.pending_push = False, True
        failed = SimpleNamespace(returncode=1, stderr=b'Could not resolve host', stdout=b'')
        success = SimpleNamespace(returncode=0, stderr=b'', stdout=b'')
        with patch.object(cp, 'git', side_effect=[failed] * 4 + [success]) as git, patch.object(C.time, 'sleep') as sleep:
            cp.push()
        self.assertEqual([c.args[0] for c in sleep.call_args_list], [2, 4, 8, 16])
        self.assertTrue(all(c.args == ('push', '-u', 'origin', 'HEAD') for c in git.call_args_list))
        self.assertFalse(cp.pending_push)
        cp.pending_push = True
        rejected = SimpleNamespace(returncode=1, stderr=b'rejected (non-fast-forward)', stdout=b'')
        with patch.object(cp, 'git', return_value=rejected), patch.object(C.time, 'sleep') as sleep:
            with self.assertRaises(C.CheckpointError):
                cp.push()
            sleep.assert_not_called()

    def test_kernel_record_dir_is_kept_and_a_failed_round_does_not_end_the_helper(self):
        # The D2 shape: high ends stuck and names its kernel record by `dir` and `record_sha256`.
        args = self.args('kernel')
        runner = self.runner(args, start=T.FakeState({1, 2}, {1: 6}))

        def stuck(ctx):
            kernel = ctx.work_dir / 'kernel'
            kernel.mkdir(parents=True)
            R.atomic_json(kernel / 'run.json', {'result': 'stuck'})
            return api.End('stuck', 'kernel_exhausted', details={
                'dir': runner.rel(kernel),
                'record_sha256': hashlib.sha256((kernel / 'run.json').read_bytes()).hexdigest()})

        runner.plugins['high'].next_step = stuck
        record = self.run_stub(runner)
        self.assertEqual(record['phases']['high']['details']['dir'], 'attempts/00000-high/kernel')
        cp = C.Checkpointer(args.out, no_push=True)
        self.assertTrue(cp.round())
        self.assertTrue((cp.checkpoint / 'attempts/00000-high/kernel/run.json').is_file())
        fresh = self.fresh_checkout('kernel')
        C.restore(fresh / 'runs/kernel')
        self.assertTrue((fresh / 'runs/kernel/attempts/00000-high/kernel/run.json').is_file())
        (cp.checkpoint / 'attempts/00000-high/kernel/run.json').write_text('{}')
        with self.assertRaises(ValueError):
            C.verify_copy(cp.checkpoint, record)
        # A round that fails while the runner lives (a busy index, say) is reported, and the helper goes on.
        rounds = []

        def round_effect():
            rounds.append(len(rounds))
            if len(rounds) == 1:
                raise C.CheckpointError('git add failed: index.lock exists')
            return True

        with patch.object(C.Checkpointer, 'round', side_effect=round_effect), \
                patch.object(C, 'alive', side_effect=[True, False]), patch.object(C.time, 'sleep'):
            self.assertEqual(C.main(['--run', str(args.out), '--pid', '1', '--no-push']), 0)
        self.assertEqual(rounds, [0, 1])

    def test_helper_restart_recovers_uncommitted_copy_and_retries_a_saved_commit(self):
        args = self.args()
        self.runner(args).initialize()
        checkpoint = args.out.with_name(args.out.name + '-checkpoint')
        C.copy_round(args.out, checkpoint)     # helper died before staging/committing
        cp = C.Checkpointer(args.out, no_push=True)
        self.assertTrue(cp.round())
        restarted = C.Checkpointer(args.out, no_push=False)
        self.assertTrue(restarted.pending_push)
        # No network call: prove the unchanged-record skip follows a pending push.
        with patch.object(restarted, 'push') as push:
            self.assertFalse(restarted.round())
            push.assert_called_once_with()


if __name__ == '__main__':
    unittest.main(verbosity=2)
