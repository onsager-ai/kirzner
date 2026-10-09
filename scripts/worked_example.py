#!/usr/bin/env python3
"""Load real product/source preparation evidence through the ordinary local API.

Requires a fresh workspace. Performs no outreach or publication, and records no
invented business observation. Research interpretation is explicitly an inference.
"""
import argparse
import json
from pathlib import Path
from urllib.request import Request, urlopen

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--url", default="http://127.0.0.1:4317")
    args = parser.parse_args()
    if not args.url.startswith(("http://127.0.0.1:", "http://localhost:")):
        parser.error("Use a loopback Kirzner instance")
    def get():
        with urlopen(args.url + "/api/workspace") as response:
            return json.load(response)
    state = get()
    if state["revision"] != 0:
        parser.error("Use a fresh data directory for the worked example")
    def save(command):
        nonlocal state
        payload = json.dumps({"expected_revision": state["revision"], "command": command}).encode()
        with urlopen(Request(args.url + "/api/workspace", data=payload, headers={"Content-Type": "application/json"})) as response:
            state = json.load(response)["workspace"]
    brief = json.loads((ROOT / "examples/real-product-brief.json").read_text())
    save({"type": "save_brief", "brief": brief})
    save({"type": "create_research", "objective": brief["objective"], "context": "Review the two pinned repository sources. Do not claim customer demand from technical documentation.", "authorization_scope": "Read sources and prepare findings only; no external writes."})
    opportunity = {
        "title": "Hypothesis: interview founders about manual agent handoffs",
        "customer_relevance": "Inference from Kirzner's intended customer and manual handoff direction; real customer relevance needs interviews.",
        "why_now": "A preparation brief can be reviewed at zero cost before any outreach is authorized.",
        "counterevidence": "The product README is a design document. Technical feasibility does not establish acquisition demand.",
        "unknowns": "Frequency of the pain, willingness to try, willingness to pay, and reachable audience are unknown.",
        "validation_action": "Founder reviews a source-backed preparation pack; consider a separately authorized small feedback request later.",
        "sources": brief["materials"],
    }
    envelope = {"attempt_id": "attempt-1", "return_id": "source-review-1", "status": "succeeded", "external_effects": "none", "summary": "Source-grounded preparation example assembled by this implementation session; opportunity interpretation is an inference, not external agent execution evidence.", "deliverables": [], "opportunities": [opportunity], "no_opportunity_reason": None, "execution_refs": []}
    save({"type": "import_return", "payload": envelope})
    save({"type": "decide_opportunity", "opportunity_id": "opportunity-1", "disposition": "choose", "reason": "Only prepare and review evidence now; demand remains unverified."})
    proposal = {"audience": brief["customer"], "action": "Prepare a feedback-request experiment for founder review; actual outreach is deferred.", "expected_deliverables": "An externally authored source excerpt with provenance and a record of uncertainty.", "observation_window": "A later seven-day window begins only if the founder performs authorized outreach.", "success_criteria": "A later real qualified customer response; no success is claimed during preparation.", "failure_criteria": "A later completed outreach window without a qualified response.", "inconclusive_criteria": "No outreach performed, incomplete window, or missing customer evidence.", "resource_limits": "Zero spend; one preparation session.", "authorization_scope": "Prepare and review only. Do not contact, publish, deploy, or purchase."}
    save({"type": "create_experiment", "opportunity_id": "opportunity-1", "proposal": proposal})
    save({"type": "decide_proposal", "experiment_id": "experiment-1", "version": 1, "approved": True, "reason": "Approve preparation at zero spend only."})
    save({"type": "prepare_handoff", "experiment_id": "experiment-1", "version": 1, "context": "Use the pinned Semon source excerpt; preserve uncertainty and provenance."})
    material = ROOT / "examples/materials/semon-capture-excerpt.md"
    with urlopen(Request(args.url + "/api/files?name=semon-capture-excerpt.md", data=material.read_bytes(), headers={"Content-Type": "text/markdown"})) as response:
        deliverable = json.load(response)
    envelope = {"attempt_id": "attempt-2", "return_id": "external-source-material-1", "status": "succeeded", "external_effects": "none", "summary": "Imported real externally authored Semon documentation. This is an existing source material, not a newly executed agent session or marketing campaign.", "deliverables": [deliverable], "opportunities": [], "no_opportunity_reason": None, "execution_refs": []}
    save({"type": "import_return", "payload": envelope})
    save({"type": "review_return", "attempt_id": "attempt-2", "return_id": "external-source-material-1", "accepted": True, "note": "Accept the source excerpt as technical preparation evidence only. It provides no proof of customer demand or durable capture of a real session in this environment."})
    assert not state["experiments"][0]["actions"] and not state["experiments"][0]["observations"]
    print("Worked example loaded: real Kirzner brief, pinned external source material, proposal v1, preparation handoff, return, and founder review.")
    print("No outreach, publication, customer metrics, business outcome, or real session capture claimed.")


if __name__ == "__main__":
    main()
