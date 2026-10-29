import { Upload } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { ImportAction } from './ImportAction'
import { importsRoutes } from './routes'

const importsModule = defineModule({
  key: 'imports',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.routes(importsRoutes)
    app.settings('data', {
      group: 'transfer',
      path: 'imports',
      label: 'history.title',
      description: 'history.menuDescription',
      icon: Upload,
      order: 120,
      lazy: async () => ({ Component: (await import('./ImportHistoryPage')).ImportHistoryPage }),
    })
    app.slot('record.list.toolbar', { key: 'import', order: 2000, render: ImportAction })
  },
})

export default importsModule
