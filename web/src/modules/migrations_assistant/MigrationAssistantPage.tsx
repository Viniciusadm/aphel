import { ArrowRight, FileUp, Info, RotateCcw } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useSearchParams } from 'react-router'
import { notifyError } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { useImportJob } from '@/modules/imports/api'
import { ListSkeleton, NativeSelect, RequirePermission, SETTINGS_MANAGE, useAllMembers, usePipelines } from '@/sdk'
import { cn } from '@/lib/utils'
import {
  emptyMaps,
  readStepImports,
  stepKind,
  useCreateMigrationImport,
  useMigrateHistory,
  useMigrationSources,
  usePreviewMigrationFile,
  writeStepImports,
} from './api'
import type {
  DateOrder,
  DecimalSeparator,
  MapTarget,
  MigrationPreview,
  MigrationSource,
  MigrationHistoryReport,
  MigrationStep,
  StepKey,
  ValueMaps,
} from './types'
import { PageHeader } from '@/components/ui/page-header'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Page } from '@/components/ui/page'

const SOURCE_PARAM = 'source'
const MAPPABLE: MapTarget[] = ['pipeline_id', 'stage_id', 'owner_id']
const SAMPLE_COLUMNS = 6

export function MigrationAssistantPage() {
  return (
    <RequirePermission permission={SETTINGS_MANAGE}>
      <MigrationAssistantSection />
    </RequirePermission>
  )
}

function useTargetLabel() {
  const { t } = useTranslation('migrations_assistant')
  return (target: string) => t(`targets.${target as 'name'}`, { defaultValue: target })
}

function MigrationAssistantSection() {
  const { t } = useTranslation('migrations_assistant')
  const sources = useMigrationSources()
  const [searchParams, setSearchParams] = useSearchParams()
  const selected = sources.data?.find((source) => source.key === searchParams.get(SOURCE_PARAM))

  const choose = (key: string) =>
    setSearchParams(
      (current) => {
        const params = new URLSearchParams(current)
        params.set(SOURCE_PARAM, key)
        return params
      },
      { replace: true },
    )

  return (
    <Page>
      <PageHeader title={t('page.title')} description={t('page.description')} />
      <p className="flex gap-2 rounded-lg border bg-muted/40 p-3 text-sm text-muted-foreground">
        <Info aria-hidden="true" className="mt-0.5 size-4 shrink-0" />
        <span>{t('page.inferredHeaders')}</span>
      </p>
      {sources.isPending ? (
        <ListSkeleton />
      ) : sources.isError ? (
        <ErrorState onRetry={() => void sources.refetch()} />
      ) : (
        <>
          <div className="grid gap-2">
            <h3 className="text-base font-semibold">{t('source.title')}</h3>
            <div role="radiogroup" aria-label={t('source.title')} className="grid gap-2 sm:grid-cols-2 lg:grid-cols-4">
              {sources.data.map((source) => (
                <button
                  key={source.key}
                  type="button"
                  role="radio"
                  aria-checked={selected?.key === source.key}
                  onClick={() => choose(source.key)}
                  className={cn(
                    'grid gap-1 rounded-lg border bg-card p-3 text-left transition-colors hover:border-primary focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none',
                    selected?.key === source.key && 'border-primary ring-1 ring-primary',
                  )}
                >
                  <span className="font-medium">{source.name}</span>
                  <span className="text-xs text-muted-foreground">
                    {t(`source.format.${source.date_order as DateOrder}`, { defaultValue: source.date_order })} ·{' '}
                    {t(`source.decimal.${source.decimal as DecimalSeparator}`, { defaultValue: source.decimal })}
                  </span>
                </button>
              ))}
            </div>
          </div>
          {selected ? <SourceSteps key={selected.key} source={selected} /> : null}
        </>
      )}
    </Page>
  )
}

function SourceSteps({ source }: { source: MigrationSource }) {
  const { t } = useTranslation('migrations_assistant')
  const [imports, setImports] = useState(() => readStepImports(source.key))

  const remember = (step: string, importId: string | undefined) => {
    const next = { ...imports }
    if (importId) next[step] = importId
    else delete next[step]
    setImports(next)
    writeStepImports(source.key, next)
  }

  return (
    <div className="grid gap-3">
      <div className="grid gap-1">
        <h3 className="text-base font-semibold">{t('steps.title', { source: source.name })}</h3>
        <p className="text-sm text-muted-foreground">{t('steps.description')}</p>
      </div>
      <ol className="grid gap-4">
        {source.steps.map((step, index) => (
          <li key={step.key} className="min-w-0">
            <StepCard
              source={source}
              step={step}
              position={index + 1}
              importId={imports[step.key]}
              onImport={(importId) => remember(step.key, importId)}
            />
          </li>
        ))}
      </ol>
    </div>
  )
}

