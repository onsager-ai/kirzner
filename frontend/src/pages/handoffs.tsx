import { useEffect, useState } from 'react'
import { Button } from '../components/ui/button'
import { Empty, Field, Form, JsonDetails, Panel, Select, SourceEditor, Sources, Status, text } from '../components/forms'
import { ProposalFields, proposalFields } from '../components/proposal'
import { useWorkspace } from '../store'
import type { Attempt, Brief, Deliverable, OpportunityInput, ProposalInput, Source } from '../types'

const briefFields: [keyof Omit<Brief, 'materials'>, string][] = [
  ['product', 'Product'], ['customer', 'Target customer'], ['capabilities', 'Verified capabilities'], ['objective', 'Current business objective'],
  ['time_budget', 'Available time'], ['money_budget', 'Available budget'], ['channels', 'Available channels'], ['constraints', 'Resource constraints'],
]
const emptyProposal = (): ProposalInput => ({ audience: '', action: '', expected_deliverables: '', observation_window: '', success_criteria: '', failure_criteria: '', inconclusive_criteria: '', resource_limits: '', authorization_scope: '' })

function OpportunityEditor({ index, onChange, remove }: { index: number; onChange: (opportunity: OpportunityInput) => void; remove: () => void }) {
  const [value, setValue] = useState<OpportunityInput>({ title: '', customer_relevance: '', why_now: '', counterevidence: '', unknowns: '', validation_action: '', sources: [] })
  const [draft, setDraft] = useState<ProposalInput>(emptyProposal())
  const [includeDraft, setIncludeDraft] = useState(false)
  function update(next: OpportunityInput) { setValue(next); onChange(next) }
  const fields: [keyof Omit<OpportunityInput, 'sources' | 'experiment_draft'>, string][] = [['title', 'Opportunity title'], ['customer_relevance', 'Customer relevance'], ['why_now', 'Why now'], ['counterevidence', 'Counterevidence'], ['unknowns', 'Unknowns'], ['validation_action', 'Cheap validation action']]
  return <fieldset className="wide subpanel"><legend>Opportunity {index + 1}</legend><div className="form-grid">{fields.map(([key, label]) => <label key={key} className="field"><span>{label}</span><textarea required rows={2} value={value[key]} onChange={e => update({ ...value, [key]: e.target.value })}/></label>)}<SourceEditor sources={value.sources} onChange={(sources: Source[]) => update({ ...value, sources })}/>
    <div className="wide"><Button type="button" variant="outline" size="sm" onClick={() => { const next = !includeDraft; setIncludeDraft(next); update({ ...value, experiment_draft: next ? draft : undefined }) }}>{includeDraft ? 'Remove experiment draft' : 'Add proposed experiment draft'}</Button>
      {includeDraft && <div className="subpanel"><p className="muted">All nine bounded proposal fields are required. The draft stays unapproved until you choose it in Opportunities.</p><ProposalFields value={draft} onChange={next => { setDraft(next); update({ ...value, experiment_draft: next }) }}/></div>}
    </div>
    </div><Button type="button" variant="ghost" onClick={remove}>Remove opportunity</Button></fieldset>
}

