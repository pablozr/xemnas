import importlib.util
from datetime import datetime, timedelta, timezone
from pathlib import Path
import sqlite3
import tempfile
import unittest

spec = importlib.util.spec_from_file_location(
    "dogfood_report", Path(__file__).with_name("dogfood-report.py")
)
report = importlib.util.module_from_spec(spec)
spec.loader.exec_module(report)

TZ = timezone(timedelta(hours=-3))
NOW = datetime(2026, 10, 7, 12, 0, tzinfo=TZ)
SECRET = "SEGREDO-pergunta-privada"

SCHEMA = """
CREATE TABLE capture_receipts (capture_id TEXT, received_at TEXT);
CREATE TABLE decision_candidates (
    id TEXT, capture_id TEXT, status TEXT, created_at TEXT, updated_at TEXT, question TEXT);
CREATE TABLE engineering_decisions (candidate_id TEXT, confirmed_at TEXT);
CREATE TABLE auto_reviews (verdict TEXT, created_at TEXT, undone_at TEXT);
CREATE TABLE context_injections (created_at TEXT, tokens INTEGER, omitted INTEGER);
CREATE TABLE jobs (state TEXT, updated_at TEXT);
CREATE TABLE assessments (outcome TEXT, finished_at TEXT);
"""


def make_db(path, with_queries=False):
    db = sqlite3.connect(path)
    db.executescript(SCHEMA)
    db.executemany("INSERT INTO capture_receipts VALUES (?, ?)", [
        # 01:00 UTC on the 6th is still the 5th in UTC-3.
        ("c1", "2026-10-06T01:00:00Z"),
        ("c2", "2026-10-06T12:00:00Z"),
    ])
    db.executemany("INSERT INTO decision_candidates VALUES (?, ?, ?, ?, ?, ?)", [
        ("k1", "c1", "accepted", "2026-10-06T01:00:10Z", "2026-10-06T12:00:00Z", SECRET),
        ("k2", "c2", "dismissed", "2026-10-06T12:00:30Z", "2026-10-06T12:10:00Z", SECRET),
    ])
    db.execute("INSERT INTO engineering_decisions VALUES ('k1', '2026-10-06T01:01:10Z')")
    db.execute("INSERT INTO auto_reviews VALUES ('accepted', '2026-10-06T13:00:00Z', "
               "'2026-10-06T14:00:00Z')")
    db.execute("INSERT INTO context_injections VALUES ('2026-10-06T13:00:00Z', 300, 4)")
    db.execute("INSERT INTO jobs VALUES ('failed', '2026-10-06T13:00:00Z')")
    db.execute("INSERT INTO assessments VALUES ('skipped', '2026-10-06T13:00:00Z')")
    if with_queries:
        db.execute("CREATE TABLE agent_queries (project_id TEXT, tool TEXT, outcome TEXT, "
                   "chars INTEGER, created_at TEXT)")
        db.executemany("INSERT INTO agent_queries VALUES ('p', 'search', ?, 10, "
                       "'2026-10-06T13:00:00Z')", [("answered",), ("empty",)])
    db.commit()
    return db


def rows_of(block):
    return [line.split(" | ") for line in block.splitlines() if line.startswith("| 2026")]


class DogfoodReportTests(unittest.TestCase):
    def test_percentile_matches_diagnostics_rule(self):
        self.assertEqual(report.percentile([10, 20, 30, 40, 50], 0.5), 30)
        self.assertEqual(report.percentile([10, 20, 30, 40, 50], 0.95), 50)
        self.assertEqual(report.percentile([1, 2], 0.5), 2)

    def test_aggregates_by_local_day_without_agent_queries(self):
        with tempfile.TemporaryDirectory() as directory:
            db = make_db(Path(directory) / "app.db")
            block = report.render(db, TZ, NOW)
            db.close()
            rows = rows_of(block)
            self.assertEqual([row[0].lstrip("| ") for row in rows], ["2026-10-05", "2026-10-06"])
            first, second = rows
            self.assertEqual(first[1:4], ["1", "1", "0"])  # capture, candidate, confirmed (decided on the 6th)
            self.assertEqual(first[8], "10 s / 10 s")  # latency
            self.assertEqual(first[9], "1.0 min / 1.0 min")  # review
            self.assertEqual(second[1:6], ["1", "1", "1", "1", "50%"])
            self.assertEqual(second[6:8], ["1", "1"])  # auto resolved, undone
            self.assertEqual(second[10:13], ["1", "300", "4"])
            self.assertEqual(second[13:15], ["—", "—"])
            self.assertEqual(second[15].rstrip(" |"), "2")  # failed job + skipped assessment
            self.assertIn("Atualizado em 2026-10-07 12:00", block)

    def test_agent_queries_when_present(self):
        with tempfile.TemporaryDirectory() as directory:
            db = make_db(Path(directory) / "app.db", with_queries=True)
            rows = rows_of(report.render(db, TZ, NOW))
            db.close()
            self.assertEqual(rows[1][13:15], ["2", "1"])

    def test_no_candidate_text_leaks(self):
        with tempfile.TemporaryDirectory() as directory:
            db = make_db(Path(directory) / "app.db", with_queries=True)
            output = report.render(db, TZ, NOW)
            db.close()
            self.assertNotIn(SECRET, output)

    def test_write_is_idempotent_and_keeps_line_endings(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            make_db(root / "app.db").close()
            doc = root / "log.md"
            doc.write_bytes(f"# T\r\n\r\n{report.START}\r\n{report.END}\r\n\r\nmanual\r\n"
                            .encode("utf-8"))
            args = ["--write", "--db", str(root / "app.db"), "--doc", str(doc)]
            self.assertEqual(report.main(args), 0)
            once = doc.read_bytes()
            self.assertEqual(report.main(args), 0)
            strip = lambda data: [line for line in data.split(b"\r\n")
                                  if not line.startswith(b"Atualizado")]
            self.assertEqual(strip(once), strip(doc.read_bytes()))
            self.assertNotIn(b"\n", once.replace(b"\r\n", b""))
            self.assertFalse(once.startswith(b"\xef\xbb\xbf"))
            self.assertTrue(once.endswith(b"\r\n\r\nmanual\r\n"))

    def test_missing_db_or_markers_do_not_touch_the_doc(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            doc = root / "log.md"
            doc.write_text("sem marcadores\n", encoding="utf-8")
            self.assertEqual(report.main(["--write", "--db", str(root / "x.db"),
                                          "--doc", str(doc)]), 2)
            make_db(root / "app.db").close()
            self.assertEqual(report.main(["--write", "--db", str(root / "app.db"),
                                          "--doc", str(doc)]), 2)
            self.assertEqual(doc.read_text(encoding="utf-8"), "sem marcadores\n")


if __name__ == "__main__":
    unittest.main()
