import { Link } from '@tanstack/react-router'
import { Button } from '../components/ui/button'
import { Empty, Field, Form, Panel, Select, Sources, Status, text } from '../components/forms'
import { proposalFields } from '../components/proposal'
import { useWorkspace } from '../store'
import { AttemptCard } from './handoffs'

export function OpportunitiesPage() {
  const { workspace, save, latestWorkspace, busy } = useWorkspace()
  const latestResearch = [...workspace.attempts].reverse().find(attempt => attempt.kind === 'research')
  return <>
    <div className="page-heading"><span className="eyebrow">02 / Discover</span><h1>Evidence before effort.</h1><p>Give your agent a focused research brief. A useful answer can be “nothing actionable yet.”</p></div>
    <Panel title="Request opportunity research"><p className="muted">Create a task for your existing agent. Copy it and paste the return here; older tasks remain in Handoffs.</p>
      {!workspace.brief ? <p>Save your <Link to="/">business brief</Link> first.</p> : <Form key={`research-${workspace.revision}`} submit="Create research handoff" busy={busy} onSubmit={data => save({ type: 'create_research', objective: text(data, 'objective'), context: text(data, 'context'), authorization_scope: text(data, 'scope') })}>
        <Field name="objective" label="Research objective" value={workspace.brief.objective}/><Field name="context" label="Selected context and previous findings" required={false} hint="The current brief, previous observations, and adopted learning are also included."/>
        <Field name="scope" label="Research authorization scope" value="Read public sources and prepare findings only. Do not contact prospects, publish, purchase, or change external systems."/>
      </Form>}
    </Panel>
    {latestResearch && <section className="workspace-section"><div className="row"><div><span className="eyebrow">Latest research handoff</span><h2>Continue research here</h2></div><Link to="/handoffs">All handoffs and older tasks</Link></div><AttemptCard key={latestResearch.id} attempt={latestResearch}/></section>}
    {workspace.opportunities.length === 0 ? <Empty title="No opportunities to review yet">Import an agent’s research return in Handoffs. Zero to three evidenced opportunities are enough.</Empty> : workspace.opportunities.map(o => {
      const linkedExperiment = workspace.experiments.find(e => e.opportunity_id === o.id)
      return <Panel key={o.id} title={o.evidence.title} eyebrow="Evidence review">
        <div className="row"><Status>{o.disposition}</Status><span className="muted">Source material is preserved with its original return.</span></div>
        <div className="evidence-grid">{([['Customer relevance', o.evidence.customer_relevance], ['Why now', o.evidence.why_now], ['Counterevidence', o.evidence.counterevidence], ['Unknowns', o.evidence.unknowns], ['Cheap validation', o.evidence.validation_action]] as const).map(([label, value]) => <div key={label}><h3>{label}</h3><p>{value}</p></div>)}</div>
        <Sources sources={o.evidence.sources}/>
        {o.evidence.experiment_draft && <section className="subpanel draft-review" aria-label="Proposed experiment draft">
          <h3>Proposed experiment draft</h3>
          <p className="muted">Review the full bounded proposal. Using it saves an unapproved first version linked to this opportunity; it does not authorize preparation or external action.</p>
          <dl className="evidence-grid">{proposalFields.map(([key, label]) => <div key={key}><dt>{label}</dt><dd>{o.evidence.experiment_draft?.[key]}</dd></div>)}</dl>
          {linkedExperiment && <p className="finding">This opportunity already has an experiment. Using the draft again opens it without replacing its proposal or decision.</p>}
          <Button disabled={busy} onClick={async () => {
            if (!await save({ type: 'create_draft_experiment', opportunity_id: o.id, proposal: null })) return
            const experiment = latestWorkspace().experiments.find(e => e.opportunity_id === o.id)
            if (experiment) window.location.assign(`/experiments?experiment=${encodeURIComponent(experiment.id)}`)
          }}>Use experiment draft</Button>
        </section>}
        <Form key={`opportunity-${o.id}-${workspace.revision}`} submit="Record opportunity decision" busy={busy} onSubmit={data => save({ type: 'decide_opportunity', opportunity_id: o.id, disposition: text(data, 'disposition'), reason: text(data, 'reason') })}>
          <Select name="disposition" label="What should happen next?" value={o.disposition === 'unreviewed' ? 'watch' : o.disposition} options={[{ value: 'choose', label: 'Choose for an experiment' }, { value: 'research', label: 'Request more research' }, { value: 'watch', label: 'Watch' }, { value: 'dismiss', label: 'Dismiss' }]}/>
          <Field name="reason" label="Decision reason"/>
        </Form>
        {linkedExperiment && <p><a href={`/experiments?experiment=${encodeURIComponent(linkedExperiment.id)}`}>Open linked experiment workspace →</a></p>}
        {!linkedExperiment && o.disposition === 'choose' && <p><Link to="/experiments">Create an experiment →</Link></p>}
        {o.disposition === 'research' && <p className="muted">Create a new research handoff above with the open questions as selected context.</p>}
        {o.decisions.length > 0 && <details><summary>Decision history</summary>{o.decisions.map((d, i) => <p key={i}><strong>{d.disposition}</strong> · {d.reason}</p>)}</details>}
      </Panel>
    })}
    {workspace.attempts.filter(a => a.kind === 'research').flatMap(a => a.returns.filter(r => r.input.opportunities.length === 0).map(r => {
      const explicitNoOpportunity = r.input.status === 'succeeded' && Boolean(r.input.no_opportunity_reason?.trim())
      const title = explicitNoOpportunity ? 'No actionable opportunity' : r.input.status === 'unknown' ? 'Research outcome unknown' : r.input.status === 'partial' ? 'Research return incomplete' : r.input.status === 'failed' ? 'Research failed' : 'Research result needs review'
      return <div className="finding" key={`${a.id}-${r.input.return_id}`}><Status>{r.input.status}</Status><strong>{title}</strong>{r.input.no_opportunity_reason && <p><strong>Reported reason:</strong> {r.input.no_opportunity_reason}</p>}{r.input.summary && <p>{r.input.summary}</p>}</div>
    }))}
  </>
}
