import { PageHeader } from '@/components/ui/page-header'
import { BarChart3, Building2, Plus, Users } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { useReports, useReportEntities } from './api'
import { Field } from '@/components/ui/field'

export function SavedReportsPage() {
  const { t } = useTranslation('reports', { keyPrefix: 'builder' })
  const entities = useReportEntities()
  const formatting = useRecordFormatting()
  const reports = useReports()
  const [recordType, setRecordType] = useState('')
  const allowedTypes = entities.data ?? []
  const visible = (reports.data ?? []).filter((report) => !recordType || report.record_type === recordType)

  return (
    <div className="grid gap-4">
      <PageHeader level="section" title={t('list.title')} description={t('list.description')} actions={<>
          <Field label={t('list.filter')}>
            {(fieldControl) => <NativeSelect {...fieldControl} value={recordType} onChange={(event) => setRecordType(event.target.value)}>
              <option value="">{t('list.allModules')}</option>
              {allowedTypes.map((type) => (
                <option key={type.recordType} value={type.recordType}>
                  {type.label}
                </option>
              ))}
            </NativeSelect>}
          </Field>
          {allowedTypes.length > 0 && (
            <Button asChild>
              <Link to="/reports/saved/new">
                <Plus />
                {t('list.create')}
              </Link>
            </Button>
          )}
              </>} />
      {reports.isPending && (
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          <Skeleton className="h-28" />
          <Skeleton className="h-28" />
          <Skeleton className="h-28" />
        </div>
      )}
      {reports.isError && <ErrorState onRetry={() => reports.refetch()} />}
      {reports.isSuccess && visible.length === 0 && (
        <EmptyState
          icon={BarChart3}
          title={t('list.emptyTitle')}
          description={t('list.emptyDescription')}
          action={
            allowedTypes.length > 0 ? (
              <Button asChild variant="outline">
                <Link to="/reports/saved/new">{t('list.create')}</Link>
              </Button>
            ) : undefined
          }
        />
      )}
      {reports.isSuccess && visible.length > 0 && (
        <ul className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          {visible.map((report) => (
            <li key={report.id}>
              <Card className="h-full transition-colors hover:bg-accent/40">
                <Link to={`/reports/saved/${report.id}`} className="grid h-full gap-2 p-4 outline-none focus-visible:ring-2 focus-visible:ring-ring">
                  <div className="flex items-start justify-between gap-2">
                    <p className="min-w-0 font-medium break-words">{report.name}</p>
                    {report.visibility === 'team' && <Users className="size-4 shrink-0 text-muted-foreground" aria-label={t('visibility.team')} />}
                    {report.visibility === 'organization' && (
                      <Building2 className="size-4 shrink-0 text-muted-foreground" aria-label={t('visibility.organization')} />
                    )}
                  </div>
                  {report.description && <p className="line-clamp-2 text-sm text-muted-foreground">{report.description}</p>}
                  <p className="mt-auto text-xs text-muted-foreground">
                    {t('list.meta', {
                      module: allowedTypes.find((entity) => entity.recordType === report.record_type)?.label ?? report.record_type,
                      visualization: t(`visualizations.${report.visualization}`),
                      date: formatDateTime(report.updated_at, formatting),
                    })}
                  </p>
                </Link>
              </Card>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}
