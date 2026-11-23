# Kirzner development

Keep one Rust crate (library plus local binary) and one frontend directory.
Core is independently usable; no Hub or commercial dependencies. External
executors own execution, credentials, retries, and scheduling.

Read README.md and docs/architecture.md for business/storage changes.
Preserve version-specific founder decisions, original handoff snapshots/returns,
conflicts, unknown effects, and explicit learning adoption.

Use an isolated task branch and preserve unrelated work. Identifiers and
technical documentation are English; follow the user's report language.
Do not exceed user limits on PRs, deployment, outreach, purchases, or publishing.

Run `cargo fmt --all -- --check` for formatting. CI runs builds, tests, and
frontend verification. UI changes require meaningful browser verification.
Label synthetic evidence and accurately report unavailable real-world validation.
