import { NativeSelect } from '@/components/ui/native-select'
import { Plus, Trash2 } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { MoneyInput } from '@/components/ui/money-input'
import { numberSeparators } from '@/components/fields/lib/number'
import { Slot, formatCurrency, useRecordFormatting } from '@/sdk'
import { DISCOUNT_TYPES, type EditableItem } from './items'
import type { QuoteDiscountType } from './types'
import { EmptyState } from '@/components/ui/empty-state'

const GRID = 'grid grid-cols-1 gap-2 md:grid-cols-[minmax(0,2fr)_5rem_7rem_11rem_5rem_7rem_2.25rem] md:items-center'

export interface QuoteItemsEditorProps {
  currency: string
  items: EditableItem[]
  editable: boolean
  onChange: (items: EditableItem[]) => void
  onAddProduct: (productId: string) => void
}

export function QuoteItemsEditor({ currency, items, editable, onChange, onAddProduct }: QuoteItemsEditorProps) {
  const { t } = useTranslation('quotes')
  const formatting = useRecordFormatting()
  const separators = numberSeparators(formatting.locale)
  const currencySymbol = String(formatCurrency(0, formatting, currency).replace(/[\d.,\s\u00a0]/g, '') || currency)
  const money = (value: string | null | undefined) => (value ? formatCurrency(value, formatting, currency) : '—')

  const update = (key: string, patch: Partial<EditableItem>) =>
    onChange(items.map((item) => (item.key === key ? { ...item, ...patch, saved: undefined } : item)))

  const addFree = () =>
    onChange([
      ...items,
      {
        key: crypto.randomUUID(),
        name: t('editor.items.newItem'),
        quantity: '1',
        unit_price: '0.00',
        discount_type: 'none',
        discount_value: '0',
        tax_percent: '0',
      },
    ])

  return (
    <div className="grid gap-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h2 className="text-base font-semibold">{t('editor.items.title')}</h2>
        {editable && (
          <div className="flex flex-wrap gap-2">
            <Slot name="line_items.catalogPicker" props={{ currency, disabled: false, onSelect: onAddProduct }} />
            <Button size="sm" variant="outline" onClick={addFree}>
              <Plus aria-hidden="true" />
              {t('editor.items.addFree')}
            </Button>
          </div>
        )}
      </div>
      {items.length === 0 ? (
        <EmptyState title={t('editor.items.empty')} />
      ) : (
        <div className="grid gap-2">
          <div className={`${GRID} hidden px-2 text-xs font-medium text-muted-foreground md:grid`}>
            <span>{t('editor.items.name')}</span>
            <span>{t('editor.items.quantity')}</span>
            <span>{t('editor.items.unitPrice')}</span>
            <span>{t('editor.items.discount')}</span>
            <span>{t('editor.items.tax')}</span>
            <span className="text-right">{t('editor.items.total')}</span>
            <span />
          </div>
          {items.map((item) => (
            <div key={item.key} className={`${GRID} rounded-lg border bg-card p-2`}>
              <div className="grid gap-1">
                <Input placeholder={t('placeholders.itemName')}
                  aria-label={t('editor.items.name')}
                  value={item.name ?? ''}
                  disabled={!editable}
                  onChange={(event) => update(item.key, { name: event.target.value })}
                />
                {item.saved && item.saved.components.length > 0 && (
                  <span className="text-xs text-muted-foreground">
                    {t('editor.items.bundle', {
                      components: item.saved.components
                        .map((component) => `${component.quantity}× ${component.name}`)
                        .join(', '),
                    })}
                  </span>
                )}
              </div>
              <Input placeholder={t('placeholders.quantity')}
                aria-label={t('editor.items.quantity')}
                inputMode="decimal"
                value={item.quantity ?? ''}
                disabled={!editable}
                onChange={(event) => update(item.key, { quantity: event.target.value })}
              />
              <MoneyInput
                aria-label={t('editor.items.unitPrice')}
                separators={separators}
                value={String(item.unit_price ?? '')}
                placeholder="0,00"
                disabled={!editable}
                onValueChange={(value) => update(item.key, { unit_price: value })}
              />
              <div className="flex gap-1">
                <NativeSelect value={item.discount_type ?? 'none'}
                  disabled={!editable}
                  onValueChange={(value) => update(item.key, { discount_type: value as QuoteDiscountType })} aria-label={t('editor.fields.discountType')} className="w-20 shrink-0">
                    {DISCOUNT_TYPES.map((type) => (
                      <option key={type} value={type}>
                        {type === 'none' ? '—' : type === 'percent' ? '%' : currencySymbol}
                      </option>
                    ))}
                  </NativeSelect>
                <MoneyInput placeholder="0,00"
                  aria-label={t('editor.items.discount')}
                  separators={separators}
                  value={item.discount_type === 'none' ? '' : String(item.discount_value ?? '')}
                  disabled={!editable || item.discount_type === 'none'}
                  onValueChange={(value) => update(item.key, { discount_value: value })}
                />
              </div>
              <MoneyInput placeholder="0,00"
                aria-label={t('editor.items.tax')}
                separators={separators}
                value={String(item.tax_percent ?? '')}
                disabled={!editable}
                onValueChange={(value) => update(item.key, { tax_percent: value })}
              />
              <span className="text-right text-sm font-medium tabular-nums">{money(item.saved?.total)}</span>
              {editable ? (
                <Button
                  size="icon"
                  variant="ghost"
                  aria-label={t('editor.items.remove')}
                  onClick={() => onChange(items.filter((entry) => entry.key !== item.key))}
                >
                  <Trash2 aria-hidden="true" />
                </Button>
              ) : (
                <span />
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
