import { useEffect, useState } from 'react'
import { Link } from '@tanstack/react-router'
import { useWorkspace } from '../store'
import { Field, Form, Panel, SourceEditor, Status, text } from '../components/forms'
import type { Brief } from '../types'

const fields: [keyof Omit<Brief, 'materials'>, string, string, boolean][] = [
  ['product', 'Product', 'What are you building?', true],
  ['customer', 'Target customer', 'Who do you hope will use or pay for it?', false],
  ['capabilities', 'Verified capabilities', 'What works today, with evidence?', false],
  ['objective', 'Current business objective', 'One result worth working toward.', true],
  ['time_budget', 'Available time', 'Hours and deadline, if known.', false],
  ['money_budget', 'Available budget', 'A limit, if known. Blank means unknown, not zero.', false],
  ['channels', 'Available channels', 'Where can you reach customers, if known?', false],
  ['constraints', 'Resource constraints', 'Time, money, authority, or other limits for this work.', true],
]

function optionalValue(value?: string) { return value?.trim() || 'Unknown until supplied' }

export function BriefPage() {
  const { workspace, save, busy } = useWorkspace()
  const [materials, setMaterials] = useState(workspace.brief?.materials || [])
  const brief = workspace.brief
  const briefSignature = brief ? JSON.stringify(brief) : ''
  useEffect(() => setMaterials(workspace.brief?.materials || []), [briefSignature])
  const selectedId = typeof window === 'undefined' ? null : window.localStorage.getItem(`kirzner:selected-experiment:${workspace.id}`)
  const currentExperiment = (selectedId && workspace.experiments.find(e => e.id === selectedId)) || workspace.experiments.at(-1)
  const recentResearch = [...workspace.attempts].reverse().find(a => a.kind === 'research')
  const saved = (data: FormData): Brief => ({
    product: text(data, 'product'),
    objective: text(data, 'objective'),
    constraints: text(data, 'constraints'),
    customer: text(data, 'customer'),
    capabilities: text(data, 'capabilities'),
    time_budget: text(data, 'time_budget'),
    money_budget: text(data, 'money_budget'),
    channels: text(data, 'channels'),
    materials,
  })

  return <>
    <div className="page-heading"><span className="eyebrow">01 / Business context</span><h1>{brief ? 'Pick up where you left off.' : 'Start with what you know.'}</h1><p>{brief ? 'Your business context and current work are saved in this local workspace.' : 'Set the product, the current objective, and the limits your existing agent must respect.'}</p></div>
    {brief && <Panel title="Your business at a glance">
      <dl className="evidence-grid">
        <div><dt>Product</dt><dd>{brief.product}</dd></div>
        <div><dt>Current business objective</dt><dd>{brief.objective}</dd></div>
        <div className="wide"><dt>Resource constraints</dt><dd>{brief.constraints}</dd></div>
        <div><dt>Target customer</dt><dd>{optionalValue(brief.customer)}</dd></div>
        <div><dt>Verified capabilities</dt><dd>{optionalValue(brief.capabilities)}</dd></div>
        <div><dt>Available time</dt><dd>{optionalValue(brief.time_budget)}</dd></div>
        <div><dt>Available budget</dt><dd>{optionalValue(brief.money_budget)}</dd></div>
        <div><dt>Available channels</dt><dd>{optionalValue(brief.channels)}</dd></div>
      </dl>
      <div className="row">
        {currentExperiment ? <a className="button button-primary" href={`/experiments?experiment=${encodeURIComponent(currentExperiment.id)}`}>Resume current experiment</a> : <Link className="button button-primary" to="/opportunities">Start opportunity research</Link>}
        {recentResearch ? <a className="button button-outline" href={`/handoffs?attempt=${encodeURIComponent(recentResearch.id)}`}>Resume research handoff · {recentResearch.returns.length ? 'return ready' : 'awaiting return'}</a> : <Link className="button button-outline" to="/opportunities">Request opportunity research</Link>}
      </div>
      {brief.materials.length === 0 && <p className="muted">Additional business details and source material are still unknown. Add them only when you can support them.</p>}
    </Panel>}

    <Panel title={brief ? 'Edit business brief' : 'Your first business brief'}>
      {!brief && <p className="muted">The three required fields are enough to begin. Leave optional details blank when you do not know them yet.</p>}
      <Form key={`brief-${briefSignature}`} submit="Save business brief" busy={busy} onSubmit={data => save({ type: 'save_brief', brief: saved(data) })}>
        {fields.slice(0, 1).map(([key, label, hint, required]) => <Field key={key} name={key} label={label} hint={hint} required={required} value={brief?.[key] || ''}/>) }
        {fields.slice(3, 4).map(([key, label, hint, required]) => <Field key={key} name={key} label={label} hint={hint} required={required} value={brief?.[key] || ''}/>) }
        {fields.slice(7, 8).map(([key, label, hint, required]) => <Field key={key} name={key} label={label} hint={hint} required={required} value={brief?.[key] || ''}/>) }
        <details className="wide"><summary>Optional business details · blank values stay unknown</summary><div className="form-grid">
          {fields.filter(([key]) => !['product', 'objective', 'constraints'].includes(key)).map(([key, label, hint]) => <Field key={key} name={key} label={label} hint={`${hint} Leave blank to keep this unknown.`} required={false} value={brief?.[key] || ''}/>) }
          <SourceEditor sources={materials} onChange={setMaterials}/>
        </div></details>
      </Form>
    </Panel>

    {workspace.adopted_learning.length > 0 && <Panel title="Learning adopted into future context"><p className="muted">You explicitly chose to carry these findings into future handoffs.</p>{workspace.adopted_learning.map(l => <div className="finding" key={`${l.experiment_id}-${l.observation_id}`}><Status tone="good">Adopted</Status><p>{l.text}</p></div>)}</Panel>}
    <p className="footnote">{workspace.brief_history.length} saved brief revisions · Local workspace · No subscription required</p>
  </>
}
