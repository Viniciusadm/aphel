import type { LucideIcon } from 'lucide-react'
import type { ComponentType, ReactNode } from 'react'
import type { RouteObject } from 'react-router'
import type { WidgetRenderer } from '@/kernel/widgets/types'
import type { TimelineRenderer } from '@/features/records/timeline/registry'
import type { TimelineSource } from '@/features/records/timeline/types'
import type { FieldDefinition, FieldSchema, RecordItem } from '@/features/records/types'
import type { ModuleEventHandler, ModuleEventName } from './events'
import type { FieldTypeRenderers } from './field-types'
import type { FilterContext, FilterName, FilterValue } from './filters'
import type { RecordTypeSelector } from './record-types'
import type { SlotName, SlotProps } from './slots'

export type ModuleLocale = 'pt-BR' | 'en'

export type TranslationResources = Record<string, unknown>

export type TranslationLoader = () => Promise<TranslationResources | { default: TranslationResources }>

export type ModuleTranslations = Partial<Record<ModuleLocale, TranslationResources | TranslationLoader>>

export type PermissionRequirement = string | string[]

export const NAV_ORDER = {
  customObjects: 500,
  modules: 600,
  administration: 900,
} as const

export interface NavContribution {
  to: string
  label: string
  icon: LucideIcon
  key?: string
  end?: boolean
  permission?: PermissionRequirement
  badge?: ComponentType<{ collapsed?: boolean }>
  order?: number
}

export type SettingsAreaKey = 'account' | 'organization' | 'integrations' | 'data'

export interface SettingsContribution {
  group: string
  path: string
  label: string
  description: string
  icon: LucideIcon
  permission?: string
  order?: number
  lazy?: RouteObject['lazy']
  element?: ReactNode
  routes?: RouteObject[]
}

export interface RecordTabRenderProps {
  recordType: string
  recordId: string
  readOnly: boolean
}

export interface RecordTabVisibilityProps extends RecordTabRenderProps {
  enabled: boolean
}

export interface RecordTabContribution {
  key: string
  label: string
  icon?: LucideIcon
  permission?: PermissionRequirement
  order?: number
  useVisible?: (props: RecordTabVisibilityProps) => boolean
  render: ComponentType<RecordTabRenderProps>
}

export interface SlotContribution<P> {
  key?: string
  recordTypes?: RecordTypeSelector
  permission?: PermissionRequirement
  order?: number
  render: ComponentType<P>
}

export interface CommandHelpers {
  navigate: (to: string) => void
}

export interface CommandContribution {
  id: string
  label: string
  icon?: LucideIcon
  keywords?: string[]
  permission?: PermissionRequirement
  to?: string
  run?: (helpers: CommandHelpers) => void
}

export interface RecordListExtensionProps {
  recordType: string
  schema: FieldSchema
  records: RecordItem[]
  enabled: boolean
}

export interface RecordListExtensionResult {
  filterFields?: FieldDefinition[]
  titleAddon?: (record: RecordItem) => ReactNode
}

export interface RecordListExtension {
  key: string
  recordTypes?: RecordTypeSelector
  permission?: PermissionRequirement
  use: (props: RecordListExtensionProps) => RecordListExtensionResult
}

export interface CreateTarget {
  recordType: string
  recordId: string
}

export interface CreateActionRenderProps {
  open: boolean
  target?: CreateTarget
  onClose: () => void
}

export interface CreateActionContribution {
  id: string
  label: string
  icon: LucideIcon
  permission?: PermissionRequirement
  recordType?: string
  targetRecordTypes?: RecordTypeSelector
  linkedLabel?: string
  order?: number
  render: ComponentType<CreateActionRenderProps>
}

export interface QuickAddRecord {
  recordType: string
  recordId: string
  title: string
}

export interface QuickAddRenderProps {
  initialRecord: QuickAddRecord | null
  onDone: () => void
}

export interface QuickAddContribution {
  key: string
  label: string
  title?: string
  icon: LucideIcon
  permission?: PermissionRequirement
  order?: number
  linksRecord?: boolean
  recordShortcut?: string
  render: ComponentType<QuickAddRenderProps>
}

export interface ReferenceOption {
  value: string
  label: string
  group?: string
}

export interface ReferenceOptionsContext {
  recordType?: string
  includeInactive?: boolean
  enabled?: boolean
}

export interface ReferenceOptionsResult {
  options: ReferenceOption[]
  isLoading?: boolean
}

export interface ReferenceOptionsProvider {
  use: (context: ReferenceOptionsContext) => ReferenceOptionsResult
  queryKey?: readonly unknown[]
}

export interface ConfigPackageLabels {
  sections?: Record<string, string>
  kinds?: Record<string, string>
  properties?: Record<string, string>
}

export interface SetupStepProps {
  onDone: () => void
}

export interface SetupStepContribution {
  key: string
  label: string
  order?: number
  usePending?: () => boolean | undefined
  render: ComponentType<SetupStepProps>
}

export interface SetupTaskResult {
  summary: string
}

export interface SetupTaskContribution {
  key: string
  progressLabel: string
  failureLabel: string
  order?: number
  run: () => Promise<SetupTaskResult | null>
}

export type FilterFunction<N extends FilterName> = (value: FilterValue<N>, context: FilterContext<N>) => FilterValue<N>

export interface RouteOptions {
  layout?: 'app' | 'bare'
}

export interface ModuleApp {
  readonly key: string
  routes(routes: RouteObject[], options?: RouteOptions): void
  publicRoutes(routes: RouteObject[]): void
  nav(item: NavContribution): void
  settings(area: SettingsAreaKey, item: SettingsContribution): void
  recordTab(recordType: RecordTypeSelector, tab: RecordTabContribution): void
  recordList(extension: RecordListExtension): void
  slot<N extends SlotName>(name: N, contribution: SlotContribution<SlotProps<N>>): void
  filter<N extends FilterName>(name: N, fn: FilterFunction<N>, priority?: number): void
  override<P extends object>(id: string, component: ComponentType<P>): void
  fieldType(type: string, renderers: FieldTypeRenderers): void
  timelineRenderer(kind: string, renderer: TimelineRenderer): void
  eventRenderer(eventType: string, renderer: TimelineRenderer): void
  widget(key: string, renderer: WidgetRenderer): void
  command(command: CommandContribution): void
  createAction(action: CreateActionContribution): void
  quickAdd(action: QuickAddContribution): void
  referenceOptions(kind: string, provider: ReferenceOptionsProvider): void
  timelineSource(factory: (recordType: string, recordId: string) => TimelineSource): void
  configPackageLabels(labels: ConfigPackageLabels): void
  setupStep(step: SetupStepContribution): void
  setupTask(task: SetupTaskContribution): void
  on<N extends ModuleEventName>(name: N, handler: ModuleEventHandler<N>): void
}

export interface ModuleSettingsPanelProps {
  moduleKey: string
}

export interface WebModule {
  key: string
  required?: boolean
  translations?: ModuleTranslations
  namespaces?: Record<string, ModuleTranslations>
  settingsPanel?: ComponentType<ModuleSettingsPanelProps>
  register?: (app: ModuleApp) => void
}

export function defineModule(module: WebModule): WebModule {
  return module
}