interface StepCardProps {
  source: MigrationSource
  step: MigrationStep
  position: number
  importId: string | undefined
  onImport: (importId: string | undefined) => void
}

function StepCard({ source, step, position, importId, onImport }: StepCardProps) {
  const { t } = useTranslation('migrations_assistant')
  const targetLabel = useTargetLabel()
  return (
    <Card className="min-w-0 gap-4">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <span className="inline-flex size-6 items-center justify-center rounded-full bg-primary-solid text-xs text-primary-foreground">
            {position}
          </span>
          {t(`steps.${step.key as StepKey}.title`, { defaultValue: step.key })}
        </CardTitle>
        <CardDescription>
          {t(`steps.${step.key as StepKey}.description`, { file: step.file_hint, defaultValue: '' })}
          {step.links.length > 0 && ` ${t(`steps.${step.key as StepKey}.links`, { defaultValue: '' })}`}
        </CardDescription>
      </CardHeader>
      <CardContent className="grid min-w-0 gap-4">
        {stepKind(step) !== 'import' ? (
          <HistoryUpload source={source} step={step} />
        ) : importId ? (
          <ImportStatus importId={importId} onReset={() => onImport(undefined)} />
        ) : (
          <StepUpload source={source} step={step} onImport={onImport} />
        )}
        <details className="text-sm">
          <summary className="cursor-pointer text-muted-foreground">{t('columns.reference')}</summary>
          <ul className="mt-2 grid gap-1">
            {step.columns.map((column) => (
              <li key={column.target} className="grid gap-0.5 sm:grid-cols-[12rem_minmax(0,1fr)] sm:gap-3">
                <span className="font-medium">{targetLabel(column.target)}</span>
                <span className="break-words text-muted-foreground">{column.headers.join(' · ')}</span>
              </li>
            ))}
          </ul>
        </details>
      </CardContent>
    </Card>
  )
}

interface StepUploadProps {
  source: MigrationSource
  step: MigrationStep
  onImport: (importId: string) => void
}

function StepUpload({ source, step, onImport }: StepUploadProps) {
  const { t } = useTranslation('migrations_assistant')
  const [file, setFile] = useState<File | null>(null)
  const [maps, setMaps] = useState<ValueMaps>(emptyMaps)
  const preview = usePreviewMigrationFile()
  const create = useCreateMigrationImport()

  const analyze = (next: File | null) => {
    setFile(next)
    setMaps(emptyMaps())
    preview.reset()
    if (!next) return
    preview.mutate(
      { source: source.key, step: step.key, file: next, maps: emptyMaps() },
      { onError: (error) => notifyError(error) },
    )
  }

  const submit = () => {
    if (!file) return
    create.mutate(
      { source: source.key, step: step.key, file, maps },
      { onSuccess: (result) => onImport(result.import.id), onError: (error) => notifyError(error) },
    )
  }

  return (
    <div className="grid min-w-0 gap-4">
      <label className="grid gap-1 text-sm">
        <span className="font-medium">{t('upload.label')}</span>
        <Input
          type="file"
          accept=".csv,text/csv"
          onChange={(event) => analyze(event.target.files?.[0] ?? null)}
          className="max-w-md"
        />
      </label>
      {preview.isPending && <p className="text-sm text-muted-foreground">{t('upload.analyzing')}</p>}
      {preview.data && <PreviewPanel preview={preview.data} maps={maps} onMapsChange={setMaps} />}
      {preview.data && (
        <div>
          <Button onClick={submit} disabled={create.isPending}>
            <FileUp aria-hidden="true" />
            {t('upload.create')}
          </Button>
        </div>
      )}
    </div>
  )
}

