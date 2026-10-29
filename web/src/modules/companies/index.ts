import { Building2 } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { CreateCompanyDialog } from './create-action'
import { COMPANIES_PATH, COMPANY_RECORD_TYPE } from './resource'
import { companiesRoutes } from './routes'

const companiesModule = defineModule({
  key: 'companies',
  required: true,
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.routes(companiesRoutes)
    app.nav({ to: COMPANIES_PATH, label: 'nav.companies', icon: Building2, permission: ['companies.read'], order: 40 })
    app.createAction({
      id: 'company',
      label: 'createAction.label',
      icon: Building2,
      permission: 'companies.create',
      recordType: COMPANY_RECORD_TYPE,
      order: 20,
      render: CreateCompanyDialog,
    })
  },
})

export default companiesModule
