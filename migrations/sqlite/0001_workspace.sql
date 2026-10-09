-- A small business aggregate, not an event log. Revision CAS protects all
-- related proposal/decision/return changes in one atomic database operation.
CREATE TABLE business_workspaces (
    workspace_id TEXT PRIMARY KEY NOT NULL,
    revision BIGINT NOT NULL CHECK (revision >= 0),
    document TEXT NOT NULL CHECK (json_valid(document)),
    CHECK (json_extract(document, '$.id') IS NOT NULL),
    CHECK (json_extract(document, '$.revision') IS NOT NULL),
    CHECK (json_extract(document, '$.id') = workspace_id),
    CHECK (json_extract(document, '$.revision') = revision)
);
