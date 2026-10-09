import { useState } from 'react'
import { useWorkspace } from '../store'
import { Button } from '../components/ui/button'
import { Field, Form, Panel, Select, Status, text } from '../components/forms'
import type { Experiment, ProposalInput } from '../types'
import { Link } from '@tanstack/react-router'
const proposalFields: [keyof ProposalInput, string][] = [
  ['audience', 'Target audience'], ['action', 'Proposed action'], ['expected_deliverables', 'Expected deliverables'], ['observation_window', 'Observation window'],
  ['success_criteria', 'Success criteria'], ['failure_criteria', 'Failure criteria'], ['inconclusive_criteria', 'Inconclusive criteria'], ['resource_limits', 'Resource limits'], ['authorization_scope', 'Preparation authorization scope'],
]
function proposal(data: FormData) { return Object.fromEntries(proposalFields.map(([key]) => [key, text(data, key)])) }
function ProposalFields({ value }: { value?: ProposalInput }) { return <>{proposalFields.map(([key, label]) => <Field key={key} name={key} label={label} value={value?.[key]}/>)}</> }
function ExperimentCard({ experiment }: { experiment: Experiment }) {
  const { save, busy } = useWorkspace()
  const current = experiment.proposals.at(-1)!
  const decision = experiment.decisions.filter(d => d.version === current.version).at(-1)
  const [editing, setEditing] = useState(false)
  return <Panel title={`${experiment.id}: ${current.content.action}`} eyebrow={`Proposal version ${current.version}`}>
    <div className="row"><Status tone={decision?.approved ? 'good' : 'amber'}>{decision?.approved ? 'Approved for this version' : decision ? 'Declined' : 'Needs your decision'}</Status><Button variant="outline" size="sm" onClick={() => setEditing(!editing)}>{editing ? 'Close editor' : 'Revise proposal'}</Button></div>
    <dl className="evidence-grid">{proposalFields.map(([key, label]) => <div key={key}><dt>{label}</dt><dd>{current.content[key]}</dd></div>)}</dl>
    {editing && <div className="subpanel"><h3>Save a new proposal version</h3><p className="muted">A change requires a new founder decision. Earlier decisions and handoffs stay attached to their original version.</p><Form key={current.version} submit="Save new version" busy={busy} onSubmit={async data => { if (await save({ type: 'revise_proposal', experiment_id: experiment.id, proposal: proposal(data) })) setEditing(false) }}><ProposalFields value={current.content}/></Form></div>}
    <div className="subpanel"><h3>Founder decision · version {current.version}</h3><Form submit="Record proposal decision" busy={busy} onSubmit={data => save({ type: 'decide_proposal', experiment_id: experiment.id, version: current.version, approved: text(data, 'decision') === 'approve', reason: text(data, 'reason') })}>
      <Select name="decision" label="Decision" options={[{ value: 'approve', label: 'Approve preparation' }, { value: 'decline', label: 'Decline' }]}/><Field name="reason" label="Proposal decision reason"/>
    </Form></div>
    {decision?.approved && <div className="subpanel"><h3>Prepare the materials</h3><p className="muted">Export a brief for your agent, or prepare the materials yourself and record the action in Results.</p><Form submit="Create preparation handoff" busy={busy} onSubmit={data => save({ type: 'prepare_handoff', experiment_id: experiment.id, version: current.version, context: text(data, 'context') })}><Field name="context" label="Additional preparation context" required={false}/></Form><p><Link to="/handoffs">Open handoffs →</Link> · <Link to="/results">Record your own action →</Link></p></div>}
    <details><summary>Proposal versions and decisions</summary>{experiment.proposals.map(p => <div className="history" key={p.version}><h3>Version {p.version}</h3><dl>{proposalFields.map(([key, label]) => <div key={key}><dt>{label}</dt><dd>{p.content[key]}</dd></div>)}</dl>{experiment.decisions.filter(d => d.version === p.version).map((d, i) => <p key={i}><strong>{d.approved ? 'Approved' : 'Declined'}</strong> · {d.reason}</p>)}</div>)}</details>
  </Panel>
}
export function ExperimentsPage() {
  const { workspace, save, busy } = useWorkspace()
  const chosen = workspace.opportunities.filter(o => o.disposition === 'choose')
  return <><div className="page-heading"><span className="eyebrow">03 / Small experiments</span><h1>A test you can afford.</h1><p>Write the criteria before the result. Your decision applies to one specific version.</p></div>
    <Panel title="Propose an experiment">{!workspace.brief ? <p>Save a <Link to="/">business brief</Link> first.</p> : <details open={workspace.experiments.length === 0}><summary>New experiment proposal</summary><Form submit="Create experiment" busy={busy} onSubmit={data => save({ type: 'create_experiment', opportunity_id: text(data, 'opportunity') || null, proposal: proposal(data) })}>
      <Select name="opportunity" label="Chosen opportunity" options={[{ value: '', label: 'Independent experiment' }, ...chosen.map(o => ({ value: o.id, label: o.evidence.title }))]}/><ProposalFields/>
    </Form></details>}</Panel>
    {workspace.experiments.map(e => <ExperimentCard key={e.id} experiment={e}/>)}
  </>
}
