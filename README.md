# Kirzner

A lightweight, local-first AI business operator for solo founders and small
AI-native teams. **Status: first local MVP.**

One customer-acquisition loop: brief → evidenced opportunity → bounded proposal
→ version-specific founder decision → manual agent handoff → deliverable review
→ actual action and observations → next decision and explicitly adopted learning.

External agents own model loops, tools, credentials, execution permissions,
native sessions, retries, and scheduling. Kirzner owns business records and
judgment. It does not publish, contact prospects, run agents, or automatically
retry external writes. Research can stand alone; founders can self-prepare.

## Run locally

Requires Rust 1.95.0 (pinned), Node 22.12+ and npm. SQLite is embedded;
PostgreSQL is not required for the default application.

```sh
npm --prefix frontend ci
npm --prefix frontend run build
cargo run --locked
# Open http://127.0.0.1:4317
```

Save a business brief. Create a research handoff in Opportunities, download
it in Handoffs, and give it to your existing agent. Import the returned JSON
envelope or use the return form. Each opportunity needs source references,
customer relevance, why now, counterevidence, unknowns, and cheap validation.
Zero actionable opportunities is valid. Choose an opportunity, create a bounded
proposal, approve its current version, and export preparation when needed.
Accept materials, perform actual outreach outside Kirzner, then record the
action, real observations, next decision, and explicitly adopted learning.

Default database: `data/kirzner.sqlite3`; migrations run at startup. Uploaded
files: `data/files`. `KIRZNER_DATA_DIR`, `KIRZNER_PORT`, and
`KIRZNER_FRONTEND_DIR` override the corresponding local paths and port.
The application binds IPv4 loopback and serves one local workspace.

For a dedicated PostgreSQL database:

```sh
DATABASE_URL=postgresql://USER@127.0.0.1:5432/kirzner cargo run --locked
```

Both backends implement equivalent business behavior with separate migrations.
Do not use Semon's database as business storage. Read [architecture](docs/architecture.md)
for the storage tradeoff and the core/Hub boundary.

## Decisions and returns

- A changed proposal needs fresh approval. Earlier decisions and late returns
  retain their original version.
- Identical returns are no-ops. Conflicting content for one return identity
  is preserved for inspection without replacing the effective return.
- Reported execution, deliverable acceptance, external effects, and business
  outcomes remain separate. Failed/partial materials remain inspectable.
- Unknown effects need explicit reconciliation. Imports never retry external writes.
- Accepting a draft creates neither publication nor business success. Results
  compare with the original criteria; learning needs an explicit adoption decision.

Uploaded files (up to 5 MB each) have durable content-hash references, title,
media type, checksum and size metadata. External URLs are references and are
not copied or crawled. Original returns and handoff snapshots remain inspectable.

## Semon

Business records work without Semon. Install its pinned CLI to enable capture;
see [Semon setup and verification](docs/semon.md) for explicit session
correlation, native continuation, and ordinary/forensic boundaries.

```sh
KIRZNER_SEMON_CODEX_BIN=/absolute/path/to/semon-codex \
  cargo run --locked -- capture-codex /native/codex/sessions /native/codex/history.jsonl
```

Semon keeps a separate store and cursor. A native viewer/index is not proof of
full capture. This cloud bootstrap verified persistence with synthetic inputs;
real capture awaits a local session with native logs.

## Backup and restore

Stop Kirzner and capture processes first. Back up the database **and** referenced
files together. Python 3 is enough for SQLite; PostgreSQL also needs matching
`pg_dump`/`pg_restore` tools.

```sh
python3 scripts/backup.py backup backups/first --data-dir data
python3 scripts/backup.py restore backups/first --data-dir restored-data
KIRZNER_DATA_DIR=restored-data cargo run --locked
```

`--sqlite-db PATH` supports a custom database location. For PostgreSQL use
`--backend postgres` and `DATABASE_URL`. Restore to a fresh dedicated database
and fresh files directory; the script verifies checksums, refuses existing
SQLite/files targets, and uses a PostgreSQL transaction without cleaning an
existing database.

```sh
DATABASE_URL=postgresql://USER@127.0.0.1:5432/kirzner \
  python3 scripts/backup.py backup backups/pg-first --backend postgres --data-dir data
# First create a fresh kirzner_restored database.
DATABASE_URL=postgresql://USER@127.0.0.1:5432/kirzner_restored \
  python3 scripts/backup.py restore backups/pg-first --backend postgres --data-dir restored-data
```

Semon backups are separate and retain the matching cursor and provenance.
Native logs and remote URLs are not included in business backups.

## Worked example

Start a fresh data directory, then `python3 scripts/worked_example.py`.
The example uses a real Kirzner brief and externally authored Semon material
with pinned, verifiable sources. It stops at preparation review because no
outreach was performed. Customer metrics are not fabricated. The complete
software loop is tested separately with labeled synthetic browser fixtures.
See [worked example](docs/worked-example.md).

## Validation

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
npm --prefix frontend run build
```

External tests are explicitly ignored without their prerequisites:

```sh
KIRZNER_TEST_POSTGRES_URL=postgresql://USER@127.0.0.1:5432/kirzner_test \
KIRZNER_TEST_SEMON_CODEX_BIN=/absolute/path/to/semon-codex \
KIRZNER_TEST_SEMON_BIN=/absolute/path/to/semon \
  cargo test --locked -- --include-ignored
```

Use a disposable PostgreSQL database. Parity tests cover the loop, restart,
migration rerun, stale writers, version authorization, duplicate/conflicting
returns, partial/failed/unknown results, and learning adoption. Semon tests
are synthetic and never count as real-session capture.

Against a **fresh** running application:
`CHROMIUM_PATH=/path/to/chromium npm --prefix frontend run test:browser`.
`KIRZNER_TEST_URL` selects an alternate loopback instance. The browser suite
creates synthetic records and checks routing/reload, uploaded files, version
decisions, return conflicts, and desktop/mobile layouts. After populating a
disposable PostgreSQL database, run `KIRZNER_TEST_POSTGRES_URL=... python3
scripts/verify_backup.py` for backup/restore validation on both backends.

See [the validation report](docs/validation.md) for actual checks and limitations.

## Core and Hub

`kirzner` is independently usable open-source core. The private `kirzner-hub`
bootstrap depends on its library and built frontend. Cloud hosting, collaboration,
identity/authorization, continuous operation, billing, and operations are future
Hub capabilities. AI Industry Observatory is a separate future module.
