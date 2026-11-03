import { ShoppingCart } from 'lucide-react'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import { defineModule } from '@/kernel/modules/define'
import './slots'
const settingsRoute = { path: 'line-items', lazy: async () => ({ Component: (await import('./SettingsPage')).LineItemSettingsPage }) }
import { LineItemsRecordTab } from './record-tabs'
import { DEAL_ITEM_TIMELINE_RENDERERS } from './timeline'

import { getRecordResource } from '@/features/records/resources'
const acceptsItems = (recordType: string) => Boolean(getRecordResource(recordType)?.traits?.['line_items.attachable'])

const dealItemsModule = defineModule({
  key: 'line_items',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.settings('organization', {
      group: 'records',
      path: 'line-items',
      label: 'settings.title',
      description: 'settings.description',
      icon: ShoppingCart,
      permission: SETTINGS_MANAGE,
      routes: [settingsRoute],
    })
    app.recordSection(acceptsItems, {
      key: 'items',
      label: 'items.tab',
      icon: ShoppingCart,
      order: 20,
      render: LineItemsRecordTab,
    })
    for (const [eventType, renderer] of Object.entries(DEAL_ITEM_TIMELINE_RENDERERS))
      app.eventRenderer(eventType, renderer)
  },
})

export default dealItemsModule
