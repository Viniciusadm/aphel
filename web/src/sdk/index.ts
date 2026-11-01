export { NAV_ORDER, defineModule } from '@/kernel/modules/define'
export type {
  CommandContribution,
  ModuleApp,
  ModuleTranslations,
  NavContribution,
  NavGroup,
  RecordSectionContribution,
  RecordTabContribution,
  RecordTabRenderProps,
  SettingsContribution,
  SlotContribution,
  WebModule,
} from '@/kernel/modules/define'
export { useModuleTranslation, type ModuleTranslate } from '@/kernel/modules/i18n'
export { Slot } from '@/kernel/modules/Slot'
export { useChangeFeed, type ChangeFeedOptions, type PluginChange } from '@/kernel/modules/change-feed'
export type { RecordSlotProps, SlotName, SlotProps } from '@/kernel/modules/slots'
export { api, apiRequest, ApiError, isApiError } from '@/api'
export { useSession } from '@/app/session/session-context'
export { RecordDetailPage, useRecord, type RecordTab } from '@/features/records/detail'
export { RecordFormDialog, RecordForm } from '@/features/records/form'
export { RecordListView } from '@/features/records/table'
export { registerRecordResource } from '@/features/records/resources'
export { recordsKeys } from '@/features/records/api'
export {
  formatCurrency,
  formatDate,
  formatDateTime,
  formatNumber,
  useRecordFormatting,
} from '@/features/records/formatting'
export type { FilterNode, RecordItem } from '@/features/records/types'
export { customObjectPath, customRecordPath, customRecordType } from '@/features/custom-records/resources'
export { ConfirmDialog, ErrorBoundary, PageLoader, errorMessage, notifyError, notifySuccess } from '@/components/feedback'
export * from './ui'
