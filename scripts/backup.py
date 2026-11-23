#!/usr/bin/env python3
"""Offline backup/restore of the SQLite business database and referenced files.

Stop Kirzner first. Never overwrite a live database.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import sqlite3


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["backup", "restore"])
    parser.add_argument("archive", type=Path)
    parser.add_argument("--data-dir", type=Path, default=Path("data"))
    parser.add_argument("--sqlite-db", type=Path)
    args = parser.parse_args()
    database = args.sqlite_db or args.data_dir / "kirzner.sqlite3"
    archive = args.archive
    if args.action == "backup":
        archive.mkdir(parents=True, exist_ok=False)
        try:
            with sqlite3.connect(f"file:{database.resolve()}?mode=ro", uri=True) as source:
                with sqlite3.connect(archive / "business.sqlite3") as destination:
                    source.backup(destination)
            files = args.data_dir / "files"
            if files.is_dir():
                shutil.copytree(files, archive / "files")
            checksums = {str(p.relative_to(archive)): sha256(p) for p in sorted(archive.rglob("*")) if p.is_file()}
            (archive / "manifest.json").write_text(json.dumps({"format": 1, "backend": "sqlite", "sha256": checksums}, indent=2) + "\n")
        except BaseException:
            shutil.rmtree(archive)
            raise
        print(f"Backup created at {archive}.")
    else:
        manifest = json.loads((archive / "manifest.json").read_text())
        if manifest["format"] != 1 or manifest["backend"] != "sqlite":
            parser.error("Backup format/backend mismatch")
        for name, digest in manifest["sha256"].items():
            path = (archive / name).resolve()
            if not path.is_relative_to(archive.resolve()) or sha256(path) != digest:
                parser.error("Backup checksum/path validation failed")
        if (args.data_dir / "files").exists() or database.exists():
            parser.error("Restore requires a fresh data directory and absent SQLite database")
        args.data_dir.mkdir(parents=True, exist_ok=True)
        database.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(archive / "business.sqlite3", database)
        if (archive / "files").is_dir():
            shutil.copytree(archive / "files", args.data_dir / "files")
        print(f"Restored business database and referenced files into {args.data_dir}.")


if __name__ == "__main__":
    main()