function HistoryUpload({ source, step }: { source: MigrationSource; step: MigrationStep }) {
  const { t } = useTranslation('migrations_assistant')
  const [file, setFile] = useState<File | null>(null)
  const [maps, setMaps] = useState<ValueMaps>(emptyMaps)
  const [report, setReport] = useState<MigrationHistoryReport | null>(null)
  const preview = usePreviewMigrationFile()
  const migrate = useMigrateHistory()

  const analyze = (next: File | null) => {
    setFile(next)
    setMaps(emptyMaps())
    preview.reset()
    if (!next) return
    preview.mutate(
      { source: source.key, step: step.key, file: next, maps: emptyMaps() },
      { onError: (error) => notifyError(error) },
    )
  }

  const submit = () => {
    if (!file) return
    migrate.mutate(
      { source: source.key, step: step.key, file, maps },
      { onSuccess: (result) => setReport(result.report), onError: (error) => notifyError(error) },
    )
  }

  const reset = () => {
    setReport(null)
    setFile(null)
    preview.reset()
    migrate.reset()
  }

  if (report) return <HistoryReportView report={report} onReset={reset} />

  return (
    <div className="grid min-w-0 gap-4">
      <label className="grid gap-1 text-sm">
        <span className="font-medium">{t('upload.label')}</span>
        <Input
          type="file"
          accept=".csv,text/csv"
          onChange={(event) => analyze(event.target.files?.[0] ?? null)}
          className="max-w-md"
        />
      </label>
      {preview.isPending && <p className="text-sm text-muted-foreground">{t('upload.analyzing')}</p>}
      {preview.data && <PreviewPanel preview={preview.data} maps={maps} onMapsChange={setMaps} />}
      {preview.data && (
        <div>
          <Button onClick={submit} disabled={migrate.isPending}>
            <FileUp aria-hidden="true" />
            {migrate.isPending ? t('history.migrating') : t('history.migrate')}
          </Button>
        </div>
      )}
    </div>
  )
}

function HistoryReportView({ report, onReset }: { report: MigrationHistoryReport; onReset: () => void }) {
  const { t } = useTranslation('migrations_assistant')
  return (
    <div className="grid min-w-0 gap-3" aria-live="polite">
      <p className="text-sm">
        {t('history.report', {
          total: report.total,
          created: report.created,
          skipped: report.skipped,
          failed: report.failed,
        })}
      </p>
      {report.errors.length > 0 && (
        <div className="grid gap-1">
          <h4 className="text-sm font-semibold">{t('history.errors')}</h4>
          <ul className="grid max-h-56 gap-1 overflow-y-auto text-sm text-muted-foreground">
            {report.errors.map((error) => (
              <li key={error.row} className="break-words">
                {t('history.row', { row: error.row, message: error.message })}
              </li>
            ))}
          </ul>
        </div>
      )}
      <div>
        <Button variant="outline" size="sm" onClick={onReset}>
          <RotateCcw aria-hidden="true" />
          {t('history.another')}
        </Button>
      </div>
    </div>
  )
}

interface PreviewPanelProps {
  preview: MigrationPreview
  maps: ValueMaps
  onMapsChange: (maps: ValueMaps) => void
}

