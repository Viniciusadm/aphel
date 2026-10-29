import { PhoneIncoming, PhoneOutgoing } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { formatCallDuration } from '../lib/format'
import type { CallDirection } from '../types'

export interface CallSummaryProps {
  direction: CallDirection
  outcomeName: string | null
  outcomeKey: string
  connected: boolean
  durationSeconds: number | null
}

export function CallSummary({ direction, outcomeName, outcomeKey, connected, durationSeconds }: CallSummaryProps) {
  const { t } = useTranslation('calls')
  const duration = formatCallDuration(durationSeconds)
  const Icon = direction === 'inbound' ? PhoneIncoming : PhoneOutgoing
  return (
    <span className="inline-flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
      <span className="inline-flex items-center gap-1">
        <Icon className="size-3.5" aria-hidden="true" />
        {t(`direction.${direction}`)}
      </span>
      <Badge variant={connected ? 'success' : 'secondary'}>{outcomeName ?? outcomeKey}</Badge>
      {duration && <span className="tabular-nums">{t('summary.duration', { duration })}</span>}
    </span>
  )
}
