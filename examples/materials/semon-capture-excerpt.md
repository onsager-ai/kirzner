# External source material: Semon capture and ordinary reads

This is externally authored technical material, quoted from Semon's public README. It is not customer evidence or an agent-written marketing draft. The worked-example script uploads it as inert text and attaches it to a clearly labeled synthetic local return wrapper. No external agent session was executed and no real Semon capture was performed for this example.

Source: https://github.com/onsager-ai/semon/blob/defe9d791a197902c5bebcba2e2b7fe87dd9a075/README.md

Pinned commit: `defe9d791a197902c5bebcba2e2b7fe87dd9a075`.
Read through the GitHub connector and the retained source archive on 2026-10-09.

## Capture Codex sessions (quoted)

> Run one cursor-aware capture pass with:

```sh
cargo run --locked -p semon-codex -- --verbose
```

> On current Codex installations, the adapter captures session rollouts from
> `~/.codex/sessions/**/*.jsonl`. It also reads the legacy `~/.codex/history.jsonl`
> when that file exists, for compatibility with older versions, and writes traces to
> `~/.local/share/semon/traces.sqlite3`, and preserves the existing tailer's
> cursor location at `~/.local/state/devlog/codex-tailer.json`. The corresponding
> XDG base-directory variables override those roots.

> Only complete JSONL records advance the cursor. Truncated files restart at
> offset zero, and an incomplete final line waits for the next pass.

## Read the log (quoted)

> `semon log` renders the occurrence log — what was captured, joined to its
> semantics, in session/sequence order:

```sh
cargo run --locked -p semon-store --bin semon -- log \
  --store /path/to/traces.sqlite3 \
  --repo repository-name \
  --day 2026-09-20 \
  --limit 100
```

> `--repo`, `--day`, and `--limit` are all optional filters; an unfiltered call
> renders everything. This is an ordinary read: it queries only `occurrences`
> joined to `canonical_traces`, and is structurally unable to reach
> `raw_carrier_records` — enforced by a test that drops that table entirely and
> checks the output is byte-for-byte unchanged.

## Founder review boundaries

The quotations substantiate technical integration choices. They do not establish
that target customers experience handoff friction, want Kirzner, or will pay.
Native session capture is unverified in this cloud environment because real
harness logs are unavailable. Session indexing/viewing is a different capability
from durable complete capture. No real session data is included here.