function PreviewPanel({ preview, maps, onMapsChange }: PreviewPanelProps) {
  const { t } = useTranslation('migrations_assistant')
  const targetLabel = useTargetLabel()
  const issues = Object.entries(preview.issues)
  const sampleColumns = preview.recognized.slice(0, SAMPLE_COLUMNS)

  return (
    <div className="grid min-w-0 gap-4 rounded-lg border p-3">
      <p className="text-sm">
        {t('preview.summary', {
          rows: preview.row_count,
          recognized: preview.recognized.length,
          total: preview.headers.length,
          changed: preview.changed_cells,
        })}
      </p>
      <div className="grid gap-2">
        <h4 className="text-sm font-semibold">{t('preview.recognized')}</h4>
        <ul className="flex flex-wrap gap-2">
          {preview.recognized.map((column) => (
            <li key={column.index}>
              <Badge variant="secondary" className="max-w-full whitespace-normal break-words">
                {column.header} <ArrowRight aria-hidden="true" className="size-3" /> {targetLabel(column.target)}
              </Badge>
            </li>
          ))}
        </ul>
      </div>
      {preview.unrecognized.length > 0 && (
        <div className="grid gap-1">
          <h4 className="text-sm font-semibold">{t('preview.unrecognized')}</h4>
          <p className="text-sm break-words text-muted-foreground">{preview.unrecognized.join(' · ')}</p>
          <p className="text-xs text-muted-foreground">{t('preview.unrecognizedHint')}</p>
        </div>
      )}
      {issues.length > 0 && (
        <ul className="grid list-disc gap-1 pl-5 text-sm text-muted-foreground">
          {issues.map(([target, count]) => (
            <li key={target}>{t('preview.issue', { field: targetLabel(target), count })}</li>
          ))}
        </ul>
      )}
      <ValueMapEditors preview={preview} maps={maps} onMapsChange={onMapsChange} />
      {sampleColumns.length > 0 && preview.sample.length > 0 && (
        <div className="grid min-w-0 gap-2">
          <h4 className="text-sm font-semibold">{t('preview.sample')}</h4>
          <Table>
            <TableHeader>
              <TableRow>
                {sampleColumns.map((column) => (
                  <TableHead key={column.index} className="px-2 py-1 whitespace-nowrap">
                    {targetLabel(column.target)}
                  </TableHead>
                ))}
              </TableRow>
            </TableHeader>
            <TableBody>
              {preview.sample.map((row, rowIndex) => (
                <TableRow key={rowIndex} className="last:border-0">
                  {sampleColumns.map((column) => (
                    <TableCell key={column.index} className="px-2 py-1 whitespace-nowrap">
                      {row[column.index]}
                    </TableCell>
                  ))}
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}
    </div>
  )
}

function ValueMapEditors({ preview, maps, onMapsChange }: PreviewPanelProps) {
  const { t } = useTranslation('migrations_assistant')
  const targetLabel = useTargetLabel()
  const workflow = usePipelines()
  const members = useAllMembers()
  const options = useMemo(() => {
    const active = (workflow.data ?? []).filter((pipeline) => pipeline.is_active)
    return {
      pipeline_id: active.map((pipeline) => ({ value: pipeline.name, label: pipeline.name })),
      stage_id: active.flatMap((pipeline) =>
        pipeline.stages
          .filter((stage) => stage.is_active)
          .map((stage) => ({ value: stage.name, label: `${stage.name} (${pipeline.name})` })),
      ),
      owner_id: (members.data ?? [])
        .filter((member) => member.status === 'active')
        .map((member) => ({ value: member.email, label: `${member.display_name} <${member.email}>` })),
    }
  }, [workflow.data, members.data])

  const editors = MAPPABLE.filter((target) => (preview.distinct[target]?.length ?? 0) > 0)
  if (editors.length === 0) return null

  const change = (target: MapTarget, sourceValue: string, value: string) => {
    const next = { ...maps, [target]: { ...maps[target] } }
    if (value) next[target][sourceValue] = value
    else delete next[target][sourceValue]
    onMapsChange(next)
  }

  return (
    <div className="grid gap-4">
      {editors.map((target) => (
        <fieldset key={target} className="grid min-w-0 gap-2">
          <legend className="text-sm font-semibold">{t('maps.title', { field: targetLabel(target) })}</legend>
          <p className="text-xs text-muted-foreground">{t(`maps.hint.${target}`)}</p>
          <div className="grid gap-2">
            {(preview.distinct[target] ?? []).map((sourceValue) => (
              <label
                key={sourceValue}
                className="grid min-w-0 gap-1 text-sm sm:grid-cols-[minmax(0,1fr)_minmax(0,1fr)] sm:items-center sm:gap-3"
              >
                <span className="break-words">{sourceValue}</span>
                <NativeSelect
                  value={maps[target][sourceValue] ?? ''}
                  onChange={(event) => change(target, sourceValue, event.target.value)}
                  className="w-full"
                >
                  <option value="">{t('maps.keep')}</option>
                  {options[target].map((option) => (
                    <option key={`${option.value}-${option.label}`} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </NativeSelect>
              </label>
            ))}
          </div>
        </fieldset>
      ))}
    </div>
  )
}

function ImportStatus({ importId, onReset }: { importId: string; onReset: () => void }) {
  const { t } = useTranslation('migrations_assistant')
  const job = useImportJob(importId)

  if (job.isPending) return <p className="text-sm text-muted-foreground">{t('import.loading')}</p>
  if (job.isError) {
    return (
      <div className="flex flex-wrap items-center gap-2 text-sm">
        <span className="text-muted-foreground">{t('import.missing')}</span>
        <Button variant="outline" size="sm" onClick={onReset}>
          <RotateCcw aria-hidden="true" />
          {t('import.another')}
        </Button>
      </div>
    )
  }
  const data = job.data
  const totals = data.totals
  return (
    <div className="flex min-w-0 flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
      <div className="grid gap-1 text-sm">
        <div className="flex flex-wrap items-center gap-2">
          <span className="font-medium break-all">{data.file_name}</span>
          <Badge variant={data.status === 'completed' ? 'success' : data.status === 'failed' ? 'destructive' : 'secondary'}>
            {t(`import.status.${data.status as 'draft'}`, { defaultValue: data.status })}
          </Badge>
        </div>
        <span className="text-muted-foreground">
          {t('import.totals', {
            total: totals.total,
            imported: totals.imported + totals.updated,
            failed: totals.failed,
            skipped: totals.skipped,
          })}
        </span>
      </div>
      <div className="flex flex-wrap gap-2">
        <Button asChild size="sm">
          <Link to={`/imports/${importId}`}>
            {data.status === 'draft' ? t('import.review') : t('import.open')}
            <ArrowRight aria-hidden="true" />
          </Link>
        </Button>
        <Button variant="outline" size="sm" onClick={onReset}>
          <RotateCcw aria-hidden="true" />
          {t('import.another')}
        </Button>
      </div>
    </div>
  )
}
