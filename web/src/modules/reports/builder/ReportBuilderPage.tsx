import { useQueryClient } from '@tanstack/react-query'
import { ArrowLeft, Download, LayoutDashboard, Save, Trash2 } from 'lucide-react'
import { useEffect, useId, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useParams } from 'react-router'
import { isApiError, localizeViolationMessage } from '@/api'
import { useSession } from '@/app/session/session-context'
import { ConfirmDialog, PageLoader, notifyError, notifySuccess, useUnsavedChangesGuard } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { Textarea } from '@/components/ui/textarea'
import { dashboardApi, dashboardKeys } from '@/modules/dashboard/api'
import { useFieldSchema } from '@/features/records/api'
import { fieldViolations } from '@/features/records/fields'
import { serializeFilter, toDraft, useFilterSchema, type DraftGroup } from '@/features/records/filters'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { useViewTeams } from '@/features/records/views'
import { useAggregate, useAggregateSchema } from '@/features/records/aggregates'
import { downloadReport, useReport, useReportMutations, type ExportFormat, type Report } from './api'
import {
  FiltersSection,
  GroupsSection,
  MetricsSection,
  PeriodSection,
  Section,
  SortSection,
  VisualizationSection,
} from './BuilderSections'
import { useGroupOptions } from './options'
import {
  MAX_REPORT_NAME_LENGTH,
  REPORT_RECORD_TYPES,
  draftFromReport,
  draftQuery,
  draftSettings,
  emptyDraft,
  isPeriodComplete,
  serializeDraft,
  type ReportDraft,
} from './model'
import { ReportResultView } from './ReportResultView'
import { SchedulesPanel } from './SchedulesPanel'
import { Alert } from '@/components/ui/alert'
import { Field } from '@/components/ui/field'

const SETTINGS_MANAGE = 'settings.manage'
const PREVIEW_DELAY_MS = 400

function useDebounced<T>(value: T, delay: number): T {
  const [debounced, setDebounced] = useState(value)
  useEffect(() => {
    const timer = window.setTimeout(() => setDebounced(value), delay)
    return () => window.clearTimeout(timer)
  }, [value, delay])
  return debounced
}

export function ReportBuilderPage() {
  const { reportId } = useParams()
  const { t } = useTranslation('reports', { keyPrefix: 'builder' })
  const report = useReport(reportId ?? null)

  if (!reportId) return <ReportEditor key="new" report={null} />
  if (report.isPending) return <PageLoader />
  if (report.isError) {
    if (isApiError(report.error) && report.error.status === 404) {
      return (
        <EmptyState
          title={t('builder.notFound.title')}
          description={t('builder.notFound.description')}
          action={
            <Button asChild variant="outline">
              <Link to="/reports/saved">{t('builder.back')}</Link>
            </Button>
          }
        />
      )
    }
    return <ErrorState onRetry={() => report.refetch()} />
  }
  return <ReportEditor key={report.data.id} report={report.data} />
}

