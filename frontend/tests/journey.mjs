import assert from 'node:assert/strict'
import { mkdir, writeFile } from 'node:fs/promises'
import { chromium } from 'playwright-core'

const base = process.env.KIRZNER_TEST_URL || 'http://127.0.0.1:4317'
const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || '/usr/bin/chromium', args: ['--no-sandbox'], headless: true })
const context = await browser.newContext({ viewport: { width: 1280, height: 900 }, permissions: ['clipboard-read', 'clipboard-write'] })
await context.tracing.start({ screenshots: true, snapshots: true, sources: true })
await context.addInitScript(() => {
  if (location.protocol === 'about:') return
  const routeKey = 'kirzner-ci-route-changes'
  const lastKey = 'kirzner-ci-last-path'
  if (sessionStorage.getItem(routeKey) === null) sessionStorage.setItem(routeKey, '[]')
  if (sessionStorage.getItem(lastKey) === null) sessionStorage.setItem(lastKey, location.pathname)
  const track = () => {
    const previous = sessionStorage.getItem(lastKey) || location.pathname
    if (previous !== location.pathname) {
      const changes = JSON.parse(sessionStorage.getItem(routeKey) || '[]')
      changes.push({ from: previous, to: location.pathname })
      sessionStorage.setItem(routeKey, JSON.stringify(changes))
      sessionStorage.setItem(lastKey, location.pathname)
    }
  }
  for (const method of ['pushState', 'replaceState']) {
    const original = history[method].bind(history)
    history[method] = (...args) => {
      const result = original(...args)
      track()
      return result
    }
  }
  window.addEventListener('pageshow', track)
  track()
})

const page = await context.newPage()
page.setDefaultTimeout(10000)
const pageErrors = []
page.on('pageerror', error => pageErrors.push(error.message))

const metrics = {
  evidence_label: 'synthetic browser journey and fixture records; no human timing, external agent execution, prospect contact, customer events, or business outcome',
  source_derived_baseline: {
    required_start_fields: 8,
    manually_reconstructed_proposal_fields: 9,
    separate_top_level_views: 5,
    top_level_navigation_changes: 8,
    prior_journey_file_transfers: { json_return_imports: 2, deliverable_uploads: 1, total: 3 },
    source: 'the preceding frontend/tests/journey.mjs flow before the three-field and selected-experiment path',
  },
  measured_primary_journey: {
    manual_business_fields_entered: { count: 3, fields: ['Product', 'Current business objective', 'Resource constraints'] },
    manually_reconstructed_proposal_fields: 0,
    prefilled_proposal_fields_edited: [],
    clipboard_task_copies: 0,
    pasted_task_returns: 0,
    local_deliverable_file_uploads: 0,
    file_downloads: 0,
    file_imports: 0,
    primary_route_changes: [],
    primary_route_change_count: 0,
    manual_task_return_cycles: 0,
    reopen_proved_same_experiment: false,
    reopen_current_next_step: null,
    primary_result_counts: null,
  },
  supplemental_regressions: {
    malformed_return_kept_visible: false,
    mismatched_task_rejected: false,
    duplicate_return_unchanged: false,
    conflicting_return_preserved: false,
    accepted_partial_material_ready: false,
    accepted_failed_material_ready_after_later_pending_attempt: false,
    unknown_effect_blocks_action_and_preparation_until_reconciled: false,
    reconciliation_does_not_retry: false,
    later_observation_appended_to_same_action: false,
    historical_seconds_preserved: false,
    self_prepared_current_action_reaches_observation: false,
    experiment_switch_resets_unsaved_proposal_and_observation_drafts: false,
    colliding_local_action_and_observation_ids_checked: false,
    zero_opportunity_status_headings_checked: [],
    new_research_task_resets_unsent_paste_and_old_task_remains_accessible: false,
    legacy_handoffs_file_import_and_download: false,
    supplemental_file_imports: 0,
    supplemental_task_brief_downloads: 0,
    clipboard_fallback_visible: false,
    mobile_overflow: null,
    supplemental_route_changes: [],
  },
  page_errors: pageErrors,
  completed: false,
}

let primaryRouteChanges = []
let primaryFrozen = false
const input = (name, value, scope = page) => scope.getByLabel(name, { exact: true }).fill(value)
function panel(name, scope = page) {
  return scope.locator('section.panel').filter({ has: scope.getByRole('heading', { name, exact: true }) }).first()
}
async function submit(name, scope = page, expectedStatus = 200) {
  const response = page.waitForResponse(r => new URL(r.url()).pathname === '/api/workspace' && r.request().method() === 'POST')
  await scope.getByRole('button', { name, exact: true }).click()
  const result = await response
  assert.equal(result.status(), expectedStatus, await result.text())
  return result
}
async function pasteReturn(task, originalText) {
  await input('Returned JSON', originalText, task)
  const response = page.waitForResponse(r => new URL(r.url()).pathname === '/api/workspace' && r.request().method() === 'POST')
  await task.getByRole('button', { name: 'Paste return', exact: true }).click()
  return response
}
async function workspace() {
  const response = await page.request.get(`${base}/api/workspace`)
  assert.equal(response.status(), 200)
  return response.json()
}
async function command(value, expectedStatus = 200) {
  const current = await workspace()
  const response = await page.request.post(`${base}/api/workspace`, { data: { expected_revision: current.revision, command: value } })
  const bodyText = await response.text()
  assert.equal(response.status(), expectedStatus, bodyText)
  return bodyText ? JSON.parse(bodyText) : null
}
async function navigate(label, pathname) {
  const nav = page.getByRole('navigation', { name: 'Business loop' })
  await nav.locator('a').filter({ hasText: label }).click()
  await page.waitForURL(url => new URL(url).pathname === pathname)
}
function routeChanges() {
  return page.evaluate(() => JSON.parse(sessionStorage.getItem('kirzner-ci-route-changes') || '[]'))
}
function localDateTime(seconds) {
  const date = new Date(seconds * 1000)
  const two = value => String(value).padStart(2, '0')
  return `${date.getFullYear()}-${two(date.getMonth() + 1)}-${two(date.getDate())}T${two(date.getHours())}:${two(date.getMinutes())}:${two(date.getSeconds())}`
}
function proposal(label) {
  return {
    audience: `SYNTHETIC regression fixture audience ${label}; not a validated segment.`,
    action: `SYNTHETIC bounded preparation plan ${label}.`,
    expected_deliverables: 'A local inert text note for regression verification only.',
    observation_window: 'No real-world observation window is started by this fixture.',
    success_criteria: 'No synthetic browser state is evidence of business success.',
    failure_criteria: 'No synthetic browser state is evidence of business failure.',
    inconclusive_criteria: 'There are no real-world observations in this fixture.',
    resource_limits: 'Zero spend; local test data only.',
    authorization_scope: 'Local test data only. No external contact, publication, purchase, deployment, or system change.',
  }
}
async function recordObservation(result, actionId, label) {
  await result.getByLabel('Observed action', { exact: true }).selectOption(actionId)
  await result.getByLabel('Observed business outcome', { exact: true }).selectOption('inconclusive')
  await input('Actual observations and evidence', `SYNTHETIC ${label}; no customer or external event is represented.`, result)
  await input('Comparison with original criteria', 'Synthetic form-state regression only; original proposal criteria remain attached.', result)
  await input('Proposed learning', 'No business learning is asserted by this synthetic record.', result)
  await submit('Record business observation', result)
}

