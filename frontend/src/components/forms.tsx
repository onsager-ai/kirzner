import { useId, useState, type ReactNode } from 'react'
import { Button } from './ui/button'
import type { Source } from '../types'
export function Form({ children, onSubmit, submit, busy = false }: { children: ReactNode; onSubmit: (data: FormData) => void | Promise<unknown>; submit: string; busy?: boolean }) {
  return <form onSubmit={e => { e.preventDefault(); void onSubmit(new FormData(e.currentTarget)) }}><div className="form-grid">{children}</div><div className="form-actions"><Button type="submit" disabled={busy}>{busy ? 'Saving…' : submit}</Button></div></form>
}
export function Field({ name, label, value = '', hint, required = true, short = false }: { name: string; label: string; value?: string; hint?: string; required?: boolean; short?: boolean }) {
  const hintId = useId()
  return <label className="field"><span>{label}</span>{short ? <input aria-label={label} aria-describedby={hint ? hintId : undefined} name={name} defaultValue={value} required={required}/> : <textarea aria-label={label} aria-describedby={hint ? hintId : undefined} name={name} defaultValue={value} required={required} rows={3}/>} {hint && <small id={hintId}>{hint}</small>}</label>
}
export function Select({ name, label, options, value }: { name: string; label: string; options: { value: string; label: string }[]; value?: string }) {
  return <label className="field"><span>{label}</span><select aria-label={label} name={name} defaultValue={value}>{options.map(o => <option key={o.value} value={o.value}>{o.label}</option>)}</select></label>
}
export const text = (data: FormData, name: string) => String(data.get(name) || '')
export function SourceEditor({ sources, onChange }: { sources: Source[]; onChange: (sources: Source[]) => void }) {
  function update(index: number, key: keyof Source, value: string) { onChange(sources.map((s, i) => i === index ? { ...s, [key]: value } : s)) }
  return <div className="wide source-editor"><div className="row"><strong>Source references</strong><Button type="button" variant="outline" size="sm" onClick={() => onChange([...sources, { title: '', url: '', evidence: '' }])}>Add source</Button></div>
    {sources.map((source, i) => <div className="source-fields" key={i}>
      <label className="field"><span>Source title</span><input required value={source.title} onChange={e => update(i, 'title', e.target.value)}/></label>
      <label className="field"><span>Source URL</span><input required type="url" value={source.url} onChange={e => update(i, 'url', e.target.value)}/></label>
      <label className="field"><span>What the source supports</span><textarea required rows={2} value={source.evidence} onChange={e => update(i, 'evidence', e.target.value)}/></label>
      <Button type="button" variant="ghost" size="sm" onClick={() => onChange(sources.filter((_, j) => j !== i))}>Remove source</Button>
    </div>)}
  </div>
}
export function Sources({ sources }: { sources: Source[] }) {
  return <ul className="sources">{sources.map((s, i) => <li key={i}><a href={s.url} target="_blank" rel="noreferrer">{s.title} ↗</a><p>{s.evidence}</p></li>)}</ul>
}
export function Empty({ title, children }: { title: string; children: ReactNode }) { return <div className="empty"><div className="empty-mark">↗</div><h3>{title}</h3><p>{children}</p></div> }
export function Panel({ title, children, eyebrow }: { title: string; children: ReactNode; eyebrow?: string }) { return <section className="panel">{eyebrow && <span className="eyebrow">{eyebrow}</span>}<h2>{title}</h2>{children}</section> }
export function Status({ children, tone = '' }: { children: ReactNode; tone?: string }) { return <span className={`badge ${tone}`}>{children}</span> }
export function JsonDetails({ title, value }: { title: string; value: unknown }) { return <details className="technical"><summary>{title}</summary><pre>{JSON.stringify(value, null, 2)}</pre></details> }
export function useLocalError() { const [error, setError] = useState(''); return { error, setError } }
