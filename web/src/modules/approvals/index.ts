import { ShieldCheck } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { ApprovalRequestsPanel } from './components/ApprovalRequestsPanel'
import { ApprovalsNavBadge } from './components/ApprovalsNavBadge'
import { QuoteApprovalsCard } from './components/QuoteApprovalsCard'
import {APPROVALS_MANAGE_PERMISSION, APPROVALS_PATH, APPROVALS_TAB_KEY, DEAL_RECORD_TYPE} from '@/sdk'
import { approvalsRoutes, approvalsSettingsRoute } from './routes'

export { ApprovalRequestsPanel } from './components/ApprovalRequestsPanel'
export type { ApprovalRequestsPanelProps } from './components/ApprovalRequestsPanel'
export { ApprovalsNavBadge } from './components/ApprovalsNavBadge'

const approvalsModule = defineModule({
  key: 'approvals',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.settingsGroup('organization', { key: 'sales', label: 'groups.sales', order: 20 })
    app.routes(approvalsRoutes)
    app.nav({
      to: APPROVALS_PATH,
      label: 'nav.approvals',
      icon: ShieldCheck,
      badge: ApprovalsNavBadge,
      order: 190,
    })
    app.settings('organization', {
      group: 'sales',
      path: 'approvals',
      label: 'settings:tabs.approvals',
      description: 'settings:items.approvals.description',
      icon: ShieldCheck,
      permission: APPROVALS_MANAGE_PERMISSION,
      routes: [approvalsSettingsRoute],
    })
    app.slot('quotes.editor.sidebar', {
      key: 'approvals',
      render: QuoteApprovalsCard,
    })
    app.recordSection(DEAL_RECORD_TYPE, {
      key: APPROVALS_TAB_KEY,
      label: 'panel.tab',
      icon: ShieldCheck,
      order: 90,
      render: ApprovalRequestsPanel,
    })
  },
})

export default approvalsModule
