"""C600 exports must stay within the interactive import budget (screening A-02)."""
from pathlib import Path
import base64, gzip, json, sys, tempfile, unittest
ROOT = Path(__file__).resolve().parents[1]; sys.path.insert(0, str(ROOT))
from core import Model
from session import Session
import log_io
from log_io import MAX_PRIMITIVES, MAX_TRANSACTIONS, decode_log, encode_export, encode_log, validate_record

MODEL = Model()


def record(counts):
    # Size checks run before any replay, so synthetic events are enough here.
    return dict(format=log_io.FORMAT, model_id=MODEL.model_id, root='labelled_identity', native_windows_equivalence=False,
                prefs={}, final_state='0' * 64,
                events=[dict(id=i + 1, recipe=[], pre='0' * 64, post='0' * 64, primitive_count=c, stars=0, note='', assistance='manual')
                        for i, c in enumerate(counts)])


class ExportBudgetTests(unittest.TestCase):
    def assert_refused(self, rec, cap):
        for encode in (encode_log, encode_export):
            with self.subTest(encode=encode.__name__), self.assertRaisesRegex(ValueError, cap + '.*session backup'):
                encode(rec)

    def test_transaction_cap(self):
        encode_log(record([1] * MAX_TRANSACTIONS)); encode_export(record([1] * MAX_TRANSACTIONS))
        self.assert_refused(record([1] * (MAX_TRANSACTIONS + 1)), 'capped at 10,000 transactions')

    def test_primitive_cap_with_string_counts(self):
        encode_log(record([str(MAX_PRIMITIVES - 1), '1']))
        self.assert_refused(record([str(MAX_PRIMITIVES), '1']), 'exceeds 2,000,000 primitive moves')
        self.assert_refused(record([MAX_PRIMITIVES + 1]), 'exceeds 2,000,000 primitive moves')

    def test_save_log_writes_nothing_over_budget(self):
        with tempfile.TemporaryDirectory(prefix='c600-log-budget-') as d:
            s = Session(MODEL, Path(d) / 'session')
            try:
                s.export = lambda: record([1] * (MAX_TRANSACTIONS + 1))
                with self.assertRaisesRegex(ValueError, 'capped at 10,000 transactions'):
                    s.save_log()
                self.assertEqual(list((Path(d) / 'session' / 'logs').glob('*')) if (Path(d) / 'session' / 'logs').exists() else [], [])
            finally:
                s.close()

    def test_real_exports_still_import(self):
        with tempfile.TemporaryDirectory(prefix='c600-log-budget-') as d:
            s = Session(MODEL, Path(d) / 'session')
            try:
                for word in ([2], [5, 7]):
                    s.commit(s.preview([{'kind': 'word', 'moves': word}])['token'])
                logged = decode_log(base64.b64encode(s.export_log()).decode())
                exported = json.loads(gzip.decompress(encode_export(s.export())))
                for rec in (logged, exported):
                    plan = validate_record(MODEL, rec)
                    self.assertEqual((plan['transactions'], plan['state'].hash), (2, s.st.hash))
            finally:
                s.close()


if __name__ == '__main__':
    unittest.main(verbosity=2)
