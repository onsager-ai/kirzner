import { useState } from 'react'
import { useWorkspace } from '../store'
import { Button } from '../components/ui/button'
import { Empty, Field, Form, JsonDetails, Panel, Select, SourceEditor, Sources, Status, text } from '../components/forms'
import type { Attempt, Deliverable, OpportunityInput, Source } from '../types'
function OpportunityEditor({ index, onChange, remove }: { index: number; onChange: (opportunity: OpportunityInput) => void; remove: () => void }) {
  const [value, setValue] = useState<OpportunityInput>({ title: '', customer_relevance: '', why_now: '', counterevidence: '', unknowns: '', validation_action: '', sources: [] })
  function update(next: OpportunityInput) { setValue(next); onChange(next) }
  const fields: [keyof Omit<OpportunityInput, 'sources'>, string][] = [['title', 'Opportunity title'], ['customer_relevance', 'Customer relevance'], ['why_now', 'Why now'], ['counterevidence', 'Counterevidence'], ['unknowns', 'Unknowns'], ['validation_action', 'Cheap validation action']]
  return <fieldset className="wide subpanel"><legend>Opportunity {index + 1}</legend><div className="form-grid">{fields.map(([key, label]) => <label key={key} className="field"><span>{label}</span><textarea required rows={2} value={value[key]} onChange={e => update({ ...value, [key]: e.target.value })}/></label>)}<SourceEditor sources={value.sources} onChange={(sources: Source[]) => update({ ...value, sources })}/></div><Button type="button" variant="ghost" onClick={remove}>Remove opportunity</Button></fieldset>
}
function ReturnEditor({ attempt }: { attempt: Attempt }) {
  const { save, busy } = useWorkspace()
  const [opportunities, setOpportunities] = useState<{ key: number; value: OpportunityInput | null }[]>([])
  const [counter, setCounter] = useState(0)
  const [deliverables, setDeliverables] = useState<Deliverable[]>([])
  const [fileError, setFileError] = useState('')
  const [uploading, setUploading] = useState(false)
  async function upload(file?: File) {
    if (!file) return
    setFileError(''); setUploading(true)
    try {
      const response = await fetch(`/api/files?name=${encodeURIComponent(file.name)}`, { method: 'POST', headers: { 'Content-Type': file.type || 'application/octet-stream' }, body: file })
      const result = await response.json()
      if (!response.ok) throw new Error(result.error || 'File upload failed')
      setDeliverables(current => [...current, result])
    } catch (e) { setFileError(e instanceof Error ? e.message : 'File upload failed') } finally { setUploading(false) }
  }
  return <div className="subpanel"><h3>Record an agent return</h3><p className="muted">Report what happened, including failed or partial work. A return does not accept the materials or record a business outcome.</p>{fileError && <p role="alert" className="error">{fileError}</p>}
    <Form submit="Import return" busy={busy || uploading} onSubmit={data => save({ type: 'import_return', payload: {
      attempt_id: attempt.id, return_id: text(data, 'return_id'), status: text(data, 'status'), external_effects: text(data, 'effects'), summary: text(data, 'summary'),
      deliverables, opportunities: opportunities.map(o => o.value).filter(Boolean), no_opportunity_reason: text(data, 'no_reason') || null, execution_refs: [],
    }, original_text: null })}>
      <Field name="return_id" label="Stable return ID" short hint="Use a new ID for changed output; reuse the same ID for an identical re-import."/>
      <Select name="status" label="Reported execution status" options={['succeeded', 'partial', 'failed', 'unknown'].map(value => ({ value, label: value }))}/>
      <Select name="effects" label="External effects" options={[{ value: 'none', label: 'No external effects' }, { value: 'confirmed', label: 'Confirmed effects (explain in summary)' }, { value: 'unknown', label: 'Unknown — needs reconciliation' }]}/>
      <Field name="summary" label="Return summary"/>
      {attempt.kind === 'research' && <>
        <Field name="no_reason" label="Reason for no actionable opportunity" required={false} hint="Required for a successful research return with zero opportunities."/>
        {opportunities.map((entry, index) => <OpportunityEditor index={index} key={entry.key} onChange={value => setOpportunities(current => current.map(o => o.key === entry.key ? { ...o, value } : o))} remove={() => setOpportunities(current => current.filter(o => o.key !== entry.key))}/>)}
        <div className="wide"><Button type="button" variant="outline" disabled={opportunities.length >= 3} onClick={() => { setOpportunities([...opportunities, { key: counter, value: null }]); setCounter(counter + 1) }}>Add opportunity</Button><small className="muted"> 0–3 opportunities</small></div>
      </>}
      <div className="wide"><div className="row"><strong>Returned deliverables</strong><Button type="button" variant="outline" size="sm" onClick={() => setDeliverables([...deliverables, { title: '', reference: '', media_type: 'text/markdown', note: '' }])}>Add deliverable reference</Button></div>
        {deliverables.map((d, i) => <div className="source-fields" key={i}>{([['title', 'Deliverable title'], ['reference', 'Deliverable reference'], ['media_type', 'Media type'], ['note', 'Deliverable note']] as const).map(([key, label]) => <label key={key} className="field"><span>{label}</span><input required={key !== 'note'} value={d[key]} onChange={e => setDeliverables(deliverables.map((v, j) => i === j ? { ...v, [key]: e.target.value } : v))}/></label>)}<Button type="button" variant="ghost" onClick={() => setDeliverables(deliverables.filter((_, j) => j !== i))}>Remove deliverable</Button></div>)}
        <label className="file-picker">Upload a deliverable file<input type="file" disabled={uploading} onChange={e => void upload(e.target.files?.[0])}/></label><small className="muted">Up to 5 MB per file. Files stay in your local data directory.</small>
      </div>
    </Form>
  </div>
}
function ExecutionEditor({ attempt }: { attempt: Attempt }) {
  const { save, busy } = useWorkspace()
  return <details><summary>Attach an explicit execution reference</summary><p className="muted">Copy the native session ID from your executor or Semon. Each attempt may reference multiple sessions. These references are recorded as supplied, without inferred correlation.</p>
    <Form submit="Attach execution reference" busy={busy} onSubmit={data => save({ type: 'attach_execution', attempt_id: attempt.id, execution_ref: {
      namespace: text(data, 'namespace'), harness: text(data, 'harness'), session_id: text(data, 'session'), machine: text(data, 'machine') || null, source_revision: text(data, 'revision'), continuation: text(data, 'continuation') || null,
      segment: text(data, 'start') || text(data, 'end') ? { start_offset: Number(text(data, 'start')), end_offset: Number(text(data, 'end')) } : null,
    } })}>
      <Field name="namespace" label="Semon instance or source namespace" short/><Select name="harness" label="Harness" options={['codex', 'claude', 'copilot'].map(value => ({ value, label: value }))}/>
      <Field name="session" label="Native session ID" short/><Field name="machine" label="Machine ID (if applicable)" short required={false}/>
      <Field name="revision" label="Provenance or revision" short hint="For example, Semon revision and the operator’s source manifest reference."/><Field name="continuation" label="Native continuation reference" short required={false}/>
      <Field name="start" label="Segment start byte offset" short required={false}/><Field name="end" label="Segment end byte offset" short required={false}/>
    </Form>
  </details>
}
function AttemptCard({ attempt }: { attempt: Attempt }) {
  const { save, busy } = useWorkspace()
  return <Panel title={`${attempt.kind === 'research' ? 'Opportunity research' : 'Experiment preparation'} · ${attempt.id}`} eyebrow={attempt.proposal_version ? `${attempt.experiment_id} · proposal v${attempt.proposal_version}` : 'Research can stand alone'}>
    <div className="row"><Status>{attempt.returns.length ? `${attempt.returns.length} returns` : 'Awaiting return'}</Status><div className="button-group"><Button asChild variant="outline" size="sm"><a href={`/api/handoffs/${attempt.id}`}>Download task brief</a></Button><Button asChild variant="outline" size="sm"><a href={`/api/handoffs/${attempt.id}/continuation`}>Download continuation brief</a></Button></div></div>
    <JsonDetails title="Preserved input snapshot" value={attempt.snapshot}/>
    {attempt.execution_refs.length > 0 && <div className="finding"><h3>Explicit execution references</h3>{attempt.execution_refs.map((r, i) => <p key={i}>{r.namespace} / {r.harness} / <code>{r.session_id}</code>{r.machine && ` · machine ${r.machine}`}{r.segment && ` · bytes ${r.segment.start_offset}–${r.segment.end_offset}`}<br/><small>{r.source_revision}{r.continuation && ` · continuation: ${r.continuation}`}</small></p>)}</div>}
    <ExecutionEditor attempt={attempt}/>
    <details open={attempt.returns.length === 0}><summary>Enter a return using the form</summary><ReturnEditor attempt={attempt}/></details>
    {attempt.returns.map(receipt => {
      const result = receipt.input
      const review = attempt.reviews.filter(r => r.return_id === result.return_id).at(-1)
      const reconciliation = attempt.reconciliations.filter(r => r.return_id === result.return_id).at(-1)
      return <article className="receipt" key={result.return_id}><h3>Return {result.return_id}</h3><div className="status-grid"><div><small>Reported execution</small><Status>{result.status}</Status></div><div><small>Deliverable acceptance</small><Status tone={review?.accepted ? 'good' : 'amber'}>{review ? review.accepted ? 'Accepted' : 'Changes requested' : 'Unreviewed'}</Status></div><div><small>External effects</small><Status tone={result.external_effects === 'unknown' && !reconciliation ? 'amber' : ''}>{reconciliation ? `${reconciliation.external_effects} (reconciled)` : result.external_effects}</Status></div></div>
        <p>{result.summary}</p>{result.deliverables.map((d, i) => <div className="deliverable" key={i}><a href={d.reference.startsWith('files/') ? `/${d.reference}` : d.reference} target="_blank" rel="noreferrer">{d.title} ↗</a><small>{d.media_type} · {d.note}</small></div>)}
        {result.opportunities.map((o, i) => <div key={i}><strong>{o.title}</strong><Sources sources={o.sources}/></div>)}
        {result.deliverables.length > 0 && <Form submit="Record deliverable review" busy={busy} onSubmit={data => save({ type: 'review_return', attempt_id: attempt.id, return_id: result.return_id, accepted: text(data, 'review') === 'accept', note: text(data, 'note') })}>
          <Select name="review" label="Deliverable review" options={[{ value: 'accept', label: 'Accept materials' }, { value: 'changes', label: 'Request changes' }]}/><Field name="note" label="Review note"/>
        </Form>}
        {result.external_effects === 'unknown' && <details open={!reconciliation}><summary>Reconcile unknown external effects</summary><Form submit="Record reconciliation" busy={busy} onSubmit={data => save({ type: 'reconcile_effects', attempt_id: attempt.id, return_id: result.return_id, external_effects: text(data, 'effects'), evidence: text(data, 'evidence') })}><Select name="effects" label="Verified external effects" options={[{ value: 'none', label: 'No effect occurred' }, { value: 'confirmed', label: 'Effect confirmed' }]}/><Field name="evidence" label="Reconciliation evidence"/></Form></details>}
        {attempt.reviews.filter(r => r.return_id === result.return_id).map((r, i) => <p className="muted" key={i}>{r.accepted ? 'Accepted' : 'Changes requested'}: {r.note}</p>)}
        {attempt.reconciliations.filter(r => r.return_id === result.return_id).map((r, i) => <p className="muted" key={i}>Reconciled as {r.external_effects}: {r.evidence}</p>)}
        <details className="technical"><summary>Original return and digest</summary><code>{receipt.digest}</code><pre>{receipt.original_text}</pre></details>
      </article>
    })}
  </Panel>
}
export function HandoffsPage() {
  const { workspace, save } = useWorkspace()
  const [error, setError] = useState('')
  async function importFile(file?: File) {
    if (!file) return
    setError('')
    try { const original_text = await file.text(); await save({ type: 'import_return', payload: JSON.parse(original_text), original_text }) }
    catch (e) { setError(e instanceof Error ? e.message : 'Invalid return file') }
  }
  return <><div className="page-heading"><span className="eyebrow">04 / Delegate and review</span><h1>Your agent. A clear handoff.</h1><p>Download the brief, use your existing agent, then bring the results back here.</p></div>
    <div className="import-bar"><div><strong>Have a returned JSON file?</strong><p>Import the agent’s envelope. The original content is preserved.</p></div><label className="file-picker">Import return file<input type="file" accept="application/json,.json" onChange={e => void importFile(e.target.files?.[0])}/></label></div>
    {error && <p className="error" role="alert">{error}</p>}
    {workspace.return_conflicts.length > 0 && <Panel title="Conflicting returns need inspection">{workspace.return_conflicts.map((c, i) => <div className="finding error" key={i}><strong>{c.attempt_id} / {c.return_id}</strong><p>This identity arrived with different content. The original effective return was retained. Use a new return ID for an intentional revision.</p><JsonDetails title="Preserved conflicting content" value={c.original}/></div>)}</Panel>}
    {workspace.attempts.length === 0 ? <Empty title="Nothing handed off yet">Create a research brief in Opportunities or approve an experiment for preparation.</Empty> : [...workspace.attempts].reverse().map(a => <AttemptCard key={a.id} attempt={a}/>)}
  </>
}
