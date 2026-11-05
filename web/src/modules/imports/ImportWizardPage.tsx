import { entityLabel, useEntityLabel } from '@/sdk/entities'
import { recordListPath } from '@/sdk'
import { Field } from '@/components/ui/field'
import { NativeSelect } from '@/components/ui/native-select'
import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { useParams, useSearchParams } from 'react-router'
import { PageLoader } from '@/components/feedback'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { cn } from '@/lib/utils'
import {isFinished, useImportEntities, isRunning, useImportJob, type ImportJob} from '@/sdk'
import { MappingStep } from './steps/MappingStep'
import { ResultStep } from './steps/ResultStep'
import { RunStep } from './steps/RunStep'
import { UploadStep } from './steps/UploadStep'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'

const STEPS = ['upload', 'mapping', 'run', 'result'] as const
type Step = (typeof STEPS)[number]

function stepOf(job: ImportJob | undefined): Step {
  if (!job) return 'upload'
  if (isFinished(job.status)) return 'result'
  if (isRunning(job.status)) return 'run'
  return 'mapping'
}

export function ImportWizardPage() {
  const { id } = useParams()
  const [params, setParams] = useSearchParams()
  const entities = useImportEntities()
  const { t, i18n } = useTranslation('imports')
  const job = useImportJob(id)
  const recordType = job.data?.record_type ?? params.get('record_type') ?? entities.data?.[0]?.record_type ?? ''

  let body
  if (!id) {
    body = <div className="grid gap-4">
      <Field label={t('upload.recordType')}>{(control) => <NativeSelect {...control} value={recordType} onChange={(event) => setParams({ record_type: event.target.value })}>
        {(entities.data ?? []).map((entity) => <option key={entity.record_type} value={entity.record_type}>{entityLabel(entity, i18n.language, entity.record_type)}</option>)}
      </NativeSelect>}</Field>
      <UploadStep key={recordType} recordType={recordType} />
    </div>
  } else if (job.isPending) {
    body = <PageLoader />
  } else if (job.isError) {
    body = <ErrorState onRetry={() => void job.refetch()} />
  } else {
    const data = job.data
    body =
      stepOf(data) === 'mapping' ? (
        <MappingStep key={data.id} job={data} />
      ) : stepOf(data) === 'run' ? (
        <RunStep job={data} />
      ) : (
        <ResultStep job={data} />
      )
  }

  if (entities.isPending) return <PageLoader />
  if (entities.isError) return <ErrorState onRetry={() => void entities.refetch()} />
  return (
    <ImportLayout recordType={recordType} step={id ? stepOf(job.data) : 'upload'}>
      {entities.data?.some((entity) => entity.record_type === recordType) ? body : <ImportUnsupported />}
    </ImportLayout>
  )
}

function ImportUnsupported() {
  const { t } = useTranslation('imports')
  return <EmptyState title={t('wizard.unsupported')} />
}

interface ImportLayoutProps {
  recordType: string
  step: Step
  children: ReactNode
}

function ImportLayout({ recordType, step, children }: ImportLayoutProps) {
  const { t } = useTranslation('imports')
  const typeLabel = useEntityLabel(recordType)
  const current = STEPS.indexOf(step)
  return (
    <Page>
      <PageHeader
        back={{ to: recordListPath(recordType) ?? '/', label: t('wizard.back') }}
        title={t('wizard.title', { recordType: typeLabel })}
      />
      <ol aria-label={t('wizard.stepsLabel')} className="flex flex-wrap gap-2 text-sm">
        {STEPS.map((item, index) => (
          <li
            key={item}
            aria-current={index === current ? 'step' : undefined}
            className={cn(
              'flex items-center gap-2 rounded-full border px-3 py-1',
              index === current && 'border-primary bg-primary/10 font-medium text-foreground',
              index < current && 'text-foreground',
              index > current && 'text-muted-foreground',
            )}
          >
            <span aria-hidden className="tabular-nums">
              {index + 1}
            </span>
            {t(`wizard.steps.${item}`)}
          </li>
        ))}
      </ol>
      {children}
    </Page>
  )
}
