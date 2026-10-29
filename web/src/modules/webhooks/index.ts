import { Send, Webhook } from 'lucide-react'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import { defineModule } from '@/kernel/modules/define'

const webhooksModule = defineModule({
  key: 'webhooks',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.settings('integrations', {
      group: 'webhooks',
      path: 'webhooks',
      label: 'settings:tabs.webhooks',
      description: 'settings:items.webhooks.description',
      icon: Send,
      permission: SETTINGS_MANAGE,
      lazy: async () => ({ Component: (await import('./outgoing/WebhooksPage')).WebhooksPage }),
    })
    app.settings('integrations', {
      group: 'webhooks',
      path: 'incoming-webhooks',
      label: 'settings:tabs.incomingWebhooks',
      description: 'settings:items.incomingWebhooks.description',
      icon: Webhook,
      permission: SETTINGS_MANAGE,
      lazy: async () => ({ Component: (await import('./incoming/IncomingWebhooksPage')).IncomingWebhooksPage }),
    })
  },
})

export default webhooksModule
