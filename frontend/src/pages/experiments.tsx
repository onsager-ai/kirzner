import { useEffect, useState } from 'react'
import { Link } from '@tanstack/react-router'
import { Button } from '../components/ui/button'
import { Empty, Field, Form, Panel, Select, Status, text } from '../components/forms'
import { ProposalFields, proposalFields, proposalFrom } from '../components/proposal'
import { useWorkspace } from '../store'
import type { Attempt, Experiment } from '../types'
import { AttemptCard } from './handoffs'
import { ResultCard } from './results'

function unresolvedEffects(attempt: Attempt) {
  return attempt.returns.some(receipt => receipt.input.external_effects === 'unknown' && !attempt.reconciliations.some(r => r.return_id === receipt.input.return_id))
}

function experimentStage(experiment: Experiment, attempts: Attempt[]) {
  const current = experiment.proposals.at(-1)!
  const decision = experiment.decisions.filter(d => d.version === current.version).at(-1)
  const relevant = attempts
  if (relevant.some(unresolvedEffects)) return 'Reconcile effects'
  if (!decision?.approved) return 'Needs decision'
  const currentAction = [...experiment.actions].reverse().find(a => a.version === current.version)
  const currentAttempt = [...relevant].reverse().find(a => a.proposal_version === current.version)
  if (!currentAction && !currentAttempt) return 'Prepare materials'
  if (!currentAction && currentAttempt) {
    if (currentAttempt.returns.length === 0) return 'Awaiting agent return'
    const acceptedMaterials = currentAttempt.returns.some(receipt => receipt.input.deliverables.length > 0 && [...currentAttempt.reviews].reverse().find(entry => entry.return_id === receipt.input.return_id)?.accepted === true)
    if (!acceptedMaterials) {
      if (currentAttempt.returns.some(receipt => receipt.input.status !== 'succeeded')) return 'Review incomplete return'
      if (currentAttempt.returns.every(receipt => receipt.input.deliverables.length === 0)) return 'Prepare materials'
      return 'Review materials'
    }
  }
  if (!currentAction) return 'Record actual action'
  const observations = experiment.observations.filter(o => o.action_id === currentAction.id)
  if (observations.length === 0) return 'Observe against original criteria'
  if (observations.some(o => !experiment.next_steps.some(n => n.observation_id === o.id))) return 'Decide next'
  return 'Recorded next decision'
}

function ProposalEditor({ experiment, blocked }: { experiment: Experiment; blocked: boolean }) {
  const { workspace, save, busy } = useWorkspace()
  const current = experiment.proposals.at(-1)!
  const decision = experiment.decisions.filter(d => d.version === current.version).at(-1)
  const [editing, setEditing] = useState(false)
  return <Panel title="Current proposal" eyebrow={`Version ${current.version}`}>
    <div className="row"><Status tone={decision?.approved ? 'good' : 'amber'}>{decision?.approved ? 'Approved for this version' : decision ? 'Declined' : 'Needs your decision'}</Status><Button variant="outline" size="sm" onClick={() => setEditing(!editing)}>{editing ? 'Close editor' : 'Revise proposal'}</Button></div>
    <dl className="evidence-grid">{proposalFields.map(([key, label]) => <div key={key}><dt>{label}</dt><dd>{current.content[key]}</dd></div>)}</dl>
    {editing && <div className="subpanel"><h3>Save a new proposal version</h3><p className="muted">A change needs a new founder decision. Earlier decisions, handoffs, and returns stay attached to their original version.</p><Form key={`revise-${experiment.id}-${current.version}`} submit="Save new version" busy={busy} onSubmit={async data => { if (await save({ type: 'revise_proposal', experiment_id: experiment.id, proposal: proposalFrom(data) })) setEditing(false) }}><ProposalFields value={current.content}/></Form></div>}
    <div className="subpanel"><h3>Founder decision · version {current.version}</h3><Form key={`decision-${experiment.id}-${current.version}-${decision?.recorded_at || 'none'}`} submit="Record proposal decision" busy={busy} onSubmit={data => save({ type: 'decide_proposal', experiment_id: experiment.id, version: current.version, approved: text(data, 'decision') === 'approve', reason: text(data, 'reason') })}>
      <Select name="decision" label="Decision" value="" required options={[{ value: '', label: 'Choose a decision' }, { value: 'approve', label: 'Approve preparation' }, { value: 'decline', label: 'Decline' }]}/><Field name="reason" label="Proposal decision reason"/>
    </Form></div>
    {decision?.approved && blocked && <p className="finding">An unresolved external effect blocks new preparation. Reconcile it in the related task before continuing.</p>}
    {decision?.approved && !blocked && <div className="subpanel"><h3>Prepare the materials</h3><p className="muted">Create a bounded handoff, or prepare the materials yourself. Preparation does not authorize an external action.</p><Form key={`prepare-${experiment.id}-${current.version}-${workspace.revision}`} submit="Create preparation handoff" busy={busy} onSubmit={data => save({ type: 'prepare_handoff', experiment_id: experiment.id, version: current.version, context: text(data, 'context') })}><Field name="context" label="Additional preparation context" required={false}/></Form><p><Link to="/handoffs">Open handoffs →</Link> · <Link to="/results">Record your own action →</Link></p></div>}
    <details><summary>Proposal versions and decisions</summary>{experiment.proposals.map(p => <div className="history" key={p.version}><h3>Version {p.version}</h3><dl>{proposalFields.map(([key, label]) => <div key={key}><dt>{label}</dt><dd>{p.content[key]}</dd></div>)}</dl>{experiment.decisions.filter(d => d.version === p.version).map((d, i) => <p key={i}><strong>{d.approved ? 'Approved' : 'Declined'}</strong> · {d.reason}</p>)}</div>)}</details>
  </Panel>
}

