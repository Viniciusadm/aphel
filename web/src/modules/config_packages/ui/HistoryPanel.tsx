import { PaginationSummary } from '@/components/ui/pagination-summary'
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
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { ListSkeleton } from '@/sdk'
import { Pagination } from '@/components/ui/pagination'
import {INSTALLS_PER_PAGE, usePackageInstall, usePackageInstalls, type PackageInstall} from '@/sdk'
import { downloadJson, packageFileName } from '@/modules/config_packages/ui/lib/download'
import {usePackageLabels} from '@/sdk'

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
      <PaginationSummary page={page} perPage={INSTALLS_PER_PAGE} total={installs.data.meta.total} />
      <Table>
        <caption className="sr-only">{t('history.listLabel')}</caption>
        <TableHeader>
          <TableRow>
            <TableHead scope="col">{t('history.columns.name')}</TableHead>
            <TableHead scope="col" className="hidden md:table-cell">{t('history.columns.by')}</TableHead>
            <TableHead scope="col">{t('history.columns.when')}</TableHead>
            <TableHead scope="col" className="hidden lg:table-cell">{t('history.columns.result')}</TableHead>
            <TableHead scope="col" className="text-right">
              <span className="sr-only">{t('history.columns.actions')}</span>
            </TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {installs.data.data.map((install) => (
            <TableRow key={install.id}>
              <TableCell className="max-w-64">
                <span className="block truncate font-medium">{install.name}</span>
                <span className="flex flex-wrap gap-1">
                  {install.template_key && <Badge variant="secondary">{t('history.template', { key: install.template_key })}</Badge>}
                  {install.summary.no_op && <Badge variant="outline">{t('history.noOp')}</Badge>}
                </span>
              </TableCell>
              <TableCell className="hidden md:table-cell">{install.installed_by?.name || t('history.unknownUser')}</TableCell>
              <TableCell className="tabular-nums">{formatter.format(new Date(install.installed_at))}</TableCell>
              <TableCell className="hidden text-muted-foreground lg:table-cell">
                {t('history.summary', {
                  create: install.summary.create,
                  update: install.summary.update,
                  unchanged: install.summary.unchanged,
                  skipped: install.summary.skip_conflict,
                })}
              </TableCell>
              <TableCell className="text-right">
                <Button
                  size="sm"
                  variant="outline"
                  aria-label={t('history.detailsLabel', { name: install.name })}
                  onClick={() => setSelected(install)}
                >
                  {t('history.details')}
                </Button>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
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
