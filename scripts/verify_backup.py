#!/usr/bin/env python3
"""Exercise the SQLite backup/restore script against disposable fixture data."""
import json
from pathlib import Path
import sqlite3
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BACKUP = ROOT / "scripts/backup.py"


def invoke(*arguments):
    subprocess.run(["python3", str(BACKUP), *map(str, arguments)], check=True)


def main():
    with tempfile.TemporaryDirectory(prefix="kirzner-backup-") as temporary:
        directory = Path(temporary)
        source = directory / "source"
        files = source / "files"
        files.mkdir(parents=True)
        (files / "fixture").write_text("Synthetic referenced file\n")
        database = source / "kirzner.sqlite3"
        with sqlite3.connect(database) as connection:
            connection.executescript((ROOT / "migrations/sqlite/0001_workspace.sql").read_text())
            brief = json.loads((ROOT / "examples/real-product-brief.json").read_text())
            document = json.dumps({"id": "backup-fixture", "revision": 1, "brief": brief, "brief_history": [brief],
                                   "opportunities": [], "experiments": [], "attempts": [], "return_conflicts": [], "adopted_learning": []})
            connection.execute("INSERT INTO business_workspaces VALUES (?, ?, ?)", ("backup-fixture", 1, document))
        invoke("backup", directory / "archive", "--data-dir", source)
        restored = directory / "restored"
        invoke("restore", directory / "archive", "--data-dir", restored)
        with sqlite3.connect(restored / "kirzner.sqlite3") as connection:
            assert connection.execute("SELECT document FROM business_workspaces").fetchone()[0] == document
        assert (restored / "files/fixture").read_text() == (files / "fixture").read_text()
        (directory / "archive/files/fixture").write_text("corrupted")
        invalid = subprocess.run(["python3", str(BACKUP), "restore", str(directory / "archive"), "--data-dir", str(directory / "must-not-exist")], capture_output=True)
        assert invalid.returncode != 0 and not (directory / "must-not-exist").exists()
        print("PASS: SQLite business migration, backup/restore, referenced files, and corruption rejection.")


if __name__ == "__main__":
    main()
