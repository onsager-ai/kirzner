#!/usr/bin/env python3
"""Exercise the actual backup/restore script against disposable fixture data."""
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import tempfile
from backup import postgres_environment

ROOT = Path(__file__).resolve().parents[1]
BACKUP = ROOT / "scripts/backup.py"


def invoke(*arguments, environment=None):
    subprocess.run(["python3", str(BACKUP), *map(str, arguments)], env=environment, check=True)


def main():
    with tempfile.TemporaryDirectory(prefix="kirzner-backup-") as temporary:
        directory = Path(temporary)
        source = directory / "source"
        files = source / "files"
        files.mkdir(parents=True)
        (files / "fixture").write_text("Synthetic referenced file\n")
        database = source / "kirzner.sqlite3"
        # Use the actual migration, not an unrelated environment fixture table.
        with sqlite3.connect(database) as connection:
            connection.executescript((ROOT / "migrations/sqlite/0001_workspace.sql").read_text())
            brief = json.loads((ROOT / "examples/real-product-brief.json").read_text())
            document = json.dumps({"id": "backup-fixture", "revision": 1, "brief": brief, "brief_history": [brief],
                                   "opportunities": [], "experiments": [], "attempts": [], "return_conflicts": [], "adopted_learning": []})
            connection.execute("INSERT INTO business_workspaces VALUES (?, ?, ?)", ("backup-fixture", 1, document))
        invoke("backup", directory / "sqlite-archive", "--data-dir", source)
        restored = directory / "sqlite-restored"
        invoke("restore", directory / "sqlite-archive", "--data-dir", restored)
        with sqlite3.connect(restored / "kirzner.sqlite3") as connection:
            assert connection.execute("SELECT document FROM business_workspaces").fetchone()[0] == document
        assert (restored / "files/fixture").read_text() == (files / "fixture").read_text()
        # Corrupt backups must fail before a target is written.
        (directory / "sqlite-archive/files/fixture").write_text("corrupted")
        invalid = subprocess.run(["python3", str(BACKUP), "restore", str(directory / "sqlite-archive"), "--data-dir", str(directory / "must-not-exist")], capture_output=True)
        assert invalid.returncode != 0 and not (directory / "must-not-exist").exists()
        print("PASS: SQLite business migration, backup/restore, referenced files, and corruption rejection.")
        postgres = os.environ.get("KIRZNER_TEST_POSTGRES_URL")
        if not postgres:
            print("NOT EXERCISED: PostgreSQL backup/restore requires KIRZNER_TEST_POSTGRES_URL and pg_dump/pg_restore.")
            return
        # The supplied database is a disposable source. Create an isolated target
        # database in the same local test cluster; never clean an existing one.
        from urllib.parse import urlsplit, urlunsplit
        split = urlsplit(postgres)
        target_name = f"kirzner_restore_{os.getpid()}"
        target_url = urlunsplit((split.scheme, split.netloc, f"/{target_name}", split.query, split.fragment))
        source_environment = {**postgres_environment(postgres), "DATABASE_URL": postgres}
        target_environment = {**postgres_environment(target_url), "DATABASE_URL": target_url}
        subprocess.run(["createdb", target_name], env=source_environment, check=True)
        try:
            invoke("backup", directory / "postgres-archive", "--backend", "postgres", "--data-dir", source, environment=source_environment)
            pg_restored = directory / "postgres-restored"
            invoke("restore", directory / "postgres-archive", "--backend", "postgres", "--data-dir", pg_restored, environment=target_environment)
            def rows(environment):
                return subprocess.check_output(["psql", "-At", "-v", "ON_ERROR_STOP=1", "-c", "SELECT workspace_id, revision, document FROM business_workspaces ORDER BY workspace_id"], env=environment)
            original = rows(source_environment)
            assert original, "Populate the disposable PostgreSQL source with business records before backup validation"
            assert original == rows(target_environment)
            assert (pg_restored / "files/fixture").read_text() == (files / "fixture").read_text()
            print("PASS: PostgreSQL pg_dump/pg_restore, actual business records, migrations, and referenced files.")
        finally:
            subprocess.run(["dropdb", target_name], env=source_environment, check=True)


if __name__ == "__main__":
    main()