function ReturnEditor({ attempt }: { attempt: Attempt }) {
  const { workspace, save, busy } = useWorkspace()
  const [opportunities, setOpportunities] = useState<{ key: number; value: OpportunityInput | null }[]>([])
  const [counter, setCounter] = useState(0)
  const [deliverables, setDeliverables] = useState<Deliverable[]>([])
  const [fileError, setFileError] = useState('')
  const [uploading, setUploading] = useState(false)
  useEffect(() => { setOpportunities([]); setDeliverables([]); setFileError('') }, [workspace.revision])
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
  return <div className="subpanel"><h3>Record an agent return</h3><p className="muted">Report what happened, including failed or partial work. A return does not accept materials or record a business outcome.</p>{fileError && <p role="alert" className="error">{fileError}</p>}
    <Form key={`manual-return-${workspace.revision}`} submit="Import return" busy={busy || uploading} onSubmit={data => {
      const payload: Record<string, unknown> = {
        attempt_id: attempt.id, status: text(data, 'status'), external_effects: text(data, 'effects'), summary: text(data, 'summary'),
        deliverables, opportunities: opportunities.map(o => o.value).filter((o): o is OpportunityInput => Boolean(o)),
        no_opportunity_reason: text(data, 'no_reason') || null, execution_refs: [],
      }
      const override = text(data, 'return_id').trim()
      if (override) payload.return_id = override
      return save({ type: 'import_task_return', attempt_id: attempt.id, payload, original_text: null })
    }}>
      <Select name="status" label="Reported execution status" value="unknown" options={['unknown', 'succeeded', 'partial', 'failed'].map(value => ({ value, label: value }))}/>
      <Select name="effects" label="External effects" value="unknown" options={[{ value: 'unknown', label: 'Unknown — needs reconciliation' }, { value: 'none', label: 'No external effects' }, { value: 'confirmed', label: 'Confirmed effects (explain in summary)' }]}/>
      <Field name="summary" label="Return summary"/>
      {attempt.kind === 'research' && <>
        <Field name="no_reason" label="Reason for no actionable opportunity" required={false} hint="Required for a successful research return with zero opportunities."/>
        {opportunities.map((entry, index) => <OpportunityEditor index={index} key={entry.key} onChange={value => setOpportunities(current => current.map(o => o.key === entry.key ? { ...o, value } : o))} remove={() => setOpportunities(current => current.filter(o => o.key !== entry.key))}/>)}
        <div className="wide"><Button type="button" variant="outline" disabled={opportunities.length >= 3} onClick={() => { setOpportunities([...opportunities, { key: counter, value: null }]); setCounter(counter + 1) }}>Add opportunity</Button><small className="muted"> 0–3 opportunities</small></div>
      </>}
      <div className="wide"><div className="row"><strong>Returned deliverables</strong><Button type="button" variant="outline" size="sm" onClick={() => setDeliverables([...deliverables, { title: '', reference: '', media_type: 'text/markdown', note: '' }])}>Add deliverable reference</Button></div>
        {deliverables.map((d, i) => <div className="source-fields" key={i}>{([['title', 'Deliverable title'], ['reference', 'Deliverable reference'], ['media_type', 'Media type'], ['note', 'Deliverable note']] as const).map(([key, label]) => <label key={key} className="field"><span>{label}</span><input required={key !== 'note'} value={d[key]} onChange={e => setDeliverables(current => current.map((v, j) => i === j ? { ...v, [key]: e.target.value } : v))}/></label>)}<Button type="button" variant="ghost" onClick={() => setDeliverables(deliverables.filter((_, j) => j !== i))}>Remove deliverable</Button></div>)}
        <label className="file-picker">Upload a deliverable file<input type="file" disabled={uploading} onChange={e => void upload(e.target.files?.[0])}/></label><small className="muted">Up to 5 MB per file. Files stay in your local data directory.</small>
      </div>
      <details className="wide"><summary>Advanced return details</summary><Field name="return_id" label="Optional return ID override" short required={false} hint="Blank uses a stable content identity. Reuse it for identical retries; changed content receives a different identity."/></details>
    </Form>
  </div>
}

function TaskReturnPaste({ attempt }: { attempt: Attempt }) {
  const { save, busy } = useWorkspace()
  const [value, setValue] = useState('')
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')
  async function importText(originalText: string) {
    setValue(originalText); setError(''); setNotice('')
    let payload: unknown
    try { payload = JSON.parse(originalText) } catch (e) { setError(e instanceof Error ? `Invalid JSON: ${e.message}` : 'Invalid JSON'); return false }
    const saved = await save({ type: 'import_task_return', attempt_id: attempt.id, payload, original_text: originalText })
    if (saved) { setValue(''); setNotice('Return imported or preserved for review. The original text is kept with the task.') }
    return saved
  }
  return <section className="task-return"><h3>Return for this task</h3><p className="muted">Paste the agent’s original JSON. This task supplies a missing task ID and a stable content identity; use a fresh identity for changed output.</p>
    <form onSubmit={event => { event.preventDefault(); void importText(value) }}><label className="field"><span>Returned JSON</span><textarea aria-label="Returned JSON" value={value} onChange={event => { setValue(event.target.value); setError(''); setNotice('') }} rows={8}/></label><div className="form-actions"><Button type="submit" disabled={busy || !value.trim()}>Paste return</Button><label className="file-picker">Import JSON for this task<input type="file" accept="application/json,.json" onChange={event => { const file = event.target.files?.[0]; if (file) void file.text().then(importText).catch(e => setError(e instanceof Error ? e.message : 'Could not read return file')) }}/></label></div></form>
    {error && <p role="alert" className="error">{error}</p>}{notice && <p role="status" className="muted">{notice}</p>}
  </section>
}

