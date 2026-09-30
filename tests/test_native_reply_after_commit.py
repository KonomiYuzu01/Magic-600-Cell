"""A durable native command is not reported as failed when its paired snapshot fails (screening C-03)."""
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
import io, json, sys, tempfile, threading, unittest
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT)); sys.path.insert(0, str(ROOT / 'work/experiments/magic600-04'))
from core import Model
from session import Session
from enhanced import Workflow
import adapter

MODEL = Model()


class FailingSnapshots:
    def read(self, *args, **kwargs):
        raise OSError('injected snapshot failure')


class NativeReplyAfterCommitTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='c600-native-reply-')
        self.addCleanup(self.tmp.cleanup)
        self.s = Session(MODEL, Path(self.tmp.name) / 'session')
        self.addCleanup(self.s.close)
        self.pool = ThreadPoolExecutor(1)
        self.addCleanup(self.pool.shutdown)
        self.jobs = {}
        context = dict(session=self.s, lock=threading.RLock(), workflow=Workflow(self.s), native_profile=[{'profile_sha256': 'fixture'}],
                       token='t', jobs=self.jobs, pool=self.pool, cancel_event=threading.Event(), model=self.s.m,
                       native_snapshots=FailingSnapshots())

        class Handler:
            def do_GET(self): pass
            def do_POST(self): self.replies.append(({'error': 'fell through'}, 404))
        adapter.install(Handler, context)
        self.Handler = Handler

    def native_command(self, body):
        h = self.Handler(); h.replies = []
        raw = json.dumps(body).encode()
        h.path = '/api/experiment/native-command'
        h.headers = {'Content-Length': str(len(raw)), 'Content-Type': 'application/json', 'Host': 'h'}
        h.rfile = io.BytesIO(raw); h.valid_host = lambda: True; h.authenticated = lambda: True
        h.js = lambda obj, status=200: h.replies.append((obj, status))
        self.Handler.do_POST(h)
        reply, status = h.replies[-1]
        self.assertEqual(status, 200, reply)
        return self.jobs[reply['job']].result(timeout=120)

    def test_committed_command_reports_success_and_requests_refresh(self):
        # E1 returns None from the command itself, so this also covers result normalization.
        reply = self.native_command({'action': 'fixture', 'name': 'e1'})
        self.assertEqual(self.s.head, 1)
        self.assertIs(reply['requires_refresh'], True)
        self.assertIs(reply['result']['committed'], True)
        self.assertIn('Operation committed; native display refresh failed: injected snapshot failure', reply['result']['warning'])

    def test_command_without_a_journal_change_still_fails(self):
        with self.assertRaisesRegex(OSError, 'injected snapshot failure'):
            self.native_command({'action': 'orbit', 'orbit': 0})
        self.assertEqual(self.s.head, 0)


if __name__ == '__main__':
    unittest.main(verbosity=2)
