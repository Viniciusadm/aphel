import { Calculator, Info, Lock, Package, Plus } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { ConfirmDialog, notifyError, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Skeleton } from '@/components/ui/skeleton'
import { formatCurrency, useRecordFormatting } from '@/sdk'
import { useLineItemMutations, useLineItems, type LineItem, type LineItemRequest } from '@/sdk/line-items'
import { LineItemRow, ITEM_GRID_CLASS } from './LineItemRow'
import { Slot } from '@/kernel/modules/Slot'

export interface LineItemsTabProps {
  recordType: string
  recordId: string
  readOnly?: boolean
}

export function LineItemsTab({ recordType, recordId, readOnly = false }: LineItemsTabProps) {
  const { t } = useTranslation('line_items')
  const formatting = useRecordFormatting()
  const query = useLineItems(recordType, recordId)
  const mutations = useLineItemMutations(recordType, recordId)
  const [freeName, setFreeName] = useState('')
  const [addingFree, setAddingFree] = useState(false)
  const [removing, setRemoving] = useState<LineItem | null>(null)

  if (query.isPending) {
    return (
      <div className="grid gap-2" aria-busy="true">
        <Skeleton className="h-16 w-full" />
        <Skeleton className="h-16 w-full" />
      </div>
    )
  }
  if (query.isError) {
    if (isApiError(query.error) && query.error.code === 'field_not_accessible') {
      return <EmptyState icon={Lock} title={t('items.noAccessTitle')} description={t('items.noAccessDescription')} />
    }
    return <ErrorState title={t('items.loadError')} onRetry={() => void query.refetch()} />
  }

  const summary = query.data
  const items = summary.items
  const currency = summary.currency
  const money = (value: string | null | undefined) => formatCurrency(value ?? '0', formatting, currency)
  const busy =
    mutations.create.isPending ||
    mutations.update.isPending ||
    mutations.remove.isPending ||
    mutations.reorder.isPending ||
    mutations.applyTotal.isPending
  const amountDiffers = summary.can_apply_total && items.length > 0 && summary.record_amount !== summary.total

  const create = async (body: LineItemRequest) => {
    try {
      await mutations.create.mutateAsync(body)
      return true
    } catch (error) {
      notifyError(error)
      return false
    }
  }

  const save = async (item: LineItem, body: LineItemRequest) => {
    try {
      await mutations.update.mutateAsync({ item, body })
    } catch (error) {
      notifyError(error)
      throw error
    }
  }

  const move = (item: LineItem, offset: -1 | 1) => {
    const ids = items.map((entry) => entry.id)
    const index = ids.indexOf(item.id)
    const target = index + offset
    if (index < 0 || target < 0 || target >= ids.length) return
    ;[ids[index], ids[target]] = [ids[target], ids[index]]
    mutations.reorder.mutate(ids, { onError: (error) => notifyError(error) })
  }

  const addFree = async () => {
    const name = freeName.trim()
    if (!name) return
    if (await create({ name, quantity: '1', unit_price: '0' })) {
      setFreeName('')
      setAddingFree(false)
    }
  }

  return (
    <div className="@container grid min-w-0 gap-4">
      {summary.amount_mode === 'items' ? (
        <p role="note" className="flex items-start gap-2 rounded-md border bg-muted/40 p-3 text-sm">
          <Info className="mt-0.5 size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
          <span>{summary.amount_managed ? t('items.autoNotice') : t('items.autoNoticeEmpty')}</span>
        </p>
      ) : (
        items.length > 0 && <p className="text-sm text-muted-foreground">{t('items.manualNotice')}</p>
      )}

      {!readOnly && (
        <div className="flex flex-wrap items-center gap-2">
          <Slot name="line_items.catalogPicker" props={{ currency, disabled: busy, onSelect: (id) => void create({ product_id: id }) }} />
          <Button size="sm" variant="outline" disabled={busy} onClick={() => setAddingFree((value) => !value)}>
            <Plus aria-hidden="true" />
            {t('items.addFree')}
          </Button>
        </div>
      )}

      {addingFree && !readOnly && (
        <form
          className="flex flex-wrap items-center gap-2 rounded-md border p-3"
          onSubmit={(event) => {
            event.preventDefault()
            void addFree()
          }}
        >
          <Input
            autoFocus
            value={freeName}
            maxLength={255}
            onChange={(event) => setFreeName(event.target.value)}
            placeholder={t('items.freeNamePlaceholder')}
            aria-label={t('items.freeNamePlaceholder')}
            className="min-w-0 flex-1 basis-48"
          />
          <div className="flex gap-2">
            <Button type="submit" size="sm" disabled={busy || !freeName.trim()}>
              {t('items.add')}
            </Button>
            <Button type="button" size="sm" variant="ghost" onClick={() => setAddingFree(false)}>
              {t('items.cancel')}
            </Button>
          </div>
        </form>
      )}

      {items.length === 0 ? (
        <EmptyState icon={Package} title={t('items.empty')} description={t('items.emptyDescription')} />
      ) : (
        <div className="min-w-0">
          <div
            aria-hidden="true"
            className={`hidden border-b px-2 pb-2 text-xs font-medium text-muted-foreground @3xl:grid ${ITEM_GRID_CLASS}`}
          >
            <span>{t('items.columns.item')}</span>
            <span>{t('items.columns.quantity')}</span>
            <span>{t('items.columns.unitPrice')}</span>
            <span>{t('items.columns.discount')}</span>
            <span className="text-right">{t('items.columns.total')}</span>
            <span />
          </div>
          <ul aria-label={t('items.title')} className="grid gap-2 @3xl:gap-0">
            {items.map((item, index) => (
              <LineItemRow
                key={`${item.id}-${item.updated_at}`}
                item={item}
                currency={currency}
                readOnly={readOnly}
                isFirst={index === 0}
                isLast={index === items.length - 1}
                busy={busy}
                onSave={save}
                onRemove={setRemoving}
                onMove={move}
              />
            ))}
          </ul>
          <dl className="mt-3 ml-auto grid w-full max-w-xs gap-1 text-sm">
            <div className="flex justify-between gap-4">
              <dt className="text-muted-foreground">{t('items.subtotal')}</dt>
              <dd className="tabular-nums">{money(summary.subtotal)}</dd>
            </div>
            <div className="flex justify-between gap-4">
              <dt className="text-muted-foreground">{t('items.discounts')}</dt>
              <dd className="tabular-nums">
                {summary.discount_total === '0.00' ? money('0') : `− ${money(summary.discount_total)}`}
              </dd>
            </div>
            <div className="flex justify-between gap-4 border-t pt-1 font-semibold">
              <dt>{t('items.total')}</dt>
              <dd className="tabular-nums">{money(summary.total)}</dd>
            </div>
          </dl>
        </div>
      )}

      {amountDiffers && (
        <div className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-dashed p-3 text-sm">
          <span className="text-muted-foreground">
            {t('items.amountDiffers', {
              amount: summary.record_amount ? money(summary.record_amount) : t('items.noAmount'),
              total: money(summary.total),
            })}
          </span>
          {!readOnly && (
            <Button
              size="sm"
              variant="outline"
              disabled={busy}
              onClick={() =>
                mutations.applyTotal.mutate(undefined, {
                  onSuccess: () => notifySuccess(t('items.applied')),
                  onError: (error) => notifyError(error),
                })
              }
            >
              <Calculator aria-hidden="true" />
              {t('items.applyTotal')}
            </Button>
          )}
        </div>
      )}

      <ConfirmDialog
        open={removing !== null}
        onOpenChange={(open) => {
          if (!open) setRemoving(null)
        }}
        title={t('items.removeTitle')}
        description={t('items.removeDescription', { name: removing?.name ?? '' })}
        confirmLabel={t('items.removeConfirm')}
        variant="destructive"
        onConfirm={async () => {
          if (removing) await mutations.remove.mutateAsync(removing)
          setRemoving(null)
        }}
      />
    </div>
  )
}