function ExecutionEditor({ attempt }: { attempt: Attempt }) {
  const { workspace, save, busy } = useWorkspace()
  return <details><summary>Advanced execution references</summary><p className="muted">Copy the native session ID from your executor or Semon. These references are recorded as supplied, without inferred correlation.</p>
    <Form key={`execution-${workspace.revision}-${attempt.id}`} submit="Attach execution reference" busy={busy} onSubmit={data => save({ type: 'attach_execution', attempt_id: attempt.id, execution_ref: {
      namespace: text(data, 'namespace'), harness: text(data, 'harness'), session_id: text(data, 'session'), machine: text(data, 'machine') || null, source_revision: text(data, 'revision'), continuation: text(data, 'continuation') || null,
      segment: text(data, 'start') || text(data, 'end') ? { start_offset: Number(text(data, 'start')), end_offset: Number(text(data, 'end')) } : null,
    } })}>
      <Field name="namespace" label="Semon instance or source namespace" short/><Select name="harness" label="Harness" options={['codex', 'claude', 'copilot'].map(value => ({ value, label: value }))}/>
      <Field name="session" label="Native session ID" short/><Field name="machine" label="Machine ID (if applicable)" short required={false}/>
      <Field name="revision" label="Provenance or revision" short hint="For example, Semon revision and the operator’s source manifest reference."/><Field name="continuation" label="Native continuation reference" short required={false}/>
      <Field name="start" label="Segment start byte offset" short required={false}/><Field name="end" label="Segment end byte offset" short required={false}/>
    </Form>
    {attempt.execution_refs.length > 0 && <div className="technical"><JsonDetails title="Stored execution references" value={attempt.execution_refs}/></div>}
  </details>
}

function BriefSuggestion({ suggestion, attemptId, returnId }: { suggestion: Brief; attemptId: string; returnId: string }) {
  const { workspace, save, busy } = useWorkspace()
  const [materials, setMaterials] = useState(suggestion.materials || [])
  const suggestionSignature = JSON.stringify(suggestion)
  useEffect(() => setMaterials(suggestion.materials || []), [suggestionSignature])
  return <section className="subpanel"><h3>Suggested business brief · not applied</h3><p className="muted">This suggestion came with the source return. Review and edit it before saving; nothing changes until you choose Save business brief.</p>
    <dl className="evidence-grid">{briefFields.map(([key, label]) => <div key={key}><dt>{label}</dt><dd>{suggestion[key] || 'Unknown until supplied'}</dd></div>)}</dl>
    <Form key={`brief-suggestion-${suggestionSignature}`} submit="Save business brief" busy={busy} onSubmit={data => save({ type: 'save_brief', brief: {
      product: text(data, 'product'), customer: text(data, 'customer'), capabilities: text(data, 'capabilities'), objective: text(data, 'objective'), time_budget: text(data, 'time_budget'), money_budget: text(data, 'money_budget'), channels: text(data, 'channels'), constraints: text(data, 'constraints'), materials,
    } })}>
      {briefFields.map(([key, label]) => <Field key={key} name={key} label={label} required={['product', 'objective', 'constraints'].includes(key)} value={suggestion[key] || ''} hint={['product', 'objective', 'constraints'].includes(key) ? undefined : 'Blank means this detail remains unknown.'}/>)}
      <SourceEditor sources={materials} onChange={setMaterials}/>
    </Form>
    <details className="technical"><summary>Suggestion source</summary><p>Returned from task {attemptId}, return {returnId}. This provenance does not adopt the content.</p></details>
  </section>
}

function localFilePath(reference: string) {
  const clean = reference.replace(/^\/+/, '')
  return /^files\/[a-f0-9]{64}$/i.test(clean) ? `/${clean}` : null
}
function previewSupported(deliverable: Deliverable) {
  return /^(text\/(plain|markdown|x-markdown)|application\/json)(;|$)/i.test(deliverable.media_type) || /\.(txt|md|markdown|json)$/i.test(deliverable.title)
}
function snapshotProposal(attempt: Attempt) {
  const snapshot = attempt.snapshot as { proposal?: { content?: ProposalInput } } | null
  return snapshot?.proposal?.content
}

