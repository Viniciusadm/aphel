import { Gauge, Pencil, Plus, RefreshCw, Star, Trash2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ConfirmDialog, PageLoader } from '@/components/feedback'
import { notifyError, notifySuccess } from '@/components/feedback/notify'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { RequirePermission } from '@/features/settings/components/RequirePermission'
import { scoringApi, useInvalidateScoring, useScoringModels } from './api'
import { useScoringRecordTypeLabel } from './labels'
import { ScoringModelDialog } from './ScoringModelDialog'
import { SCORING_MANAGE, type ScoringModel } from './types'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

export function ScoringPage() {
  return (
    <RequirePermission permission={SCORING_MANAGE}>
      <ScoringSettings />
    </RequirePermission>
  )
}

interface EditorState {
  model: ScoringModel | null
}

function ScoringSettings() {
  const { t, i18n } = useTranslation('scoring')
  const models = useScoringModels()
  const invalidate = useInvalidateScoring()
  const recordTypeLabel = useScoringRecordTypeLabel()
  const [editor, setEditor] = useState<EditorState | null>(null)
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

  return (
    <Page>
      <PageHeader
        title={t('title')}
        description={t('description')}
        actions={
          <Button onClick={() => setEditor({ model: null })}>
            <Plus aria-hidden="true" />
            {t('new')}
          </Button>
        }
      />
      <p className="text-sm text-muted-foreground">{t('help')}</p>
      {models.isPending ? (
        <PageLoader />
      ) : models.isError ? (
        <ErrorState onRetry={() => void models.refetch()} />
      ) : models.data.length === 0 ? (
        <EmptyState icon={Gauge} title={t('empty.title')} description={t('empty.description')} />
      ) : (
        <ul className="grid gap-2" aria-label={t('list.label')}>
          {models.data.map((model) => (
            <li key={model.id} className="grid min-w-0 gap-2 rounded-md border bg-card p-3">
              <div className="flex min-w-0 flex-wrap items-start justify-between gap-2">
                <div className="grid min-w-0 gap-1">
                  <div className="flex min-w-0 flex-wrap items-center gap-2">
                    <span className="font-medium break-words">{model.name}</span>
                    <Badge variant="secondary">{recordTypeLabel(model.record_type)}</Badge>
                    {model.is_primary && <Badge variant="default">{t('list.primary')}</Badge>}
                    {!model.is_active && <Badge variant="outline">{t('list.inactive')}</Badge>}
                  </div>
                  {model.description && (
                    <p className="text-sm break-words text-muted-foreground">{model.description}</p>
                  )}
                  <p className="text-xs text-muted-foreground">
                    {t('list.criteria', { count: model.criteria.length })} ·{' '}
                    {t('list.bands', { warm: model.warm_from, hot: model.hot_from })} ·{' '}
                    {model.recalculated_at
                      ? t('list.recalculatedAt', { date: dateFormat.format(new Date(model.recalculated_at)) })
                      : t('list.neverRecalculated')}
                  </p>
                </div>
                <div className="flex shrink-0 flex-wrap items-center gap-1">
                  {!model.is_primary && (
                    <Button
                      size="sm"
                      variant="outline"
                      disabled={busy}
                      onClick={() =>
                        void run(
                          () => scoringApi.update(model.id, { is_primary: true }, model.updated_at),
                          t('toasts.primary'),
                        )
                      }
                    >
                      <Star aria-hidden="true" />
                      {t('list.makePrimary')}
                    </Button>
                  )}
                  <Button
                    size="sm"
                    variant="outline"
                    disabled={busy}
                    onClick={() =>
                      void run(
                        () => scoringApi.update(model.id, { is_active: !model.is_active }, model.updated_at),
                        model.is_active ? t('toasts.deactivated') : t('toasts.activated'),
                      )
                    }
                  >
                    {model.is_active ? t('list.deactivate') : t('list.activate')}
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    disabled={busy || !model.is_active}
                    onClick={() => void run(() => scoringApi.recalculate(model.id), t('toasts.recalculating'))}
                  >
                    <RefreshCw aria-hidden="true" />
                    {t('list.recalculate')}
                  </Button>
                  <Button
                    size="icon"
                    variant="ghost"
                    aria-label={t('list.edit', { name: model.name })}
                    onClick={() => setEditor({ model })}
                  >
                    <Pencil aria-hidden="true" />
                  </Button>
                  <Button
                    size="icon"
                    variant="ghost"
                    className="text-destructive"
                    aria-label={t('list.remove', { name: model.name })}
                    onClick={() => setRemoving(model)}
                  >
                    <Trash2 aria-hidden="true" />
                  </Button>
                </div>
              </div>
            </li>
          ))}
        </ul>
      )}
      <ScoringModelDialog
        open={editor !== null}
        model={editor?.model ?? null}
        hasPrimary={(recordType) =>
          (models.data ?? []).some((model) => model.record_type === recordType && model.is_primary)
        }
        onOpenChange={(open) => {
          if (!open) setEditor(null)
        }}
        onSaved={(created) => {
          setEditor(null)
          void invalidate()
          notifySuccess(created ? t('toasts.created') : t('toasts.updated'))
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
