"""Offline operator recovery. Never invoked by the application."""
import argparse
import csv
import io
import os
from pathlib import Path
import sqlite3
import subprocess
import uuid

parser = argparse.ArgumentParser(description="Recover local admin access after a verified full backup.")
parser.add_argument("database", type=Path)
parser.add_argument("--reset-admin", action="store_true")
args = parser.parse_args()
if not args.reset_admin:
    parser.error("Explicit --reset-admin is required")
db_path = args.database.resolve(strict=True)
if not db_path.is_file() or db_path.suffix != ".sqlite":
    parser.error("An existing application .sqlite database is required")
if os.name == "nt":
    processes = subprocess.run(["tasklist", "/FO", "CSV", "/NH"], check=True, capture_output=True, text=True).stdout
    if any(row and row[0].lower() == "reddot-desktop.exe" for row in csv.reader(io.StringIO(processes))):
        parser.error("Close every RotPunktArena instance first")
with sqlite3.connect(db_path.as_uri()+"?mode=rw", uri=True, timeout=1) as db:
    if db.execute("PRAGMA integrity_check").fetchone()[0] != "ok":
        raise SystemExit("Database integrity failed; no changes made")
    if not db.execute("SELECT 1 FROM schema_migrations WHERE version>=18 LIMIT 1").fetchone():
        raise SystemExit("Unsupported application database; no changes made")
    if not db.execute("SELECT 1 FROM settings WHERE key='admin.auth'").fetchone():
        raise SystemExit("No password record exists; no changes made")
    backup = db_path.with_name("admin-recovery-"+str(uuid.uuid4())+".sqlite")
    with backup.open("xb"):
        pass
    with sqlite3.connect(backup) as snapshot:
        db.backup(snapshot)
        if snapshot.execute("PRAGMA integrity_check").fetchone()[0] != "ok":
            raise SystemExit("Backup integrity failed; password unchanged")
    with backup.open("rb+") as snapshot:
        os.fsync(snapshot.fileno())
    db.execute("BEGIN IMMEDIATE")
    db.execute("DELETE FROM settings WHERE key='admin.auth'")
    db.commit()
print("Verified backup:", backup)
print("Only the admin credential was removed. Start the app and set a new password immediately.")
