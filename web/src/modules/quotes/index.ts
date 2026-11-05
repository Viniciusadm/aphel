import { FileText } from 'lucide-react'
import { SETTINGS_MANAGE } from '@/sdk'
import { defineModule } from '@/kernel/modules/define'
import { QUOTES_TAB_KEY } from './DealQuotesTab'
import { DealQuotesRecordTab } from './DealQuotesRecordTab'
import { publicQuoteRoutes } from './public/routes'
import { quotesRoutes, quotesSettingsRoute } from './routes'
import './slots'

export { QUOTE_RECORD_TYPE } from './resource'
export type { QuoteEditorSlotProps } from './slots'

const DEAL_RECORD_TYPE = 'deals.deal'

const quotesModule = defineModule({
  key: 'quotes',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.settingsGroup('organization', { key: 'sales', label: 'groups.sales', order: 20 })
    app.publicRoutes(publicQuoteRoutes)
    app.routes(quotesRoutes)
    app.nav({ to: '/quotes', label: 'nav.quotes', icon: FileText, permission: ['quotes.read'], order: 70 })
    app.settings('organization', {
      group: 'sales',
      path: 'quotes',
      label: 'settings:tabs.quotes',
      description: 'settings:items.quotes.description',
      icon: FileText,
      permission: SETTINGS_MANAGE,
      routes: [quotesSettingsRoute],
    })
    app.recordSection(DEAL_RECORD_TYPE, {
      key: QUOTES_TAB_KEY,
      group: 'relationships',
      label: 'deal.tab',
      icon: FileText,
      permission: 'quotes.read',
      order: 25,
      render: DealQuotesRecordTab,
    })
  },
})

export default quotesModule
