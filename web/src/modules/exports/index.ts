import { FileDown } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { ExportAction } from './ExportAction'

const exportsModule = defineModule({
  key: 'exports',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.settings('data', {
      group: 'transfer',
      path: 'exports',
      label: 'history.title',
      description: 'history.menuDescription',
      icon: FileDown,
      order: 130,
      lazy: async () => ({ Component: (await import('./ExportHistoryPage')).ExportHistoryPage }),
    })
    app.slot('record.list.toolbar', { key: 'export', order: 2010, render: ExportAction })
  },
})

export default exportsModule
