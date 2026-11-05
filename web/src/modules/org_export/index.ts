import { Download } from 'lucide-react'
import { ORGANIZATION_EXPORT } from '@/sdk'
import { defineModule } from '@/kernel/modules/define'

const orgExportModule = defineModule({
  key: 'org_export',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.settings('data', {
      group: 'transfer',
      path: 'org-export',
      label: 'settings:tabs.orgExport',
      description: 'settings:items.orgExport.description',
      icon: Download,
      permission: ORGANIZATION_EXPORT,
      order: 110,
      lazy: async () => ({ Component: (await import('./OrgExportPage')).OrgExportPage }),
    })
  },
})

export default orgExportModule
