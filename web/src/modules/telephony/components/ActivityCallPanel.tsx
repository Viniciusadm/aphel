import { Phone } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { RecordSlotProps } from '@/kernel/modules/slots'
import type { ActivityCall } from '../types'
import { CallSummary } from './CallSummary'

function callOf(record: RecordSlotProps['record']): ActivityCall | null {
  const call = record.call
  return call !== null && typeof call === 'object' && 'outcome' in call ? (call as ActivityCall) : null
}

export function ActivityCallPanel({ record }: RecordSlotProps) {
  const { t } = useTranslation('telephony')
  const call = callOf(record)
  if (!call) return null
  return (
    <section className="flex flex-wrap items-center gap-x-3 gap-y-1 rounded-lg border bg-card px-3 py-2" aria-label={t('summary.title')}>
      <span className="inline-flex items-center gap-1.5 text-sm font-medium">
        <Phone className="size-4 text-muted-foreground" aria-hidden="true" />
        {t('summary.title')}
      </span>
      <CallSummary
        direction={call.direction}
        outcomeKey={call.outcome.key}
        outcomeName={call.outcome.name ?? null}
        connected={call.outcome.is_connected}
        durationSeconds={call.duration_seconds ?? null}
      />
      {call.phone_number && (
        <span className="text-xs text-muted-foreground">{t('summary.phone', { phone: call.phone_number })}</span>
      )}
    </section>
  )
}
