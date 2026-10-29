import { ListChecks } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { BULK_RECORD_TYPES } from './api'
import { BulkActionBar } from './BulkActionBar'

const bulkActionsModule = defineModule({
  key: 'bulk_actions',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  namespaces: {
    notifications: {
      'pt-BR': {
        headline: {
          bulk_operation_finished: 'Sua ação em massa foi concluída',
          bulk_operation_failed: 'Sua ação em massa falhou',
        },
        kinds: {
          bulk_operation_finished: {
            label: 'Ação em massa concluída',
            description: 'Quando uma ação em massa sua termina de processar os registros.',
          },
          bulk_operation_failed: {
            label: 'Ação em massa com falha',
            description: 'Quando uma ação em massa sua não consegue ser concluída.',
          },
        },
      },
      en: {
        headline: {
          bulk_operation_finished: 'Your bulk action finished',
          bulk_operation_failed: 'Your bulk action failed',
        },
        kinds: {
          bulk_operation_finished: {
            label: 'Bulk action finished',
            description: 'When a bulk action of yours finishes processing the records.',
          },
          bulk_operation_failed: {
            label: 'Bulk action failed',
            description: 'When a bulk action of yours could not be finished.',
          },
        },
      },
    },
  },
  register(app) {
    app.settings('data', {
      group: 'transfer',
      path: 'bulk-operations',
      label: 'history.title',
      description: 'history.menuDescription',
      icon: ListChecks,
      order: 140,
      lazy: async () => ({ Component: (await import('./BulkHistoryPage')).BulkHistoryPage }),
    })
    app.slot('record.list.bulkActions', {
      key: 'bulk-actions',
      recordTypes: [...BULK_RECORD_TYPES],
      render: BulkActionBar,
    })
  },
})

export default bulkActionsModule