function ReportEditor({ report }: { report: Report | null }) {
  const { t } = useTranslation('reports', { keyPrefix: 'builder' })
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const { me, hasPermission } = useSession()
  const canManage = hasPermission(SETTINGS_MANAGE)
  const allowedTypes = REPORT_RECORD_TYPES.filter((type) => hasPermission(type.permission))
  const [initial] = useState(() =>
    report ? draftFromReport(report) : emptyDraft(allowedTypes[0]?.recordType ?? 'deal'),
  )
  const [draft, setDraft] = useState<ReportDraft>(initial)
  const [baseline, setBaseline] = useState(() => serializeDraft(initial))
  const [filterDraft, setFilterDraft] = useState<DraftGroup>(() => toDraft(initial.filter))
  const [confirmDelete, setConfirmDelete] = useState(false)
  const [exporting, setExporting] = useState(false)
  const [addingToDashboard, setAddingToDashboard] = useState(false)
  const leaving = useRef(false)
  const mutations = useReportMutations()
  const ids = { name: useId(), description: useId(), visibility: useId(), team: useId(), module: useId() }
  const editable = report ? report.can_edit : true
  const dirty = serializeDraft(draft) !== baseline

  const guard = useUnsavedChangesGuard(() => editable && dirty && !leaving.current)
  const aggregateSchema = useAggregateSchema(draft.recordType)
  const fieldSchema = useFieldSchema(draft.recordType)
  const { schema: filterSchema, extensions: filterExtensions } = useFilterSchema(fieldSchema.data)
  const groupOptions = useGroupOptions(aggregateSchema.data)
  const fields = aggregateSchema.data?.fields ?? []
  const teams = useViewTeams(draft.visibility === 'team')

  const update = (change: Partial<ReportDraft>) => setDraft((current) => ({ ...current, ...change }))

  const query = draftQuery(draft)
  const ready =
    query.metrics !== undefined &&
    query.metrics !== null &&
    query.metrics.length === draft.metrics.length &&
    (query.group_by?.length ?? 0) === draft.groupBy.length &&
    isPeriodComplete(draft.period)
  const previewQuery = useDebounced(ready ? query : null, PREVIEW_DELAY_MS)
  const preview = useAggregate(previewQuery)

  const visibilities: ReportDraft['visibility'][] = ['private']
  if (teams.canShare || draft.visibility === 'team') visibilities.push('team')
  if (canManage || draft.visibility === 'organization') visibilities.push('organization')
  const knownTeams = teams.data ?? []
  const teamOptions =
    draft.teamId && !knownTeams.some((team) => team.id === draft.teamId)
      ? [...knownTeams, { id: draft.teamId, name: draft.teamId }]
      : knownTeams

  const trimmedName = draft.name.trim()
  const nameError = !trimmedName
    ? t('builder.nameRequired')
    : trimmedName.length > MAX_REPORT_NAME_LENGTH
      ? t('builder.nameTooLong', { max: MAX_REPORT_NAME_LENGTH })
      : null
  const teamError = draft.visibility === 'team' && !draft.teamId ? t('builder.teamRequired') : null
  const [submitted, setSubmitted] = useState(false)
  const saving = mutations.create.isPending || mutations.update.isPending

  const changeRecordType = (recordType: string) => {
    setDraft({
      ...emptyDraft(recordType),
      name: draft.name,
      description: draft.description,
      visibility: draft.visibility,
      teamId: draft.teamId,
      visualization: draft.visualization,
    })
    setFilterDraft(toDraft(null))
  }

  const changeFilter = (next: DraftGroup) => {
    setFilterDraft(next)
    if (filterSchema) update({ filter: serializeFilter(next, filterSchema) })
  }

  const save = () => {
    setSubmitted(true)
    if (nameError || teamError || !ready) return
    const body = {
      name: trimmedName,
      description: draft.description.trim() || null,
      visibility: draft.visibility,
      team_id: draft.visibility === 'team' ? draft.teamId : null,
      visualization: draft.visualization,
      definition: query,
      settings: draftSettings(draft),
    }
    const snapshot = serializeDraft(draft)
    const onError = (error: unknown) => notifyError(error)
    if (report) {
      mutations.update.mutate(
        { id: report.id, request: body, version: report.updated_at },
        {
          onSuccess: () => {
            setBaseline(snapshot)
            notifySuccess(t('builder.saved'))
          },
          onError,
        },
      )
    } else {
      mutations.create.mutate(body, {
        onSuccess: (created) => {
          leaving.current = true
          setBaseline(snapshot)
          notifySuccess(t('builder.saved'))
          navigate(`/reports/saved/${created.id}`, { replace: true })
        },
        onError,
      })
    }
  }

  const exportAs = async (format: ExportFormat) => {
    if (!report) return
    setExporting(true)
    try {
      await downloadReport(report.id, format, {})
    } catch (error) {
      notifyError(error)
    } finally {
      setExporting(false)
    }
  }

  const addToDashboard = async () => {
    if (!report) return
    setAddingToDashboard(true)
    try {
      const layout = await dashboardApi.layout()
      const id = `report-${report.id.slice(-12)}-${Date.now().toString(36)}`.slice(0, 64)
      await dashboardApi.saveLayout({
        period: layout.period,
        widgets: [
          ...layout.widgets,
          { id, key: 'saved_report', size: 'large', settings: { report_id: report.id, use_dashboard_period: true } },
        ],
      })
      await queryClient.invalidateQueries({ queryKey: dashboardKeys.layout() })
      notifySuccess(t('builder.addedToDashboard'))
    } catch (error) {
      notifyError(error)
    } finally {
      setAddingToDashboard(false)
    }
  }

  const removeReport = async () => {
    if (!report) return
    await mutations.remove.mutateAsync(report.id)
    leaving.current = true
    notifySuccess(t('builder.deleted'))
    navigate('/reports/saved', { replace: true })
  }

  if (allowedTypes.length === 0) {
    return <EmptyState title={t('builder.noModules.title')} description={t('builder.noModules.description')} />
  }

  const previewError = preview.error
  const violations = previewError && isApiError(previewError) ? fieldViolations(previewError.details) : []
  const previewEmpty =
    preview.data !== undefined && preview.data.rows.length === 0 && preview.data.groups.length > 0

  return (
    <div className="grid gap-4">
      {guard.dialog}
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex min-w-0 items-center gap-2">
          <Button asChild variant="ghost" size="icon" aria-label={t('builder.back')}>
            <Link to="/reports/saved">
              <ArrowLeft />
            </Link>
          </Button>
          <h2 className="min-w-0 truncate text-lg font-semibold">
            {report ? report.name : t('builder.newTitle')}
          </h2>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          {report && (
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button type="button" variant="outline" size="sm" disabled={exporting}>
                  <Download />
                  {t('builder.export')}
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem onSelect={() => void exportAs('xlsx')}>{t('builder.exportXlsx')}</DropdownMenuItem>
                <DropdownMenuItem onSelect={() => void exportAs('csv')}>{t('builder.exportCsv')}</DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          )}
          {report && (
            <Button type="button" variant="outline" size="sm" disabled={addingToDashboard || dirty} onClick={() => void addToDashboard()}>
              <LayoutDashboard />
              {t('builder.addToDashboard')}
            </Button>
          )}
          {report?.can_edit && (
            <Button type="button" variant="outline" size="sm" onClick={() => setConfirmDelete(true)}>
              <Trash2 />
              {t('builder.delete')}
            </Button>
          )}
          {editable && (
            <Button type="button" size="sm" disabled={saving || (report !== null && !dirty)} onClick={save}>
              <Save />
              {saving ? t('common.saving') : t('common.save')}
            </Button>
          )}
        </div>
      </div>
      {!editable && <p className="text-sm text-muted-foreground">{t('builder.readOnly')}</p>}

      <div className="grid gap-4 lg:grid-cols-[minmax(0,26rem)_minmax(0,1fr)] lg:items-start">
        <fieldset disabled={!editable} className="grid min-w-0 gap-4">
          <Section title={t('sections.details.title')}>
            <div className="grid gap-2.5">
              <Label htmlFor={ids.name}>{t('sections.details.name')}</Label>
              <Input
                id={ids.name}
                value={draft.name}
                maxLength={MAX_REPORT_NAME_LENGTH + 10}
                placeholder={t('sections.details.namePlaceholder')}
                aria-invalid={submitted && Boolean(nameError)}
                onChange={(event) => update({ name: event.target.value })}
              />
              {submitted && nameError && <p className="text-sm text-destructive">{nameError}</p>}
            </div>
            <Field label={t('sections.details.description')}>
              {(control) => <Textarea placeholder={t('placeholders.description')}
                {...control}
                rows={2}
                value={draft.description}
                onChange={(event) => update({ description: event.target.value })}
              />}
            </Field>
            <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-1 xl:grid-cols-2">
              <div className="grid gap-2.5">
                <Label htmlFor={ids.visibility}>{t('sections.details.visibility')}</Label>
                <NativeSelect
                  id={ids.visibility}
                  value={draft.visibility}
                  onChange={(event) => update({ visibility: event.target.value as ReportDraft['visibility'] })}
                >
                  {visibilities.map((visibility) => (
                    <option key={visibility} value={visibility}>
                      {t(`visibility.${visibility}`)}
                    </option>
                  ))}
                </NativeSelect>
              </div>
              {draft.visibility === 'team' && (
                <div className="grid gap-2.5">
                  <Label htmlFor={ids.team}>{t('sections.details.team')}</Label>
                  <NativeSelect id={ids.team} value={draft.teamId} onChange={(event) => update({ teamId: event.target.value })}>
                    <option value="">{t('common.choose')}</option>
                    {teamOptions.map((team) => (
                      <option key={team.id} value={team.id}>
                        {team.name}
                      </option>
                    ))}
                  </NativeSelect>
                  {submitted && teamError && <p className="text-sm text-destructive">{teamError}</p>}
                </div>
              )}
            </div>
          </Section>
          <Section title={t('sections.module.title')} description={t('sections.module.description')}>
            <Label htmlFor={ids.module} className="sr-only">
              {t('sections.module.title')}
            </Label>
            <NativeSelect id={ids.module} value={draft.recordType} onChange={(event) => changeRecordType(event.target.value)}>
              {allowedTypes.map((type) => (
                <option key={type.recordType} value={type.recordType}>
                  {t(`modules.${type.recordType}`)}
                </option>
              ))}
            </NativeSelect>
          </Section>
          {aggregateSchema.isError ? (
            <ErrorState onRetry={() => aggregateSchema.refetch()} />
          ) : (
            <>
              <GroupsSection draft={draft} onChange={update} options={groupOptions} />
              <MetricsSection draft={draft} onChange={update} fields={fields} />
              <FiltersSection
                schema={filterSchema}
                value={filterDraft}
                onChange={changeFilter}
                extensions={filterExtensions}
              />
              <PeriodSection draft={draft} onChange={update} fields={fields} />
              <SortSection draft={draft} onChange={update} options={groupOptions} fields={fields} />
              <VisualizationSection draft={draft} onChange={update} fields={fields} />
            </>
          )}
        </fieldset>

        <div className="grid min-w-0 gap-4 lg:sticky lg:top-4">
          <Section title={t('preview.title')} description={t('preview.description')}>
            <div aria-live="polite" aria-busy={preview.isFetching} className="grid min-w-0 gap-3">
              {!ready && <p className="text-sm text-muted-foreground">{t('preview.incomplete')}</p>}
              {ready && preview.isPending && <Skeleton className="h-64 w-full" />}
              {ready && previewError && (
                <Alert className="grid gap-1">
                  <p className="font-medium">{t('preview.error')}</p>
                  {violations.length > 0 ? (
                    <ul className="list-inside list-disc text-muted-foreground">
                      {violations.map((violation) => (
                        <li key={`${violation.field}:${violation.message}`}>
                          <code>{violation.field}</code>: {localizeViolationMessage(violation.message)}
                        </li>
                      ))}
                    </ul>
                  ) : (
                    <p className="text-muted-foreground">{isApiError(previewError) ? previewError.message : String(previewError)}</p>
                  )}
                </Alert>
              )}
              {ready && !previewError && preview.data && (
                <>
                  {previewEmpty ? (
                    <EmptyState title={t('preview.emptyTitle')} description={t('preview.emptyDescription')} />
                  ) : (
                    <ReportResultView
                      result={preview.data}
                      visualization={draft.visualization}
                      chartMetric={draft.chartMetric}
                    />
                  )}
                  {preview.data.truncated && <p className="text-xs text-muted-foreground">{t('preview.truncated')}</p>}
                </>
              )}
            </div>
          </Section>
          {report?.can_edit && <SchedulesPanel report={report} currentUserId={me.user.id} />}
        </div>
      </div>
      <ConfirmDialog
        open={confirmDelete}
        onOpenChange={setConfirmDelete}
        title={t('builder.deleteTitle')}
        description={t('builder.deleteDescription')}
        confirmLabel={t('builder.delete')}
        variant="destructive"
        pending={mutations.remove.isPending}
        onConfirm={removeReport}
      />
    </div>
  )
}
