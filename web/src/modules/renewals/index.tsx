import { defineModule } from '@/kernel/modules/define'
import { PipelineRenewalSection } from './PipelineRenewalSection'
import { RenewalsDueWidget } from './RenewalsDueWidget'
import type { RenewalsDueData } from './types'

const renewalsModule = defineModule({
  key: 'renewals',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  namespaces: {
    notifications: {
      'pt-BR': {
        kinds: {
          renewal_created: {
            label: 'Renovação criada',
            description: 'Quando uma oportunidade de renovação é criada para um contrato seu que está terminando.',
          },
        },
      },
      en: {
        kinds: {
          renewal_created: {
            label: 'Renewal created',
            description: 'When a renewal deal is created for a contract of yours that is ending.',
          },
        },
      },
    },
  },
  register(app) {
    app.slot('pipelines.settings.sections', {
      key: 'renewal',
      permission: 'settings.manage',
      render: PipelineRenewalSection,
    })
    app.widget('renewals_due', {
      render: (data) => <RenewalsDueWidget data={data as RenewalsDueData} />,
      isEmpty: (data) => (data as RenewalsDueData).count === 0,
      settings: [
        { kind: 'integer', key: 'days', label: 'renewals:widgets.renewals_due.days', min: 1, max: 365 },
        { kind: 'boolean', key: 'only_mine', label: 'renewals:widgets.renewals_due.onlyMine' },
        { kind: 'integer', key: 'limit', label: 'renewals:widgets.renewals_due.limit', min: 1, max: 20 },
      ],
    })
  },
})

export default renewalsModule
