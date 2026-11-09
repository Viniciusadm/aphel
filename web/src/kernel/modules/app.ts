import type { RouteObject } from 'react-router'
import type { WidgetRenderer } from '@/kernel/widgets/types'
import type { TimelineRenderer } from '@/features/records/timeline/registry'
import {
  NAV_ORDER,
  type KeybindingContribution,
  type ModuleApp,
  type RecordSectionContribution,
  type SettingsAreaContribution,
} from './define'
import type { RecordTypeSelector } from './record-types'
import type { FieldTypeRenderers } from './field-types'
import { registerRuleError } from '@/components/feedback/rule-errors'
import { contractBucket } from './contract-components'
import { contribute } from './registry'
import type { SettingsEntry, SettingsGroupEntry } from './settings'

export const DEFAULT_FILTER_PRIORITY = 10

export interface RouteEntry {
  routes: RouteObject[]
  layout: 'app' | 'bare'
}

export interface RecordTabEntry {
  recordTypes: RecordTypeSelector
  tab: RecordSectionContribution
}

export function createModuleApp(key: string): ModuleApp {
  return {
    key,
    routes: (routes, options) => contribute<RouteEntry>('routes', key, { routes, layout: options?.layout ?? 'app' }),
    publicRoutes: (routes) => contribute<RouteObject[]>('publicRoutes', key, routes),
    nav: (item) => contribute('nav', key, item, item.order ?? NAV_ORDER.modules),
    ruleError: (code, formatter) => registerRuleError(code, formatter),
    traitListView: (view) => contribute('traitListViews', key, view),
    navSection: (section) => contribute('navSections', key, section, section.order),
    settings: (area, item) => contribute<SettingsEntry>('settings', key, { area, item }, item.order),
    settingsArea: (area) => contribute<SettingsAreaContribution>('settingsAreas', key, area, area.order),
    settingsGroup: (area, group) =>
      contribute<SettingsGroupEntry>('settingsGroups', key, { area, group }, group.order),
    recordSection: (recordType, tab) =>
      contribute<RecordTabEntry>('recordTabs', key, { recordTypes: recordType, tab }, tab.order),
    recordTab: (recordType, tab) =>
      contribute<RecordTabEntry>('recordTabs', key, { recordTypes: recordType, tab }, tab.order),
    recordList: (extension) => contribute('recordList', key, extension),
    slot: (name, contribution) => contribute(`slot:${name}`, key, contribution, contribution.order),
    filter: (name, fn, priority = DEFAULT_FILTER_PRIORITY) => contribute(`filter:${name}`, key, { fn }, priority),
    provideComponent: (contractKey, component) => contribute(contractBucket(contractKey), key, component),
    override: (id, component) => contribute(`override:${id}`, key, component),
    fieldType: (type, renderers) => contribute<FieldTypeRenderers>(`fieldType:${type}`, key, renderers),
    timelineRenderer: (kind, renderer) => contribute<TimelineRenderer>(`timeline:${kind}`, key, renderer),
    eventRenderer: (eventType, renderer) => contribute<TimelineRenderer>(`event:${eventType}`, key, renderer),
    widget: (widgetKey, renderer) => contribute<WidgetRenderer>(`widget:${widgetKey}`, key, renderer),
    command: (command) => {
      contribute('commands', key, command)
      if (command.keys) {
        contribute<KeybindingContribution>('keybindings', key, {
          id: command.id,
          label: command.label,
          keys: command.keys,
          permission: command.permission,
          to: command.to,
          run: command.run,
        })
      }
    },
    keybinding: (keybinding) => contribute('keybindings', key, keybinding),
    keybindingScope: (scope) => contribute('keybindingScopes', key, scope),
    createAction: (action) => contribute('createActions', key, action, action.order),
    quickAdd: (action) => contribute('quickAdd', key, action, action.order),
    referenceOptions: (kind, provider) => contribute(`reference:${kind}`, key, provider),
    timelineSource: (factory) => contribute('timelineSource', key, factory),
    configPackageLabels: (labels) => contribute('configPackageLabels', key, labels),
    setupStep: (step) => contribute('setupSteps', key, step, step.order),
    setupTask: (task) => contribute('setupTasks', key, task, task.order),
    on: (name, handler) => contribute(`on:${name}`, key, handler),
  }
}
