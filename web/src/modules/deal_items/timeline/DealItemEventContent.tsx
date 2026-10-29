import { useTranslation } from 'react-i18next'
import type { TimelineItem } from '@/features/records/timeline'
import { DEAL_ITEM_EVENT_TYPES, type DealItemEventType } from './events'

type Payload = Record<string, unknown>

const HEADLINES = {
  'deal.item_added': 'timeline.added',
  'deal.item_updated': 'timeline.updated',
  'deal.item_removed': 'timeline.removed',
  'deal.items_reordered': 'timeline.reordered',
} as const satisfies Record<DealItemEventType, string>

const FIELD_LABELS = {
  name: 'timeline.fields.name',
  product_id: 'timeline.fields.product',
  unit: 'timeline.fields.unit',
  quantity: 'timeline.fields.quantity',
  unit_price: 'timeline.fields.unitPrice',
  discount_type: 'timeline.fields.discountType',
  discount_value: 'timeline.fields.discountValue',
  recurrence: 'timeline.fields.recurrence',
  term_months: 'timeline.fields.term',
  total: 'timeline.fields.total',
} as const

function asObject(value: unknown): Payload {
  return value !== null && typeof value === 'object' && !Array.isArray(value) ? (value as Payload) : {}
}

function asText(value: unknown): string | null {
  return typeof value === 'string' && value.trim() !== '' ? value : null
}

function isDealItemEvent(value: string): value is DealItemEventType {
  return (DEAL_ITEM_EVENT_TYPES as readonly string[]).includes(value)
}

function changedFields(payload: Payload): string[] {
  const changes = Array.isArray(payload.changes) ? payload.changes : []
  return changes.map((change) => asText(asObject(change).field)).filter((field): field is string => field !== null)
}

export function DealItemEventContent({ item }: { item: TimelineItem }) {
  const { t } = useTranslation('deal_items')
  const payload = asObject(item.metadata.payload)
  const rawType = asText(item.metadata.event_type) ?? ''
  const eventType: DealItemEventType = isDealItemEvent(rawType) ? rawType : 'deal.item_updated'
  const actor = item.actor?.display_name?.trim() || t('timeline.someone')
  const name = asText(payload.name) ?? asText(asObject(payload.item).name) ?? t('timeline.someItem')
  const fields =
    eventType === 'deal.item_updated'
      ? changedFields(payload).map((field) =>
          field in FIELD_LABELS ? t(FIELD_LABELS[field as keyof typeof FIELD_LABELS]) : field,
        )
      : []

  return (
    <div className="grid min-w-0 gap-1">
      <p className="text-sm font-medium break-words">{t(HEADLINES[eventType], { actor, name })}</p>
      {fields.length > 0 && (
        <p className="text-sm text-muted-foreground break-words">
          {t('timeline.changedFields', { fields: fields.join(', ') })}
        </p>
      )}
    </div>
  )
}
