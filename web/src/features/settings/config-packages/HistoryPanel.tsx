import { Download, History } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { ListSkeleton } from '@/features/settings/components/ListSkeleton'
import { Pagination } from '@/features/settings/components/Pagination'
import { INSTALLS_PER_PAGE, usePackageInstall, usePackageInstalls, type PackageInstall } from './api'
import { downloadJson, packageFileName } from './lib/download'
import { usePackageLabels } from './lib/labels'

function useDateFormatter() {
  const { i18n } = useTranslation()
  return useMemo(
    () => new Intl.DateTimeFormat(i18n.language, { dateStyle: 'short', timeStyle: 'short' }),
    [i18n.language],
  )
}

export function HistoryPanel() {
  const { t } = useTranslation('config-packages')
  const [page, setPage] = useState(1)
  const [selected, setSelected] = useState<PackageInstall | null>(null)
  const installs = usePackageInstalls(page)
  const formatter = useDateFormatter()

  if (installs.isPending) return <ListSkeleton rows={3} />
  if (installs.isError) return <ErrorState onRetry={() => void installs.refetch()} />
  if (installs.data.data.length === 0) {
    return <EmptyState icon={History} title={t('history.emptyTitle')} description={t('history.emptyDescription')} />
  }

  return (
    <div className="grid gap-3">
      <ul className="grid gap-2" aria-label={t('history.listLabel')}>
        {installs.data.data.map((install) => (
          <li key={install.id} className="flex min-w-0 flex-wrap items-center gap-3 rounded-md border bg-card px-3 py-2">
            <div className="grid min-w-0 flex-1 gap-1">
              <div className="flex min-w-0 flex-wrap items-center gap-2">
                <span className="min-w-0 font-medium break-words">{install.name}</span>
                {install.template_key && (
                  <Badge variant="secondary">{t('history.template', { key: install.template_key })}</Badge>
                )}
                {install.summary.no_op && <Badge variant="outline">{t('history.noOp')}</Badge>}
              </div>
              <p className="text-xs text-muted-foreground">
                {t('history.installedBy', {
                  name: install.installed_by?.name || t('history.unknownUser'),
                  date: formatter.format(new Date(install.installed_at)),
                })}
              </p>
              <p className="text-xs text-muted-foreground">
                {t('history.summary', {
                  create: install.summary.create,
                  update: install.summary.update,
                  unchanged: install.summary.unchanged,
                  skipped: install.summary.skip_conflict,
                })}
              </p>
            </div>
            <Button
              size="sm"
              variant="outline"
              aria-label={t('history.detailsLabel', { name: install.name })}
              onClick={() => setSelected(install)}
            >
              {t('history.details')}
            </Button>
          </li>
        ))}
      </ul>
      <Pagination page={page} perPage={INSTALLS_PER_PAGE} total={installs.data.meta.total} onPageChange={setPage} />
      <Dialog open={selected !== null} onOpenChange={(open) => !open && setSelected(null)}>
        <DialogContent>
          {selected && <InstallDetail install={selected} onClose={() => setSelected(null)} />}
        </DialogContent>
      </Dialog>
    </div>
  )
}

function InstallDetail({ install, onClose }: { install: PackageInstall; onClose: () => void }) {
  const { t } = useTranslation('config-packages')
  const labels = usePackageLabels()
  const detail = usePackageInstall(install.id)
  const formatter = useDateFormatter()

  return (
    <>
      <DialogHeader>
        <DialogTitle className="break-words">{install.name}</DialogTitle>
        <DialogDescription>
          {t('history.installedBy', {
            name: install.installed_by?.name || t('history.unknownUser'),
            date: formatter.format(new Date(install.installed_at)),
          })}
        </DialogDescription>
      </DialogHeader>
      <dl className="grid gap-3 text-sm">
        {install.template_key && (
          <div className="grid gap-0.5">
            <dt className="text-xs text-muted-foreground">{t('history.templateLabel')}</dt>
            <dd>{install.template_key}</dd>
          </div>
        )}
        <div className="grid gap-0.5">
          <dt className="text-xs text-muted-foreground">{t('history.sectionsLabel')}</dt>
          <dd className="flex flex-wrap gap-1.5">
            {install.summary.sections.map((section) => (
              <Badge key={section} variant="secondary">
                {labels.section(section)}
              </Badge>
            ))}
          </dd>
        </div>
        <div className="grid gap-0.5">
          <dt className="text-xs text-muted-foreground">{t('history.summaryLabel')}</dt>
          <dd>
            {t('history.summary', {
              create: install.summary.create,
              update: install.summary.update,
              unchanged: install.summary.unchanged,
              skipped: install.summary.skip_conflict,
            })}
          </dd>
        </div>
      </dl>
      {detail.isError && <ErrorState onRetry={() => void detail.refetch()} />}
      {detail.isPending && <Skeleton className="h-9 w-40" />}
      <DialogFooter>
        <Button variant="outline" onClick={onClose}>
          {t('history.close')}
        </Button>
        <Button
          disabled={!detail.data}
          onClick={() =>
            detail.data &&
            downloadJson(packageFileName(install.name, new Date(install.installed_at)), detail.data.package)
          }
        >
          <Download aria-hidden="true" />
          {t('history.download')}
        </Button>
      </DialogFooter>
    </>
  )
}
