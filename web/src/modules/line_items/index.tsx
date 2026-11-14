import { ShoppingCart } from 'lucide-react'
import { getRecordResource } from '@/sdk'
import { defineModule } from '@/kernel/modules/define'
import './slots'
import { LineItemsRecordTab } from './record-tabs'
import { DEAL_ITEM_TIMELINE_RENDERERS } from './timeline'

const acceptsItems = (recordType: string) => Boolean(getRecordResource(recordType)?.traits?.['line_items.attachable'])

const dealItemsModule = defineModule({
  key: 'line_items',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
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
