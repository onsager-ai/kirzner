CREATE TABLE business_workspaces (
    workspace_id TEXT PRIMARY KEY NOT NULL,
    revision BIGINT NOT NULL CHECK (revision >= 0),
    document TEXT NOT NULL,
    CHECK (document::jsonb ->> 'id' IS NOT NULL),
    CHECK (document::jsonb ->> 'revision' IS NOT NULL),
    CHECK (document::jsonb ->> 'id' = workspace_id),
    CHECK ((document::jsonb ->> 'revision')::bigint = revision)
);
