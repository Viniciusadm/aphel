import type { TFunction } from 'i18next'
import { formatCurrency, formatDate, type RecordFormatting, type TimelineItem } from '@/sdk'
import { asText, eventType, nameOf, payloadOf } from '@/modules/timeline'

export function dealChangeText(
  item: TimelineItem,
  t: TFunction<'tasks'>,
  formatting: RecordFormatting,
): string {
  const payload = payloadOf(item)
  const currency = asText(payload.currency)
  const money = (value: unknown) =>
    value === null || value === undefined ? t('timeline.empty') : formatCurrency(value, formatting, currency)
  const date = (value: unknown) => (asText(value) ? formatDate(value, formatting) : t('timeline.empty'))
  switch (eventType(item)) {
    case 'deal.amount_changed':
      return t('timeline.amountChanged', { from: money(payload.old), to: money(payload.new) })
    case 'deal.owner_changed': {
      const owner = nameOf(payload.new)
      return owner ? t('timeline.ownerChanged', { name: owner }) : t('timeline.ownerRemoved')
    }
    case 'deal.expected_close_date_changed':
      return t('timeline.closeDateChanged', { from: date(payload.old), to: date(payload.new) })
    case 'deal.won':
      return payload.amount !== undefined && payload.amount !== null
        ? t('timeline.dealWonAmount', { amount: money(payload.amount) })
        : t('timeline.dealWon')
    case 'deal.lost': {
      const reason = nameOf(payload.lost_reason)
      return reason ? t('timeline.dealLostReason', { reason }) : t('timeline.dealLost')
    }
    case 'deal.reopened':
      return t('timeline.dealReopened')
    default:
      return t('timeline.dealChanged')
  }
}
