import { Plus, Tags, Trash2, X } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { PageLoader, UnsavedChangesGuard } from '@/components/feedback'
import { RecordFieldInput } from '@/components/fields'
import { numberSeparators } from '@/components/fields/lib/number'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { MoneyInput } from '@/components/ui/money-input'
import { toast } from '@/components/ui/toast-store'
import { formatCurrency, priceListsApi, useInvalidatePriceLists, usePriceListItems, useRecordFormatting } from '@/sdk'
import type { PriceList, PriceListItem } from '@/sdk'
import { fieldErrors, priceListErrorKey } from './errors'

const PRODUCT_RECORD_TYPE = 'catalog_items.item'
const QUANTITY_PATTERN = /^\d+([.,]\d{1,4})?$/

interface Tier {
  key: string
  minQuantity: string
  price: string
}

interface ProductGroup {
  key: string
  productId: string | null
  tiers: Tier[]
}

let sequence = 0
const nextKey = () => `draft-${++sequence}`

const decimal = (value: string) => value.trim().replace(',', '.')

function money(value: string): string {
  const number = Number(decimal(value))
  return Number.isFinite(number) && value.trim() !== '' ? number.toFixed(2) : ''
}

function groupItems(items: PriceListItem[]): ProductGroup[] {
  const groups = new Map<string, ProductGroup>()
  for (const item of items) {
    const group = groups.get(item.product_id) ?? { key: nextKey(), productId: item.product_id, tiers: [] }
    group.tiers.push({ key: item.id, minQuantity: String(Number(item.min_quantity)), price: money(item.unit_price) })
    groups.set(item.product_id, group)
  }
  return [...groups.values()].map((group) => ({
    ...group,
    tiers: [...group.tiers].sort((left, right) => Number(left.minQuantity) - Number(right.minQuantity)),
  }))
}

function signature(groups: ProductGroup[]): string {
  return JSON.stringify(groups.map((group) => [group.productId, group.tiers.map((tier) => [decimal(tier.minQuantity), tier.price])]))
}

export function PriceListItemsEditor({ list, canEdit }: { list: PriceList; canEdit: boolean }) {
  const items = usePriceListItems(list.id)
  if (items.isPending) return <PageLoader />
  if (items.isError) return <ErrorState onRetry={() => void items.refetch()} />
  return <ItemsForm key={items.dataUpdatedAt} list={list} items={items.data} canEdit={canEdit} />
}

