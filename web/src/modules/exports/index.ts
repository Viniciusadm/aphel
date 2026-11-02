import { FileDown } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { EXPORTABLE_RECORD_TYPES, exportPermission } from './api'
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
    for (const recordType of EXPORTABLE_RECORD_TYPES) {
      app.slot('record.list.actions', { key: recordType, recordTypes: recordType, permission: exportPermission(recordType)!, order: 2010, render: ExportAction })
    }
    app.slot('record.list.actions', { key: 'custom', recordTypes: 'co_*', order: 2010, render: ExportAction })
  },
})

export default exportsModule
