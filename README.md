# Kirzner

A lightweight, local-first business workspace for solo founders and small
AI-native teams. Kirzner helps turn a brief into an evidence-backed opportunity
and experiment, prepare work for an external agent, review returned materials,
and record actual actions, outcomes, and adopted learning.

Kirzner owns business records and founder decisions. External agents own their
model loops, tools, credentials, execution permissions, retries, and scheduling.
Kirzner does not run agents, publish, contact prospects, or automatically retry
external writes. Research can stand alone; founders can self-prepare.

## Run locally

Requires Rust 1.95.0, Node 22.12+, and npm. SQLite is embedded.

```sh
npm --prefix frontend ci
npm --prefix frontend run build
cargo run --locked
# Open http://127.0.0.1:4317
```

Start with the product, objective, and constraints. Keep unsupported customer,
capability, budget, time, and channel details unknown. Create research in
Opportunities and give its brief to your existing agent. Paste or import its
return, inspect the source evidence and unknowns, then separately approve any
experiment draft. Copy preparation to your agent, review its returned material,
and perform actual external actions yourself. Record evidence and choose what
happens next. Handoffs, Results, file transfers, and manual forms are available.

Core rules:

- Changed proposals need fresh approval; earlier decisions and late returns
  retain their original version.
- Execution, deliverable acceptance, external effects, and business outcomes
  are separate records.
- Unknown effects need explicit reconciliation before more preparation or an
  action. Imports never retry external writes.
- Conflicting returns remain inspectable; they do not replace the effective
  return.
- Results use the original criteria. Learning enters future context only after
  explicit adoption.

The default database is `data/kirzner.sqlite3`; uploaded files are in
`data/files`. Migrations run at startup. `KIRZNER_DATA_DIR`, `KIRZNER_PORT`, and
`KIRZNER_FRONTEND_DIR` customize local paths and port. The app binds IPv4
loopback and serves one local workspace.

## Backup

Stop Kirzner first. Back up the SQLite database and referenced files together.
Restore into a fresh data directory.

```sh
python3 scripts/backup.py backup backups/first --data-dir data
python3 scripts/backup.py restore backups/first --data-dir restored-data
KIRZNER_DATA_DIR=restored-data cargo run --locked
```

`--sqlite-db PATH` selects a custom database location. External URLs are
references and are not copied. See [architecture](docs/architecture.md) for
storage, return, and compatibility boundaries.

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
npm --prefix frontend run build
```

The browser journey requires a fresh running app and Chromium:
`CHROMIUM_PATH=/path/to/chromium npm --prefix frontend run test:browser`.
CI runs the browser journey and worked example with synthetic data. See
[worked example](docs/worked-example.md) for its evidence labels and limits.