function OpportunitySource({ opportunityId }: { opportunityId: string | null }) {
  const { workspace } = useWorkspace()
  const opportunity = workspace.opportunities.find(o => o.id === opportunityId)
  if (!opportunityId) return <Panel title="Independent experiment"><p className="muted">This experiment was created without a linked opportunity.</p></Panel>
  if (!opportunity) return <Panel title="Source opportunity unavailable"><p className="muted">The linked source record is not available in this workspace.</p></Panel>
  return <Panel title={opportunity.evidence.title} eyebrow="Source opportunity">
    <div className="row"><Status>{opportunity.disposition}</Status><Link to="/opportunities">Review opportunity</Link></div>
    <div className="evidence-grid">{([['Customer relevance', opportunity.evidence.customer_relevance], ['Why now', opportunity.evidence.why_now], ['Counterevidence', opportunity.evidence.counterevidence], ['Unknowns', opportunity.evidence.unknowns], ['Cheap validation', opportunity.evidence.validation_action]] as const).map(([label, value]) => <div key={label}><h3>{label}</h3><p>{value}</p></div>)}</div>
    <ul className="sources">{opportunity.evidence.sources.map((source, i) => <li key={i}><a href={source.url} target="_blank" rel="noreferrer">{source.title} ↗</a><p>{source.evidence}</p></li>)}</ul>
    <details><summary>Draft and source history</summary><p className="muted">The opportunity and its original research return remain available in Handoffs. Using a draft did not approve or execute it.</p>{opportunity.evidence.experiment_draft && <dl>{proposalFields.map(([key, label]) => <div key={key}><dt>{label}</dt><dd>{opportunity.evidence.experiment_draft?.[key]}</dd></div>)}</dl>}</details>
  </Panel>
}

