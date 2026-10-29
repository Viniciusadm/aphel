import { ArrowLeft } from 'lucide-react'
import { useCustomObjectLabel } from '@/features/custom-records/api'
import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useParams, useSearchParams } from 'react-router'
import { PageLoader } from '@/components/feedback'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { cn } from '@/lib/utils'
import { isFinished, isImportable, supportsImport, isRunning, useImportJob, type ImportJob } from './api'
import { LIST_PATHS } from './mapping'
import { MappingStep } from './steps/MappingStep'
import { ResultStep } from './steps/ResultStep'
import { RunStep } from './steps/RunStep'
import { UploadStep } from './steps/UploadStep'

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
  const [params] = useSearchParams()
  const job = useImportJob(id)
  const recordType = job.data?.record_type ?? params.get('record_type') ?? 'contact'

  let body
  if (!id) {
    body = <UploadStep recordType={recordType} />
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

  return (
    <ImportLayout recordType={recordType} step={id ? stepOf(job.data) : 'upload'}>
      {supportsImport(recordType) ? body : <ImportUnsupported />}
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
  const customLabel = useCustomObjectLabel(recordType)
  const typeLabel = isImportable(recordType) ? t(`recordTypes.${recordType}`) : (customLabel ?? recordType)
  const current = STEPS.indexOf(step)
  return (
    <div className="flex min-w-0 flex-col gap-4">
      <div className="grid gap-2">
        <Link
          to={LIST_PATHS[recordType] ?? '/'}
          className="inline-flex w-fit items-center gap-1 text-sm text-muted-foreground hover:text-foreground"
        >
          <ArrowLeft className="size-4" aria-hidden />
          {t('wizard.back')}
        </Link>
        <h1 className="text-xl font-semibold">{t('wizard.title', { recordType: typeLabel })}</h1>
      </div>
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
    </div>
  )
}
