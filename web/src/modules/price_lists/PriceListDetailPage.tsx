import { ArrowLeft, Pencil, Plus, Tags, Trash2, X } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useParams } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { ConfirmDialog, PageLoader, UnsavedChangesGuard } from '@/components/feedback'
import { RecordFieldInput } from '@/components/fields'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { toast } from '@/components/ui/toast-store'
import { formatCurrency, useRecordFormatting } from '@/features/records/formatting'
import { PRICE_LISTS_PATH, priceListsApi, useInvalidatePriceLists, usePriceList, usePriceListItems } from './api'
import { fieldErrors, priceListErrorKey } from './errors'
import { PriceListBadges, PriceListValidity } from './PriceListBadges'
import { PriceListDialog } from './PriceListDialog'
import type { PriceList, PriceListItem } from './types'

const PRODUCT_RECORD_TYPE = 'product'
const MONEY_PATTERN = /^\d+([.,]\d{1,2})?$/
const QUANTITY_PATTERN = /^\d+([.,]\d{1,4})?$/

interface DraftItem {
  key: string
  productId: string | null
  minQuantity: string
  unitPrice: string
}

let draftSequence = 0

function nextKey(): string {
  draftSequence += 1
  return `draft-${draftSequence}`
}

function toDraft(item: PriceListItem): DraftItem {
  return {
    key: item.id,
    productId: item.product_id,
    minQuantity: item.min_quantity,
    unitPrice: item.unit_price,
  }
}

function normalizeDecimal(value: string): string {
  return value.trim().replace(',', '.')
}

function signature(items: DraftItem[]): string {
  return JSON.stringify(
    items.map((item) => [item.productId, normalizeDecimal(item.minQuantity), normalizeDecimal(item.unitPrice)]),
  )
}

export function PriceListDetailPage() {
  const { id = '' } = useParams()
  const list = usePriceList(id)

  if (list.isPending) return <PageLoader />
  if (list.isError) return <ErrorState onRetry={() => void list.refetch()} />
  return <PriceListDetail key={list.data.id} list={list.data} />
}

function PriceListDetail({ list }: { list: PriceList }) {
  const { t } = useTranslation('price_lists')
  const navigate = useNavigate()
  const { hasPermission } = useSession()
  const canEdit = hasPermission('products.update')
  const invalidate = useInvalidatePriceLists()
  const [editing, setEditing] = useState(false)
  const [removing, setRemoving] = useState(false)

  const remove = async () => {
    try {
      await priceListsApi.remove(list.id)
      await invalidate()
      toast({ title: t('detail.deleted'), variant: 'success' })
      navigate(PRICE_LISTS_PATH)
    } catch (error) {
      toast({ title: t(priceListErrorKey(error)), variant: 'destructive' })
    }
  }

  return (
    <div className="flex flex-col gap-6">
      <div className="grid gap-2">
        <Button asChild size="sm" variant="ghost" className="w-fit">
          <Link to={PRICE_LISTS_PATH}>
            <ArrowLeft aria-hidden="true" />
            {t('detail.back')}
          </Link>
        </Button>
        <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
          <div className="grid gap-1">
            <div className="flex flex-wrap items-center gap-2">
              <h1 className="text-xl font-semibold">{list.name}</h1>
              <PriceListBadges list={list} />
            </div>
            {list.description && <p className="text-sm text-muted-foreground">{list.description}</p>}
          </div>
          {canEdit && (
            <div className="flex flex-wrap gap-2">
              <Button size="sm" variant="outline" onClick={() => setEditing(true)}>
                <Pencil aria-hidden="true" />
                {t('detail.edit')}
              </Button>
              <Button size="sm" variant="outline" onClick={() => setRemoving(true)}>
                <Trash2 aria-hidden="true" />
                {t('detail.delete')}
              </Button>
            </div>
          )}
        </div>
        <dl className="grid gap-3 rounded-lg border bg-card p-4 text-sm sm:grid-cols-4">
          <div className="grid gap-0.5">
            <dt className="text-muted-foreground">{t('columns.currency')}</dt>
            <dd className="font-mono">{list.currency}</dd>
          </div>
          <div className="grid gap-0.5">
            <dt className="text-muted-foreground">{t('columns.appliesTo')}</dt>
            <dd>
              {list.applies_to === 'companies'
                ? t('detail.companies', { count: list.company_ids.length })
                : t(`appliesTo.${list.applies_to}`)}
            </dd>
          </div>
          <div className="grid gap-0.5">
            <dt className="text-muted-foreground">{t('columns.validity')}</dt>
            <dd>
              <PriceListValidity list={list} />
            </dd>
          </div>
          <div className="grid gap-0.5">
            <dt className="text-muted-foreground">{t('columns.key')}</dt>
            <dd className="font-mono">{list.key}</dd>
          </div>
        </dl>
      </div>
      <PriceListItemsEditor list={list} canEdit={canEdit} />
      <PriceListDialog open={editing} onOpenChange={setEditing} list={list} />
      <ConfirmDialog
        open={removing}
        onOpenChange={setRemoving}
        title={t('detail.deleteTitle')}
        description={t('detail.deleteDescription', { name: list.name })}
        confirmLabel={t('detail.delete')}
        variant="destructive"
        onConfirm={remove}
      />
    </div>
  )
}

