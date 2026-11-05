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
  SettingsAreaContribution,
  SettingsContribution,
  SettingsGroupContribution,
  SlotContribution,
  WebModule,
} from '@/kernel/modules/define'
export { useModuleTranslation, type ModuleTranslate } from '@/kernel/modules/i18n'
export { Slot } from '@/kernel/modules/Slot'
export { useChangeFeed, type ChangeFeedOptions, type PluginChange } from '@/kernel/modules/change-feed'
export type { RecordSlotProps, SlotName, SlotProps, SlotPropsMap } from '@/kernel/modules/slots'
export { api, apiRequest, ApiError, isApiError } from '@/api'
export { useSession } from '@/app/session/session-context'
export { ConfirmDialog, ErrorBoundary, PageLoader, errorMessage, notifyError, notifySuccess } from '@/components/feedback'
export * from './mobile'
export * from './platform'
export * from './records'
export * from './settings'
export * from './ui'
