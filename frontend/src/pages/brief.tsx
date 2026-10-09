import { useState } from 'react'
import { useWorkspace } from '../store'
import { Field, Form, Panel, SourceEditor, text, Status } from '../components/forms'
import type { Brief } from '../types'
const fields: [keyof Omit<Brief, 'materials'>, string, string][] = [
  ['product', 'Product', 'What are you building?'], ['customer', 'Target customer', 'Be specific about who needs it.'],
  ['capabilities', 'Verified capabilities', 'What works today, with evidence?'], ['objective', 'Current business objective', 'One result worth working toward.'],
  ['time_budget', 'Available time', 'Hours and deadline.'], ['money_budget', 'Available budget', 'A clear upper limit, including zero.'],
  ['channels', 'Available channels', 'Where can you reach customers?'], ['constraints', 'Constraints', 'What is outside the scope?'],
]
export function BriefPage() {
  const { workspace, save, busy } = useWorkspace()
  const [materials, setMaterials] = useState(workspace.brief?.materials || [])
  return <><div className="page-heading"><span className="eyebrow">01 / Business context</span><h1>Start with what you know.</h1><p>A clear brief gives your existing agent the context to find opportunities worth testing.</p></div>
    <Panel title="Your business brief"><Form submit="Save business brief" busy={busy} onSubmit={data => save({ type: 'save_brief', brief: { ...Object.fromEntries(fields.map(([key]) => [key, text(data, key)])), materials } })}>
      {fields.map(([key, label, hint]) => <Field key={key} name={key} label={label} hint={hint} value={workspace.brief?.[key]}/>) }
      <SourceEditor sources={materials} onChange={setMaterials}/>
    </Form></Panel>
    {workspace.adopted_learning.length > 0 && <Panel title="Learning adopted into future context"><p className="muted">You explicitly chose to carry these findings into future handoffs.</p>{workspace.adopted_learning.map(l => <div className="finding" key={`${l.experiment_id}-${l.observation_id}`}><Status tone="good">Adopted</Status><p>{l.text}</p></div>)}</Panel>}
    <p className="footnote">{workspace.brief_history.length} saved brief revisions · Local workspace · No subscription required</p>
  </>
}