function PreviewMaterial({ attempt, deliverable }: { attempt: Attempt; deliverable: Deliverable }) {
  const criteria = snapshotProposal(attempt)
  const path = localFilePath(deliverable.reference)
  const supported = path && previewSupported(deliverable)
  const generatedUploadNote = /^SHA-256 [a-f0-9]{64}; [0-9]+ bytes$/.test(deliverable.note)
  const [open, setOpen] = useState(false)
  const [loading, setLoading] = useState(false)
  const [content, setContent] = useState('')
  const [error, setError] = useState('')
  async function preview() {
    if (open) { setOpen(false); return }
    if (content) { setOpen(true); return }
    if (!path) return
    setLoading(true); setError('')
    try {
      const response = await fetch(path)
      if (!response.ok) throw new Error('Could not open this local file')
      setContent(await response.text()); setOpen(true)
    } catch (e) { setError(e instanceof Error ? e.message : 'Could not preview this file') } finally { setLoading(false) }
  }
  const href = path || deliverable.reference
  return <div className="deliverable"><div className="row"><a href={href} target="_blank" rel="noreferrer">{deliverable.title} ↗</a>{supported ? <Button variant="outline" size="sm" disabled={loading} onClick={() => void preview()}>{loading ? 'Loading…' : open ? 'Hide material preview' : 'Preview material'}</Button> : <span className="muted">Download or open this reference</span>}</div>
    {deliverable.note && !generatedUploadNote && <small>{deliverable.note}</small>}
    <details className="technical"><summary>Advanced material details</summary><p><strong>Reference:</strong> <code>{deliverable.reference}</code></p><p><strong>Media type:</strong> {deliverable.media_type}</p>{generatedUploadNote && <p><strong>Upload metadata:</strong> {deliverable.note}</p>}</details>
    {!path && <small>External references are opened as links and are not fetched by Kirzner.</small>}
    {error && <p className="error" role="alert">{error}</p>}
    {open && <div className="preview-panel"><h4>Original acceptance criteria · proposal v{attempt.proposal_version || 'research'}</h4>{criteria ? <><p><strong>Expected deliverables:</strong> {criteria.expected_deliverables}</p><p><strong>Resource limits:</strong> {criteria.resource_limits}</p><p><strong>Preparation scope:</strong> {criteria.authorization_scope}</p></> : <p className="muted">This research task has no experiment proposal criteria.</p>}<pre>{content.slice(0, 20000)}{content.length > 20000 ? '\n\nPreview limited to 20,000 characters.' : ''}</pre></div>}
  </div>
}

