# Kirzner

A lightweight, local-first business workspace for solo founders and small
AI-native teams. Kirzner helps turn a brief into an evidence-backed opportunity
and experiment, prepare work for an external agent, review returned materials,
and record actual actions, outcomes, and adopted learning.

Kirzner owns business records and founder decisions. External agents own their
model loops, tools, credentials, execution permissions, retries, and scheduling.
Kirzner does not run agents, contact prospects, or automatically retry external
writes. Research can stand alone; founders can self-prepare.

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

## Manual publication handoff

The ArtiPub publishing workflow is being migrated from its legacy Puppeteer
and cookie-scraping architecture to deterministic local handoff packages.
A founder reviews and publishes each handoff package manually, then records
the live URL against the approved Kirzner experiment version.

Import UTF-8 Markdown with frontmatter and local image references, then package
it for an approved experiment proposal:

```sh
cargo run --locked -- article import article.md
cargo run --locked -- publish package --experiment experiment-1 --version 1 \
  --article <id-or-path> --platform juejin --out packages
```

Each package has a deterministic `<article-id>/<platform>/` directory with
`manifest.json`, `article.md`, `assets/`, and `checklist.md`. For local images,
the checklist gives their upload order and reminds you to replace each relative
`assets/...` link in `article.md` with the URL returned by the platform's image
uploader. After publishing, use the package digest printed by `publish package`
to record the founder-confirmed URL:

```sh
cargo run --locked -- publish record --experiment experiment-1 --version 1 \
  --package-digest <sha256> --url https://example.com/published-article
```

Supported manual handoff platforms are `generic`, `juejin` (掘金), `zhihu`
(知乎), `csdn` (CSDN), `segmentfault` (思否), `cnblogs` (博客园), `v2ex`
(V2EX), `jianshu` (简书), and `oschina` (开源中国). Recorded live URLs are tied
to the Kirzner experiment and proposal version; Kirzner records the founder's
evidence and does not verify remote publication. If you edit an experiment
proposal after creating a package, approve the new proposal version before
making a current recording. Otherwise, keep the package's original version
when recording evidence for a publication already made.

### Copyable article and walkthrough

Save this as `article.md`:

```markdown
---
title: A small experiment
summary: What I tried and what I learned
tags: [founder-tools, experiment]
---

## What I tested

I shared a short guide with readers and tracked whether it led to a useful
conversation. This is a sample article; replace it with your own work.
```

Import the article, then package it for an experiment whose current proposal
you have approved. Use that experiment's ID and approved version from its
Kirzner workspace:

```sh
cargo run --locked -- article import article.md
cargo run --locked -- publish package --experiment <experiment-id> \
  --version <approved-version> --article <id-from-import> \
  --platform juejin --out packages
```

Open `packages/<article-id>/juejin/`, follow `checklist.md` in 掘金's editor,
and review the rendered post before publishing. Then record the published URL
using the package digest and the same experiment version. In the experiment's
Results view, review the recorded action against the original success criteria
and add the observed outcome; Kirzner keeps that evidence with the experiment.
## Approved campaigns

Campaigns use a standalone Markdown file. Run these commands from its directory.

```sh
kirzner campaign plan campaign.md
kirzner campaign approve campaign.md
kirzner campaign run campaign.md
kirzner campaign status campaign.md
```

The runner uses `BLUESKY_HANDLE` and `BLUESKY_APP_PASSWORD` for Bluesky, and
`GITHUB_TOKEN` for GitHub releases. `HTTPS_PROXY` (or `https_proxy`) applies to
Bluesky requests and link cards. GitHub requests connect directly. Check the
Bluesky route with `kirzner campaign check-bluesky-health`.

The runner exits with status 1 if any result is still unresolved or held. For
an unknown Bluesky write, `kirzner campaign reconcile campaign.md <item> --absent`
records absence only when the saved PDS returns HTTP 400 `RecordNotFound`.
A later approved run checks that same record key before writing. If a crashed
process leaves `.kirzner.lock`, confirm that no campaign command is active
before removing the empty lock directory and running again.

## Local data

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

`KIRZNER_DATA_DIR` selects the data directory; Kirzner stores its database at
`<data-dir>/kirzner.sqlite3` (default `data/kirzner.sqlite3`) and uploads at
`<data-dir>/files`. External URLs are references and are not copied. See
[architecture](docs/architecture.md) for storage, return, and compatibility
boundaries.

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