export function ExperimentsPage() {
  const { workspace, save, latestWorkspace, busy } = useWorkspace()
  const [searchVersion, setSearchVersion] = useState(0)
  const savedId = typeof window === 'undefined' ? null : window.localStorage.getItem(`kirzner:selected-experiment:${workspace.id}`)
  const queryId = typeof window === 'undefined' ? null : new URLSearchParams(window.location.search).get('experiment')
  const queried = workspace.experiments.find(e => e.id === queryId)
  const saved = workspace.experiments.find(e => e.id === savedId)
  const selected = queried || saved || workspace.experiments.at(-1) || null
  const invalidQuery = Boolean(queryId && !queried)
  const chosen = workspace.opportunities.filter(o => o.disposition === 'choose')

  function selectExperiment(id: string) {
    if (!id) return
    const url = new URL(window.location.href)
    url.pathname = '/experiments'
    url.searchParams.set('experiment', id)
    window.history.replaceState(window.history.state, '', `${url.pathname}${url.search}${url.hash}`)
    window.localStorage.setItem(`kirzner:selected-experiment:${workspace.id}`, id)
    setSearchVersion(value => value + 1)
  }

  useEffect(() => {
    const sync = () => setSearchVersion(value => value + 1)
    window.addEventListener('popstate', sync)
    return () => window.removeEventListener('popstate', sync)
  }, [])
  useEffect(() => {
    if (selected) window.localStorage.setItem(`kirzner:selected-experiment:${workspace.id}`, selected.id)
  }, [workspace.id, selected?.id, searchVersion])

  const relatedOpportunity = selected?.opportunity_id ? workspace.opportunities.find(o => o.id === selected.opportunity_id) : undefined
  const relatedAttempts = selected ? workspace.attempts.filter(a => a.experiment_id === selected.id || (relatedOpportunity && a.kind === 'research' && a.id === relatedOpportunity.attempt_id)) : []
  const stage = selected ? experimentStage(selected, relatedAttempts) : null
  const effectsBlocked = relatedAttempts.some(unresolvedEffects)

  return <>
    <div className="page-heading"><span className="eyebrow">03 / Small experiments</span><h1>Work from the evidence.</h1><p>Review the proposal, prepare materials, and keep each action and observation connected to the version that informed it.</p></div>
    {invalidQuery && <p role="status" className="finding">This experiment link points to a record that is unavailable in this workspace. Showing a saved experiment when one exists.</p>}
    {workspace.experiments.length > 0 && <Panel title="Experiment workspace">
      <label className="field"><span>Select experiment</span><select aria-label="Select experiment" value={selected?.id || ''} onChange={event => selectExperiment(event.target.value)}>{workspace.experiments.map(e => <option key={e.id} value={e.id}>{e.proposals.at(-1)?.content.action || 'Experiment'} · version {e.proposals.at(-1)?.version}</option>)}</select></label>
      {selected && <><div className="stage-banner"><div><span className="eyebrow">Current next step</span><h3>{stage}</h3></div><Status tone={stage === 'Recorded next decision' ? 'good' : stage === 'Reconcile effects' ? 'amber' : ''}>{stage}</Status></div>
        <OpportunitySource opportunityId={selected.opportunity_id}/>
        <ProposalEditor experiment={selected} blocked={effectsBlocked}/>
        <section className="workspace-section"><div className="row"><div><span className="eyebrow">Original handoffs and returns</span><h2>Preparation and research materials</h2></div><Link to="/handoffs">All handoffs</Link></div>
          {relatedAttempts.length === 0 ? <p className="muted">No research or preparation task is linked yet. Approve the current proposal to prepare a handoff.</p> : [...relatedAttempts].reverse().map(attempt => <AttemptCard key={attempt.id} attempt={attempt} embedded/>)}
        </section>
        <ResultCard experiment={selected} embedded/>
      </>}
    </Panel>}
    {workspace.experiments.length === 0 && <Empty title="No experiment yet">Review an evidenced opportunity draft or create a bounded proposal below.</Empty>}
    <Panel title="Propose an experiment"><p className="muted">Keep the existing full form for an independent proposal or a proposal you write yourself.</p>{!workspace.brief ? <p>Save a <Link to="/">business brief</Link> first.</p> : <details open={workspace.experiments.length === 0}><summary>New experiment proposal</summary><Form key={`new-experiment-${workspace.revision}`} submit="Create experiment" busy={busy} onSubmit={async data => {
      if (!await save({ type: 'create_experiment', opportunity_id: text(data, 'opportunity') || null, proposal: proposalFrom(data) })) return
      const created = latestWorkspace().experiments.at(-1)
      if (created) selectExperiment(created.id)
    }}>
      <Select name="opportunity" label="Chosen opportunity" options={[{ value: '', label: 'Independent experiment' }, ...chosen.map(o => ({ value: o.id, label: o.evidence.title }))]}/><ProposalFields/>
    </Form></details>}</Panel>
  </>
}
