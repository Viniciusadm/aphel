import { ShieldCheck } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { ApprovalRequestsPanel } from './components/ApprovalRequestsPanel'
import { ApprovalsNavBadge } from './components/ApprovalsNavBadge'
import { QuoteApprovalsCard } from './components/QuoteApprovalsCard'
import { APPROVALS_MANAGE_PERMISSION, APPROVALS_PATH, APPROVALS_TAB_KEY, DEAL_RECORD_TYPE } from './constants'
import { approvalsRoutes, approvalsSettingsRoute } from './routes'

export { ApprovalRequestsPanel } from './components/ApprovalRequestsPanel'
export type { ApprovalRequestsPanelProps } from './components/ApprovalRequestsPanel'
export { RequestApprovalDialog } from './components/RequestApprovalDialog'
export type { RequestApprovalDialogProps } from './components/RequestApprovalDialog'
export { ApprovalsNavBadge } from './components/ApprovalsNavBadge'
export { notifyApprovalGate, notifyApprovalPending, useApprovalGate } from './components/useApprovalGate'
export type { ApprovalGate } from './components/useApprovalGate'
export { approvalsApi, approvalsKeys, usePendingApprovalsCount, useRecordApprovalRequests } from './api'
export { approvalPendingDetails, approvalRequiredDetails, isApprovalGateError } from './errors'
export {
  APPROVALS_MANAGE_PERMISSION,
  APPROVALS_PATH,
  APPROVALS_SETTINGS_PATH,
  APPROVALS_TAB_KEY,
  APPROVAL_PENDING,
  APPROVAL_REQUIRED,
  approvalPath,
} from './constants'
export type * from './types'

const approvalsModule = defineModule({
  key: 'approvals',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
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
    app.recordTab(DEAL_RECORD_TYPE, {
      key: APPROVALS_TAB_KEY,
      label: 'panel.tab',
      icon: ShieldCheck,
      order: 90,
      render: ApprovalRequestsPanel,
    })
  },
})

export default approvalsModule
