import { Shuffle } from 'lucide-react'
import { ASSIGNMENT_MANAGE } from '@/sdk'
import { defineModule } from '@/kernel/modules/define'
import { DistributionRuleSelect } from './DistributionRuleSelect'
import './slots'

const leadDistributionModule = defineModule({
  key: 'assignment',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.settingsGroup('organization', { key: 'sales', label: 'groups.sales', order: 20 })
    app.settings('organization', {
      group: 'sales',
      path: 'assignment',
      label: 'settings:tabs.leadDistribution',
      description: 'settings:items.leadDistribution.description',
      icon: Shuffle,
      permission: ASSIGNMENT_MANAGE,
      lazy: async () => ({ Component: (await import('./LeadDistributionPage')).LeadDistributionPage }),
      routes: [
        { path: 'assignment/new', lazy: async () => ({ Component: (await import('./RuleEditorPage')).RuleEditorPage }) },
        { path: 'assignment/:id/edit', lazy: async () => ({ Component: (await import('./RuleEditorPage')).RuleEditorPage }) },
      ],
    })
    app.slot('assignment.ruleSelect', {
      key: 'rule-select',
      permission: ASSIGNMENT_MANAGE,
      render: DistributionRuleSelect,
    })
  },
})

export default leadDistributionModule
