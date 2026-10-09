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

The latest founder decision must approve the current proposal version before
preparation export or recording an action. Revising a proposal preserves older
decisions and handoffs but requires new approval. Late returns remain attached
to their original attempt/version. Return identity is `(attempt_id, return_id)`.
Identical content is a no-op; differing content preserves an inspectable
conflict without replacing the effective return. A corrected return uses a new ID.

Reported execution, deliverable acceptance, external effects, and observed
business outcome are separate. Unknown external effects need explicit evidence
before another preparation attempt or action. Reconciliation appends a record;
it does not rewrite the original return. No import retries an external action.
An accepted draft creates neither an action nor a business observation.
Founders may self-prepare, and research needs no experiment proposal.

Observations refer to the recorded action's original proposal version. Results
compare to the original success/failure/inconclusive criteria even after further
proposal edits. Learning enters future handoff context only after explicit
adoption. The founder records actual execution and observed results; Kirzner
does not verify remote publication or manufacture outcome metrics.

## Local HTTP and Hub

The local server checks loopback Host and same-origin browser requests. It does
not implement multi-user identity or permission policies. Data/Semon database
files are not web assets; the only downloadable data path accepts content hashes.
Hub composes the library and frontend without copying business logic. Hosted
identity, authorization, collaboration, billing, and continuous operation are
future Hub work; core remains independently usable.