try {
  await page.goto(base)
  const starter = panel('Your first business brief')
  const starterValues = {
    Product: 'Semon: local session capture and ordinary occurrence-log reads for coding agents',
    'Current business objective': 'Prepare a source-backed validation plan for whether founders using multiple external agent sessions need a simpler way to resume experiments.',
    'Resource constraints': 'Preparation only; zero spend; no prospect contact, publication, deployment, purchase, or private-log inspection. Other business details remain unknown.',
  }
  for (const [name, value] of Object.entries(starterValues)) await input(name, value, starter)
  await submit('Save business brief', starter)
  let current = await workspace()
  assert.deepEqual(Object.keys(current.brief).sort(), ['capabilities', 'channels', 'constraints', 'customer', 'materials', 'money_budget', 'objective', 'product', 'time_budget'].sort())
  for (const name of ['customer', 'capabilities', 'time_budget', 'money_budget', 'channels']) assert.equal(current.brief[name], '')
  assert.deepEqual(current.brief.materials, [])

  await navigate('Opportunities', '/opportunities')
  const researchForm = panel('Request opportunity research')
  await submit('Create research handoff', researchForm)
  let researchAttempt = (await workspace()).attempts.find(attempt => attempt.kind === 'research')
  assert.ok(researchAttempt)
  const researchTask = panel('Opportunity research task')
  await page.getByRole('heading', { name: 'Continue research here', exact: true }).waitFor()
  await researchTask.getByRole('button', { name: 'Copy task brief', exact: true }).click()
  await researchTask.getByText('Exact task brief copied. The selectable text below is the fallback.', { exact: true }).waitFor()
  const researchBrief = await researchTask.getByLabel('Task brief text', { exact: true }).inputValue()
  assert.equal(await page.evaluate(() => navigator.clipboard.readText()), researchBrief)
  assert.match(researchBrief, /unknown/i)
  metrics.measured_primary_journey.clipboard_task_copies += 1

  const source = {
    url: 'https://github.com/onsager-ai/semon/blob/defe9d791a197902c5bebcba2e2b7fe87dd9a075/README.md',
    title: 'Semon interfaces at the inspected revision',
    evidence: 'The pinned README documents incremental local capture and ordinary occurrence-log reads. This supports technical context only; local availability and customer demand are unverified. Verified README blob: e3b46edb940efd51ca4e9f3ca6da958daee68892.',
  }
  const experimentDraft = {
    audience: 'Hypothesis: solo founders already using multiple external agent sessions; actual reachable participants are unknown.',
    action: 'Prepare a source-backed interview pack for founder review. The founder may later choose to conduct three conversations outside Kirzner; no conversations have occurred in this example.',
    expected_deliverables: 'A one-page source-backed context note, five neutral questions about the participant\'s last real experiment, a blank observation worksheet, and an explicit list of unknowns. Acceptance requires pinned source links, no invented user quotations or demand claims, and no private session data.',
    observation_window: 'Proposed seven-day window begins only after a separately confirmed real first conversation; until then the experiment is unstarted and inconclusive.',
    success_criteria: 'Proposed criterion for founder review: at least two participants independently describe a recent concrete coordination interruption and one voluntarily agrees to review a follow-up prototype. No such evidence exists yet.',
    failure_criteria: 'After the agreed conversations and window are actually completed, no participant can describe a relevant recent interruption and none requests follow-up. Failure cannot be inferred from an unstarted window.',
    inconclusive_criteria: 'Fewer than three completed conversations, incomplete observation window, unavailable evidence, or only leading or hypothetical answers.',
    resource_limits: 'Proposed cap for founder review: two hours of preparation and zero spend. Actual available time and participant access are unknown.',
    authorization_scope: 'Read the pinned public source and prepare local materials only. Do not contact prospects, publish, purchase, deploy, inspect private logs, or change external systems. Importing this draft grants no approval.',
  }
  const researchReturn = {
    status: 'succeeded',
    external_effects: 'none',
    summary: 'SYNTHETIC source-grounded planning fixture for browser verification. No external agent ran this task; no customer evidence or interview is represented.',
    deliverables: [],
    opportunities: [{
      title: 'SYNTHETIC hypothesis: founders may need a simpler way to resume experiments across agent sessions',
      customer_relevance: 'Proposed audience: solo founders who already use multiple external coding or research agents. Reachable participants, coordination pain, and current workarounds remain unknown.',
      why_now: 'The pinned Semon documentation gives technical context for a source-backed interview pack. It does not establish timing, urgency, or demand.',
      counterevidence: 'Native agent history, existing notes, or Semon’s viewer may already be sufficient. Repository documentation does not demonstrate a business need or demand.',
      unknowns: 'Participant access, frequency and cost of coordination interruptions, existing alternatives, urgency, willingness to use or pay, and permission to inspect private logs remain unknown.',
      validation_action: 'Prepare and review a small interview pack. Any conversations or use of private session data require a separate founder action outside Kirzner.',
      sources: [source],
      experiment_draft: experimentDraft,
    }],
    execution_refs: [],
  }
  const researchReturnText = JSON.stringify(researchReturn)
  const researchImport = await pasteReturn(researchTask, researchReturnText)
  assert.equal(researchImport.status(), 200, await researchImport.text())
  metrics.measured_primary_journey.pasted_task_returns += 1
  current = await workspace()
  assert.equal(current.experiments.length, 0, 'Importing a draft must not create or approve an experiment')
  const importedOpportunity = current.opportunities[0]
  researchAttempt = current.attempts.find(attempt => attempt.id === researchAttempt.id)
  const importedResearchReceipt = researchAttempt.returns[0]
  assert.equal(importedResearchReceipt.original_text, researchReturnText)
  assert.match(importedResearchReceipt.input.return_id, /^return-[0-9a-f]{64}$/)
  assert.equal(importedResearchReceipt.input.attempt_id, researchAttempt.id)
  assert.equal('return_id' in researchReturn, false)
  assert.equal('attempt_id' in researchReturn, false)
  assert.deepEqual(importedOpportunity.evidence.experiment_draft, experimentDraft)

  const opportunityPanel = panel(importedOpportunity.evidence.title)
  await opportunityPanel.getByRole('heading', { name: 'Proposed experiment draft', exact: true }).waitFor()
  await opportunityPanel.getByRole('button', { name: 'Use experiment draft', exact: true }).click()
  await page.waitForURL(url => new URL(url).pathname === '/experiments' && Boolean(new URL(url).searchParams.get('experiment')))
  const experimentId = new URL(page.url()).searchParams.get('experiment')
  let experimentWorkspace = panel('Experiment workspace')
  await experimentWorkspace.getByLabel('Select experiment', { exact: true }).waitFor()
  assert.equal(await experimentWorkspace.getByLabel('Select experiment', { exact: true }).inputValue(), experimentId)
  await experimentWorkspace.getByRole('heading', { name: 'Needs decision', exact: true }).waitFor()
  current = await workspace()
  let experiment = current.experiments.find(item => item.id === experimentId)
  assert.equal(experiment.proposals.length, 1)
  assert.equal(experiment.decisions.length, 0)
  assert.equal(experiment.opportunity_id, importedOpportunity.id)

  let proposalPanel = panel('Current proposal', experimentWorkspace)
  const v1 = experiment.proposals.find(proposal => proposal.version === 1)
  await proposalPanel.getByRole('button', { name: 'Revise proposal', exact: true }).click()
  const v2Action = `${v1.content.action} The browser journey adds a clearly labeled synthetic review note.`
  await input('Proposed action', v2Action, proposalPanel)
  metrics.measured_primary_journey.prefilled_proposal_fields_edited.push('Proposed action (version 1 to version 2)')
  await submit('Save new version', proposalPanel)
  current = await workspace()
  experiment = current.experiments.find(item => item.id === experimentId)
  const v2 = experiment.proposals.find(proposal => proposal.version === 2)
  assert.ok(v2)
  assert.equal(experiment.decisions.length, 0, 'The edited version must still require a separate founder decision')
  assert.equal(current.attempts.length, 1, 'Editing a proposal must not create a handoff')
  await experimentWorkspace.getByRole('heading', { name: 'Needs decision', exact: true }).waitFor()

  proposalPanel = panel('Current proposal', experimentWorkspace)
  await proposalPanel.getByLabel('Decision', { exact: true }).selectOption('approve')
  await input('Proposal decision reason', 'SYNTHETIC browser fixture: approve preparation for v2 only.', proposalPanel)
  await submit('Record proposal decision', proposalPanel)
  current = await workspace()
  experiment = current.experiments.find(item => item.id === experimentId)
  const v2Approval = experiment.decisions.find(decision => decision.version === 2 && decision.approved)
  assert.ok(v2Approval)
  await submit('Create preparation handoff', proposalPanel)
  current = await workspace()
  const preparationAttempt = current.attempts.find(attempt => attempt.experiment_id === experimentId && attempt.proposal_version === 2)
  assert.ok(preparationAttempt)
  assert.equal(preparationAttempt.kind, 'preparation')

  const preparationTask = panel('Experiment preparation task')
  await preparationTask.getByRole('button', { name: 'Copy task brief', exact: true }).click()
  await preparationTask.getByText('Exact task brief copied. The selectable text below is the fallback.', { exact: true }).waitFor()
  const preparationBrief = await preparationTask.getByLabel('Task brief text', { exact: true }).inputValue()
  assert.equal(await page.evaluate(() => navigator.clipboard.readText()), preparationBrief)
  metrics.measured_primary_journey.clipboard_task_copies += 1

  const materialText = '# SYNTHETIC inert text deliverable\n\nPinned Semon source context for preview only. This local fixture is not customer feedback or external-agent output.\n'
  const uploadResponse = page.waitForResponse(r => new URL(r.url()).pathname === '/api/files' && r.request().method() === 'POST')
  await preparationTask.getByLabel('Upload a deliverable file', { exact: true }).setInputFiles({ name: 'synthetic-source-note.md', mimeType: 'text/markdown', buffer: Buffer.from(materialText) })
  const uploadResult = await uploadResponse
  assert.equal(uploadResult.status(), 200)
  const deliverable = await uploadResult.json()
  assert.ok(deliverable.reference)
  metrics.measured_primary_journey.local_deliverable_file_uploads += 1
  const materialDownload = await page.request.get(new URL(deliverable.reference, base).toString())
  assert.equal(materialDownload.status(), 200)
  assert.match(materialDownload.headers()['content-disposition'] || '', /attachment/i)
  assert.equal(await materialDownload.text(), materialText)

  const partialReturn = {
    status: 'partial',
    external_effects: 'none',
    summary: 'SYNTHETIC partial execution report for a local material-review test; no external write occurred.',
    deliverables: [deliverable],
    opportunities: [],
    execution_refs: [],
  }
  const partialReturnText = JSON.stringify(partialReturn)
  await input('Returned JSON', '{"status":', preparationTask)
  await preparationTask.getByRole('button', { name: 'Paste return', exact: true }).click()
  await preparationTask.getByRole('alert').filter({ hasText: 'Invalid JSON' }).waitFor()
  assert.equal(await preparationTask.getByLabel('Returned JSON', { exact: true }).inputValue(), '{"status":')
  metrics.supplemental_regressions.malformed_return_kept_visible = true

  const mismatchedText = JSON.stringify({ ...partialReturn, attempt_id: 'attempt-from-another-task' })
  const mismatchResponse = await pasteReturn(preparationTask, mismatchedText)
  assert.ok(mismatchResponse.status() >= 400)
  assert.match(await mismatchResponse.text(), /(task|attempt)/i)
  await page.getByRole('alert').filter({ hasText: /(task|attempt)/i }).first().waitFor()
  assert.equal(await preparationTask.getByLabel('Returned JSON', { exact: true }).inputValue(), mismatchedText)
  metrics.supplemental_regressions.mismatched_task_rejected = true

  const partialResponse = await pasteReturn(preparationTask, partialReturnText)
  assert.equal(partialResponse.status(), 200, await partialResponse.text())
  metrics.measured_primary_journey.pasted_task_returns += 1
  current = await workspace()
  let storedPreparation = current.attempts.find(attempt => attempt.id === preparationAttempt.id)
  let partialReceipt = storedPreparation.returns[0]
  const partialReturnId = partialReceipt.input.return_id
  assert.equal(storedPreparation.returns.length, 1)
  assert.equal(partialReceipt.input.status, 'partial')
  assert.equal(partialReceipt.input.external_effects, 'none')
  assert.equal(partialReceipt.input.attempt_id, preparationAttempt.id)
  assert.match(partialReturnId, /^return-[0-9a-f]{64}$/)
  assert.equal(partialReceipt.original_text, partialReturnText)
  assert.equal('attempt_id' in partialReturn, false)
  assert.equal('return_id' in partialReturn, false)

  const duplicateResponse = await pasteReturn(preparationTask, partialReturnText)
  assert.equal(duplicateResponse.status(), 200)
  current = await workspace()
  storedPreparation = current.attempts.find(attempt => attempt.id === preparationAttempt.id)
  assert.equal(storedPreparation.returns.length, 1)
  assert.equal(storedPreparation.returns[0].original_text, partialReturnText)
  metrics.supplemental_regressions.duplicate_return_unchanged = true

  const conflictText = JSON.stringify({ ...partialReturn, return_id: partialReturnId, summary: 'SYNTHETIC changed content under the original return identity.' })
  const conflictResponse = await pasteReturn(preparationTask, conflictText)
  assert.equal(conflictResponse.status(), 409, await conflictResponse.text())
  await preparationTask.getByRole('heading', { name: 'Conflicting return needs review', exact: true }).waitFor()
  current = await workspace()
  storedPreparation = current.attempts.find(attempt => attempt.id === preparationAttempt.id)
  assert.equal(storedPreparation.returns.length, 1)
  assert.equal(storedPreparation.returns[0].original_text, partialReturnText)
  assert.equal(current.return_conflicts.filter(conflict => conflict.attempt_id === preparationAttempt.id).length, 1)
  metrics.supplemental_regressions.conflicting_return_preserved = true

  const partialArticle = preparationTask.locator('article.receipt').filter({ hasText: partialReturn.summary })
  await partialArticle.getByRole('button', { name: 'Preview material', exact: true }).click()
  await partialArticle.getByRole('heading', { name: 'Original acceptance criteria · proposal v2', exact: true }).waitFor()
  assert.equal(await partialArticle.locator('.preview-panel pre').innerText(), materialText)
  await partialArticle.getByLabel('Deliverable review', { exact: true }).selectOption('accept')
  await input('Review note', 'SYNTHETIC fixture: accept this inert text as preparation material only.', partialArticle)
  await submit('Record deliverable review', partialArticle)
  current = await workspace()
  storedPreparation = current.attempts.find(attempt => attempt.id === preparationAttempt.id)
  assert.equal(storedPreparation.reviews.find(review => review.return_id === partialReturnId)?.accepted, true)
  assert.equal(storedPreparation.returns[0].input.status, 'partial', 'Acceptance must not rewrite the partial execution report')

  experimentWorkspace = panel('Experiment workspace')
  await experimentWorkspace.getByRole('heading', { name: 'Record actual action', exact: true }).waitFor()
  let result = panel('Actual actions and observations', experimentWorkspace)
  const actionForm = result.locator('details').filter({ hasText: 'Record an actual action · proposal v2' })
  await input('Actual action reference', 'SYNTHETIC browser record only; no outreach, delivery, or external write occurred.', actionForm)
  await input('What actually happened', 'This fixture records the software path and represents no real-world action.', actionForm)
  await actionForm.getByLabel('Material source', { exact: true }).selectOption('accepted')
  await submit('Record actual action', actionForm)
  current = await workspace()
  experiment = current.experiments.find(item => item.id === experimentId)
  const primaryAction = experiment.actions.find(action => action.version === 2 && !action.historical)
  assert.ok(primaryAction)
  assert.equal(primaryAction.self_prepared, false)
  assert.equal(current.attempts.find(attempt => attempt.id === preparationAttempt.id).returns[0].input.status, 'partial')
  metrics.supplemental_regressions.accepted_partial_material_ready = true

  result = panel('Actual actions and observations', experimentWorkspace)
  await recordObservation(result, primaryAction.id, 'primary inconclusive observation')
  current = await workspace()
  experiment = current.experiments.find(item => item.id === experimentId)
  const primaryObservation = experiment.observations.find(observation => observation.action_id === primaryAction.id)
  assert.ok(primaryObservation)
  assert.equal(primaryObservation.outcome, 'inconclusive')
  const firstObservation = result.locator('article.receipt').filter({ hasText: 'Observation for proposal v2' }).filter({ hasText: 'SYNTHETIC primary inconclusive observation' })
  await firstObservation.getByLabel('Next step', { exact: true }).selectOption('continue')
  await input('Next decision reason', 'SYNTHETIC browser fixture: keep the proposed preparation plan available for founder review.', firstObservation)
  await submit('Record next decision', firstObservation)
  const adoptionResponse = page.waitForResponse(r => new URL(r.url()).pathname === '/api/workspace' && r.request().method() === 'POST')
  await firstObservation.getByRole('button', { name: 'Adopt learning', exact: true }).click()
  assert.equal((await adoptionResponse).status(), 200)
  current = await workspace()
  experiment = current.experiments.find(item => item.id === experimentId)
  assert.equal(experiment.next_steps.find(step => step.observation_id === primaryObservation.id)?.choice, 'continue')
  assert.equal(current.adopted_learning.filter(item => item.experiment_id === experimentId && item.observation_id === primaryObservation.id).length, 1)

  await page.goto(`${base}/experiments?experiment=${encodeURIComponent(experimentId)}`)
  await page.reload()
  experimentWorkspace = panel('Experiment workspace')
  await experimentWorkspace.getByLabel('Select experiment', { exact: true }).waitFor()
  assert.equal(await experimentWorkspace.getByLabel('Select experiment', { exact: true }).inputValue(), experimentId)
  await experimentWorkspace.getByRole('heading', { name: 'Recorded next decision', exact: true }).waitFor()
  await panel('Current proposal', experimentWorkspace).getByText('Approved for this version', { exact: true }).waitFor()
  current = await workspace()
  experiment = current.experiments.find(item => item.id === experimentId)
  assert.equal(experiment.actions.filter(action => action.version === 2).length, 1)
  assert.equal(experiment.observations.filter(observation => observation.action_id === primaryAction.id).length, 1)
  assert.equal(experiment.next_steps.filter(step => step.observation_id === primaryObservation.id).length, 1)
  assert.equal(current.adopted_learning.filter(item => item.experiment_id === experimentId).length, 1)

  primaryRouteChanges = await routeChanges()
  assert.deepEqual(primaryRouteChanges, [
    { from: '/', to: '/opportunities' },
    { from: '/opportunities', to: '/experiments' },
  ])
  metrics.measured_primary_journey.primary_route_changes = primaryRouteChanges
  metrics.measured_primary_journey.primary_route_change_count = primaryRouteChanges.length
  metrics.measured_primary_journey.manual_task_return_cycles = 2
  metrics.measured_primary_journey.reopen_proved_same_experiment = true
  metrics.measured_primary_journey.reopen_current_next_step = 'Recorded next decision'
  metrics.measured_primary_journey.primary_result_counts = {
    actions: experiment.actions.length,
    observations: experiment.observations.length,
    next_decisions: experiment.next_steps.length,
    adopted_learning: current.adopted_learning.filter(item => item.experiment_id === experimentId).length,
    linked_research_and_preparation_attempts: current.attempts.filter(attempt => attempt.id === researchAttempt.id || attempt.id === preparationAttempt.id).length,
  }
  primaryFrozen = true

  // Separate regression fixtures start only after the primary friction counts are frozen.
  result = panel('Actual actions and observations', experimentWorkspace)
  await recordObservation(result, primaryAction.id, 'second appended observation on the same action')
  current = await workspace()
  experiment = current.experiments.find(item => item.id === experimentId)
  const appendedObservation = experiment.observations.find(observation => observation.id !== primaryObservation.id && observation.action_id === primaryAction.id)
  assert.ok(appendedObservation)
  assert.equal(appendedObservation.version, 2)
  assert.equal(experiment.actions.filter(action => action.id === primaryAction.id).length, 1)
  assert.equal(experiment.observations.filter(observation => observation.action_id === primaryAction.id).length, 2)
  metrics.supplemental_regressions.later_observation_appended_to_same_action = true

  const secondExperiment = {
    id: null,
    attempt: null,
    report: {
      status: 'failed',
      external_effects: 'none',
      summary: 'SYNTHETIC failed execution report carrying an available local text file; no external action occurred.',
      deliverables: [deliverable],
      opportunities: [],
      execution_refs: [],
    },
  }
  await command({ type: 'create_experiment', opportunity_id: null, proposal: proposal('accepted-material-pending-attempt') })
  current = await workspace()
  secondExperiment.id = current.experiments.at(-1).id
  await command({ type: 'decide_proposal', experiment_id: secondExperiment.id, version: 1, approved: true, reason: 'SYNTHETIC regression fixture approval for local state testing.' })
  await command({ type: 'prepare_handoff', experiment_id: secondExperiment.id, version: 1, context: 'SYNTHETIC fixture: no external executor is called.' })
  current = await workspace()
  secondExperiment.attempt = current.attempts.find(attempt => attempt.experiment_id === secondExperiment.id && attempt.proposal_version === 1)
  const failedText = JSON.stringify(secondExperiment.report)
  await command({ type: 'import_task_return', attempt_id: secondExperiment.attempt.id, payload: secondExperiment.report, original_text: failedText })
  current = await workspace()
  const failedReturn = current.attempts.find(attempt => attempt.id === secondExperiment.attempt.id).returns[0]
  await command({ type: 'review_return', attempt_id: secondExperiment.attempt.id, return_id: failedReturn.input.return_id, accepted: true, note: 'SYNTHETIC local material acceptance fixture despite a failed report.' })
  await command({ type: 'prepare_handoff', experiment_id: secondExperiment.id, version: 1, context: 'SYNTHETIC second pending task to verify accepted material remains available.' })
  current = await workspace()
  const secondPendingAttempt = current.attempts.filter(attempt => attempt.experiment_id === secondExperiment.id && attempt.proposal_version === 1).at(-1)
  assert.ok(secondPendingAttempt)
  assert.equal(secondPendingAttempt.returns.length, 0)
  assert.equal(current.attempts.find(attempt => attempt.id === secondExperiment.attempt.id).returns[0].input.status, 'failed')
  assert.equal(current.attempts.find(attempt => attempt.id === secondExperiment.attempt.id).reviews[0].accepted, true)

  await page.goto(`${base}/experiments?experiment=${encodeURIComponent(secondExperiment.id)}`)
  await page.reload()
  let secondWorkspace = panel('Experiment workspace')
  await secondWorkspace.getByRole('heading', { name: 'Record actual action', exact: true }).waitFor()
  result = panel('Actual actions and observations', secondWorkspace)
  await result.getByText('Previously accepted materials for this proposal version remain available; a later preparation task is still pending or has no accepted return.', { exact: true }).waitFor()
  const secondActionForm = result.locator('details').filter({ hasText: 'Record an actual action · proposal v1' })
  assert.equal(await secondActionForm.getByRole('button', { name: 'Record actual action', exact: true }).count(), 1)
  metrics.supplemental_regressions.accepted_failed_material_ready_after_later_pending_attempt = true

  // The legacy task card still exports briefs and imports the same JSON return from a file.
  const secondTaskCard = panel('Experiment preparation task')
  const downloadPath = await secondTaskCard.getByRole('link', { name: 'Download task brief', exact: true }).getAttribute('href')
  const downloadedBrief = await page.request.get(new URL(downloadPath, base).toString())
  assert.equal(downloadedBrief.status(), 200)
  assert.equal(await downloadedBrief.text(), secondPendingAttempt.brief_markdown)
  metrics.supplemental_regressions.supplemental_task_brief_downloads += 1
  await page.evaluate(() => Object.defineProperty(navigator, 'clipboard', { configurable: true, value: undefined }))
  await secondTaskCard.getByRole('button', { name: 'Copy task brief', exact: true }).click()
  await secondTaskCard.getByText('Copy did not work. Select the task brief text below and copy it.', { exact: true }).waitFor()
  assert.equal(await secondTaskCard.getByLabel('Task brief text', { exact: true }).inputValue(), secondPendingAttempt.brief_markdown)
  metrics.supplemental_regressions.clipboard_fallback_visible = true

  const secondActionReference = 'SYNTHETIC second experiment action for local state regression.'
  await input('Actual action reference', secondActionReference, secondActionForm)
  await input('What actually happened', 'This synthetic record tests form isolation and reports no external event.', secondActionForm)
  await secondActionForm.getByLabel('Material source', { exact: true }).selectOption('accepted')
  await submit('Record actual action', secondActionForm)
  current = await workspace()
  let second = current.experiments.find(item => item.id === secondExperiment.id)
  const secondAction = second.actions.find(action => action.version === 1)
  assert.ok(secondAction)
  result = panel('Actual actions and observations', secondWorkspace)
  await recordObservation(result, secondAction.id, 'second experiment first observation')
  result = panel('Actual actions and observations', secondWorkspace)
  await recordObservation(result, secondAction.id, 'second experiment second observation for collision test')
  current = await workspace()
  second = current.experiments.find(item => item.id === secondExperiment.id)
  const secondObservation = second.observations.find(observation => observation.evidence.includes('second experiment second observation for collision test'))
  assert.ok(secondObservation)

  await page.goto(`${base}/experiments?experiment=${encodeURIComponent(experimentId)}`)
  await page.reload()
  experimentWorkspace = panel('Experiment workspace')
  await experimentWorkspace.getByLabel('Select experiment', { exact: true }).waitFor()
  result = panel('Actual actions and observations', experimentWorkspace)
  const appendedArticle = result.locator('article.receipt').filter({ hasText: 'SYNTHETIC second appended observation on the same action' })
  await input('Next decision reason', 'UNSAVED e1 next-step text must not appear in experiment e2.', appendedArticle)
  proposalPanel = panel('Current proposal', experimentWorkspace)
  await proposalPanel.getByRole('button', { name: 'Revise proposal', exact: true }).click()
  const unsavedProposal = 'UNSAVED e1 proposal text must not appear in experiment e2.'
  await input('Proposed action', unsavedProposal, proposalPanel)

  await experimentWorkspace.getByLabel('Select experiment', { exact: true }).selectOption(secondExperiment.id)
  const secondProposalPanel = panel('Current proposal', experimentWorkspace)
  assert.equal(await secondProposalPanel.getByRole('button', { name: 'Save new version', exact: true }).count(), 0)
  assert.equal(await page.getByText(unsavedProposal, { exact: true }).count(), 0)
  const secondObservationArticle = panel('Actual actions and observations', experimentWorkspace).locator('article.receipt').filter({ hasText: 'SYNTHETIC second experiment second observation for collision test' })
  assert.equal(await secondObservationArticle.getByLabel('Next decision reason', { exact: true }).inputValue(), '')
  current = await workspace()
  experiment = current.experiments.find(item => item.id === experimentId)
  second = current.experiments.find(item => item.id === secondExperiment.id)
  assert.equal(experiment.actions[0].id, second.actions[0].id, 'Fixture should use the same experiment-local action identifier')
  assert.deepEqual(experiment.observations.map(item => item.id), second.observations.map(item => item.id), 'Fixture should use the same experiment-local observation identifiers')
  metrics.supplemental_regressions.colliding_local_action_and_observation_ids_checked = true

  await experimentWorkspace.getByLabel('Select experiment', { exact: true }).selectOption(experimentId)
  proposalPanel = panel('Current proposal', experimentWorkspace)
  await proposalPanel.getByRole('button', { name: 'Revise proposal', exact: true }).click()
  assert.equal(await proposalPanel.getByLabel('Proposed action', { exact: true }).inputValue(), v2Action)
  assert.notEqual(await proposalPanel.getByLabel('Proposed action', { exact: true }).inputValue(), unsavedProposal)
  const v3Action = `${v2Action} A later synthetic regression version checks fresh approval and historical boundaries.`
  await input('Proposed action', v3Action, proposalPanel)
  await page.waitForTimeout(1100)
  await submit('Save new version', proposalPanel)
  current = await workspace()
  experiment = current.experiments.find(item => item.id === experimentId)
  const v3 = experiment.proposals.find(item => item.version === 3)
  assert.ok(v3)
  assert.ok(v3.created_at > v2Approval.recorded_at + 1)
  assert.equal(experiment.decisions.some(decision => decision.version === 3), false)
  assert.equal(current.attempts.find(attempt => attempt.id === preparationAttempt.id).proposal_version, 2)
  await experimentWorkspace.getByRole('heading', { name: 'Needs decision', exact: true }).waitFor()
  proposalPanel = panel('Current proposal', experimentWorkspace)
  assert.equal(await proposalPanel.getByRole('button', { name: 'Create preparation handoff', exact: true }).count(), 0)

  result = panel('Actual actions and observations', experimentWorkspace)
  await result.getByText('Record a completed action for an older version', { exact: true }).click()
  const historicalCompletedAt = v3.created_at - 1
  assert.ok(historicalCompletedAt > v2Approval.recorded_at)
  const completedInput = result.getByLabel('Completion date and time', { exact: true })
  assert.equal(await completedInput.getAttribute('step'), '1')
  await completedInput.fill(localDateTime(historicalCompletedAt))
  await input('Historical action reference', 'SYNTHETIC founder-attested completed v2 record; no real outreach occurred.', result)
  await input('What happened then', 'A synthetic local preparation record; no participant or customer event is represented.', result)
  await result.getByLabel('Material source', { exact: true }).selectOption('accepted')
  await submit('Record historical action', result)
  current = await workspace()
  experiment = current.experiments.find(item => item.id === experimentId)
  const historicalAction = experiment.actions.find(action => action.version === 2 && action.historical)
  assert.ok(historicalAction)
  assert.equal(historicalAction.completed_at, historicalCompletedAt)
  assert.equal(experiment.decisions.some(decision => decision.version === 3), false)
  assert.equal(current.attempts.filter(attempt => attempt.experiment_id === experimentId).length, 1, 'A historical record must not create a task')
  metrics.supplemental_regressions.historical_seconds_preserved = true

  proposalPanel = panel('Current proposal', experimentWorkspace)
  await proposalPanel.getByLabel('Decision', { exact: true }).selectOption('approve')
  await input('Proposal decision reason', 'SYNTHETIC browser fixture: approve v3 for self-preparation only.', proposalPanel)
  await submit('Record proposal decision', proposalPanel)
  current = await workspace()
  experiment = current.experiments.find(item => item.id === experimentId)
  assert.equal(experiment.decisions.find(decision => decision.version === 3)?.approved, true)
  assert.equal(current.attempts.some(attempt => attempt.experiment_id === experimentId && attempt.proposal_version === 3), false)
  result = panel('Actual actions and observations', experimentWorkspace)
  const selfPreparedActionForm = result.locator('details').filter({ hasText: 'Record an actual action · proposal v3' })
  await input('Actual action reference', 'SYNTHETIC self-prepared action fixture; no external event occurred.', selfPreparedActionForm)
  await input('What actually happened', 'A synthetic local preparation record was entered without an agent handoff.', selfPreparedActionForm)
  await selfPreparedActionForm.getByLabel('Material source', { exact: true }).selectOption('self')
  await submit('Record actual action', selfPreparedActionForm)
  await experimentWorkspace.getByRole('heading', { name: 'Observe against original criteria', exact: true }).waitFor()
  current = await workspace()
  experiment = current.experiments.find(item => item.id === experimentId)
  assert.ok(experiment.actions.some(action => action.version === 3 && action.self_prepared && !action.historical))
  assert.equal(current.attempts.some(attempt => attempt.experiment_id === experimentId && attempt.proposal_version === 3), false)
  metrics.supplemental_regressions.self_prepared_current_action_reaches_observation = true

  const unknownExperiment = { id: null, attempt: null }
  await command({ type: 'create_experiment', opportunity_id: null, proposal: proposal('unknown-effects-blocker') })
  current = await workspace()
  unknownExperiment.id = current.experiments.at(-1).id
  await command({ type: 'decide_proposal', experiment_id: unknownExperiment.id, version: 1, approved: true, reason: 'SYNTHETIC blocker fixture only.' })
  await command({ type: 'prepare_handoff', experiment_id: unknownExperiment.id, version: 1, context: 'SYNTHETIC unknown-effects barrier fixture.' })
  current = await workspace()
  unknownExperiment.attempt = current.attempts.find(attempt => attempt.experiment_id === unknownExperiment.id)
  const unknownReturn = { status: 'partial', external_effects: 'unknown', summary: 'SYNTHETIC unknown-effects blocker fixture; effect requires reconciliation.', deliverables: [], opportunities: [], execution_refs: [] }
  const unknownText = JSON.stringify(unknownReturn)
  await command({ type: 'import_task_return', attempt_id: unknownExperiment.attempt.id, payload: unknownReturn, original_text: unknownText })
  current = await workspace()
  const attemptsBeforeReconciliation = current.attempts.length
  await page.goto(`${base}/experiments?experiment=${encodeURIComponent(unknownExperiment.id)}`)
  await page.reload()
  let unknownWorkspace = panel('Experiment workspace')
  await unknownWorkspace.getByRole('heading', { name: 'Reconcile effects', exact: true }).waitFor()
  assert.equal(await panel('Current proposal', unknownWorkspace).getByRole('button', { name: 'Create preparation handoff', exact: true }).count(), 0)
  assert.equal(await panel('Actual actions and observations', unknownWorkspace).getByRole('button', { name: 'Record actual action', exact: true }).count(), 0)
  await page.goto(`${base}/handoffs?attempt=${encodeURIComponent(unknownExperiment.attempt.id)}`)
  const unknownTask = panel('Experiment preparation task')
  const unknownArticle = unknownTask.locator('article.receipt').filter({ hasText: unknownReturn.summary })
  await unknownArticle.getByText('Reconcile unknown external effects', { exact: true }).click()
  await unknownArticle.getByLabel('Verified external effects', { exact: true }).selectOption('none')
  await input('Reconciliation evidence', 'SYNTHETIC fixture confirms no external write occurred.', unknownArticle)
  await submit('Record reconciliation', unknownArticle)
  current = await workspace()
  assert.equal(current.attempts.length, attemptsBeforeReconciliation, 'Reconciliation must not schedule or retry an attempt')
  assert.equal(current.attempts.find(attempt => attempt.id === unknownExperiment.attempt.id).reconciliations.length, 1)
  await page.goto(`${base}/experiments?experiment=${encodeURIComponent(unknownExperiment.id)}`)
  await page.reload()
  unknownWorkspace = panel('Experiment workspace')
  assert.equal(await unknownWorkspace.getByRole('heading', { name: 'Reconcile effects', exact: true }).count(), 0)
  assert.equal(await panel('Current proposal', unknownWorkspace).getByRole('button', { name: 'Create preparation handoff', exact: true }).count(), 1)
  metrics.supplemental_regressions.unknown_effect_blocks_action_and_preparation_until_reconciled = true
  metrics.supplemental_regressions.reconciliation_does_not_retry = true

  await navigate('Opportunities', '/opportunities')
  const emptyResearchCases = [
    ['unknown', null, 'Research outcome unknown'],
    ['partial', null, 'Research return incomplete'],
    ['failed', null, 'Research failed'],
    ['succeeded', null, 'Research result needs review'],
    ['succeeded', 'SYNTHETIC explicit no-actionable reason for status-label regression only.', 'No actionable opportunity'],
  ]
  for (const [status, reason, expectedHeading] of emptyResearchCases) {
    await submit('Create research handoff', panel('Request opportunity research'))
    current = await workspace()
    const attempt = current.attempts.filter(item => item.kind === 'research').at(-1)
    const task = panel('Opportunity research task')
    const payload = {
      status,
      external_effects: 'none',
      summary: `SYNTHETIC ${status} zero-opportunity label fixture; no external research or customer event occurred.`,
      deliverables: [],
      opportunities: [],
      ...(reason ? { no_opportunity_reason: reason } : {}),
      execution_refs: [],
    }
    const text = JSON.stringify(payload)
    const imported = await pasteReturn(task, text)
    assert.equal(imported.status(), 200, await imported.text())
    await page.getByText(expectedHeading, { exact: true }).waitFor()
    metrics.supplemental_regressions.zero_opportunity_status_headings_checked.push({ status, expected_heading: expectedHeading })
  }
  await page.getByText('Research outcome unknown', { exact: true }).waitFor()
  await page.getByText('Research return incomplete', { exact: true }).waitFor()
  assert.equal(await page.getByText('No actionable opportunity', { exact: true }).count(), 1)

  current = await workspace()
  const oldResearchAttempt = current.attempts.filter(attempt => attempt.kind === 'research').at(-1)
  const oldResearchReturnCount = oldResearchAttempt.returns.length
  const oldResearchTask = panel('Opportunity research task')
  const oldResearchBrief = await oldResearchTask.getByLabel('Task brief text', { exact: true }).inputValue()
  await input('Returned JSON', '{invalid task A JSON', oldResearchTask)
  await oldResearchTask.getByRole('button', { name: 'Paste return', exact: true }).click()
  await oldResearchTask.getByRole('alert').filter({ hasText: 'Invalid JSON' }).waitFor()
  await submit('Create research handoff', panel('Request opportunity research'))
  current = await workspace()
  const newResearchAttempt = current.attempts.filter(attempt => attempt.kind === 'research').at(-1)
  assert.notEqual(newResearchAttempt.id, oldResearchAttempt.id)
  assert.equal(current.attempts.find(attempt => attempt.id === oldResearchAttempt.id).returns.length, oldResearchReturnCount, 'Unsent task A text must not import a return')
  assert.equal(newResearchAttempt.returns.length, 0)
  const newResearchTask = panel('Opportunity research task')
  assert.equal(await newResearchTask.getByLabel('Returned JSON', { exact: true }).inputValue(), '')
  assert.equal(await newResearchTask.getByRole('alert').count(), 0)
  metrics.supplemental_regressions.new_research_task_resets_unsent_paste_and_old_task_remains_accessible = true

  const allHandoffs = page.getByRole('link', { name: 'All handoffs and older tasks', exact: true })
  await allHandoffs.click()
  await page.waitForURL(url => new URL(url).pathname === '/handoffs')
  await page.goto(`${base}/handoffs?attempt=${encodeURIComponent(oldResearchAttempt.id)}`)
  await page.getByText('Selected handoff is shown first so you can resume its return or review.', { exact: true }).waitFor()
  const selectedOldResearchTask = panel('Opportunity research task')
  assert.equal(await selectedOldResearchTask.getByLabel('Task brief text', { exact: true }).inputValue(), oldResearchBrief)
  assert.equal(await selectedOldResearchTask.getByRole('link', { name: 'Download task brief', exact: true }).getAttribute('href'), `/api/handoffs/${oldResearchAttempt.id}`)
  current = await workspace()
  const latestPreparation = current.attempts.filter(attempt => attempt.kind === 'preparation').at(-1)
  assert.ok(latestPreparation)
  const handoffsPreparationTask = panel('Experiment preparation task')
  const handoffsDownloadPath = await handoffsPreparationTask.getByRole('link', { name: 'Download task brief', exact: true }).getAttribute('href')
  assert.equal(handoffsDownloadPath, `/api/handoffs/${latestPreparation.id}`)
  const handoffsDownload = await page.request.get(new URL(handoffsDownloadPath, base).toString())
  assert.equal(handoffsDownload.status(), 200)
  assert.equal(await handoffsDownload.text(), latestPreparation.brief_markdown)
  metrics.supplemental_regressions.supplemental_task_brief_downloads += 1
  const failedLegacyPayload = { ...secondExperiment.report, attempt_id: secondExperiment.attempt.id, return_id: failedReturn.input.return_id }
  const legacyImport = page.waitForResponse(r => new URL(r.url()).pathname === '/api/workspace' && r.request().method() === 'POST')
  await page.getByLabel('Import return file', { exact: true }).setInputFiles({ name: 'legacy-failed-return.json', mimeType: 'application/json', buffer: Buffer.from(JSON.stringify(failedLegacyPayload)) })
  assert.equal((await legacyImport).status(), 200)
  metrics.supplemental_regressions.supplemental_file_imports += 1
  current = await workspace()
  assert.equal(current.attempts.find(attempt => attempt.id === secondExperiment.attempt.id).returns.length, 1)
  const routeRegressionEvents = await routeChanges()
  metrics.supplemental_regressions.legacy_handoffs_file_import_and_download = true

  await page.goto(`${base}/experiments?experiment=${encodeURIComponent(experimentId)}`)
  await page.reload()
  await page.setViewportSize({ width: 390, height: 844 })
  await page.emulateMedia({ colorScheme: 'light' })
  metrics.supplemental_regressions.mobile_overflow = await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth)
  assert.equal(metrics.supplemental_regressions.mobile_overflow, false, 'The phone-width layout must not overflow')
  await page.setViewportSize({ width: 1280, height: 900 })

  current = await workspace()
  experiment = current.experiments.find(item => item.id === experimentId)
  metrics.supplemental_regressions.supplemental_route_changes = routeRegressionEvents.slice(primaryRouteChanges.length)
  metrics.page_errors = pageErrors
  assert.deepEqual(pageErrors, [])
  metrics.completed = true
  console.log('PASS: synthetic primary founder flow and separately measured return, version, historical, multiple-experiment, status-label, reconciliation, file-compatibility, and responsive regressions.')
} finally {
  metrics.page_errors = pageErrors
  try {
    const changes = await routeChanges()
    metrics.supplemental_regressions.supplemental_route_changes = primaryFrozen ? changes.slice(primaryRouteChanges.length) : changes
    if (!primaryFrozen) metrics.measured_primary_journey.primary_route_changes = changes
  } catch { /* retain events captured before a navigation failure */ }
  metrics.measured_primary_journey.primary_route_change_count = metrics.measured_primary_journey.primary_route_changes.length
  metrics.measured_primary_journey.manual_proposal_edit_count = metrics.measured_primary_journey.prefilled_proposal_fields_edited.length
  metrics.measured_primary_journey.measured_transfer_steps = metrics.measured_primary_journey.clipboard_task_copies + metrics.measured_primary_journey.pasted_task_returns + metrics.measured_primary_journey.local_deliverable_file_uploads
  await mkdir('test-results', { recursive: true })
  for (const [width, theme, filename] of [
    [1280, 'light', '1280-light.png'],
    [1280, 'dark', '1280-dark.png'],
    [390, 'light', '390-light.png'],
    [390, 'dark', '390-dark.png'],
  ]) {
    try {
      await page.setViewportSize({ width, height: 900 })
      await page.emulateMedia({ colorScheme: theme })
      await page.screenshot({ path: `test-results/${filename}`, fullPage: true, animations: 'disabled' })
    } catch { /* keep the trace and result JSON even when navigation or capture fails */ }
  }
  try { await context.tracing.stop({ path: 'test-results/browser-trace.zip' }) } catch { /* best-effort trace artifact */ }
  await writeFile('test-results/founder-friction.json', `${JSON.stringify(metrics, null, 2)}\n`)
  await browser.close()
}
