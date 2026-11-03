import { CopyCheck } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { MergedRecordRedirect } from './MergedRecordRedirect'
import { MergeWithAction } from './MergeWithAction'
import { warnPossibleDuplicates } from './warnings'
import { duplicatesSettingsRoutes } from './routes'

const duplicatesModule = defineModule({
  key: 'duplicates',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.settings('data', {
      group: 'compliance',
      path: 'duplicates',
      label: 'settings:tabs.duplicates',
      description: 'settings:items.duplicates.description',
      icon: CopyCheck,
      permission: 'people.read',
      order: 100,
      routes: duplicatesSettingsRoutes,
    })
    app.on('record.created', ({ recordType, record }) => {
      warnPossibleDuplicates(recordType, record)
    })
    app.slot('record.detail.header', {
      key: 'merged-redirect',
      render: MergedRecordRedirect,
    })
    app.slot('record.detail.actions', { key: 'merge-with', render: MergeWithAction })
  },
})

export default duplicatesModule
