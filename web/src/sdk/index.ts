export { NAV_ORDER, defineModule } from '@/kernel/modules/define'
export type {
  CommandContribution,
  KeybindingContribution,
  KeybindingScopeContribution,
  KeybindingScopeKind,
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
  TraitListViewContribution,
  TraitListViewProps,
  WebModule,
} from '@/kernel/modules/define'
export { useModuleTranslation, type ModuleTranslate } from '@/kernel/modules/i18n'
export { Slot } from '@/kernel/modules/Slot'
export {
  KeybindingHint,
  KeybindingKeys,
  ariaKeyShortcut,
  chordText,
  formatChord,
  isEditableTarget,
  useActionKeys,
  useKeybinding,
  useKeybindingHandlers,
  useKeybindingProps,
  useKeybindingScope,
  type KeybindingHandler,
  type KeybindingHintProps,
  type KeybindingKeysProps,
  type UseKeybindingOptions,
} from '@/kernel/keybindings'
export {
  useContractComponent,
  useHasContractComponent,
  type ContractComponentKey,
  type ContractComponentMap,
  type ContractComponentProps,
} from '@/kernel/modules/contract-components'
export { ContractComponent, type ContractComponentRenderProps } from '@/kernel/modules/ContractComponent'
export { useChangeFeed, type ChangeFeedOptions, type PluginChange } from '@/kernel/modules/change-feed'
export type { RecordSlotProps, SlotName, SlotProps, SlotPropsMap } from '@/kernel/modules/slots'
export { api, apiRequest, ApiError, isApiError } from '@/api'
export { appBase, appPath, appUrl, onSiteSurface, publicUrl } from './app-base'
export { useSession } from '@/app/session/session-context'
export { ConfirmDialog, ErrorBoundary, PageLoader, errorMessage, notifyError, notifySuccess } from '@/components/feedback'
export { PhoneFieldInput } from '@/components/fields/LazyPhoneFieldInput'
export type { PhoneFieldInputProps } from '@/components/fields/PhoneFieldInput'
export * from './mobile'
export * from './period'
export * from './platform'
export * from './records'
export * from './rotting'
export * from './settings'
export * from './timeline'
export * from './entities'
export * from './formulas'
export * from './approvals'
export * from './money'
export * from './email'
export * from './saved_views'
export * from './tasks'
export * from './config_packages'
export * from './people'
export * from './proposals'
export * from './esign'
export * from './pricing'
export * from './segments'
export * from './imports'
export * from './sso'
export * from './dashboard'
export * from './marketplace'
export { KanbanBoard } from '@/features/custom-records/KanbanBoard'
export * from './ui'
export * from './workflow'
export * from './blocks'
export * from './media'

export { APP_FRAME_CONTRACT, DEFAULT_APP_FRAME, type AppFrameProps } from '@/kernel/modules/app-frame'
export { sendLocalPrintJob, PrintDeliveryError } from './printing'
export type { PrintBridgeConfiguration, LocalPrintJob, PrintAcknowledgement } from './printing'
