import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { Field } from '@/components/ui/field'
import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { useMutation } from '@tanstack/react-query'
import { AlertTriangle, MoreHorizontal, Pause, Play, Rocket, Save, Trash2 } from 'lucide-react'
import { useActiveTab, useCustomObjects, useFieldSchema } from '@/sdk'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useParams } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { isApiError } from '@/api'
import { ConfirmDialog, PageLoader, UnsavedChangesGuard, notifyError, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { SectionWorkspace, SectionContent, SectionIndex, SectionLink } from '@/components/ui/section-workspace'
import { AUTOMATIONS_PATH, automationsApi, useAutomation, useAutomationCache, useAutomationCatalog } from './api'
import { AUTOMATIONS_PERMISSION } from './AutomationsPage'
import { IssueList } from './components/IssueList'
import { AutomationStatusBadge } from './components/StatusBadges'
import { FlowEditor, type FlowSelection } from './editor/FlowEditor'
import { StepPanel } from './editor/StepPanel'
import { createStep, type NewStepKind } from './editor/steps'
import { useStepSummary } from './editor/summary'
import { TriggerPanel } from './editor/TriggerPanel'
import { EMPTY_REPORT, VERSION_CONFLICT, issueCount, reportFromError, type ValidationReport } from './lib/errors'
import {
  collectStepIds,
  collectSteps,
  definitionToFlow,
  findStepNode,
  flowToDefinition,
  hasDescendants,
  insertStep,
  removeJump,
  removeStep,
  updateStep,
  type FlowNode,
  type InsertionPoint,
} from './lib/flow'
import { automatableRecordTypes, useAutomationLabels } from './lib/labels'
import { useTemplateVariables } from './lib/variables'
import { DryRunPanel } from './panels/DryRunPanel'
import { RunsPanel } from './panels/RunsPanel'
import { VersionsPanel } from './panels/VersionsPanel'
import type {
  Automation,
  AutomationCatalog,
  AutomationDefinition,
  AutomationPayload,
  AutomationVersion,
  DefinitionOptions,
  StepSpec,
  TriggerSpec,
} from './types'
import { Alert } from '@/components/ui/alert'


interface Draft {
  name: string
  description: string
  trigger: TriggerSpec
  nodes: FlowNode[]
  options: DefinitionOptions
}

function draftOf(source: Pick<Automation, 'name' | 'trigger' | 'definition'> & { description?: string | null }): Draft {
  return {
    name: source.name,
    description: source.description ?? '',
    trigger: source.trigger,
    nodes: definitionToFlow(source.definition),
    options: source.definition.options ?? {},
  }
}

function payloadOf(draft: Draft): AutomationPayload & { definition: AutomationDefinition } {
  return {
    name: draft.name.trim(),
    description: draft.description.trim() || null,
    trigger: draft.trigger,
    definition: flowToDefinition(draft.nodes, draft.options),
  }
}

function snapshotOf(draft: Draft): string {
  return JSON.stringify(payloadOf(draft))
}

export function AutomationPage() {
  const { t } = useTranslation('automations')
  const { id = '' } = useParams()
  const { hasPermission } = useSession()
  const automation = useAutomation(id)
  const catalog = useAutomationCatalog()

  if (!hasPermission(AUTOMATIONS_PERMISSION)) {
    return <EmptyState title={t('page.noAccessTitle')} description={t('page.noAccessDescription')} />
  }
  if (automation.isPending || catalog.isPending) return <PageLoader />
  if (automation.isError) {
    if (isApiError(automation.error) && automation.error.status === 404) {
      return (
        <EmptyState
          title={t('editor.notFoundTitle')}
          description={t('editor.notFoundDescription')}
          action={
            <Button asChild variant="outline" size="sm">
              <Link to={AUTOMATIONS_PATH}>{t('editor.back')}</Link>
            </Button>
          }
        />
      )
    }
    return <ErrorState onRetry={() => void automation.refetch()} />
  }
  if (catalog.isError) return <ErrorState onRetry={() => void catalog.refetch()} />
  return <AutomationWorkspace key={automation.data.id} initial={automation.data} catalog={catalog.data} />
}

function AutomationWorkspace({ initial, catalog }: { initial: Automation; catalog: AutomationCatalog }) {
  const { t } = useTranslation('automations')
  const navigate = useNavigate()
  const cache = useAutomationCache()
  const labels = useAutomationLabels(catalog)
  const objects = useCustomObjects()
  const [automation, setAutomation] = useState(initial)
  const [draft, setDraft] = useState<Draft>(() => draftOf(initial))
  const [saved, setSaved] = useState(() => snapshotOf(draftOf(initial)))
  const [report, setReport] = useState<ValidationReport>(EMPTY_REPORT)
  const [selection, setSelection] = useState<FlowSelection | null>(null)
  const [tab, setTab] = useActiveTab(['flow', 'test', 'runs', 'versions'], 'flow')
  const [pendingRemoval, setPendingRemoval] = useState<string | null>(null)
  const [confirmDelete, setConfirmDelete] = useState(false)
  const [conflict, setConflict] = useState(false)

  const recordType = automation.record_type
  const schema = useFieldSchema(recordType)
  const variables = useTemplateVariables(recordType, schema.data, catalog)
  const summary = useStepSummary(schema.data, catalog)
  const recordTypes = useMemo(
    () =>
      automatableRecordTypes(
        catalog,
        (objects.data ?? []).filter((object) => object.is_active).map((object) => object.record_type),
      ),
    [catalog, objects.data],
  )

  const snapshot = snapshotOf(draft)
  const dirty = snapshot !== saved
  const draftSteps = useMemo(() => collectSteps(draft.nodes), [draft.nodes])
  const selectedNode = selection?.kind === 'step' ? findStepNode(draft.nodes, selection.id) : null

  const handleFailure = (error: unknown, definition: AutomationDefinition) => {
    if (isApiError(error) && error.code === VERSION_CONFLICT) {
      setConflict(true)
      return
    }
    const mapped = reportFromError(error, definition)
    if (mapped) {
      setReport(mapped)
      notifyError(error, { title: t('toast.invalid', { count: issueCount(mapped) }) })
      return
    }
    notifyError(error)
  }

  const saveDraft = async (): Promise<Automation | null> => {
    const payload = payloadOf(draft)
    const sentSnapshot = snapshotOf(draft)
    try {
      const updated = await automationsApi.update(automation.id, payload, automation.updated_at)
      setAutomation(updated)
      setSaved(sentSnapshot)
      setReport(EMPTY_REPORT)
      await cache.store(updated)
      return updated
    } catch (error) {
      handleFailure(error, payload.definition)
      return null
    }
  }

  const save = useMutation({
    mutationFn: saveDraft,
    onSuccess: (updated) => {
      if (updated) notifySuccess(t('toast.saved'))
    },
  })

  const publish = useMutation({
    mutationFn: async () => {
      const current = dirty ? await saveDraft() : automation
      if (!current) return null
      const definition = payloadOf(draft).definition
      try {
        return await automationsApi.publish(current.id, current.updated_at)
      } catch (error) {
        handleFailure(error, definition)
        return null
      }
    },
    onSuccess: async (published) => {
      if (!published) return
      setAutomation(published)
      setReport(EMPTY_REPORT)
      await cache.store(published)
      void cache.invalidateVersions(published.id)
      notifySuccess(t('toast.published', { version: published.version }))
    },
  })

  const toggle = useMutation({
    mutationFn: () => (automation.status === 'active' ? automationsApi.pause(automation.id) : automationsApi.activate(automation.id)),
    onSuccess: async (updated) => {
      setAutomation(updated)
      await cache.store(updated)
      notifySuccess(updated.status === 'active' ? t('toast.activated') : t('toast.paused'))
    },
    onError: (error) => notifyError(error),
  })

  const reload = async () => {
    const fresh = await automationsApi.get(automation.id)
    setAutomation(fresh)
    const next = draftOf(fresh)
    setDraft(next)
    setSaved(snapshotOf(next))
    setReport(EMPTY_REPORT)
    setSelection(null)
    await cache.store(fresh)
  }

  const setNodes = (update: (nodes: FlowNode[]) => FlowNode[]) => setDraft((current) => ({ ...current, nodes: update(current.nodes) }))

  const insert = (point: InsertionPoint, kind: NewStepKind) => {
    const taken = collectStepIds(draft.nodes)
    const step = createStep(kind, taken, {
      actions: catalog.actions,
      recordHasOwner: Boolean(schema.data?.fields.some((field) => field.key === 'owner_id')),
      caseLabel: t('flow.yes'),
    })
    setNodes((nodes) => insertStep(nodes, point, step))
    setSelection({ kind: 'step', id: step.id })
  }

  const changeStep = (step: StepSpec) => setNodes((nodes) => updateStep(nodes, step.id, step))

  const requestRemoval = (stepId: string) => {
    const node = findStepNode(draft.nodes, stepId)
    if (node && hasDescendants(node)) {
      setPendingRemoval(stepId)
      return
    }
    removeNow(stepId)
  }

  const removeNow = (stepId: string) => {
    setNodes((nodes) => removeStep(nodes, stepId))
    setSelection(null)
    setPendingRemoval(null)
  }

  const restore = (version: AutomationVersion) => {
    setDraft((current) => ({ ...draftOf({ ...version, description: current.description }) }))
    setSelection(null)
    setTab('flow')
    notifySuccess(t('toast.restored', { version: version.version }))
  }

  const busy = save.isPending || publish.isPending || toggle.isPending
  const generalIssues = [...report.general, ...report.details]
  const totalIssues = issueCount(report)
  const panelTitle =
    selection?.kind === 'trigger'
      ? t('panel.triggerTitle')
      : selectedNode
        ? summary(selectedNode.step).title
        : ''

  return (
    <Page>
      <PageHeader title={draft.name || t('placeholders.name')} back={{ to: AUTOMATIONS_PATH, label: t('editor.back') }} badges={<>
        <AutomationStatusBadge status={automation.status} />
        <span className="text-xs text-muted-foreground">{automation.version > 0 ? t('page.version', { version: automation.version }) : t('editor.neverPublished')}</span>
        {dirty && <span className="text-xs font-medium text-warning">{t('editor.unsaved')}</span>}
      </>} description={<p>{t('editor.recordType', { type: labels.recordTypeLabel(recordType) })} · <code>{automation.key}</code></p>} actions={<>

            <Button type="button" variant="outline" disabled={!dirty || busy} onClick={() => save.mutate()}>
              <Save aria-hidden="true" />
              {save.isPending ? t('editor.saving') : t('editor.save')}
            </Button>
            <Button type="button" disabled={busy} onClick={() => publish.mutate()}>
              <Rocket aria-hidden="true" />
              {publish.isPending ? t('editor.publishing') : t('editor.publish')}
            </Button>
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button type="button" variant="ghost" size="icon" aria-label={t('editor.more')}>
                  <MoreHorizontal aria-hidden="true" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                {automation.version > 0 && <DropdownMenuItem disabled={busy} onSelect={() => toggle.mutate()}>
                  {automation.status === 'active' ? <Pause aria-hidden /> : <Play aria-hidden />}
                  {automation.status === 'active' ? t('actionsBar.pause') : t('actionsBar.activate')}
                </DropdownMenuItem>}
                <DropdownMenuItem className="text-destructive" onSelect={() => setConfirmDelete(true)}>
                  <Trash2 aria-hidden="true" />
                  {t('editor.delete')}
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
                </>} />
      <UnsavedChangesGuard when={dirty} />
      <div className="grid gap-3">
        <details className="rounded-md border p-3">
          <summary className="cursor-pointer text-sm font-medium">{t('editor.settings')}</summary>
          <div className="grid gap-3 pt-3">
        <Field label={t('editor.name')}>
          {(control) => <Input {...control} placeholder={t('placeholders.name')} value={draft.name} onChange={(event) => setDraft((current) => ({ ...current, name: event.target.value }))} maxLength={120} className="max-w-md" />}
        </Field>
        <Field label={t('editor.description')}>
          {(control) => <Input {...control} placeholder={t('editor.descriptionPlaceholder')} value={draft.description} onChange={(event) => setDraft((current) => ({ ...current, description: event.target.value }))} maxLength={1000} className="max-w-2xl" />}
        </Field>
          </div>
        </details>
        {totalIssues > 0 && (
          <Alert className="grid gap-2">
            <SharedFieldError className="flex items-center gap-2 font-medium">
              <AlertTriangle className="size-4" aria-hidden="true" />
              {t('editor.issues', { count: totalIssues })}
            </SharedFieldError>
            <IssueList issues={generalIssues} showPath />
          </Alert>
        )}
      </div>
      <SectionWorkspace value={tab} onValueChange={setTab}>
        <SectionIndex>
          <SectionLink value="flow">{t('tabs.flow')}</SectionLink>
          <SectionLink value="test">{t('tabs.test')}</SectionLink>
          <SectionLink value="runs">{t('tabs.runs')}</SectionLink>
          <SectionLink value="versions">{t('tabs.versions')}</SectionLink>
        </SectionIndex>
        <SectionContent value="flow" className="pt-4">
          <p className="mb-2 text-xs text-muted-foreground">{t('editor.flowHint')}</p>
          <FlowEditor
            recordType={recordType}
            trigger={draft.trigger}
            nodes={draft.nodes}
            catalog={catalog}
            schema={schema.data}
            report={report}
            selection={selection}
            onSelect={setSelection}
            onInsert={insert}
            onRemoveJump={(uid) => setNodes((nodes) => removeJump(nodes, uid))}
          />
        </SectionContent>
        <SectionContent value="test" className="pt-4">
          <DryRunPanel
            automation={automation}
            catalog={catalog}
            schema={schema.data}
            draftSteps={draftSteps}
            dirty={dirty}
            onSaveDraft={saveDraft}
          />
        </SectionContent>
        <SectionContent value="runs" className="pt-4">
          <RunsPanel automation={automation} catalog={catalog} schema={schema.data} draftSteps={draftSteps} />
        </SectionContent>
        <SectionContent value="versions" className="pt-4">
          <VersionsPanel automation={automation} catalog={catalog} schema={schema.data} onRestore={restore} />
        </SectionContent>
      </SectionWorkspace>
      <Dialog open={selection !== null && (selection.kind === 'trigger' || selectedNode !== null)} onOpenChange={(open) => !open && setSelection(null)}>
        <DialogContent side="right" className="w-full max-w-full sm:w-[34rem] sm:max-w-[92vw]">
          <DialogHeader className="sticky top-0 border-b bg-background p-4 pr-12">
            <DialogTitle className="truncate">{panelTitle}</DialogTitle>
            <DialogDescription>
              {selection?.kind === 'trigger' ? t('panel.triggerDescription') : t('panel.stepDescription')}
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-6 p-4">
            {!schema.data ? (
              <PageLoader />
            ) : selection?.kind === 'trigger' ? (
              <TriggerPanel
                trigger={draft.trigger}
                options={draft.options}
                recordType={recordType}
                schema={schema.data}
                catalog={catalog}
                variables={variables}
                recordTypes={recordTypes}
                issues={report.trigger}
                onChange={(trigger) => setDraft((current) => ({ ...current, trigger }))}
                onOptionsChange={(options) => setDraft((current) => ({ ...current, options }))}
              />
            ) : selectedNode ? (
              <>
                <StepPanel
                  key={selectedNode.step.id}
                  step={selectedNode.step}
                  recordType={recordType}
                  schema={schema.data}
                  catalog={catalog}
                  variables={variables}
                  recordTypes={recordTypes}
                  issues={report.steps[selectedNode.step.id] ?? []}
                  onChange={changeStep}
                />
                <div className="flex justify-between gap-2 border-t pt-4">
                  <Button type="button" variant="ghost" className="text-destructive" onClick={() => requestRemoval(selectedNode.step.id)}>
                    <Trash2 aria-hidden="true" />
                    {t('panel.removeStep')}
                  </Button>
                  <Button type="button" variant="secondary" onClick={() => setSelection(null)}>
                    {t('panel.done')}
                  </Button>
                </div>
              </>
            ) : null}
          </div>
        </DialogContent>
      </Dialog>
      <ConfirmDialog
        open={pendingRemoval !== null}
        onOpenChange={(open) => !open && setPendingRemoval(null)}
        title={t('panel.removeBranchTitle')}
        description={t('panel.removeBranchDescription')}
        confirmLabel={t('panel.removeStep')}
        variant="destructive"
        onConfirm={() => {
          if (pendingRemoval) removeNow(pendingRemoval)
        }}
      />
      <ConfirmDialog
        open={confirmDelete}
        onOpenChange={setConfirmDelete}
        title={t('editor.deleteTitle', { name: automation.name })}
        description={t('editor.deleteDescription')}
        confirmLabel={t('editor.delete')}
        variant="destructive"
        onConfirm={async () => {
          await automationsApi.remove(automation.id)
          setSaved(snapshot)
          await cache.invalidateAll()
          notifySuccess(t('toast.deleted', { name: automation.name }))
          void navigate(AUTOMATIONS_PATH, { replace: false })
        }}
      />
      <ConfirmDialog
        open={conflict}
        onOpenChange={setConflict}
        title={t('editor.conflictTitle')}
        description={t('editor.conflictDescription')}
        confirmLabel={t('editor.reload')}
        onConfirm={reload}
      />
    </Page>
  )
}
