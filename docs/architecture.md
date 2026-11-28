# Local MVP boundaries

The application is one Rust crate with a reusable library and local binary,
and one React/TypeScript frontend. The core owns business records and founder
judgment; external executors own model loops, tools, credentials, execution
permissions, retries, and scheduling. TanStack Query owns server state and
TanStack Router owns navigation. There are no shared frontend packages,
executor loops, event buses, plugin registries, or workflow engines.

## Storage

`BusinessStore` stores one workspace document in SQLite. Migrations run at
startup. Related decisions, proposal revisions, handoff snapshots, returns
(including original text), conflicting contents, reviews, actions,
observations, reconciliations, and adopted learning are retained. Revision
compare-and-swap and a transaction make each business operation atomic and
reject stale writers.

A complete workspace is read and written per mutation. This keeps the first
loop understandable and testable; it is not a large-workspace analytics store.
The logical workspace ID is not identity or tenant authorization. The local app
selects one workspace and binds only IPv4 loopback.

Deliverable uploads are capped at 5 MB, named by SHA-256, published by atomic
rename, and downloaded as attachments. Uploaded content is never interpreted
as HTML. External references are not crawled or copied; their continued
availability belongs to the owner. Uploaded unreferenced files can remain
after an abandoned form; no automatic deletion is introduced.

## Decisions and returns

The latest founder decision must approve the current proposal version before
preparation export or recording a current action. Revising a proposal preserves
older decisions and handoffs but requires new approval. A distinct historical
command records founder-supplied evidence of an already completed action on an
older version. It grants no current approval. Late returns remain attached to
their original attempt and version. Return identity is `(attempt_id, return_id)`.
Identical content is a no-op; differing content preserves an inspectable
conflict without replacing the effective return. A corrected return uses a new
ID.

Reported execution, deliverable acceptance, external effects, and observed
business outcome are separate. Unknown external effects need explicit evidence
before another preparation attempt or action. Reconciliation appends a record;
it does not rewrite the original return. Imports never retry an external action.
An accepted draft creates neither an action nor a business observation.

Observations refer to the recorded action's original proposal version. Results
compare to the original success, failure, or inconclusive criteria even after
further proposal edits. Learning enters future handoff context only after
explicit adoption. Founders record actual execution and observed results;
Kirzner does not verify remote publication or manufacture outcome metrics.
Old immutable snapshots are preserved.

## Manual publication packages

The Rust publishing workflow replaces legacy ArtiPub's Puppeteer and
cookie-scraping design with local deterministic handoff packages. The core
makes no network calls, uses no publishing credentials, and does not automate
browsers or remote posting. A founder imports UTF-8 Markdown, packages it for
an approved experiment proposal, publishes through the platform's own editor,
and records the founder-confirmed live URL against that experiment version.

The CLI commands are `article import <markdown-path>`, `publish package
--experiment <id> --version <n> --article <id-or-path> --platform <platform>
--out <dir>`, and `publish record --experiment <id> --version <n>
--package-digest <sha256> --url <https-url>`. Packages contain
`manifest.json`, `article.md`, `assets/`, and `checklist.md` under the article
ID and platform directories. Their canonical digest uses sorted file paths
and length-prefixed contents, so identical inputs produce identical package
digests. The checklist preserves image upload order and platform-specific
manual steps.

Manual handoff adapters are `generic`, `juejin`, `zhihu`, `csdn`,
`segmentfault`, `cnblogs`, `v2ex`, `jianshu`, and `oschina`. The founder's
recorded URL is evidence associated with a package attempt; Kirzner does not
fetch it or claim that a remote post was verified. This Rust crate does not
provide automatic posting or Bluesky/GitHub execution.

## Founder workspace and return protocol

The aggregate and command boundary serve the founder workspace. Product,
objective, and constraints are required in the brief; omitted optional fields
deserialize as empty unknowns. Agent suggestions remain preserved return data
until a founder reviews and saves them. Research opportunities optionally carry
an existing bounded proposal shape as `experiment_draft`; explicit selection
atomically links a new unapproved experiment. Repeated selection of the same
source reuses the existing experiment.

Task-scoped return imports preserve the incoming JSON value and exact text,
then normalize only missing task and return identities for effective matching.
Generated return IDs use a deterministic content hash. Added optional fields
and defaults keep older workspace documents and return envelopes readable. No
new agent runtime or external retry is introduced.

The frontend derives next steps from durable version-specific records, persists
only experiment selection in browser storage, and supports direct links.
Related handoff and result components are reused inside the experiment view.
Local text previews use content-hash file references and inert React text
nodes; external references and binary materials remain links or downloads.
Technical identifiers, provenance, digests, snapshots, and native execution
metadata stay available in advanced details.

## Local HTTP and Hub

The local server checks loopback Host and same-origin browser requests. It does
not implement multi-user identity or permission policies. Data files are not
web assets; the only downloadable data path accepts content hashes. Hub
composes the library and frontend without copying business logic. Hosted
identity, authorization, collaboration, billing, and continuous operation are
future Hub work; core remains independently usable.