export function AttemptCard({ attempt, embedded = false }: { attempt: Attempt; embedded?: boolean }) {
  const { workspace, save, busy } = useWorkspace()
  const [copyMessage, setCopyMessage] = useState('')
  const proposal = snapshotProposal(attempt)
  async function copyTask() {
    try {
      if (!navigator.clipboard?.writeText) throw new Error('Clipboard access is unavailable')
      await navigator.clipboard.writeText(attempt.brief_markdown)
      setCopyMessage('Exact task brief copied. The selectable text below is the fallback.')
    } catch { setCopyMessage('Copy did not work. Select the task brief text below and copy it.') }
  }
  const title = attempt.kind === 'research' ? 'Opportunity research task' : 'Experiment preparation task'
  const eyebrow = attempt.proposal_version ? `${embedded ? 'Linked task · ' : ''}Proposal version ${attempt.proposal_version}` : embedded ? 'Source research task' : 'Research can stand alone'
  return <Panel title={title} eyebrow={eyebrow}>
    <div className="row"><Status>{attempt.returns.length ? `${attempt.returns.length} returns` : 'Awaiting return'}</Status><div className="button-group"><Button variant="outline" size="sm" onClick={() => void copyTask()}>Copy task brief</Button><Button asChild variant="outline" size="sm"><a href={`/api/handoffs/${attempt.id}`}>Download task brief</a></Button><Button asChild variant="outline" size="sm"><a href={`/api/handoffs/${attempt.id}/continuation`}>Download continuation brief</a></Button></div></div>
    {copyMessage && <p role="status" className="muted">{copyMessage}</p>}
    <label className="field task-brief-fallback"><span>Task brief text · selectable copy fallback</span><textarea aria-label="Task brief text" readOnly value={attempt.brief_markdown} rows={8}/></label>
    {proposal && <div className="finding acceptance-criteria"><h3>Original acceptance criteria · proposal v{attempt.proposal_version}</h3><p><strong>Expected deliverables:</strong> {proposal.expected_deliverables}</p><p><strong>Resource limits:</strong> {proposal.resource_limits}</p><p><strong>Preparation scope:</strong> {proposal.authorization_scope}</p></div>}
    {attempt.kind === 'research' && <p className="muted">Research returns may include an unapproved experiment_draft and a business brief_suggestion. State what is evidence, what is inference, and what remains unknown.</p>}
    <TaskReturnPaste attempt={attempt}/>
    <details open={attempt.returns.length === 0}><summary>Enter a return using the full form</summary><ReturnEditor attempt={attempt}/></details>
    <ExecutionEditor attempt={attempt}/>
    {attempt.returns.map(receipt => {
      const result = receipt.input
      const review = attempt.reviews.filter(r => r.return_id === result.return_id).at(-1)
      const reconciliation = attempt.reconciliations.filter(r => r.return_id === result.return_id).at(-1)
      return <article className="receipt" key={`${result.return_id}-${receipt.digest}`}><h3>Agent return</h3><div className="status-grid"><div><small>Reported execution</small><Status>{result.status}</Status></div><div><small>Deliverable acceptance</small><Status tone={review?.accepted ? 'good' : 'amber'}>{review ? review.accepted ? 'Accepted' : 'Changes requested' : 'Unreviewed'}</Status></div><div><small>External effects</small><Status tone={result.external_effects === 'unknown' && !reconciliation ? 'amber' : ''}>{reconciliation ? `${reconciliation.external_effects} (reconciled)` : result.external_effects}</Status></div></div>
        <p>{result.summary}</p>{result.deliverables.map((deliverable, i) => <PreviewMaterial key={`${deliverable.reference}-${i}`} attempt={attempt} deliverable={deliverable}/>)}
        {result.opportunities.map((opportunity, i) => <section className="subpanel" key={`${opportunity.title}-${i}`}><h3>{opportunity.title}</h3><div className="evidence-grid">{([['Customer relevance', opportunity.customer_relevance], ['Why now', opportunity.why_now], ['Counterevidence', opportunity.counterevidence], ['Unknowns', opportunity.unknowns], ['Cheap validation', opportunity.validation_action]] as const).map(([label, value]) => <div key={label}><dt>{label}</dt><dd>{value}</dd></div>)}</div><Sources sources={opportunity.sources}/>{opportunity.experiment_draft && <div className="finding"><h4>Proposed experiment draft · unapproved</h4><dl>{proposalFields.map(([key, label]) => <div key={key}><dt>{label}</dt><dd>{opportunity.experiment_draft?.[key]}</dd></div>)}</dl></div>}</section>)}
        {result.brief_suggestion && <BriefSuggestion suggestion={result.brief_suggestion} attemptId={attempt.id} returnId={result.return_id}/>}
        {result.deliverables.length > 0 && <Form key={`review-${workspace.revision}-${attempt.id}-${result.return_id}`} submit="Record deliverable review" busy={busy} onSubmit={data => save({ type: 'review_return', attempt_id: attempt.id, return_id: result.return_id, accepted: text(data, 'review') === 'accept', note: text(data, 'note') })}>
          <Select name="review" label="Deliverable review" value="" required options={[{ value: '', label: 'Choose a review decision' }, { value: 'accept', label: 'Accept materials' }, { value: 'changes', label: 'Request changes' }]}/><Field name="note" label="Review note"/>
        </Form>}
        {result.external_effects === 'unknown' && <details open={!reconciliation}><summary>Reconcile unknown external effects</summary><Form key={`reconcile-${workspace.revision}-${attempt.id}-${result.return_id}`} submit="Record reconciliation" busy={busy} onSubmit={data => save({ type: 'reconcile_effects', attempt_id: attempt.id, return_id: result.return_id, external_effects: text(data, 'effects'), evidence: text(data, 'evidence') })}><Select name="effects" label="Verified external effects" options={[{ value: 'none', label: 'No effect occurred' }, { value: 'confirmed', label: 'Effect confirmed' }]}/><Field name="evidence" label="Reconciliation evidence"/></Form></details>}
        {attempt.reviews.filter(r => r.return_id === result.return_id).map((r, i) => <p className="muted" key={i}>{r.accepted ? 'Accepted' : 'Changes requested'}: {r.note}</p>)}
        {attempt.reconciliations.filter(r => r.return_id === result.return_id).map((r, i) => <p className="muted" key={i}>Reconciled as {r.external_effects}: {r.evidence}</p>)}
        <details className="technical"><summary>Advanced return details</summary><p>Task reference: <code>{attempt.id}</code> · Return reference: <code>{result.return_id}</code></p><p>Digest: <code>{receipt.digest}</code></p><pre>{receipt.original_text}</pre></details>
      </article>
    })}
    <details className="technical"><summary>Advanced task details</summary><p>Task reference: <code>{attempt.id}</code>{attempt.experiment_id && <> · Experiment reference: <code>{attempt.experiment_id}</code></>}{attempt.proposal_version && ` · Proposal version ${attempt.proposal_version}`}</p><JsonDetails title="Preserved input snapshot" value={attempt.snapshot}/>{attempt.execution_refs.length > 0 && <JsonDetails title="Execution references" value={attempt.execution_refs}/>}</details>
    {workspace.return_conflicts.filter(conflict => conflict.attempt_id === attempt.id).length > 0 && <div className="finding"><h3>Conflicting return needs review</h3><p>The original return remains effective. Corrected content needs a new return identity.</p>{workspace.return_conflicts.filter(conflict => conflict.attempt_id === attempt.id).map((conflict, i) => <JsonDetails key={i} title="Preserved conflicting content" value={{ task: conflict.attempt_id, return: conflict.return_id, original: conflict.original }}/>)}</div>}
  </Panel>
}

