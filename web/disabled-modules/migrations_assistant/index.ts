import { ArrowRightLeft } from 'lucide-react'
import { SETTINGS_MANAGE } from '@/sdk'
import { defineModule } from '@/kernel/modules/define'

const migrationsAssistantModule = defineModule({
  key: 'migrations_assistant',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.settings('data', {
      group: 'transfer',
      path: 'migration-assistant',
      label: 'settings:tabs.migrationAssistant',
      description: 'settings:items.migrationAssistant.description',
      icon: ArrowRightLeft,
      permission: SETTINGS_MANAGE,
      order: 100,
      lazy: async () => ({ Component: (await import('./MigrationAssistantPage')).MigrationAssistantPage }),
    })
  },
})

export default migrationsAssistantModule
