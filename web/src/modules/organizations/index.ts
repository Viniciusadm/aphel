import { OrganizationsDashboardEventLabel } from './dashboard/EventLabel'
import { createElement } from 'react'
import { Building2 } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { CreateCompanyDialog } from './create-action'
import { COMPANIES_PATH, COMPANY_RECORD_TYPE } from './resource'
import { companiesRoutes } from './routes'

const companiesModule = defineModule({
  key: 'organizations',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.filter('dashboard.eventLabel', (label, { event }) => event.event_type.startsWith('company.') ? createElement(OrganizationsDashboardEventLabel, { event }) : label)
    app.routes(companiesRoutes)
    app.nav({ section: 'records', shortcut: 'e', to: COMPANIES_PATH, label: 'nav.organizations', icon: Building2, permission: ['organizations.read'], order: 40 })
    app.createAction({
      id: 'company',
      label: 'createAction.label',
      icon: Building2,
      permission: 'organizations.create',
      recordType: COMPANY_RECORD_TYPE,
      order: 20,
      render: CreateCompanyDialog,
    })
  },
})

export default companiesModule
