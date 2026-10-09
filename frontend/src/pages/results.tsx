import { useWorkspace } from '../store'
import { Field, Form, Panel, Select, Status, text } from '../components/forms'
import type { Experiment, Workspace } from '../types'

function hasUnknownEffects(experimentId: string, workspace: Workspace) {
  const experiment = workspace.experiments.find(e => e.id === experimentId)
  const sourceAttemptId = experiment?.opportunity_id ? workspace.opportunities.find(o => o.id === experiment.opportunity_id)?.attempt_id : undefined
  return workspace.attempts.filter(a => a.experiment_id === experimentId || a.id === sourceAttemptId).some(attempt => attempt.returns.some(receipt => receipt.input.external_effects === 'unknown' && !attempt.reconciliations.some(r => r.return_id === receipt.input.return_id)))
}

export function ResultCard({ experiment, embedded = false }: { experiment: Experiment; embedded?: boolean }) {
  const { workspace, save, busy } = useWorkspace()
  const current = experiment.proposals.at(-1)!
  const decision = experiment.decisions.filter(d => d.version === current.version).at(-1)
  const currentApproved = decision?.approved === true
  const unknownEffects = hasUnknownEffects(experiment.id, workspace)
  const historicalVersions = experiment.proposals.filter(p => p.version < current.version && experiment.decisions.some(d => d.version === p.version && d.approved))
  const currentAction = [...experiment.actions].reverse().find(a => a.version === current.version)
  const currentAttempt = [...workspace.attempts].reverse().find(a => a.experiment_id === experiment.id && a.proposal_version === current.version)
  const acceptedMaterials = currentAttempt ? currentAttempt.returns.some(receipt => receipt.input.deliverables.length > 0 && [...currentAttempt.reviews].reverse().find(review => review.return_id === receipt.input.return_id)?.accepted === true) : false
  const needsSelfPrepare = Boolean(!currentAction && currentAttempt?.returns.length && !acceptedMaterials)
  const currentActionDecision = currentAction && experiment.observations.filter(o => o.action_id === currentAction.id).flatMap(o => experiment.next_steps.filter(n => n.observation_id === o.id)).at(-1)
  const currentActionClosed = currentActionDecision?.choice === 'adjust' || currentActionDecision?.choice === 'stop'
  const unobservedActions = experiment.actions.filter(a => !experiment.observations.some(o => o.action_id === a.id))

  return <Panel title="Actual actions and observations" eyebrow={embedded ? 'Keep action and result tied to their original proposal' : undefined}>
    <p className="muted">Record only what happened outside Kirzner. Each observation stays attached to the proposal version that guided that action.</p>
    {unknownEffects && <p className="finding" role="status"><Status tone="amber">Reconcile effects first</Status><br/>An attempt reports an unknown external effect. Review and reconcile it before recording another action or preparing more work.</p>}
    {!currentApproved && !unknownEffects && <p className="finding">Approve the current proposal version before recording a new action. An older approval does not authorize the current version.</p>}
    {needsSelfPrepare && !unknownEffects && <p className="finding">The current agent return does not establish accepted materials. Review a deliverable, or choose “I prepared the materials myself” only after preparing them.</p>}
    {currentActionClosed && currentActionDecision && <p className="finding"><strong>Recorded next decision: {currentActionDecision.choice}</strong><br/>{currentActionDecision.choice === 'adjust' ? 'Revise the proposal to reflect what you learned, then make a separate decision on the new version.' : 'This experiment is stopped. Keep its evidence and version history for future reference.'}</p>}
    {currentApproved && !unknownEffects && !currentActionClosed && <details open={experiment.actions.every(a => a.version !== current.version)}><summary>Record an actual action · proposal v{current.version}</summary><p className="muted">A reference is your record of the completed action. Kirzner does not send or verify it.</p><Form key={`action-${workspace.revision}-${experiment.id}`} submit="Record actual action" busy={busy} onSubmit={data => save({ type: 'record_action', experiment_id: experiment.id, version: current.version, reference: text(data, 'reference'), note: text(data, 'note'), self_prepared: text(data, 'materials') === 'self' })}>
      <Field name="reference" label="Actual action reference" hint="A sent-message ID, published URL, or your own dated action log."/><Field name="note" label="What actually happened"/>
      <Select name="materials" label="Material source" required options={[{ value: '', label: 'Choose how the materials were prepared' }, { value: 'accepted', label: 'Agent materials accepted in Kirzner' }, { value: 'self', label: 'I prepared the materials myself' }]}/>
    </Form></details>}
    {historicalVersions.length > 0 && <details><summary>Record a completed action for an older version</summary><p className="muted">Use this only for an action that already happened while that older version was approved. The completion date is founder-supplied evidence; it does not create current approval or a handoff.</p>{historicalVersions.map(proposal => <section className="subpanel" key={proposal.version}>
      <h3>Proposal version {proposal.version} · {proposal.content.action}</h3>
      <Form key={`historical-${workspace.revision}-${proposal.version}`} submit="Record historical action" busy={busy} onSubmit={data => save({ type: 'record_historical_action', experiment_id: experiment.id, version: proposal.version, reference: text(data, 'reference'), note: text(data, 'note'), self_prepared: text(data, 'materials') === 'self', completed_at: Math.floor(new Date(text(data, 'completed_at')).getTime() / 1000) })}>
        <label className="field"><span>Completion date and time</span><input aria-label="Completion date and time" type="datetime-local" name="completed_at" required/></label>
        <Field name="reference" label="Historical action reference"/>
        <Field name="note" label="What happened then"/>
        <Select name="materials" label="Material source" required options={[{ value: '', label: 'Choose how the materials were prepared' }, { value: 'accepted', label: 'Accepted materials for this version' }, { value: 'self', label: 'I prepared the materials myself' }]}/>
      </Form>
    </section>)}</details>}
    {experiment.actions.map(action => <div className="finding" key={action.id}>
      <div className="row"><Status tone={action.historical ? 'amber' : 'good'}>{action.historical ? 'Historical action' : 'Action recorded'} · proposal v{action.version}</Status>{action.historical && action.completed_at && <span className="muted">Completed {new Date(action.completed_at * 1000).toLocaleString()}</span>}</div>
      <p><strong>{action.reference}</strong></p><p>{action.note}</p><small>{action.self_prepared ? 'Materials prepared by founder' : 'Accepted materials recorded for this version'}</small>
    </div>)}
    {unobservedActions.length > 0 && <div className="subpanel"><h3>Record a business observation</h3><p className="muted">Enter actual observations and evidence. Missing information stays unknown; it is not a positive result.</p><Form key={`observation-${workspace.revision}-${experiment.id}`} submit="Record business observation" busy={busy} onSubmit={data => save({ type: 'record_observation', experiment_id: experiment.id, action_id: text(data, 'action'), outcome: text(data, 'outcome'), evidence: text(data, 'evidence'), comparison: text(data, 'comparison'), learning: text(data, 'learning') })}>
      <Select name="action" label="Observed action" value="" required options={[{ value: '', label: 'Choose an action' }, ...unobservedActions.map(a => ({ value: a.id, label: `Proposal v${a.version} · ${a.reference}${a.historical ? ' · historical' : ''}` }))]}/>
      <Select name="outcome" label="Observed business outcome" value="" required options={[{ value: '', label: 'Choose an outcome' }, { value: 'inconclusive', label: 'Inconclusive' }, { value: 'success', label: 'Success' }, { value: 'failure', label: 'Failure' }]}/>
      <Field name="evidence" label="Actual observations and evidence" hint="Report measured facts. Missing data is not a positive result."/><Field name="comparison" label="Comparison with original criteria"/><Field name="learning" label="Proposed learning" hint="This enters future context only if you explicitly adopt it."/>
    </Form></div>}
    {experiment.observations.map(observation => {
      const original = experiment.proposals.find(p => p.version === observation.version)
      const action = experiment.actions.find(a => a.id === observation.action_id)
      const adopted = workspace.adopted_learning.some(l => l.experiment_id === experiment.id && l.observation_id === observation.id)
      const next = experiment.next_steps.filter(n => n.observation_id === observation.id)
      return <article className="receipt" key={observation.id}><div className="row"><h3>Observation for proposal v{observation.version}</h3><Status tone={observation.outcome === 'success' ? 'good' : 'amber'}>{observation.outcome}</Status></div>
        {observation.version !== current.version && <p className="finding">This observation belongs to version {observation.version}. The current proposal is version {current.version}; these findings do not approve or describe the current version.</p>}
        <div className="comparison"><div><span className="eyebrow">Original judgment · v{observation.version}</span><h3>{original?.content.action || action?.reference || 'Original proposal'}</h3><p><strong>Window:</strong> {original?.content.observation_window}</p><p><strong>Deliverables:</strong> {original?.content.expected_deliverables}</p><p><strong>Resource limits:</strong> {original?.content.resource_limits}</p><p><strong>Preparation scope:</strong> {original?.content.authorization_scope}</p><p><strong>Success:</strong> {original?.content.success_criteria}</p><p><strong>Failure:</strong> {original?.content.failure_criteria}</p><p><strong>Inconclusive:</strong> {original?.content.inconclusive_criteria}</p></div><div><span className="eyebrow">Actual observation</span><p>{observation.evidence}</p><p><strong>Comparison:</strong> {observation.comparison}</p><p><strong>Learning:</strong> {observation.learning}</p></div></div>
        {next.length === 0 ? <Form key={`next-${workspace.revision}-${observation.id}`} submit="Record next decision" busy={busy} onSubmit={data => save({ type: 'decide_next', experiment_id: experiment.id, observation_id: observation.id, choice: text(data, 'choice'), reason: text(data, 'reason') })}><Select name="choice" label="Next step" value="" required options={[{ value: '', label: 'Choose a next step' }, { value: 'continue', label: 'Continue' }, { value: 'adjust', label: 'Adjust' }, { value: 'stop', label: 'Stop' }]}/><Field name="reason" label="Next decision reason"/></Form> : next.map((entry, i) => <div className="finding" key={i}><strong>Recorded next decision: {entry.choice}</strong><p>{entry.reason}</p>{entry.choice === 'adjust' && <p><a href={`/experiments?experiment=${encodeURIComponent(experiment.id)}`}>Revise the current proposal when ready →</a></p>}{entry.choice === 'stop' && <p className="muted">This version’s follow-up was stopped. Keep its evidence and version history for future reference.</p>}</div>)}
        <div className="row"><p className="muted">Adopt this learning into future business context?</p><button className="button button-outline" disabled={adopted || busy} onClick={() => void save({ type: 'adopt_learning', experiment_id: experiment.id, observation_id: observation.id })}>{adopted ? 'Learning adopted' : 'Adopt learning'}</button></div>
      </article>
    })}
  </Panel>
}

export function ResultsPage() {
  const { workspace } = useWorkspace()
  return <><div className="page-heading"><span className="eyebrow">05 / Observe and decide</span><h1>Keep the judgment and the result.</h1><p>Accepted materials are a starting point. Real observations tell you what to do next.</p></div>
    {workspace.experiments.length === 0 ? <div className="empty"><h3>No experiment results yet</h3><p>Create a small experiment with written criteria. A negative or inconclusive result can still be useful.</p></div> : workspace.experiments.map(experiment => <ResultCard key={experiment.id} experiment={experiment}/>)}</>
}
