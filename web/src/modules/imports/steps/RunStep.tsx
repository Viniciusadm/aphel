import { LoaderCircle } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import type {ImportJob} from '@/sdk'
import { processedRows } from '../mapping'

export interface RunStepProps {
  job: ImportJob
}

export function RunStep({ job }: RunStepProps) {
  const { t } = useTranslation('imports')
  const total = job.totals.total || job.row_count
  const processed = processedRows(job)
  const percent = total > 0 ? Math.round((processed / total) * 100) : 0
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <LoaderCircle className="size-5 animate-spin" aria-hidden />
          {t('run.title')}
        </CardTitle>
        <CardDescription>
          {job.status === 'queued' ? t('run.queued') : t('run.progress', { processed, total })}
        </CardDescription>
      </CardHeader>
      <CardContent className="grid gap-3">
        <div
          role="progressbar"
          aria-label={t('run.progressLabel')}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={percent}
          className="h-2 w-full overflow-hidden rounded-full bg-muted"
        >
          <div className="h-full bg-primary transition-[width]" style={{ width: `${percent}%` }} />
        </div>
        <p className="text-sm text-muted-foreground">{t('run.hint')}</p>
      </CardContent>
    </Card>
  )
}