function ItemsForm({ list, items, canEdit }: { list: PriceList; items: PriceListItem[]; canEdit: boolean }) {
  const { t } = useTranslation('pricing')
  const formatting = useRecordFormatting()
  const invalidate = useInvalidatePriceLists()
  const initial = groupItems(items)
  const [groups, setGroups] = useState<ProductGroup[]>(initial)
  const [errors, setErrors] = useState<Record<string, string>>({})
  const [saving, setSaving] = useState(false)
  const catalog = new Map(items.map((item) => [item.product_id, item]))
  const dirty = signature(groups) !== signature(initial)
  const separators = numberSeparators(formatting.locale)

  const change = (next: ProductGroup[]) => {
    setGroups(next)
    setErrors({})
  }
  const patchGroup = (key: string, patch: Partial<ProductGroup>) =>
    change(groups.map((group) => (group.key === key ? { ...group, ...patch } : group)))
  const patchTier = (groupKey: string, tierKey: string, patch: Partial<Tier>) =>
    change(
      groups.map((group) =>
        group.key === groupKey
          ? { ...group, tiers: group.tiers.map((tier) => (tier.key === tierKey ? { ...tier, ...patch } : tier)) }
          : group,
      ),
    )

  const validate = (): Record<string, string> => {
    const found: Record<string, string> = {}
    const seen = new Set<string>()
    groups.forEach((group) => {
      if (!group.productId) found[`${group.key}.product`] = t('items.productRequired')
      else if (seen.has(group.productId)) found[`${group.key}.product`] = t('items.productRepeated')
      else seen.add(group.productId)
      const quantities = new Set<string>()
      group.tiers.forEach((tier) => {
        if (!tier.price) found[`${tier.key}.price`] = t('items.invalidPrice')
        const quantity = decimal(tier.minQuantity)
        if (!QUANTITY_PATTERN.test(quantity) || Number(quantity) <= 0) found[`${tier.key}.quantity`] = t('items.invalidQuantity')
        else if (quantities.has(String(Number(quantity)))) found[`${tier.key}.quantity`] = t('items.quantityRepeated')
        else quantities.add(String(Number(quantity)))
      })
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
        groups.flatMap((group) =>
          group.tiers.map((tier) => ({
            product_id: group.productId ?? '',
            unit_price: tier.price,
            min_quantity: decimal(tier.minQuantity),
          })),
        ),
        list.updated_at,
      )
      await invalidate()
      toast({ title: t('items.saved'), variant: 'success' })
    } catch (error) {
      const first = fieldErrors(error).find((entry) => entry.field.startsWith('items.'))
      if (first) setErrors({ form: first.message })
      toast({ title: t(priceListErrorKey(error)), variant: 'destructive' })
    } finally {
      setSaving(false)
    }
  }

  return (
    <section className="grid gap-4" aria-labelledby="price-list-items-title">
      <UnsavedChangesGuard when={dirty && !saving} />
      <div className="flex flex-col gap-2 sm:flex-row sm:items-end sm:justify-between">
        <div className="grid gap-1">
          <h2 id="price-list-items-title" className="text-base font-semibold">
            {t('items.title')}
          </h2>
          <p className="text-sm text-muted-foreground">{t('items.description', { currency: list.currency })}</p>
        </div>
        {canEdit && (
          <div className="flex flex-wrap gap-2">
            <Button
              size="sm"
              variant="outline"
              onClick={() => change([...groups, { key: nextKey(), productId: null, tiers: [{ key: nextKey(), minQuantity: '1', price: '' }] }])}
            >
              <Plus aria-hidden="true" />
              {t('items.addProduct')}
            </Button>
            <Button size="sm" disabled={!dirty || saving} onClick={() => void save()}>
              {t('items.save')}
            </Button>
          </div>
        )}
      </div>
      {errors.form && <p role="alert" className="text-sm text-destructive">{errors.form}</p>}
      {groups.length === 0 ? (
        <EmptyState icon={Tags} title={t('items.empty')} description={t('items.emptyHint')} />
      ) : (
        <ul className="grid gap-3">
          {groups.map((group) => {
            const reference = group.productId ? catalog.get(group.productId) : undefined
            return (
              <li key={group.key} className="grid gap-3 rounded-lg border bg-card p-4">
                <div className="flex flex-wrap items-start gap-3">
                  <div className="grid min-w-56 flex-1 gap-1">
                    <Field label={t('items.product')} error={errors[`${group.key}.product`]}>
                      {(control) => (
                        <RecordFieldInput
                          {...control}
                          targetRecordType={PRODUCT_RECORD_TYPE}
                          value={group.productId}
                          clearable={false}
                          disabled={!canEdit}
                          invalid={Boolean(errors[`${group.key}.product`])}
                          onChange={(value) => patchGroup(group.key, { productId: value })}
                        />
                      )}
                    </Field>
                    {reference && (
                      <p className="text-xs text-muted-foreground">
                        {t('items.catalogPriceOf', { price: formatCurrency(reference.product_price, formatting, reference.product_currency) })}
                      </p>
                    )}
                  </div>
                  {canEdit && (
                    <Button
                      size="sm"
                      variant="ghost"
                      className="mt-6 text-destructive"
                      onClick={() => change(groups.filter((current) => current.key !== group.key))}
                    >
                      <Trash2 aria-hidden="true" />
                      {t('items.removeProduct')}
                    </Button>
                  )}
                </div>
                <div className="grid gap-2">
                  {group.tiers.map((tier, index) => (
                    <div key={tier.key} className="flex flex-wrap items-start gap-3">
                      <Field label={index === 0 ? t('items.fromQuantity') : undefined} hideLabel={index > 0} error={errors[`${tier.key}.quantity`]} className="w-40">
                        {(control) => (
                          <Input
                            {...control}
                            inputMode="decimal"
                            value={tier.minQuantity}
                            disabled={!canEdit}
                            aria-label={t('items.fromQuantity')}
                            onChange={(event) => patchTier(group.key, tier.key, { minQuantity: event.target.value })}
                          />
                        )}
                      </Field>
                      <Field label={index === 0 ? t('items.unitPrice', { currency: list.currency }) : undefined} hideLabel={index > 0} error={errors[`${tier.key}.price`]} className="w-44">
                        {(control) => (
                          <MoneyInput
                            {...control}
                            separators={separators}
                            placeholder={t('placeholders.unitPrice')}
                            value={tier.price}
                            disabled={!canEdit}
                            aria-label={t('items.unitPrice', { currency: list.currency })}
                            onValueChange={(value) => patchTier(group.key, tier.key, { price: value })}
                          />
                        )}
                      </Field>
                      {canEdit && group.tiers.length > 1 && (
                        <Button
                          size="icon"
                          variant="ghost"
                          className={index === 0 ? 'mt-6' : undefined}
                          aria-label={t('items.removeTier')}
                          onClick={() => patchGroup(group.key, { tiers: group.tiers.filter((current) => current.key !== tier.key) })}
                        >
                          <X aria-hidden="true" />
                        </Button>
                      )}
                    </div>
                  ))}
                </div>
                {canEdit && (
                  <div>
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() => patchGroup(group.key, { tiers: [...group.tiers, { key: nextKey(), minQuantity: '', price: '' }] })}
                    >
                      <Plus aria-hidden="true" />
                      {t('items.addTier')}
                    </Button>
                  </div>
                )}
              </li>
            )
          })}
        </ul>
      )}
      <p className="text-xs text-muted-foreground">{t('items.tiersHint')}</p>
    </section>
  )
}
