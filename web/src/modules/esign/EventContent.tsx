import { type TimelineItem } from '@/sdk'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import {ESIGN_KEY} from '@/sdk'

interface EsignEventPayload {
  file_name?: string
  signer_name?: string
  signed_file_name?: string
  source?: string
  signers?: { name?: string; status?: string }[]
}

function payloadOf(item: TimelineItem): EsignEventPayload {
  const payload = item.metadata.payload
  return typeof payload === 'object' && payload !== null ? (payload as EsignEventPayload) : {}
}

export function EsignEventContent({ item }: { item: TimelineItem }) {
  const t = useModuleTranslation(ESIGN_KEY)
  const eventType = typeof item.metadata.event_type === 'string' ? item.metadata.event_type : ''
  const action = eventType.replace(/^esign\./, '')
  const payload = payloadOf(item)
  const signers = (payload.signers ?? []).map((signer) => signer.name).filter(Boolean)
  return (
    <div className="grid min-w-0 gap-1">
      <p className="text-sm font-medium break-words">
        {t(`timeline.${action}`, { name: payload.signer_name ?? '', file: payload.file_name ?? '' })}
      </p>
      {payload.file_name && <p className="truncate text-sm text-muted-foreground">{payload.file_name}</p>}
      {action === 'sent' && signers.length > 0 && (
        <p className="text-xs text-muted-foreground break-words">{t('timeline.signers', { names: signers.join(', ') })}</p>
      )}
      {payload.source === 'provider' && action === 'cancelled' && (
        <p className="text-xs text-muted-foreground">{t('timeline.byProvider')}</p>
      )}
    </div>
  )
}