export function HandoffsPage() {
  const { workspace, save } = useWorkspace()
  const [error, setError] = useState('')
  const requestedAttemptId = typeof window === 'undefined' ? null : new URLSearchParams(window.location.search).get('attempt')
  const requestedAttempt = workspace.attempts.find(attempt => attempt.id === requestedAttemptId)
  const orderedAttempts = requestedAttempt ? [requestedAttempt, ...[...workspace.attempts].reverse().filter(attempt => attempt.id !== requestedAttempt.id)] : [...workspace.attempts].reverse()
  async function importFile(file?: File) {
    if (!file) return
    setError('')
    try { const original_text = await file.text(); await save({ type: 'import_return', payload: JSON.parse(original_text), original_text }) }
    catch (e) { setError(e instanceof Error ? e.message : 'Invalid return file') }
  }
  return <><div className="page-heading"><span className="eyebrow">04 / Delegate and review</span><h1>Your agent. A clear handoff.</h1><p>Copy or download the brief, use your existing agent, then bring the results back here.</p></div>
    <div className="import-bar"><div><strong>Have a returned JSON file?</strong><p>Import the agent’s envelope. The original content is preserved.</p></div><label className="file-picker">Import return file<input type="file" accept="application/json,.json" onChange={e => void importFile(e.target.files?.[0])}/></label></div>
    {requestedAttemptId && !requestedAttempt && <p role="status" className="finding">This handoff link points to a task that is unavailable in this workspace. Showing the available handoffs.</p>}
    {requestedAttempt && <p role="status" className="finding">Selected handoff is shown first so you can resume its return or review.</p>}
    {error && <p className="error" role="alert">{error}</p>}
    {workspace.return_conflicts.length > 0 && <Panel title="Conflicting returns need inspection">{workspace.return_conflicts.map((conflict, i) => <div className="finding error" key={i}><strong>Conflicting agent return</strong><p>This identity arrived with different content. The original effective return was retained. Use a new return identity for intentional revisions.</p><JsonDetails title="Advanced conflict details" value={conflict}/></div>)}</Panel>}
    {workspace.attempts.length === 0 ? <Empty title="Nothing handed off yet">Create a research brief in Opportunities or approve an experiment for preparation.</Empty> : orderedAttempts.map(attempt => <AttemptCard key={attempt.id} attempt={attempt}/>)}</>
}
