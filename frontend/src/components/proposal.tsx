import { Field, text } from './forms'
import type { ProposalInput } from '../types'

export const proposalFields: [keyof ProposalInput, string][] = [
  ['audience', 'Target audience'],
  ['action', 'Proposed action'],
  ['expected_deliverables', 'Expected deliverables'],
  ['observation_window', 'Observation window'],
  ['success_criteria', 'Success criteria'],
  ['failure_criteria', 'Failure criteria'],
  ['inconclusive_criteria', 'Inconclusive criteria'],
  ['resource_limits', 'Resource limits'],
  ['authorization_scope', 'Preparation authorization scope'],
]

export function proposalFrom(data: FormData): ProposalInput {
  return Object.fromEntries(proposalFields.map(([key]) => [key, text(data, key)])) as ProposalInput
}

export function ProposalFields({ value, onChange }: { value?: ProposalInput; onChange?: (value: ProposalInput) => void }) {
  return <>{proposalFields.map(([key, label]) => onChange ? <label className="field" key={key}><span>{label}</span><textarea aria-label={label} name={key} required rows={3} value={value?.[key] || ''} onChange={event => onChange({ ...value, [key]: event.target.value } as ProposalInput)}/></label> : <Field key={key} name={key} label={label} value={value?.[key]}/>)}</>
}
