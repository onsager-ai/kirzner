import { useWorkspace } from '../store'
import { Empty, Field, Form, Panel, Select, Sources, Status, text } from '../components/forms'
import { Link } from '@tanstack/react-router'
export function OpportunitiesPage() {
  const { workspace, save, busy } = useWorkspace()
  return <><div className="page-heading"><span className="eyebrow">02 / Discover</span><h1>Evidence before effort.</h1><p>Give your agent a focused research brief. A useful answer can be “nothing actionable yet.”</p></div>
    <Panel title="Request opportunity research"><p className="muted">Create a handoff, download it in Handoffs, and give it to your existing agent.</p>
      {!workspace.brief ? <p>Save your <Link to="/">business brief</Link> first.</p> : <Form submit="Create research handoff" busy={busy} onSubmit={data => save({ type: 'create_research', objective: text(data, 'objective'), context: text(data, 'context'), authorization_scope: text(data, 'scope') })}>
        <Field name="objective" label="Research objective" value={workspace.brief.objective}/><Field name="context" label="Selected context and previous findings" required={false} hint="The current brief, previous observations, and adopted learning are also included."/>
        <Field name="scope" label="Research authorization scope" value="Read public sources and prepare findings only. Do not contact prospects, publish, purchase, or change external systems."/>
      </Form>}
    </Panel>
    {workspace.opportunities.length === 0 ? <Empty title="No opportunities to review yet">Import an agent’s research return in Handoffs. Zero to three evidenced opportunities are enough.</Empty> : workspace.opportunities.map(o => <Panel key={o.id} title={o.evidence.title} eyebrow={`${o.id} · ${o.attempt_id}`}>
      <div className="row"><Status>{o.disposition}</Status><span className="muted">Return {o.return_id}</span></div>
      <div className="evidence-grid">{([['Customer relevance', o.evidence.customer_relevance], ['Why now', o.evidence.why_now], ['Counterevidence', o.evidence.counterevidence], ['Unknowns', o.evidence.unknowns], ['Cheap validation', o.evidence.validation_action]] as const).map(([label, value]) => <div key={label}><h3>{label}</h3><p>{value}</p></div>)}</div>
      <Sources sources={o.evidence.sources}/>
      <Form submit="Record opportunity decision" busy={busy} onSubmit={data => save({ type: 'decide_opportunity', opportunity_id: o.id, disposition: text(data, 'disposition'), reason: text(data, 'reason') })}>
        <Select name="disposition" label="What should happen next?" value={o.disposition === 'unreviewed' ? 'watch' : o.disposition} options={[{ value: 'choose', label: 'Choose for an experiment' }, { value: 'research', label: 'Request more research' }, { value: 'watch', label: 'Watch' }, { value: 'dismiss', label: 'Dismiss' }]}/>
        <Field name="reason" label="Decision reason"/>
      </Form>
      {o.disposition === 'choose' && <p><Link to="/experiments">Create an experiment →</Link></p>}
      {o.disposition === 'research' && <p className="muted">Create a new research handoff above with the open questions as selected context.</p>}
      {o.decisions.length > 0 && <details><summary>Decision history</summary>{o.decisions.map((d, i) => <p key={i}><strong>{d.disposition}</strong> · {d.reason}</p>)}</details>}
    </Panel>)}
    {workspace.attempts.filter(a => a.kind === 'research').flatMap(a => a.returns.filter(r => r.input.opportunities.length === 0).map(r => <div className="finding" key={`${a.id}-${r.input.return_id}`}><Status>{r.input.status}</Status><strong>No actionable opportunity · {a.id}</strong><p>{r.input.no_opportunity_reason || r.input.summary}</p></div>))}
  </>
}
