# Kirzner

A lightweight AI business operator for solo founders and small AI-native teams.

Kirzner helps founders notice opportunities, make informed business decisions, hand off work to agents and tools, and learn from the results. Its guiding idea is entrepreneurial alertness; the founder retains judgment and authority over business commitments.

**Status: design stage.** This is a fresh repository. The application and integrations described below are planned, not implemented.

## Product direction

The work centers on four questions:

- What is our business trying to achieve, and under which constraints?
- Which opportunities or changes deserve attention, and why now?
- Which decisions require the founder's judgment?
- What work has been delegated, what came back, and what should happen next?

Marketing and distribution provide an initial business use case. AI Industry Observatory / Living AI Competition Research is a separate planned module.

## Lightweight architecture

Keep one application with a small core, business modules, and thin external connections.

| Layer | Responsibility |
| --- | --- |
| Core | Business context, artifacts, decisions, handoffs, and receipts |
| Business modules | Domain-specific records, task briefs, acceptance rules, and views |
| Adapters | Transfer work to external agents or tools and receive results |
| Storage | Durable records, source references, and files |
| Application | A readable workspace for opportunities, decisions, delegated work, and results |

The core stays independent of agent vendors and business modules. External agents and tools handle research execution, model and tool operation, coding, publishing, browser interaction, and media production.

Modules use explicit imports and shared contracts. New abstractions should follow demonstrated needs across real modules.

## Handoff and return

Each handoff records a versioned objective, selected context, inputs, expected deliverables, acceptance criteria, execution target, and authorization scope.

- Decisions persist across sessions and refer to a specific proposal version.
- A changed proposal requires a new applicable decision.
- A returned result remains distinct from an accepted business outcome.
- Repeated receipts are deduplicated.
- Ambiguous external effects remain awaiting confirmation until reconciled.

Start with exportable task briefs and importable results. Add one practical external adapter after the manual round trip works. The executing tool owns its execution environment and operational scheduling.

## Planned modules

### Marketing

Business positioning, evidenced opportunities, campaign proposals, and review of returned deliverables. Production and distribution are delegated through handoffs.

### Observatory

A living record of AI competition research:

- Events and source evidence, with versions and provenance.
- Hypotheses, supporting and opposing evidence, and falsification conditions.
- Forecasts with probabilities, deadlines, resolution rules, and revision history.
- Historical replay and evaluation.
- Later, explicit multi-party simulation experiments executed externally.

Observed facts, forecasts, and simulated results remain distinguishable. Historical replay is not sufficient proof of prospective forecasting skill.

## Initial development sequence

1. Define the small shared contracts and one worked example.
2. Complete a durable context → proposal → decision → handoff → receipt → acceptance round trip.
3. Add the marketing use case and one external connection.
4. Add Observatory evidence, hypothesis, and forecast records.
5. Expand capabilities only when a concrete user need justifies them.

The first milestone is a recoverable delegation loop, rather than completion of every named business function.

## Development principles

- Keep modules independently understandable and testable.
- Preserve source references and important business revisions.
- Keep runtime, workflow, browser, and media infrastructure with external executors.
- Test restart recovery, proposal-version decisions, receipt deduplication, and external failure handling.
- Evaluate agent-produced content against a small set of real business cases.
- Treat storage and transport choices as implementation decisions to validate in the first slice.

## 中文简介

Kirzner 的定位是面向独立创业者和小型 AI 团队的 AI 商业运营者：发现机会、支持判断、交接工作、回收结果并持续学习。

新版采用轻量核心、独立业务模块和薄连接层。重活交给外部 agent 或工具；商业上下文、决策、来源和结果记录保留在 Kirzner。行业研究方向作为 Observatory 模块逐步加入。

当前仓库处于设计阶段，尚未实现应用或连接器。
