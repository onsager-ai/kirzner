# Kirzner

A lightweight, local-first AI business operator for solo founders and small
AI-native teams. **Status: first local MVP.**

One customer-acquisition loop: minimal brief → evidenced opportunity and experiment draft → version-specific founder decision → external agent handoff → deliverable review → actual action and observations → next decision and explicitly adopted learning.

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

Begin with the product, current objective, and resource constraints. Optional customer, capabilities, budget, time, and channels stay unknown until supported. Create a research task in Opportunities and copy its brief to your existing agent; downloads remain available. Paste the returned JSON beside that task, or import a file. An opportunity can include a bounded experiment draft and a proposed brief enrichment. Review the source evidence and unknowns, open the draft, edit prefilled fields, and separately approve its current version. Importing creates no approval or adopted business facts.

Use Experiments as the working view: source opportunity, proposal, next step, preparation tasks, returned materials, reviews, actions, observations, and decisions are connected. Copy preparation to your agent, paste the return, and inspect supported local text/Markdown/JSON against the task's original acceptance criteria. Accepting material is separate from execution status and business outcome. Perform any actual action outside Kirzner, record its evidence, compare observations with the original criteria, and explicitly choose what happens next. A direct experiment link and the home resume control restore the selected experiment after reopening. Existing Handoffs, Results, file transfers, and full manual forms remain available.

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
- An already completed action on an older approved proposal can be recorded through the separate historical-action form, with completion time and evidence. This does not authorize a new action; current actions still require current approval. Completion must fall within the original approval/revision interval. Whole-second timestamps and founder-supplied evidence are not authenticated execution proof.
- Identical returns are no-ops. Conflicting content for one return identity
  is preserved for inspection without replacing the effective return.
- Reported execution, deliverable acceptance, external effects, and business
  outcomes remain separate. Failed/partial materials remain inspectable.
- Unknown effects need explicit reconciliation. Imports never retry external writes.
- The unknown-effect barrier includes an experiment's explicitly linked source research task. Missing pasted task/return IDs are supplied automatically; explicit mismatched identities are rejected, and exact incoming originals remain recoverable under advanced details.
- Accepting a draft creates neither publication nor business success. Results
  compare with the original criteria; learning needs an explicit adoption decision.
- Historical observations retain factual evidence and original versions in future handoffs. Tentative proposed learning stays out of reusable knowledge until explicitly adopted.

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
The example uses Semon as the real product, a minimal brief with honest unknowns, a source-grounded but hypothetical experiment draft, and externally authored material with pinned sources. It stops at preparation review because no outreach was performed. Customer metrics are not fabricated. The complete software loop is tested separately with labeled synthetic browser fixtures.
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
See [the founder experience](docs/founder-experience.md) for the flow, friction measurements, and evidence labels.

## Core and Hub

`kirzner` is independently usable open-source core. The private `kirzner-hub`
bootstrap depends on its library and built frontend. Cloud hosting, collaboration,
identity/authorization, continuous operation, billing, and operations are future
Hub capabilities. AI Industry Observatory is a separate future module.
