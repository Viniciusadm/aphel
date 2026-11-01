import { Building2 } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { lazyComponent } from '@/lib/lazy-component'
import { ENRICHMENT_KEY, USE_PERMISSION } from './api'
import { EnrichmentEventContent } from './EventContent'

const CompanyEnrichmentTab = lazyComponent(async () => (await import('./CompanyEnrichmentTab')).CompanyEnrichmentTab)

const enrichmentModule = defineModule({
  key: ENRICHMENT_KEY,
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.recordSection('company', {
      key: `${ENRICHMENT_KEY}.enrichment`,
      label: 'tab',
      icon: Building2,
      permission: [USE_PERMISSION],
      render: CompanyEnrichmentTab,
    })
    app.eventRenderer('company.enriched', {
      icon: Building2,
      render: (item) => <EnrichmentEventContent item={item} />,
    })
  },
})

export default enrichmentModule
