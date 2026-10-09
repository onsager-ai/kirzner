# Local MVP boundaries

The application is one Rust crate with a reusable library and local binary,
and one React/TypeScript frontend. Ordinary modules own `marketing` business
rules, `handoffs`, `storage`, `http`, and the `semon` CLI connection. TanStack
Query owns server state, TanStack Router owns navigation, and locally owned
shadcn/ui Button source uses Radix Slot/CVA. There are no shared frontend
packages, executor loops, event buses, plugin registries, or workflow engines.

## Storage

`SqliteBusinessStore` and `PostgresBusinessStore` implement load/commit of a
business workspace aggregate. Both use backend-specific SQLx migrations.
Important brief/proposal revisions, founder decisions, handoff input snapshots,
returns (including original text), conflicting contents, reviews, actions,
observations, reconciliations, and adopted learning are retained in the
document. Revision compare-and-swap plus an ordinary transaction makes each
business operation atomic and rejects stale writers. The row key and embedded
workspace/revision fields are constrained in SQL.

This is a deliberate small-MVP tradeoff: a complete workspace is read/written
per mutation. It keeps the first loop understandable and testable; it is not a
large-workspace analytics store. Future measured scale may justify separate
business tables. The logical workspace ID is not identity or tenant authorization.
The local app selects one workspace and binds only IPv4 loopback.

Deliverable uploads are capped at 5 MB, named by SHA-256, published by atomic
rename, and downloaded as attachments. A return stores title, reference,
media type, and notes (upload notes include checksum and byte size). Uploaded
content is never interpreted as HTML. External references are not crawled or
copied; their continued availability belongs to the owner. Uploaded unreferenced
files can remain after an abandoned form; no automatic deletion is introduced.

## Decisions and returns

The latest founder decision must approve the current proposal version before preparation export or recording a current action. Revising a proposal preserves older decisions and handoffs but requires new approval. A distinct historical command records founder-supplied evidence of an already completed action on an older version: completion must be within that proposal's interval, at or after an approving decision, and before any applicable revocation. It grants no current approval. Whole-second revision boundaries are inclusive; the evidence and completion time remain supplied assertions. Late returns remain attached to their original attempt/version. Return identity is `(attempt_id, return_id)`.
Identical content is a no-op; differing content preserves an inspectable
conflict without replacing the effective return. A corrected return uses a new ID.

Reported execution, deliverable acceptance, external effects, and observed
business outcome are separate. Unknown external effects need explicit evidence
before another preparation attempt or action. Reconciliation appends a record;
it does not rewrite the original return. No import retries an external action.
An accepted draft creates neither an action nor a business observation. Founders may self-prepare, and research needs no experiment proposal. Unknown effects on the explicitly linked source research task also block further preparation and action recording; relationships use opportunity/attempt IDs, never text or timestamps.

Observations refer to the recorded action's original proposal version. Results
compare to the original success/failure/inconclusive criteria even after further
proposal edits. Learning enters future handoff context only after explicit
adoption. The founder records actual execution and observed results; Kirzner
does not verify remote publication or manufacture outcome metrics. Future handoff history contains factual observation fields and recorded decisions; tentative learning text is omitted there and appears as reusable knowledge only after explicit adoption. Old immutable snapshots are preserved.

## Founder workspace and compatible return protocol

The existing aggregate and command boundary serve the founder workspace. Three brief fields (product, objective, constraints) are required; omitted optional fields deserialize as empty unknowns. Agent suggestions remain preserved return data until a founder reviews and saves them. Research opportunities optionally carry the existing bounded proposal shape as `experiment_draft`; explicit draft selection atomically links a new unapproved experiment, without reconstructing proposal fields. Repeated selection of the same source reuses the existing experiment.

Task-scoped return imports preserve the incoming JSON value and exact text, then normalize only missing task/return identities for effective matching. Generated return IDs use a deterministic content hash; existing explicit-ID imports, original digests, duplicate no-ops and inspectable conflicts remain supported. Added optional fields/defaults keep old workspace documents and return envelopes readable. No schema migration, new agent runtime or external retry is introduced.

The frontend derives next steps from durable version-specific records, persists only experiment selection in browser storage, and supports direct links. Related handoff/result components are reused inside the experiment view. Local text previews use the existing content-hash file endpoint and inert React text nodes; external references and binary materials remain links/downloads. Technical identifiers, provenance, digests, snapshots and native metadata stay available in advanced details.

## Local HTTP and Hub

The local server checks loopback Host and same-origin browser requests. It does
not implement multi-user identity or permission policies. Data/Semon database
files are not web assets; the only downloadable data path accepts content hashes.
Hub composes the library and frontend without copying business logic. Hosted
identity, authorization, collaboration, billing, and continuous operation are
future Hub work; core remains independently usable.
