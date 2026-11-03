import { Gauge } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { RecordScorePanel } from './RecordScorePanel'
import { scoringSettingsRoute } from './routes'

const SCORED_RECORD_TYPES = ['people.person', 'organizations.organization']

const scoringModule = defineModule({
  key: 'scoring',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.settings('organization', {
      group: 'sales',
      path: 'scoring',
      label: 'settings:tabs.scoring',
      description: 'settings:items.scoring.description',
      icon: Gauge,
      permission: 'scoring.manage',
      routes: [scoringSettingsRoute],
    })
    app.recordSection(SCORED_RECORD_TYPES, {
      key: 'scoring',
      label: 'record.tab',
      icon: Gauge,
      order: 20,
      render: RecordScorePanel,
    })
  },
})

export default scoringModule
