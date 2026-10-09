#!/usr/bin/env python3
"""Offline backup/restore of the business DB and referenced files.

Stop Kirzner and capture processes first. Never overwrite a live database.
PostgreSQL uses DATABASE_URL in the subprocess environment, not process arguments.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
from urllib.parse import parse_qsl, unquote, urlsplit


def postgres_environment(url):
    parts = urlsplit(url)
    environment = {**os.environ, "PGDATABASE": unquote(parts.path.lstrip("/"))}
    for variable, value in [("PGHOST", parts.hostname), ("PGPORT", str(parts.port) if parts.port else None),
                            ("PGUSER", unquote(parts.username) if parts.username else None),
                            ("PGPASSWORD", unquote(parts.password) if parts.password else None)]:
        if value is not None:
            environment[variable] = value
    parameters = {"host": "PGHOST", "port": "PGPORT", "sslmode": "PGSSLMODE", "sslrootcert": "PGSSLROOTCERT",
                  "sslcert": "PGSSLCERT", "sslkey": "PGSSLKEY", "connect_timeout": "PGCONNECT_TIMEOUT",
                  "application_name": "PGAPPNAME", "options": "PGOPTIONS", "target_session_attrs": "PGTARGETSESSIONATTRS"}
    for key, value in parse_qsl(parts.query):
        if key not in parameters:
            raise ValueError(f"Unsupported PostgreSQL URL parameter: {key}")
        environment[parameters[key]] = value
    return environment


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["backup", "restore"])
    parser.add_argument("archive", type=Path)
    parser.add_argument("--backend", choices=["sqlite", "postgres"], default="sqlite")
    parser.add_argument("--data-dir", type=Path, default=Path("data"))
    parser.add_argument("--sqlite-db", type=Path)
    args = parser.parse_args()
    database = args.sqlite_db or args.data_dir / "kirzner.sqlite3"
    archive = args.archive
    if args.backend == "postgres":
        url = os.environ.get("DATABASE_URL", "")
        if not url.startswith(("postgres://", "postgresql://")):
            parser.error("Set DATABASE_URL to the dedicated backup/restore database")
        environment = postgres_environment(url)
    if args.action == "backup":
        archive.mkdir(parents=True, exist_ok=False)
        try:
            if args.backend == "sqlite":
                with sqlite3.connect(f"file:{database.resolve()}?mode=ro", uri=True) as source:
                    with sqlite3.connect(archive / "business.sqlite3") as destination:
                        source.backup(destination)
            else:
                subprocess.run(["pg_dump", "--format=custom", "--file", str(archive / "business.dump")], env=environment, check=True)
            files = args.data_dir / "files"
            if files.is_dir():
                shutil.copytree(files, archive / "files")
            checksums = {str(p.relative_to(archive)): sha256(p) for p in sorted(archive.rglob("*")) if p.is_file()}
            (archive / "manifest.json").write_text(json.dumps({"format": 1, "backend": args.backend, "sha256": checksums}, indent=2) + "\n")
        except BaseException:
            shutil.rmtree(archive)
            raise
        print(f"Backup created at {archive}; Semon is a separate operator-managed backup.")
    else:
        manifest = json.loads((archive / "manifest.json").read_text())
        if manifest["format"] != 1 or manifest["backend"] != args.backend:
            parser.error("Backup format/backend mismatch")
        for name, digest in manifest["sha256"].items():
            path = (archive / name).resolve()
            if not path.is_relative_to(archive.resolve()) or sha256(path) != digest:
                parser.error("Backup checksum/path validation failed")
        if (args.data_dir / "files").exists() or (args.backend == "sqlite" and database.exists()):
            parser.error("Restore requires a fresh data directory and absent SQLite database")
        args.data_dir.mkdir(parents=True, exist_ok=True)
        if args.backend == "sqlite":
            database.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(archive / "business.sqlite3", database)
        else:
            # No --clean: restoring into a nonempty business database fails.
            subprocess.run(["pg_restore", "--exit-on-error", "--single-transaction", "--dbname=", str(archive / "business.dump")], env=environment, check=True)
        if (archive / "files").is_dir():
            shutil.copytree(archive / "files", args.data_dir / "files")
        print(f"Restored business database and referenced files into {args.data_dir}.")


if __name__ == "__main__":
    main()
