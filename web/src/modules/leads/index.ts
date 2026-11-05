import { History, Route, UserPlus } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { CONTACT_RECORD_TYPE, LEADS_PATH, LIFECYCLE_MANAGE_PERMISSION, LIFECYCLE_TAB_KEY } from './constants'
import { LeadRecordActions } from './LeadRecordActions'
import { LeadsQuickFilter } from './LeadsQuickFilter'
import { LifecycleRecordTab } from './LifecycleHistoryTab'
import { leadsRoutes, lifecycleSettingsRoute } from './routes'

const LIFECYCLE_RECORD_TYPES = ['people.person', 'organizations.organization']

const leadsModule = defineModule({
  key: 'leads',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.settingsGroup('organization', { key: 'sales', label: 'groups.sales', order: 20 })
    app.routes(leadsRoutes)
    app.nav({ section: 'records', to: LEADS_PATH, label: 'nav.leads', icon: UserPlus, permission: ['people.read'], order: 30 })
    app.settings('organization', {
      group: 'sales',
      path: 'lifecycle',
      label: 'settings:tabs.lifecycle',
      description: 'settings:items.lifecycle.description',
      icon: Route,
      permission: LIFECYCLE_MANAGE_PERMISSION,
      routes: [lifecycleSettingsRoute],
    })
    app.recordSection(LIFECYCLE_RECORD_TYPES, {
      key: LIFECYCLE_TAB_KEY,
      group: 'insights',
      label: 'history.tab',
      icon: History,
      order: 10,
      render: LifecycleRecordTab,
    })
    app.slot('record.detail.actions', {
      key: 'lead-actions',
      recordTypes: [CONTACT_RECORD_TYPE],
      permission: 'people.update',
      render: LeadRecordActions,
    })
    app.slot('record.list.toolbar', {
      key: 'lead-filter',
      recordTypes: [CONTACT_RECORD_TYPE],
      render: LeadsQuickFilter,
    })
  },
})

export default leadsModule
