import { ClipboardList } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { LEAD_FORMS_KEY, MANAGE_PERMISSION } from './api'

const leadFormsModule = defineModule({
  key: LEAD_FORMS_KEY,
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.routes([
      {
        path: 'lead-forms',
        children: [{ index: true, lazy: async () => ({ Component: (await import('./LeadFormsPage')).LeadFormsPage }) }],
      },
    ])
    app.nav({
      to: '/lead-forms',
      label: 'nav',
      icon: ClipboardList,
      permission: [MANAGE_PERMISSION],
    })
  },
})

export default leadFormsModule
