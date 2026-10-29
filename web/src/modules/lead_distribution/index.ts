import { Shuffle } from 'lucide-react'
import { LEAD_DISTRIBUTION_MANAGE } from '@/features/settings/lib/permissions'
import { defineModule } from '@/kernel/modules/define'
import { DistributionRuleSelect } from './DistributionRuleSelect'
import './slots'

const leadDistributionModule = defineModule({
  key: 'lead_distribution',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.settings('organization', {
      group: 'sales',
      path: 'lead-distribution',
      label: 'settings:tabs.leadDistribution',
      description: 'settings:items.leadDistribution.description',
      icon: Shuffle,
      permission: LEAD_DISTRIBUTION_MANAGE,
      lazy: async () => ({ Component: (await import('./LeadDistributionPage')).LeadDistributionPage }),
    })
    app.slot('leadDistribution.ruleSelect', {
      key: 'rule-select',
      permission: LEAD_DISTRIBUTION_MANAGE,
      render: DistributionRuleSelect,
    })
  },
})

export default leadDistributionModule
