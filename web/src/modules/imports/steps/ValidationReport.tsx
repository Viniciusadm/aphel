import { CircleAlert, CircleCheck } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import type {ImportValidation} from '@/sdk'
import type { ImportRowReport } from '../mapping'
import { RowErrors } from './RowErrors'

export interface ValidationReportProps {
  report: ImportValidation
}

export function ValidationReport({ report }: ValidationReportProps) {
  const { t } = useTranslation('imports')
  const rows: ImportRowReport[] = report.rows
  return (
    <Card role="region" aria-label={t('validation.title')}>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          {report.invalid_rows === 0 ? (
            <CircleCheck className="size-5 text-success" aria-hidden />
          ) : (
            <CircleAlert className="size-5 text-warning" aria-hidden />
          )}
          {t('validation.title')}
        </CardTitle>
        <CardDescription>
          {t('validation.summary', { valid: report.valid_rows, total: report.total_rows })}
          {' · '}
          {report.invalid_rows === 0
            ? t('validation.allValid')
            : t('validation.invalid', { count: report.invalid_rows })}
        </CardDescription>
      </CardHeader>
      <CardContent className="grid gap-3">
        <RowErrors rows={rows} />
        {report.invalid_rows > rows.length && (
          <p className="text-sm text-muted-foreground">{t('validation.more', { count: rows.length })}</p>
        )}
        <p className="text-xs text-muted-foreground">{t('validation.note')}</p>
      </CardContent>
    </Card>
  )
}
