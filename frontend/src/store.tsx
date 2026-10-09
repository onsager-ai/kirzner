import { createContext, useContext, useState, type ReactNode } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import type { Workspace } from './types'
async function read() {
  const response = await fetch('/api/workspace')
  if (!response.ok) throw new Error('Could not load the workspace. Check the local application and refresh.')
  return response.json() as Promise<Workspace>
}
const WorkspaceContext = createContext<{ workspace: Workspace; save: (command: Record<string, unknown>) => Promise<boolean>; latestWorkspace: () => Workspace; busy: boolean } | null>(null)
export function WorkspaceProvider({ children }: { children: ReactNode }) {
  const query = useQuery({ queryKey: ['workspace'], queryFn: read, retry: 1 })
  const client = useQueryClient()
  const [notice, setNotice] = useState('')
  const [error, setError] = useState('')
  const mutation = useMutation({ mutationFn: async (command: Record<string, unknown>) => {
    const current = client.getQueryData<Workspace>(['workspace'])!
    const response = await fetch('/api/workspace', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ expected_revision: current.revision, command }) })
    const body = await response.json()
    if (body.workspace) client.setQueryData(['workspace'], body.workspace)
    if (!response.ok) { await client.invalidateQueries({ queryKey: ['workspace'] }); throw new Error(body.error || body.message || 'Could not save') }
    return body.message as string
  } })
  async function save(command: Record<string, unknown>) {
    setError(''); setNotice('')
    try { setNotice(await mutation.mutateAsync(command)); return true } catch (e) { setError(e instanceof Error ? e.message : 'Could not save'); return false }
  }
  if (query.isPending) return <div className="loading">Opening your workspace…</div>
  if (query.error) return <div role="alert" className="error"><p>{query.error.message}</p><button onClick={() => query.refetch()}>Try again</button></div>
  return <WorkspaceContext value={{ workspace: query.data!, save, latestWorkspace: () => client.getQueryData<Workspace>(['workspace']) || query.data!, busy: mutation.isPending }}>
    {error && <div role="alert" className="toast error">{error}<button aria-label="Dismiss error" onClick={() => setError('')}>×</button></div>}
    {notice && <div role="status" className="toast success">{notice}<button aria-label="Dismiss notice" onClick={() => setNotice('')}>×</button></div>}
    {children}
  </WorkspaceContext>
}
export function useWorkspace() {
  const context = useContext(WorkspaceContext)
  if (!context) throw new Error('Workspace provider missing')
  return context
}
