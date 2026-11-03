import { ArrowDown, ArrowUp, Package, Trash2 } from 'lucide-react'
import { useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { formatCurrency, useRecordFormatting } from '@/sdk'
import { DISCOUNT_TYPES, toDecimalInput, type LineItem, type LineItemRequest, type DiscountType } from '@/sdk/line-items'
import { ItemRecurrenceFields } from './recurring/ItemRecurrenceFields'

export const ITEM_GRID_CLASS =
  '@3xl:grid-cols-[minmax(9rem,1fr)_5rem_7rem_11rem_6rem_7rem_6.5rem] @3xl:items-center @3xl:gap-2'

interface Draft {
  name: string
  quantity: string
  unit_price: string
  discount_type: DiscountType
  discount_value: string
  tax_percent: string
}

function draftOf(item: LineItem): Draft {
  return {
    name: item.name,
    quantity: item.quantity,
    unit_price: item.unit_price,
    discount_type: item.discount_type,
    discount_value: item.discount_value,
    tax_percent: item.tax_percent,
  }
}

export interface LineItemRowProps {
  item: LineItem
  currency: string
  readOnly: boolean
  isFirst: boolean
  isLast: boolean
  busy: boolean
  onSave: (item: LineItem, body: LineItemRequest) => Promise<void>
  onRemove: (item: LineItem) => void
  onMove: (item: LineItem, offset: -1 | 1) => void
}

function Cell({ label, className, children }: { label: string; className?: string; children: ReactNode }) {
  return (
    <div className={className}>
      <span className="mb-1 block text-xs text-muted-foreground @3xl:sr-only">{label}</span>
      {children}
    </div>
  )
}

export function LineItemRow({
  item,
  currency,
  readOnly,
  isFirst,
  isLast,
  busy,
  onSave,
  onRemove,
  onMove,
}: LineItemRowProps) {
  const { t } = useTranslation('line_items')
  const formatting = useRecordFormatting()
  const [draft, setDraft] = useState<Draft>(() => draftOf(item))
  const money = (value: string) => formatCurrency(value, formatting, currency)

  const commit = async (patch: Partial<Draft>) => {
    const next = { ...draft, ...patch }
    const body: LineItemRequest = {}
    if (next.name.trim() !== item.name) body.name = next.name.trim()
    if (toDecimalInput(next.quantity) !== item.quantity) body.quantity = toDecimalInput(next.quantity)
    if (toDecimalInput(next.unit_price) !== item.unit_price) body.unit_price = toDecimalInput(next.unit_price)
    if (toDecimalInput(next.tax_percent) !== item.tax_percent) body.tax_percent = toDecimalInput(next.tax_percent)
    if (next.discount_type !== item.discount_type) body.discount_type = next.discount_type
    if (next.discount_type !== 'none' && toDecimalInput(next.discount_value) !== item.discount_value) {
      body.discount_value = toDecimalInput(next.discount_value)
    }
    if (Object.keys(body).length === 0) return
    try {
      await onSave(item, body)
    } catch {
      setDraft(draftOf(item))
    }
  }

  const field = (key: keyof Draft) => ({
    value: draft[key],
    disabled: readOnly || busy,
    onChange: (event: { target: { value: string } }) =>
      setDraft((current) => ({ ...current, [key]: event.target.value })),
    onBlur: () => void commit({}),
    onKeyDown: (event: { key: string; currentTarget: HTMLInputElement }) => {
      if (event.key === 'Enter') event.currentTarget.blur()
    },
  })

  const discountSuffix = draft.discount_type === 'percent' ? '%' : currency

  return (
    <li
      className={`grid grid-cols-2 gap-3 rounded-md border p-3 @3xl:rounded-none @3xl:border-0 @3xl:border-b @3xl:px-2 @3xl:py-2 ${ITEM_GRID_CLASS}`}
    >
      <Cell label={t('items.columns.item')} className="col-span-2 min-w-0 @3xl:col-span-1">
        <Input placeholder={t('placeholders.item')} aria-label={t('items.columns.item')} maxLength={255} {...field('name')} />
        <span className="mt-1 flex min-w-0 items-center gap-1 text-xs text-muted-foreground">
          {item.product_id && <Package className="size-3 shrink-0" aria-label={t('items.fromCatalog')} />}
          <span className="truncate">
            {[item.unit, t('items.unitPriceHint', { price: money(item.unit_price) })].filter(Boolean).join(' · ')}
          </span>
        </span>
      </Cell>
      <Cell label={t('items.columns.quantity')} className="min-w-0">
        <Input placeholder={t('placeholders.quantity')}
          aria-label={t('items.columns.quantity')}
          inputMode="decimal"
          className="tabular-nums"
          {...field('quantity')}
        />
      </Cell>
      <Cell label={t('items.columns.unitPrice')} className="min-w-0">
        <Input placeholder={t('placeholders.unitPrice')}
          aria-label={t('items.columns.unitPrice')}
          inputMode="decimal"
          className="tabular-nums"
          {...field('unit_price')}
        />
      </Cell>
      <Cell label={t('items.columns.discount')} className="col-span-2 min-w-0 @3xl:col-span-1">
        <div className="flex min-w-0 gap-2">
          <NativeSelect
            aria-label={t('items.discountType')}
            className="w-28 shrink-0"
            value={draft.discount_type}
            disabled={readOnly || busy}
            onChange={(event) => {
              const discountType = event.target.value as DiscountType
              const patch: Partial<Draft> = {
                discount_type: discountType,
                discount_value: discountType === 'none' ? '0.00' : draft.discount_value,
              }
              setDraft((current) => ({ ...current, ...patch }))
              void commit(patch)
            }}
          >
            {DISCOUNT_TYPES.map((value) => (
              <option key={value} value={value}>
                {t(`items.discountTypes.${value}`)}
              </option>
            ))}
          </NativeSelect>
          <div className="relative min-w-0 flex-1">
            <Input placeholder={t('placeholders.discount')}
              aria-label={t('items.discountValue')}
              inputMode="decimal"
              className="pr-10 tabular-nums"
              {...field('discount_value')}
              disabled={readOnly || busy || draft.discount_type === 'none'}
            />
            <span className="pointer-events-none absolute top-1/2 right-2.5 -translate-y-1/2 text-xs text-muted-foreground">
              {discountSuffix}
            </span>
          </div>
        </div>
      </Cell>
      <Cell label={t('items.columns.tax')} className="min-w-0">
        <Input aria-label={t('items.columns.tax')} inputMode="decimal" {...field('tax_percent')} />
        <span className="text-xs text-muted-foreground">{money(item.tax)}</span>
      </Cell>
      <Cell label={t('items.columns.total')} className="min-w-0 @3xl:text-right">
        <span className="block truncate font-medium tabular-nums">{money(item.total)}</span>
        {item.discount_amount !== '0.00' && (
          <span className="block truncate text-xs text-muted-foreground tabular-nums line-through">
            {money(item.gross)}
          </span>
        )}
      </Cell>
      <div className="flex items-end justify-end gap-1 @3xl:items-center">
        {!readOnly && (
          <>
            <Button
              type="button"
              size="icon"
              variant="ghost"
              className="size-8"
              aria-label={t('items.moveUp')}
              disabled={busy || isFirst}
              onClick={() => onMove(item, -1)}
            >
              <ArrowUp aria-hidden="true" />
            </Button>
            <Button
              type="button"
              size="icon"
              variant="ghost"
              className="size-8"
              aria-label={t('items.moveDown')}
              disabled={busy || isLast}
              onClick={() => onMove(item, 1)}
            >
              <ArrowDown aria-hidden="true" />
            </Button>
            <Button
              type="button"
              size="icon"
              variant="ghost"
              className="size-8 text-destructive hover:text-destructive"
              aria-label={t('items.remove', { name: item.name })}
              disabled={busy}
              onClick={() => onRemove(item)}
            >
              <Trash2 aria-hidden="true" />
            </Button>
          </>
        )}
      </div>
      <div className="col-span-2 @3xl:col-span-6">
        <ItemRecurrenceFields
          item={item}
          currency={currency}
          disabled={readOnly || busy}
          onSave={(body) => onSave(item, body)}
        />
      </div>
    </li>
  )
}