function PriceListItemsEditor({ list, canEdit }: { list: PriceList; canEdit: boolean }) {
  const { t } = useTranslation('price_lists')
  const items = usePriceListItems(list.id)

  if (items.isPending) return <PageLoader />
  if (items.isError) return <ErrorState onRetry={() => void items.refetch()} />
  return <ItemsForm key={items.dataUpdatedAt} list={list} items={items.data} canEdit={canEdit} title={t('items.title')} />
}

interface ItemsFormProps {
  list: PriceList
  items: PriceListItem[]
  canEdit: boolean
  title: string
}

function ItemsForm({ list, items, canEdit, title }: ItemsFormProps) {
  const { t } = useTranslation('price_lists')
  const formatting = useRecordFormatting()
  const invalidate = useInvalidatePriceLists()
  const initial = items.map(toDraft)
  const [drafts, setDrafts] = useState<DraftItem[]>(initial)
  const [errors, setErrors] = useState<Record<string, string>>({})
  const [saving, setSaving] = useState(false)
  const catalog = new Map(items.map((item) => [item.product_id, item]))
  const dirty = signature(drafts) !== signature(initial)

  const update = (key: string, patch: Partial<DraftItem>) => {
    setDrafts(drafts.map((draft) => (draft.key === key ? { ...draft, ...patch } : draft)))
    setErrors({})
  }

  const validate = (): Record<string, string> => {
    const found: Record<string, string> = {}
    drafts.forEach((draft, index) => {
      if (!draft.productId) found[`${index}.product_id`] = t('items.productRequired')
      if (!MONEY_PATTERN.test(draft.unitPrice.trim())) found[`${index}.unit_price`] = t('items.invalidPrice')
      if (draft.minQuantity.trim() && !QUANTITY_PATTERN.test(draft.minQuantity.trim()))
        found[`${index}.min_quantity`] = t('items.invalidQuantity')
    })
    return found
  }

  const save = async () => {
    const found = validate()
    if (Object.keys(found).length > 0) {
      setErrors(found)
      return
    }
    setSaving(true)
    try {
      await priceListsApi.replaceItems(
        list.id,
        drafts.map((draft) => ({
          product_id: draft.productId ?? '',
          unit_price: normalizeDecimal(draft.unitPrice),
          min_quantity: draft.minQuantity.trim() ? normalizeDecimal(draft.minQuantity) : null,
        })),
        list.updated_at,
      )
      await invalidate()
      toast({ title: t('items.saved'), variant: 'success' })
    } catch (error) {
      setErrors(
        Object.fromEntries(
          fieldErrors(error)
            .filter((entry) => entry.field.startsWith('items.'))
            .map((entry) => [entry.field.slice('items.'.length), entry.message]),
        ),
      )
      toast({ title: t(priceListErrorKey(error)), variant: 'destructive' })
    } finally {
      setSaving(false)
    }
  }

  const errorText = (index: number, field: string) => errors[`${index}.${field}`]

  return (
    <section className="grid gap-4" aria-labelledby="price-list-items-title">
      <UnsavedChangesGuard when={dirty && !saving} />
      <div className="flex flex-col gap-2 sm:flex-row sm:items-end sm:justify-between">
        <div className="grid gap-1">
          <h2 id="price-list-items-title" className="text-base font-semibold">
            {title}
          </h2>
          <p className="text-sm text-muted-foreground">{t('items.description', { currency: list.currency })}</p>
        </div>
        {canEdit && (
          <div className="flex flex-wrap gap-2">
            <Button
              size="sm"
              variant="outline"
              onClick={() => setDrafts([...drafts, { key: nextKey(), productId: null, minQuantity: '1', unitPrice: '' }])}
            >
              <Plus aria-hidden="true" />
              {t('items.add')}
            </Button>
            <Button size="sm" disabled={!dirty || saving} onClick={() => void save()}>
              {t('items.save')}
            </Button>
          </div>
        )}
      </div>
      {drafts.length === 0 ? (
        <EmptyState icon={Tags} title={t('items.empty')} description={t('items.emptyHint')} />
      ) : (
        <div className="relative overflow-x-auto rounded-lg border">
          <table className="w-full text-sm">
            <thead className="bg-muted/50 text-left text-muted-foreground">
              <tr>
                <th scope="col" className="min-w-56 px-4 py-2 font-medium">
                  {t('items.product')}
                </th>
                <th scope="col" className="w-32 px-4 py-2 font-medium">
                  {t('items.minQuantity')}
                </th>
                <th scope="col" className="w-40 px-4 py-2 font-medium">
                  {t('items.unitPrice', { currency: list.currency })}
                </th>
                <th scope="col" className="hidden px-4 py-2 text-right font-medium md:table-cell">
                  {t('items.catalogPrice')}
                </th>
                {canEdit && (
                  <th scope="col" className="w-12 px-4 py-2">
                    <span className="sr-only">{t('items.actions')}</span>
                  </th>
                )}
              </tr>
            </thead>
            <tbody>
              {drafts.map((draft, index) => {
                const reference = draft.productId ? catalog.get(draft.productId) : undefined
                return (
                  <tr key={draft.key} className="border-t align-top">
                    <td className="px-4 py-2">
                      <RecordFieldInput
                        targetRecordType={PRODUCT_RECORD_TYPE}
                        value={draft.productId}
                        clearable={false}
                        disabled={!canEdit}
                        invalid={Boolean(errorText(index, 'product_id'))}
                        aria-label={t('items.productOf', { position: index + 1 })}
                        onChange={(value) => update(draft.key, { productId: value })}
                      />
                      {errorText(index, 'product_id') && (
                        <p className="mt-1 text-xs text-destructive">{errorText(index, 'product_id')}</p>
                      )}
                    </td>
                    <td className="px-4 py-2">
                      <Input placeholder={t('placeholders.minQuantity')}
                        inputMode="decimal"
                        value={draft.minQuantity}
                        disabled={!canEdit}
                        aria-label={t('items.minQuantityOf', { position: index + 1 })}
                        aria-invalid={errorText(index, 'min_quantity') ? true : undefined}
                        onChange={(event) => update(draft.key, { minQuantity: event.target.value })}
                      />
                      {errorText(index, 'min_quantity') && (
                        <p className="mt-1 text-xs text-destructive">{errorText(index, 'min_quantity')}</p>
                      )}
                    </td>
                    <td className="px-4 py-2">
                      <Input placeholder={t('placeholders.unitPrice')}
                        inputMode="decimal"
                        value={draft.unitPrice}
                        disabled={!canEdit}
                        aria-label={t('items.unitPriceOf', { position: index + 1 })}
                        aria-invalid={errorText(index, 'unit_price') ? true : undefined}
                        onChange={(event) => update(draft.key, { unitPrice: event.target.value })}
                      />
                      {errorText(index, 'unit_price') && (
                        <p className="mt-1 text-xs text-destructive">{errorText(index, 'unit_price')}</p>
                      )}
                    </td>
                    <td className="hidden px-4 py-2 text-right tabular-nums text-muted-foreground md:table-cell">
                      {reference ? formatCurrency(reference.product_price, formatting, reference.product_currency) : '—'}
                    </td>
                    {canEdit && (
                      <td className="px-4 py-2">
                        <Button
                          size="icon"
                          variant="ghost"
                          aria-label={t('items.remove', { position: index + 1 })}
                          onClick={() => setDrafts(drafts.filter((current) => current.key !== draft.key))}
                        >
                          <X aria-hidden="true" />
                        </Button>
                      </td>
                    )}
                  </tr>
                )
              })}
            </tbody>
          </table>
        </div>
      )}
      <p className="text-xs text-muted-foreground">{t('items.tiersHint')}</p>
    </section>
  )
}
