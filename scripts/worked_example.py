#!/usr/bin/env python3
"""Load a source-grounded Semon preparation example into a fresh local workspace.

The opportunity and proposal are labeled planning data authored for this
example. The script does not invoke an external agent or claim customer
evidence, outreach, publication, or a real Semon session capture.
"""
import argparse
import json
from pathlib import Path
from urllib.request import Request, urlopen

ROOT = Path(__file__).resolve().parents[1]
SOURCE_URL = "https://github.com/onsager-ai/semon/blob/defe9d791a197902c5bebcba2e2b7fe87dd9a075/README.md"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--url", default="http://127.0.0.1:4317")
    parser.add_argument("--result", default="test-results/worked-example.json")
    args = parser.parse_args()
    if not args.url.startswith(("http://127.0.0.1:", "http://localhost:")):
        parser.error("Use a loopback Kirzner instance")

    def get(path):
        with urlopen(args.url + path) as response:
            return json.load(response)

    state = get("/api/workspace")
    if state["revision"] != 0:
        parser.error("Use a fresh data directory for the worked example")

    def mutate(command):
        nonlocal state
        payload = json.dumps(
            {"expected_revision": state["revision"], "command": command}
        ).encode()
        with urlopen(
            Request(
                args.url + "/api/workspace",
                data=payload,
                headers={"Content-Type": "application/json"},
            )
        ) as response:
            state = json.load(response)["workspace"]
        return state

    brief = json.loads((ROOT / "examples/real-product-brief.json").read_text())
    if set(brief) != {"product", "objective", "constraints"}:
        parser.error("The starter brief must contain only its three required values")
    mutate({"type": "save_brief", "brief": brief})
    mutate(
        {
            "type": "create_research",
            "objective": brief["objective"],
            "context": "Prepare source-grounded planning data from the pinned public Semon README. Keep customer pain, participant access, installed capability, urgency, and willingness to pay unknown. This script does not run an external agent.",
            "authorization_scope": "Read public source material and prepare local notes only; no external writes, private-log inspection, prospect contact, publication, purchase, or deployment.",
        }
    )
    research_attempt = state["attempts"][-1]

    source = {
        "url": SOURCE_URL,
        "title": "Semon interfaces at the inspected revision",
        "evidence": "The pinned README documents incremental local capture and ordinary occurrence-log reads. This supports technical context only; local availability and customer demand are unverified. Verified README blob: e3b46edb940efd51ca4e9f3ca6da958daee68892.",
    }
    opportunity = {
        "title": "Hypothesis: founders may need a simpler way to resume experiments across agent sessions",
        "customer_relevance": "Proposed audience: solo founders who already use multiple external coding or research agents. Reachable participants, coordination pain, and current workarounds are unknown.",
        "why_now": "The pinned Semon documentation gives technical context for a source-backed interview pack. It does not establish timing, urgency, or demand.",
        "counterevidence": "Native agent history, existing notes, or Semon's viewer may already be sufficient. Repository documentation does not demonstrate a business need or demand.",
        "unknowns": "Participant access, frequency and cost of coordination interruptions, existing alternatives, urgency, willingness to use or pay, and permission to inspect any private logs remain unknown.",
        "validation_action": "Prepare and review a small interview pack. Any conversations or use of private session data require a separate founder action outside Kirzner.",
        "sources": [source],
        "experiment_draft": {
            "audience": "Hypothesis: solo founders already using multiple external agent sessions; actual reachable participants are unknown.",
            "action": "Prepare a source-backed interview pack for founder review. The founder may later choose to conduct three conversations outside Kirzner; no conversations have occurred in this example.",
            "expected_deliverables": "A one-page source-backed context note, five neutral questions about the participant's last real experiment, a blank observation worksheet, and an explicit list of unknowns. Acceptance requires pinned source links, no invented user quotations or demand claims, and no private session data.",
            "observation_window": "Proposed seven-day window begins only after a separately confirmed real first conversation; until then the experiment is unstarted and inconclusive.",
            "success_criteria": "Proposed criterion for founder review: at least two participants independently describe a recent concrete coordination interruption and one voluntarily agrees to review a follow-up prototype. No such evidence exists yet.",
            "failure_criteria": "After the agreed conversations and window are actually completed, no participant can describe a relevant recent interruption and none requests follow-up. Failure cannot be inferred from an unstarted window.",
            "inconclusive_criteria": "Fewer than three completed conversations, incomplete observation window, unavailable evidence, or only leading or hypothetical answers.",
            "resource_limits": "Proposed cap for founder review: two hours of preparation and zero spend. Actual available time and participant access are unknown.",
            "authorization_scope": "Read the pinned public source and prepare local materials only. Do not contact prospects, publish, purchase, deploy, inspect private logs, or change external systems. Importing this draft grants no approval.",
        },
    }
    research_return = {
        "status": "partial",
        "external_effects": "none",
        "summary": "Locally prepared source-grounded planning fixture. It is not an external agent return, customer research, or evidence that the proposed interviews occurred.",
        "deliverables": [],
        "opportunities": [opportunity],
        "execution_refs": [],
    }
    original_research_text = json.dumps(
        research_return, ensure_ascii=False, separators=(",", ":")
    )
    mutate(
        {
            "type": "import_task_return",
            "attempt_id": research_attempt["id"],
            "payload": research_return,
            "original_text": original_research_text,
        }
    )
    source_opportunity = state["opportunities"][-1]
    research_receipt = state["attempts"][-1]["returns"][-1]
    assert not state["experiments"], "Importing a returned draft must not create an experiment"
    assert source_opportunity["evidence"]["experiment_draft"] == opportunity["experiment_draft"]
    mutate(
        {
            "type": "create_draft_experiment",
            "opportunity_id": source_opportunity["id"],
        }
    )
    experiment = state["experiments"][-1]
    assert not experiment["decisions"], "Using a draft must create an unapproved v1"
    experiment_id = experiment["id"]
    mutate(
        {
            "type": "decide_proposal",
            "experiment_id": experiment_id,
            "version": experiment["proposals"][-1]["version"],
            "approved": True,
            "reason": "Approve local source review and preparation only; no interviews or external writes are authorized.",
        }
    )
    mutate(
        {
            "type": "prepare_handoff",
            "experiment_id": experiment_id,
            "version": experiment["proposals"][-1]["version"],
            "context": "Review the pinned public README excerpt as technical preparation evidence only. Keep demand, customer pain, installed availability, and urgency unknown.",
        }
    )
    preparation_attempt = state["attempts"][-1]

    material = ROOT / "examples/materials/semon-capture-excerpt.md"
    with urlopen(
        Request(
            args.url + "/api/files?name=semon-capture-excerpt.md",
            data=material.read_bytes(),
            headers={"Content-Type": "text/markdown"},
        )
    ) as response:
        deliverable = json.load(response)
    material_return = {
        "status": "partial",
        "external_effects": "none",
        "summary": "Synthetic local return wrapper attaching the real pinned Semon README excerpt. No external agent session, interview, or business action is represented.",
        "deliverables": [deliverable],
        "opportunities": [],
        "execution_refs": [],
    }
    original_material_text = json.dumps(
        material_return, ensure_ascii=False, separators=(",", ":")
    )
    mutate(
        {
            "type": "import_task_return",
            "attempt_id": preparation_attempt["id"],
            "payload": material_return,
            "original_text": original_material_text,
        }
    )
    material_return_id = state["attempts"][-1]["returns"][-1]["input"]["return_id"]
    mutate(
        {
            "type": "review_return",
            "attempt_id": preparation_attempt["id"],
            "return_id": material_return_id,
            "accepted": True,
            "note": "Accept only as pinned technical source material. It provides no evidence of customer demand or real Semon session capture.",
        }
    )

    experiment = next(item for item in state["experiments"] if item["id"] == experiment_id)
    final_attempt = next(
        item for item in state["attempts"] if item["id"] == preparation_attempt["id"]
    )
    assert research_receipt["original_text"] == original_research_text
    assert research_receipt["input"]["attempt_id"] == research_attempt["id"]
    assert research_receipt["input"]["return_id"].startswith("return-")
    assert len(research_receipt["input"]["return_id"]) == len("return-") + 64
    assert "attempt_id" not in research_return and "return_id" not in research_return
    material_receipt = next(
        item for item in final_attempt["returns"] if item["input"]["return_id"] == material_return_id
    )
    assert material_receipt["input"]["return_id"].startswith("return-")
    assert len(material_receipt["input"]["return_id"]) == len("return-") + 64
    assert "attempt_id" not in material_return and "return_id" not in material_return
    material_review = next(
        item for item in final_attempt["reviews"] if item["return_id"] == material_return_id
    )
    assert experiment["proposals"][0]["content"] == opportunity["experiment_draft"]
    assert experiment["decisions"][-1]["approved"] is True
    assert not experiment["actions"] and not experiment["observations"]
    assert not state["adopted_learning"]
    assert research_receipt["input"]["attempt_id"] == research_attempt["id"]
    assert material_receipt["input"]["attempt_id"] == preparation_attempt["id"]
    assert material_receipt["original_text"] == original_material_text
    assert material_review["accepted"] is True
    assert all(
        not state["brief"].get(field)
        for field in ("customer", "capabilities", "time_budget", "money_budget", "channels", "materials")
    )
    evidence = {
        "evidence_kind": "source-grounded example with a synthetic local return wrapper",
        "external_agent_execution": False,
        "customer_events_or_evidence": False,
        "real_outreach_or_business_outcome": False,
        "source": {
            "product": "Semon",
            "commit": "defe9d791a197902c5bebcba2e2b7fe87dd9a075",
            "readme_blob": "e3b46edb940efd51ca4e9f3ca6da958daee68892",
        },
        "starter_brief": {
            "provided_values": ["product", "objective", "constraints"],
            "unknown_optional_fields": ["customer", "capabilities", "time_budget", "money_budget", "channels", "materials"],
        },
        "research_return": {
            "task_id": research_attempt["id"],
            "return_id": research_receipt["input"]["return_id"],
            "status": research_receipt["input"]["status"],
            "source_opportunity_id": source_opportunity["id"],
            "draft_imported_unapproved": True,
        },
        "experiment": {
            "id": experiment_id,
            "source_opportunity_id": source_opportunity["id"],
            "proposal_version": experiment["proposals"][0]["version"],
            "preparation_only_approved": experiment["decisions"][-1]["approved"],
        },
        "material_review": {
            "task_id": preparation_attempt["id"],
            "return_id": material_return_id,
            "accepted": material_review["accepted"],
            "source_excerpt": "real pinned public Semon README material",
        },
        "actions": len(experiment["actions"]),
        "observations": len(experiment["observations"]),
        "adopted_learning": len(state["adopted_learning"]),
    }
    result_path = Path(args.result)
    result_path.parent.mkdir(parents=True, exist_ok=True)
    result_path.write_text(json.dumps(evidence, indent=2) + "\n")
    print(
        f"Prepared {experiment_id} from {source_opportunity['id']} with an explicit v1 decision; "
        f"source material review recorded for task {preparation_attempt['id']}."
    )
    print("The brief starts with three values; all other business details remain unknown.")
    print(f"Evidence written to {result_path}.")
    print("No external agent execution, prospect contact, publication, customer evidence, or business outcome is claimed.")


if __name__ == "__main__":
    main()
