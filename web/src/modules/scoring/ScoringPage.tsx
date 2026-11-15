import { Gauge, MoreHorizontal, Pencil, Plus, Power, RefreshCw, Star, Trash2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate } from 'react-router'
import { ConfirmDialog, PageLoader } from '@/components/feedback'
import { notifyError, notifySuccess } from '@/components/feedback/notify'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { RequirePermission } from '@/sdk'
import { scoringApi, useInvalidateScoring, useScoringModels } from './api'
import { useScoringRecordTypeLabel } from './labels'
import { SCORING_PATH } from './routes'
import { ScoringCreateDialog } from './ScoringCreateDialog'
import { SCORING_MANAGE, type ScoringModel } from './types'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

export function ScoringPage() {
  return (
    <RequirePermission permission={SCORING_MANAGE}>
      <ScoringSettings />
    </RequirePermission>
  )
}

function ScoringSettings() {
  const { t, i18n } = useTranslation('scoring')
  const models = useScoringModels()
  const invalidate = useInvalidateScoring()
  const recordTypeLabel = useScoringRecordTypeLabel()
  const [creating, setCreating] = useState(false)
  const navigate = useNavigate()
  const [removing, setRemoving] = useState<ScoringModel | null>(null)
  const [busy, setBusy] = useState(false)
  const dateFormat = new Intl.DateTimeFormat(i18n.language, { dateStyle: 'short', timeStyle: 'short' })

  const run = async (task: () => Promise<unknown>, message: string) => {
    setBusy(true)
    try {
      await task()
      await invalidate()
      notifySuccess(message)
    } catch (error) {
      notifyError(error)
    } finally {
      setBusy(false)
    }
  }

  const hasPrimary = (recordType: string) =>
    (models.data ?? []).some((model) => model.record_type === recordType && model.is_primary)

  return (
    <Page>
      <PageHeader
        title={t('title')}
        description={t('description')}
        actions={
          <Button onClick={() => setCreating(true)}>
            <Plus aria-hidden="true" />
            {t('new')}
          </Button>
        }
      />
      {models.isPending ? (
        <PageLoader />
      ) : models.isError ? (
        <ErrorState onRetry={() => void models.refetch()} />
      ) : models.data.length === 0 ? (
        <EmptyState
          icon={Gauge}
          title={t('empty.title')}
          description={t('empty.description')}
          action={
            <Button onClick={() => setCreating(true)}>
              <Plus aria-hidden="true" />
              {t('new')}
            </Button>
          }
        />
      ) : (
        <div className="grid gap-3">
          <Table>
            <caption className="sr-only">{t('list.label')}</caption>
            <TableHeader>
              <TableRow>
                <TableHead scope="col">{t('list.columns.name')}</TableHead>
                <TableHead scope="col" className="hidden sm:table-cell">{t('list.columns.type')}</TableHead>
                <TableHead scope="col" className="hidden md:table-cell">{t('list.columns.criteria')}</TableHead>
                <TableHead scope="col" className="hidden lg:table-cell">{t('list.columns.bands')}</TableHead>
                <TableHead scope="col" className="hidden lg:table-cell">{t('list.columns.recalculated')}</TableHead>
                <TableHead scope="col" className="text-right">
                  <span className="sr-only">{t('list.columns.actions')}</span>
                </TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {models.data.map((model) => (
                <TableRow key={model.id}>
                  <TableCell className="max-w-72">
                    <Link to={`${SCORING_PATH}/${model.id}`} className="block truncate font-medium underline-offset-4 hover:underline">
                      {model.name}
                    </Link>
                    <span className="flex flex-wrap gap-1">
                      {model.is_primary && <Badge variant="default">{t('list.primary')}</Badge>}
                      {!model.is_active && <Badge variant="outline">{t('list.inactive')}</Badge>}
                    </span>
                  </TableCell>
                  <TableCell className="hidden sm:table-cell">{recordTypeLabel(model.record_type)}</TableCell>
                  <TableCell className="hidden tabular-nums md:table-cell">{model.criteria.length}</TableCell>
                  <TableCell className="hidden text-muted-foreground lg:table-cell">
                    {t('list.bands', { warm: model.warm_from, hot: model.hot_from })}
                  </TableCell>
                  <TableCell className="hidden text-muted-foreground lg:table-cell">
                    {model.recalculated_at
                      ? dateFormat.format(new Date(model.recalculated_at))
                      : t('list.neverRecalculated')}
                  </TableCell>
                  <TableCell className="text-right">
                    <div className="flex items-center justify-end gap-1">
                      <Button asChild size="sm" variant="outline">
                        <Link to={`${SCORING_PATH}/${model.id}`} aria-label={t('list.edit', { name: model.name })}>
                          <Pencil aria-hidden="true" />
                          {t('list.editShort')}
                        </Link>
                      </Button>
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button size="icon" variant="ghost" aria-label={t('list.actions', { name: model.name })}>
                            <MoreHorizontal aria-hidden="true" />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          {!model.is_primary && (
                            <DropdownMenuItem
                              disabled={busy}
                              onSelect={() =>
                                void run(
                                  () => scoringApi.update(model.id, { is_primary: true }, model.updated_at),
                                  t('toasts.primary'),
                                )
                              }
                            >
                              <Star aria-hidden="true" />
                              {t('list.makePrimary')}
                            </DropdownMenuItem>
                          )}
                          <DropdownMenuItem
                            disabled={busy || !model.is_active}
                            onSelect={() => void run(() => scoringApi.recalculate(model.id), t('toasts.recalculating'))}
                          >
                            <RefreshCw aria-hidden="true" />
                            {t('list.recalculate')}
                          </DropdownMenuItem>
                          <DropdownMenuItem
                            disabled={busy}
                            onSelect={() =>
                              void run(
                                () => scoringApi.update(model.id, { is_active: !model.is_active }, model.updated_at),
                                model.is_active ? t('toasts.deactivated') : t('toasts.activated'),
                              )
                            }
                          >
                            <Power aria-hidden="true" />
                            {model.is_active ? t('list.deactivate') : t('list.activate')}
                          </DropdownMenuItem>
                          <DropdownMenuSeparator />
                          <DropdownMenuItem variant="destructive" onSelect={() => setRemoving(model)}>
                            <Trash2 aria-hidden="true" />
                            {t('list.removeShort')}
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
          <p className="text-xs text-muted-foreground">{t('help')}</p>
        </div>
      )}
      <ScoringCreateDialog
        open={creating}
        hasPrimary={hasPrimary}
        onOpenChange={setCreating}
        onCreated={(model) => {
          setCreating(false)
          void invalidate()
          notifySuccess(t('toasts.created'))
          void navigate(`${SCORING_PATH}/${model.id}`)
        }}
      />
      <ConfirmDialog
        open={removing !== null}
        onOpenChange={(open) => {
          if (!open) setRemoving(null)
        }}
        title={t('removeDialog.title')}
        description={t('removeDialog.description', { name: removing?.name ?? '' })}
        confirmLabel={t('removeDialog.confirm')}
        variant="destructive"
        pending={busy}
        onConfirm={async () => {
          const model = removing
          if (!model) return
          await run(() => scoringApi.remove(model.id), t('toasts.removed'))
          setRemoving(null)
        }}
      />
    </Page>
  )
}
