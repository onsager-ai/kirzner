# Semon connection and local capture verification

The inspected implementation is `onsager-ai/semon` at
`defe9d791a197902c5bebcba2e2b7fe87dd9a075`. Its capture, persistence,
ordinary-read, query/export and viewer interfaces were read from README and
the relevant `semon-codex`, `semon-store`, and model/renderer sources.
The capture README was also verified through the GitHub connector at this ref.
No Semon Hub or Duhem code was needed to define the local CLI contract.

Implemented interfaces relevant here:

- `semon-codex --sessions ... --history ... --state ... --store ... --repo ...`
  performs a cursor-aware pass into a separate, owner-protected Semon store.
  Only complete lines advance capture. Native source deletion does not erase
  the store's retained semantic/forensic records.
- `semon log --store ... --repo ... --day ... --limit ...` is an ordinary
  occurrence/semantic read. It cannot expose raw forensic bytes. The pinned
  `log` command has no documented session filter; Kirzner does not invent one.
- `semon sessions`, `query`, `--model-json` and the native viewer expose exact
  IDs/links and native transcript byte cursors. Index/viewer caches are not
  full durable capture. Native transcript reads may become unavailable after
  source loss even when captured semantic records remain.
- Canonical trace export/replication excludes forensic records. It is not a
  full raw-capture backup. Raw forensic access stays with Semon's explicit
  authorized operator surface. Kirzner exposes no forensic HTTP route.

Install/build the pinned CLI outside the business repository:

```sh
git clone https://github.com/onsager-ai/semon.git ../semon
git -C ../semon checkout defe9d791a197902c5bebcba2e2b7fe87dd9a075
cargo build --manifest-path ../semon/Cargo.toml --locked -p semon-codex -p semon-store
export KIRZNER_SEMON_CODEX_BIN="$(pwd)/../semon/target/debug/semon-codex"
```

Capture a real local session once native data is available:

```sh
cargo run --locked -- capture-codex /absolute/native/codex/sessions /absolute/native/codex/history.jsonl
# History may be absent on current Codex versions.
../semon/target/debug/semon log --store data/semon/traces.sqlite3 --repo kirzner --limit 100
```

`KIRZNER_SEMON_STORE` and `KIRZNER_SEMON_STATE` override the separate store
and cursor paths. The browser never starts a capture process. Store/binary
presence is reported accurately without claiming transcript completeness.
Missing installation, failed capture, or offline/missing native transcript
does not gate access to business records.

In Handoffs, attach the exact namespace, harness, native session ID, machine when
applicable, provenance/revision, optional native continuation reference, and
optional segment start/end byte offsets. Multiple sessions are supported per
attempt. No relationship is inferred from text, dates, directories, or trace IDs.
These supplied references are not authenticated proof of capture. Use the
executor's native continuation; the downloadable continuation brief is the fallback.

For real-session acceptance, select one known native session, capture it, check
Semon's reported coverage/cursor, stop capture, and temporarily move a **copy**
of that source out of the verification fixture. Confirm ordinary reads still
return its semantic records and verify complete retained line custody through
Semon's operator-authorized forensic interface. Do not delete original logs or
commit real session data. Keep evidence local, with redacted counts/provenance
in reports. A semantic trace ID alone cannot establish this session's identity.

The cloud bootstrap exercised this persistence/exposure boundary with three
explicitly synthetic supported Codex-format records: three raw rows, one
semantic occurrence, idempotent recapture, unchanged ordinary reads after
source removal, and unchanged reads after deleting disposable forensic rows.
This is adapter evidence only. Real-session capture is deferred to a local
session because this cloud environment exposes no native logs, as confirmed
by the user. It is not claimed as completed.

Stop capture before a Semon backup. Use SQLite's backup facility on its store,
retain the matching cursor and source namespace/provenance, and preserve
owner-only permissions. The Kirzner business backup intentionally excludes
Semon and raw logs; manage that sensitive backup through Semon's owning surface.
