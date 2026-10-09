import React from 'react'
import ReactDOM from 'react-dom/client'
import { QueryClient, QueryClientProvider, useQuery } from '@tanstack/react-query'
import { createRootRoute, createRoute, createRouter, Link, Outlet, RouterProvider } from '@tanstack/react-router'
import { WorkspaceProvider, useWorkspace } from './store'
import { BriefPage } from './pages/brief'
import { OpportunitiesPage } from './pages/opportunities'
import { ExperimentsPage } from './pages/experiments'
import { HandoffsPage } from './pages/handoffs'
import { ResultsPage } from './pages/results'
import './styles.css'
function Shell() {
  const { workspace } = useWorkspace()
  const semon = useQuery({ queryKey: ['semon'], queryFn: async () => { const r = await fetch('/api/semon'); if (!r.ok) throw new Error('Unavailable'); return r.json() as Promise<{ capture_available: boolean; store_present: boolean; limitation: string }> }, retry: false })
  const navigation = [{ path: '/', label: 'Business brief', number: '01' }, { path: '/opportunities', label: 'Opportunities', number: '02' }, { path: '/experiments', label: 'Experiments', number: '03' }, { path: '/handoffs', label: 'Handoffs', number: '04' }, { path: '/results', label: 'Results & learning', number: '05' }] as const
  const pending = workspace.attempts.filter(a => a.returns.length === 0).length
  return <div className="app"><aside className="sidebar"><Link to="/" className="brand"><span className="brand-icon">k</span><span>kirzner<small>Business workspace</small></span></Link>
    <div className="workspace-label"><span className="green-dot"/>LOCAL WORKSPACE</div><nav aria-label="Business loop">{navigation.map(item => <Link to={item.path} key={item.path} className="nav-item" activeProps={{ className: 'nav-item active' }} activeOptions={{ exact: true }}><span>{item.number}</span>{item.label}{item.path === '/handoffs' && pending > 0 && <b>{pending}</b>}</Link>)}</nav>
    <div className="sidebar-note"><strong>You keep the decisions.</strong><p>Your existing agent prepares the work. You decide what to accept, act on, and learn.</p></div>
    <details className="semon-note"><summary>Semon · {semon.data?.capture_available ? 'Capture configured' : 'Not installed'}</summary><p>{semon.data?.store_present ? 'Separate store found. Coverage is not verified.' : 'No separate capture store found.'}</p><p>{semon.data?.limitation || 'Semon unavailable. Your business records are accessible.'}</p></details>
  </aside><div className="main"><header className="topbar"><span>{workspace.brief?.product || 'Your first business loop'}</span><span className="local-label"><span className="green-dot"/>Saved locally · revision {workspace.revision}</span></header><main><Outlet/></main><footer>Kirzner core · A small experiment, an honest result, a better next decision.</footer></div></div>
}
const rootRoute = createRootRoute({ component: () => <WorkspaceProvider><Shell/></WorkspaceProvider> })
const routes = [{ path: '/', component: BriefPage }, { path: '/opportunities', component: OpportunitiesPage }, { path: '/experiments', component: ExperimentsPage }, { path: '/handoffs', component: HandoffsPage }, { path: '/results', component: ResultsPage }].map(route => createRoute({ getParentRoute: () => rootRoute, ...route }))
const router = createRouter({ routeTree: rootRoute.addChildren(routes) })
const queryClient = new QueryClient({ defaultOptions: { queries: { staleTime: 5000, refetchOnWindowFocus: false } } })
ReactDOM.createRoot(document.getElementById('root')!).render(<React.StrictMode><QueryClientProvider client={queryClient}><RouterProvider router={router}/></QueryClientProvider></React.StrictMode>)
