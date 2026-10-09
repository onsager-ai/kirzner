import { useWorkspace } from '../store'
import { Button } from '../components/ui/button'
import { Empty, Field, Form, Panel, Select, Status, text } from '../components/forms'
import type { Experiment } from '../types'
function ResultCard({ experiment }: { experiment: Experiment }) {
  const { workspace, save, busy } = useWorkspace()
  const current = experiment.proposals.at(-1)!
  const approved = experiment.decisions.filter(d => d.version === current.version).at(-1)?.approved
  return <Panel title={`${experiment.id}: ${current.content.action}`}>
    <p className="muted">Perform outreach or publication outside Kirzner. Record what actually happened with a reference you can revisit.</p>
    {approved ? <details open={experiment.actions.length === 0}><summary>Record an actual action · proposal v{current.version}</summary><Form submit="Record actual action" busy={busy} onSubmit={data => save({ type: 'record_action', experiment_id: experiment.id, version: current.version, reference: text(data, 'reference'), note: text(data, 'note'), self_prepared: data.has('self_prepared') })}>
      <Field name="reference" label="Actual action reference" hint="A sent-message ID, published URL, or your own dated action log."/><Field name="note" label="What actually happened"/>
      <label className="checkbox wide"><input type="checkbox" name="self_prepared"/>I prepared the materials myself, without an agent return.</label>
    </Form></details> : <p className="finding">Approve the current proposal version before recording an action.</p>}
    {experiment.actions.map(a => <div className="finding" key={a.id}><Status>Action recorded · v{a.version}</Status><p><strong>{a.reference}</strong></p><p>{a.note}</p><small>{a.self_prepared ? 'Materials prepared by founder' : 'Returned materials accepted'}</small></div>)}
    {experiment.actions.length > 0 && <div className="subpanel"><h3>Record real observations</h3><Form submit="Record business observation" busy={busy} onSubmit={data => save({ type: 'record_observation', experiment_id: experiment.id, action_id: text(data, 'action'), outcome: text(data, 'outcome'), evidence: text(data, 'evidence'), comparison: text(data, 'comparison'), learning: text(data, 'learning') })}>
      <Select name="action" label="Observed action" options={experiment.actions.map(a => ({ value: a.id, label: `${a.id} · proposal v${a.version} · ${a.reference}` }))}/>
      <Select name="outcome" label="Observed business outcome" options={[{ value: 'inconclusive', label: 'Inconclusive' }, { value: 'success', label: 'Success' }, { value: 'failure', label: 'Failure' }]}/>
      <Field name="evidence" label="Actual observations and evidence" hint="Report measured facts. Missing data is not a positive result."/><Field name="comparison" label="Comparison with original criteria"/><Field name="learning" label="Proposed learning" hint="This will enter future context only if you explicitly adopt it."/>
    </Form></div>}
    {experiment.observations.map(o => {
      const original = experiment.proposals.find(p => p.version === o.version)!
      const adopted = workspace.adopted_learning.some(l => l.experiment_id === experiment.id && l.observation_id === o.id)
      return <article className="receipt" key={o.id}><div className="row"><h3>Observation {o.id}</h3><Status tone={o.outcome === 'success' ? 'good' : 'amber'}>{o.outcome}</Status></div>
        <div className="comparison"><div><span className="eyebrow">Original judgment · v{o.version}</span><h3>{original.content.action}</h3><p><strong>Window:</strong> {original.content.observation_window}</p><p><strong>Success:</strong> {original.content.success_criteria}</p><p><strong>Failure:</strong> {original.content.failure_criteria}</p><p><strong>Inconclusive:</strong> {original.content.inconclusive_criteria}</p></div><div><span className="eyebrow">Actual observation</span><p>{o.evidence}</p><p><strong>Comparison:</strong> {o.comparison}</p><p><strong>Learning:</strong> {o.learning}</p></div></div>
        <Form submit="Record next decision" busy={busy} onSubmit={data => save({ type: 'decide_next', experiment_id: experiment.id, observation_id: o.id, choice: text(data, 'choice'), reason: text(data, 'reason') })}><Select name="choice" label="Next step" options={[{ value: 'continue', label: 'Continue' }, { value: 'adjust', label: 'Adjust' }, { value: 'stop', label: 'Stop' }]}/><Field name="reason" label="Next decision reason"/></Form>
        {experiment.next_steps.filter(n => n.observation_id === o.id).map((n, i) => <p key={i}><strong>{n.choice}</strong> · {n.reason}</p>)}
        <div className="row"><p className="muted">Adopt this learning into future business context?</p><Button disabled={adopted || busy} variant="outline" onClick={() => void save({ type: 'adopt_learning', experiment_id: experiment.id, observation_id: o.id })}>{adopted ? 'Learning adopted' : 'Adopt learning'}</Button></div>
      </article>
    })}
  </Panel>
}
export function ResultsPage() {
  const { workspace } = useWorkspace()
  return <><div className="page-heading"><span className="eyebrow">05 / Observe and decide</span><h1>Keep the judgment and the result.</h1><p>Accepted materials are a starting point. Real observations tell you what to do next.</p></div>
    {workspace.experiments.length === 0 ? <Empty title="No experiment results yet">Create a small experiment with written criteria. A negative or inconclusive result can still be useful.</Empty> : workspace.experiments.map(e => <ResultCard key={e.id} experiment={e}/>)}
  </>
}
