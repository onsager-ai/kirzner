# Bootstrap validation report

Date: 2026-10-09. Implementation and validation ran in an isolated Codex cloud
workspace. Reports to the user are in Chinese; this technical record is English.

## Repository discovery

| Repository | Starting ref | Implementation branch |
| --- | --- | --- |
| `onsager-ai/kirzner` | local `work`, `6ee900ac7042e25866622d1958c520b3ba8a5784` | `codex/bootstrap-mvp` |
| `onsager-ai/kirzner-hub` | local unborn `work`; no commit or remote refs | `codex/bootstrap-hub` |
| Semon reference | `defe9d791a197902c5bebcba2e2b7fe87dd9a075` | read-only source cache |

Both target repositories initially lacked `AGENTS.md`, `.agents/manifest.json`,
module instructions, and canonical skill selections. Development instructions
were added with the bootstrap. Both targets began clean. These checks were
completed before publication. The user subsequently authorized committing both
bootstraps and integrating them into `main`. Published commit refs are reported
in the accompanying completion message; use `git rev-parse HEAD` to identify
the local checkout. No PR, issue, deployment, cloud resource, prospect contact,
or marketing publication was created.

Semon's local README blob hash, `e3b46edb940efd51ca4e9f3ca6da958daee68892`,
matches the GitHub connector response for the pinned ref. Implemented capture,
ordinary reads, persistence, query/export, native viewer and provenance
boundaries were inspected. Semon Hub and Duhem were not read during
implementation because their contracts were not needed by this local CLI
adapter. No sibling source was changed; only the explicitly requested Hub
target received its companion bootstrap.

## Delivered behavior

One Rust library/local binary, backend-specific SQLx migrations, equivalent
concrete SQLite/PostgreSQL stores, and one React/TypeScript frontend. Server
state and routes use TanStack Query/Router. Locally owned shadcn/ui Button source
uses Radix/CVA. Pinned Cargo/npm locks reproduce the tested dependencies.

The UI implements brief/materials, independent research, evidenced opportunity
review, bounded proposal versions, founder decisions, manual handoff/continuation
downloads, form/file returns, local file uploads, acceptance/change requests,
explicit multi-session references, unknown-effect reconciliation, actual action
references, observations against original criteria, next decisions and explicit
learning adoption. Original/conflicting returns and failed/partial work remain
inspectable. An import schedules no external operation.

Hub composes core HTTP, storage and the built frontend. It has no separate
business implementation. Hosting, collaboration, identity, authorization,
continuous operation and billing are not implemented. Core requires no Hub.

## Checks actually executed

Toolchain: Rust/Cargo 1.95.0, Node 24.19.0, npm 11.9.0, PostgreSQL 17.11,
Chromium 151. Dependencies were installed from the prepared local caches.
`CARGO_TARGET_DIR` pointed at the environment's shared compilation cache; it
does not change the repository's dependency contract. Socket/network commands
used the execution tool's supported additional network permission.

| Check | Actual result |
| --- | --- |
| Core `cargo fmt --check` | passed |
| Core `cargo clippy --all-targets --locked -- -D warnings` | passed |
| Core `cargo test --locked -- --include-ignored`, with both external prerequisites | 5 integration tests passed; none ignored in this run |
| Frontend locked install and `npm run build` | passed TypeScript and production build |
| `npm run test:browser` against a fresh SQLite instance | complete synthetic business loop passed; no page errors |
| Desktop/mobile screenshots and overflow check | inspected; 390px mobile viewport had no horizontal overflow |
| `python3 scripts/verify_backup.py`, with PostgreSQL configured and populated | SQLite and PostgreSQL restoration passed; corrupted SQLite backup rejected |
| Worked example script through the real HTTP API, on both backends | real product brief and external source material loaded; preparation accepted |
| Backup/restore of those actual example workspaces/files into fresh targets | complete workspace JSON and downloaded bytes matched; two process restarts per backend passed |
| Hub fmt/clippy/build/test | passed; the thin Hub binary has zero dedicated unit tests |
| Hub actual loopback startup, health, fresh workspace, frontend response | passed using core SQLite and routes |
| Python script compilation and repository whitespace checks | passed |

The behavioral parity suite exercises research without an experiment,
proposal-version approval and revocation boundaries, preserved snapshots,
late returns, identical/conflicting return handling, original-text retention,
failed/partial/unknown execution, explicit effect reconciliation, acceptance
without publication/outcome, self-preparation, original-version observations,
next decisions, explicit learning, stale writers, migration reruns and reopening.
The HTTP suite exercises loopback Host/origin protection, file persistence and
attachment download, hash validation, missing/forensic paths, stale revisions,
and access to business records without Semon capture.

The browser journey uses clearly labeled synthetic opportunities, drafts,
actions and observations. It creates the records through UI forms, uploads a
draft, accepts it, records an inconclusive result, adopts learning, revises the
proposal, checks fresh approval is needed, imports duplicate/conflicting JSON,
and reloads direct routes. It demonstrates software acceptance, not customer
acquisition or business success. The newly added GitHub workflow was authored
but was not run remotely.

## Semon evidence and remaining gaps

The connection invoked the actual pinned `semon-codex` CLI twice on an isolated
synthetic supported Codex JSONL source. Semon reported 3 records, then 0 on
recapture. Test-only store inspection confirmed 3 retained raw records and
1 semantic occurrence under the explicit native session ID. Ordinary `semon log`
output remained identical after removing the disposable native source, and
after deleting its disposable forensic rows. A forensic-only synthetic marker
never appeared in ordinary output. Business storage never reads Semon tables;
the inspection exists only in the boundary test.

Real native sessions are unavailable in this Codex cloud environment, confirmed
by the user. Durable capture of a **real** harness session has not been verified.
The user plans to provide that evidence in a later local session. The exact
local capture/verification procedure is in [Semon documentation](semon.md).
Native viewer availability and full raw capture completeness remain separate;
stored execution references are supplied provenance, not authenticated proof.

The worked example uses real, verifiable external repository documentation.
Its research interpretation is labeled as an inference assembled during this
implementation session; it is not claimed as a separately executed research
agent. No acquisition outreach was performed, so the example stops after
preparation review and has no actual customer metrics or business outcome.
See [the worked example](worked-example.md).

The workspace aggregate favors a small local vertical slice over large-scale
querying. Authentication/tenancy, operational hosting, automatic execution,
remote transcript embedding, continuous scanning, Observatory and billing
remain outside this implementation. Business backups include the database and
local referenced files; remote URLs/native logs and sensitive Semon backups
remain operator-owned. Validation processes were stopped after their checks.
