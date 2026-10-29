import { ShieldCheck } from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { AI_FEATURES, AI_KEY, providerLabelKey, type AiStatus } from './api'

export interface DataDisclosureProps {
  status?: AiStatus
  compact?: boolean
}

export function DataDisclosure({ status, compact = false }: DataDisclosureProps) {
  const t = useModuleTranslation(AI_KEY)
  return (
    <section className="grid gap-3 rounded-md border bg-card p-4" aria-label={t('disclosure.title')}>
      <div className="flex items-start gap-2">
        <ShieldCheck className="mt-0.5 size-5 shrink-0 text-muted-foreground" aria-hidden="true" />
        <div className="grid gap-1">
          <h2 className="text-sm font-semibold">{t('disclosure.title')}</h2>
          <p className="text-sm text-muted-foreground">{t('disclosure.intro')}</p>
        </div>
      </div>
      {status && (
        <dl className="grid gap-x-4 gap-y-1 text-sm sm:grid-cols-[max-content_1fr]">
          <dt className="text-muted-foreground">{t('disclosure.provider')}</dt>
          <dd className="break-words">{t(providerLabelKey(status.provider))}</dd>
          <dt className="text-muted-foreground">{t('disclosure.url')}</dt>
          <dd className="font-mono text-xs break-all">{status.base_url}</dd>
          <dt className="text-muted-foreground">{t('disclosure.model')}</dt>
          <dd className="font-mono text-xs break-all">{status.model || '—'}</dd>
          <dt className="text-muted-foreground">{t('disclosure.personalData')}</dt>
          <dd>{t(status.send_personal_data ? 'disclosure.personalOn' : 'disclosure.personalOff')}</dd>
        </dl>
      )}
      <ul className="grid gap-2">
        {AI_FEATURES.map((feature) => (
          <li key={feature} className="grid gap-0.5 rounded-md border px-3 py-2">
            <div className="flex flex-wrap items-center gap-2">
              <span className="text-sm font-medium">{t(`features.${feature}`)}</span>
              {status && !status.features[feature] && <Badge variant="secondary">{t('disclosure.off')}</Badge>}
            </div>
            <p className="text-xs text-muted-foreground">{t(`disclosure.sends.${feature}`)}</p>
          </li>
        ))}
      </ul>
      <ul className="grid list-disc gap-1 pl-5 text-xs text-muted-foreground">
        <li>{t('disclosure.neverSensitive')}</li>
        <li>{t('disclosure.neverHidden')}</li>
        {!compact && <li>{t('disclosure.neverStored')}</li>}
        {!compact && <li>{t('disclosure.untrusted')}</li>}
      </ul>
    </section>
  )
}
