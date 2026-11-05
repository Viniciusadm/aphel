import { type TimelineItem } from '@/sdk'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { ENRICHMENT_KEY } from './api'

interface EnrichedPayload {
  cnpj?: string
  source?: string
  fields?: string[]
  changes?: { field?: string }[]
}

export function EnrichmentEventContent({ item }: { item: TimelineItem }) {
  const t = useModuleTranslation(ENRICHMENT_KEY)
  const raw = item.metadata.payload
  const payload = typeof raw === 'object' && raw !== null ? (raw as EnrichedPayload) : {}
  const count = payload.changes?.length ?? payload.fields?.length ?? 0
  return (
    <div className="grid min-w-0 gap-1">
      <p className="text-sm font-medium break-words">{t('timeline.enriched', { count })}</p>
      <p className="text-xs text-muted-foreground break-words">
        {t('timeline.detail', {
          source: payload.source ? t(`sources.${payload.source}`) : '',
          cnpj: payload.cnpj ?? '',
        })}
      </p>
    </div>
  )
}
