import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { Clock, RefreshCw } from 'lucide-react'
import { useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { RecordFieldInput } from '@/components/fields'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import type { FieldSchema } from '@/features/records/types'
import { RUNS_PER_PAGE, useAutomationRun, useAutomationRuns, useAutomationVersions } from '../api'
import { Pager } from '../components/Pager'
import { RecordName } from '../components/RecordName'
import { RunStatusBadge } from '../components/StatusBadges'
import { StepTimeline } from '../components/StepTimeline'
import { useStepSummary } from '../editor/summary'
import { useAutomationLabels } from '../lib/labels'
import { RUN_STATUSES, type Automation, type AutomationCatalog, type AutomationRunDetails, type RunStatus, type StepSpec } from '../types'
import { Field } from '@/components/ui/field'

export interface RunsPanelProps {
  automation: Automation
  catalog: AutomationCatalog
  schema: FieldSchema | undefined
  draftSteps: StepSpec[]
}

export function RunsPanel({ automation, catalog, schema, draftSteps }: RunsPanelProps) {
  const { t } = useTranslation('automations')
  const formatting = useRecordFormatting()
  const labels = useAutomationLabels(catalog)
  const [status, setStatus] = useState<RunStatus | ''>('')
  const [record, setRecord] = useState<string | null>(null)
  const [page, setPage] = useState(1)
  const [openRun, setOpenRun] = useState<string | null>(null)
  const runs = useAutomationRuns(automation.id, { status: status || undefined, record_id: record ?? undefined, page })
  const versions = useAutomationVersions(automation.id)
  const stepsOf = (version: number) =>
    versions.data?.find((item) => item.version === version)?.definition.steps ?? draftSteps

  let body
  if (runs.isPending) {
    body = (
      <div className="grid gap-2" aria-busy="true">
        <Skeleton className="h-16 w-full" />
        <Skeleton className="h-16 w-full" />
      </div>
    )
  } else if (runs.isError) {
    body = <ErrorState onRetry={() => void runs.refetch()} />
  } else if (runs.data.data.length === 0) {
    body = <EmptyState title={t('runs.emptyTitle')} description={t('runs.emptyDescription')} />
  } else {
    body = (
      <>
        <ul className="grid gap-2" aria-label={t('runs.listLabel')}>
          {runs.data.data.map((run) => (
            <li key={run.id}>
              <button
                type="button"
                onClick={() => setOpenRun(run.id)}
                className="grid w-full gap-1 rounded-lg border bg-card p-3 text-left transition-colors hover:border-primary/60 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none sm:grid-cols-[auto_minmax(0,1fr)_auto] sm:items-center sm:gap-3"
              >
                <RunStatusBadge status={run.status} />
                <span className="grid min-w-0 gap-0.5">
                  <span className="truncate text-sm font-medium">
                    <RecordName recordType={run.record_type} recordId={run.record_id} link={false} />
                  </span>
                  <span className="truncate text-xs text-muted-foreground">
                    {labels.triggerLabel(run.trigger_type)} · {t('runs.version', { version: run.version })}
                    {run.error ? ` · ${run.error}` : ''}
                    {run.reason ? ` · ${t(`skipReasons.${run.reason}` as 'skipReasons.max_depth', { defaultValue: run.reason })}` : ''}
                  </span>
                  {run.status === 'waiting' && run.resume_at && (
                    <span className="flex items-center gap-1 text-xs text-warning">
                      <Clock className="size-3" aria-hidden="true" />
                      {t('runs.resumeAt', { date: formatDateTime(run.resume_at, formatting) })}
                    </span>
                  )}
                </span>
                <span className="text-xs text-muted-foreground tabular-nums">{formatDateTime(run.started_at, formatting)}</span>
              </button>
            </li>
          ))}
        </ul>
        <Pager page={page} perPage={RUNS_PER_PAGE} total={runs.data.meta.total} onPageChange={setPage} />
      </>
    )
  }

  return (
    <div className="grid gap-4">
      <div className="flex flex-wrap items-end gap-3">
        <Field label={t('runs.status')}>
          {(control) => <NativeSelect
            {...control}
            value={status}
            onChange={(event) => {
              setStatus(event.target.value as RunStatus | '')
              setPage(1)
            }}
            className="w-44"
          >
            <option value="">{t('runs.allStatuses')}</option>
            {RUN_STATUSES.map((item) => (
              <option key={item} value={item}>
                {t(`runStatus.${item}`)}
              </option>
            ))}
          </NativeSelect>}
        </Field>
        <Field label={t('runs.record')} className="min-w-0 flex-1 sm:max-w-xs">
          {(fieldControl) => <RecordFieldInput
            id={fieldControl.id}
            targetRecordType={automation.record_type}
            value={record}
            onChange={(next) => {
              setRecord(next)
              setPage(1)
            }}
          />}
        </Field>
        <Button type="button" variant="outline" size="icon" aria-label={t('runs.refresh')} onClick={() => void runs.refetch()}>
          <RefreshCw aria-hidden="true" className={runs.isFetching ? 'animate-spin' : undefined} />
        </Button>
      </div>
      {body}
      <RunDetailDialog
        automation={automation}
        runId={openRun}
        onClose={() => setOpenRun(null)}
        catalog={catalog}
        schema={schema}
        stepsOf={stepsOf}
      />
    </div>
  )
}

interface RunDetailDialogProps {
  automation: Automation
  runId: string | null
  onClose: () => void
  catalog: AutomationCatalog
  schema: FieldSchema | undefined
  stepsOf: (version: number) => StepSpec[]
}

function RunDetailDialog({ automation, runId, onClose, catalog, schema, stepsOf }: RunDetailDialogProps) {
  const { t } = useTranslation('automations')
  const run = useAutomationRun(automation.id, runId)
  return (
    <Dialog open={Boolean(runId)} onOpenChange={(open) => !open && onClose()}>
      <DialogContent size="2xl">
        <DialogHeader>
          <DialogTitle>{t('runs.detailTitle')}</DialogTitle>
          <DialogDescription>{automation.name}</DialogDescription>
        </DialogHeader>
        {run.isPending ? (
          <Skeleton className="h-40 w-full" />
        ) : run.isError ? (
          <ErrorState onRetry={() => void run.refetch()} />
        ) : (
          <RunDetail run={run.data} catalog={catalog} schema={schema} steps={stepsOf(run.data.version)} />
        )}
      </DialogContent>
    </Dialog>
  )
}

function RunDetail({
  run,
  catalog,
  schema,
  steps,
}: {
  run: AutomationRunDetails
  catalog: AutomationCatalog
  schema: FieldSchema | undefined
  steps: StepSpec[]
}) {
  const { t } = useTranslation('automations')
  const formatting = useRecordFormatting()
  const labels = useAutomationLabels(catalog)
  const summary = useStepSummary(schema, catalog)
  const waitingStep = run.waiting_step_id ? steps.find((step) => step.id === run.waiting_step_id) : undefined
  const rows: [string, ReactNode][] = [
    [t('runs.statusLabel'), <RunStatusBadge key="status" status={run.status} />],
    [t('runs.record'), <RecordName key="record" recordType={run.record_type} recordId={run.record_id} />],
    [t('runs.trigger'), labels.triggerLabel(run.trigger_type)],
    [t('runs.versionLabel'), String(run.version)],
    [t('runs.startedAt'), formatDateTime(run.started_at, formatting)],
    [t('runs.finishedAt'), run.finished_at ? formatDateTime(run.finished_at, formatting) : '—'],
    [t('runs.attempts'), String(run.attempts)],
    [t('runs.depth'), String(run.depth)],
  ]
  if (run.reason) {
    rows.push([t('runs.reason'), t(`skipReasons.${run.reason}` as 'skipReasons.max_depth', { defaultValue: run.reason })])
  }
  return (
    <div className="grid gap-4">
      <dl className="grid gap-x-4 gap-y-2 text-sm sm:grid-cols-2">
        {rows.map(([label, value]) => (
          <div key={label} className="grid min-w-0 gap-0.5">
            <dt className="text-xs text-muted-foreground">{label}</dt>
            <dd className="min-w-0 truncate">{value}</dd>
          </div>
        ))}
      </dl>
      {run.status === 'waiting' && (
        <div className="flex items-start gap-2 rounded-md border border-warning/60 bg-warning/10 px-3 py-2 text-sm">
          <Clock className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
          <span>
            {t('runs.waitingInfo', {
              step: waitingStep ? summary(waitingStep).title : (run.waiting_step_id ?? '—'),
              date: run.resume_at ? formatDateTime(run.resume_at, formatting) : '—',
            })}
          </span>
        </div>
      )}
      {run.error && (
        <SharedFieldError className="rounded-md bg-destructive/10 px-3 py-2 break-words">
          {run.error}
        </SharedFieldError>
      )}
      <h3 className="text-sm font-medium">{t('runs.steps')}</h3>
      <StepTimeline
        steps={run.steps}
        definitionSteps={steps}
        catalog={catalog}
        schema={schema}
        waitingStepId={run.waiting_step_id}
      />
    </div>
  )
}
