import { Archive, Download, PackageCheck, Terminal } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useSession } from '@/app/session/session-context'
import { isApiError } from '@/api'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { ListSkeleton } from '@/features/settings/components/ListSkeleton'
import { Pagination } from '@/components/ui/pagination'
import { RequirePermission } from '@/features/settings/components/RequirePermission'
import { ORGANIZATION_EXPORT } from '@/features/settings/lib/permissions'
import { EXPORTS_PER_PAGE, downloadUrl, isPending, useOrgExports, useRequestOrgExport } from './api'
import { asOrgExportStatus, type OrgExport, type OrgExportStatus } from './types'
import { PageHeader } from '@/components/ui/page-header'

type BadgeVariant = 'secondary' | 'destructive' | 'success' | 'warning'

const STATUS_VARIANTS: Record<OrgExportStatus, BadgeVariant> = {
  queued: 'warning',
  running: 'secondary',
  completed: 'success',
  failed: 'destructive',
}

const IN_PROGRESS_CODE = 'organization_export_in_progress'

export function OrgExportPage() {
  return (
    <RequirePermission permission={ORGANIZATION_EXPORT}>
      <OrgExportSection />
    </RequirePermission>
  )
}

function OrgExportSection() {
  const { t } = useTranslation('org_export')
  const [page, setPage] = useState(1)
  const exports = useOrgExports(page)
  const hasPending = exports.data?.data.some((item) => isPending(item.status)) ?? false

  return (
    <section className="flex min-w-0 flex-col gap-6">
      <PageHeader title={t('page.title')} description={t('page.description')} />
      <div className="grid min-w-0 gap-4 lg:grid-cols-[minmax(0,3fr)_minmax(0,2fr)]">
        <RequestCard disabled={hasPending} />
        <RestoreCard />
      </div>
      <div className="grid min-w-0 gap-3">
        <h3 className="text-base font-semibold">{t('history.title')}</h3>
        {exports.isPending ? (
          <ListSkeleton />
        ) : exports.isError ? (
          <ErrorState onRetry={() => void exports.refetch()} />
        ) : exports.data.data.length === 0 ? (
          <EmptyState icon={Archive} title={t('history.empty')} description={t('history.emptyDescription')} />
        ) : (
          <>
            <ul className="grid gap-2">
              {exports.data.data.map((item) => (
                <ExportRow key={item.id} item={item} />
              ))}
            </ul>
            <Pagination
              page={page}
              perPage={EXPORTS_PER_PAGE}
              total={exports.data.meta.total}
              onPageChange={setPage}
            />
          </>
        )}
      </div>
    </section>
  )
}

function RequestCard({ disabled }: { disabled: boolean }) {
  const { t } = useTranslation('org_export')
  const [includeFiles, setIncludeFiles] = useState(true)
  const request = useRequestOrgExport()

  const submit = () =>
    request.mutate(
      { include_files: includeFiles },
      {
        onSuccess: () => notifySuccess(t('request.queued')),
        onError: (error) =>
          notifyError(error, {
            title: isApiError(error) && error.code === IN_PROGRESS_CODE ? t('request.inProgress') : undefined,
          }),
      },
    )

  return (
    <Card className="min-w-0">
      <CardHeader>
        <CardTitle>{t('request.title')}</CardTitle>
        <CardDescription>{t('request.description')}</CardDescription>
      </CardHeader>
      <CardContent className="grid gap-4">
        <ul className="grid gap-1 text-sm text-muted-foreground">
          {(['records', 'relations', 'history', 'files', 'config', 'manifest'] as const).map((key) => (
            <li key={key} className="flex gap-2">
              <PackageCheck aria-hidden="true" className="mt-0.5 size-4 shrink-0 text-primary" />
              <span>{t(`contents.${key}`)}</span>
            </li>
          ))}
        </ul>
        <label className="flex items-start gap-2 text-sm">
          <Checkbox
            checked={includeFiles}
            onCheckedChange={(value) => setIncludeFiles(value === true)}
            className="mt-0.5"
          />
          <span className="grid gap-0.5">
            <span className="font-medium">{t('request.includeFiles')}</span>
            <span className="text-muted-foreground">{t('request.includeFilesHint')}</span>
          </span>
        </label>
        <p className="text-xs text-muted-foreground">{t('request.privacyNote')}</p>
        <div>
          <Button onClick={submit} disabled={disabled || request.isPending}>
            <Archive aria-hidden="true" />
            {disabled ? t('request.waiting') : t('request.submit')}
          </Button>
        </div>
      </CardContent>
    </Card>
  )
}

function RestoreCard() {
  const { t } = useTranslation('org_export')
  return (
    <Card className="min-w-0">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Terminal aria-hidden="true" className="size-4" />
          {t('restore.title')}
        </CardTitle>
        <CardDescription>{t('restore.description')}</CardDescription>
      </CardHeader>
      <CardContent className="grid gap-3 text-sm">
        <pre className="overflow-x-auto rounded-md bg-muted px-3 py-2 font-mono text-xs">
          aphel org import aphel-export.zip --organization nova-organizacao
        </pre>
        <p className="text-muted-foreground">{t('restore.empty')}</p>
        <p className="text-muted-foreground">{t('restore.members')}</p>
      </CardContent>
    </Card>
  )
}

function formatBytes(bytes: number, locale: string): string {
  const units = ['B', 'KB', 'MB', 'GB']
  let value = bytes
  let unit = 0
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024
    unit += 1
  }
  return `${new Intl.NumberFormat(locale, { maximumFractionDigits: unit === 0 ? 0 : 1 }).format(value)} ${units[unit]}`
}

function ExportRow({ item }: { item: OrgExport }) {
  const { t } = useTranslation('org_export')
  const formatting = useRecordFormatting()
  const { me } = useSession()
  const summary = item.summary
  const status = asOrgExportStatus(item.status)
  const records = summary ? Object.values(summary.records).reduce((total, count) => total + count, 0) : 0
  const requester = item.requested_by.display_name ?? me.user.display_name

  return (
    <li className="flex min-w-0 flex-col gap-3 rounded-lg border bg-card p-3 sm:flex-row sm:items-center sm:justify-between">
      <div className="grid min-w-0 gap-1">
        <div className="flex flex-wrap items-center gap-2">
          <span className="font-medium">{formatDateTime(item.created_at, formatting)}</span>
          <Badge variant={STATUS_VARIANTS[status]}>{t(`status.${status}`)}</Badge>
          {!item.include_files && <Badge variant="secondary">{t('history.withoutFiles')}</Badge>}
        </div>
        <p className="text-sm break-words text-muted-foreground">{t('history.requestedBy', { name: requester })}</p>
        {summary && (
          <p className="text-sm text-muted-foreground">
            {t('history.summary', {
              records,
              files: summary.files,
              size: formatBytes(summary.archive_bytes, formatting.locale),
            })}
            {summary.missing_files > 0 && ` · ${t('history.missingFiles', { count: summary.missing_files })}`}
          </p>
        )}
        {item.status === 'failed' && item.failure_reason && (
          <p className="text-sm break-words text-destructive">{item.failure_reason}</p>
        )}
      </div>
      {item.status === 'completed' && (
        <Button asChild variant="outline" size="sm" className="self-start sm:self-center">
          <a href={downloadUrl(item.id)} download>
            <Download aria-hidden="true" />
            {t('history.download')}
          </a>
        </Button>
      )}
    </li>
  )
}
