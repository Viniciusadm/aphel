import type { RouteObject } from 'react-router'
import type { WidgetRenderer } from '@/kernel/widgets/types'
import type { TimelineRenderer } from '@/features/records/timeline/registry'
import { NAV_ORDER, type ModuleApp, type RecordTabContribution } from './define'
import type { RecordTypeSelector } from './record-types'
import type { FieldTypeRenderers } from './field-types'
import { contribute } from './registry'
import type { SettingsEntry } from './settings'

export const DEFAULT_FILTER_PRIORITY = 10

export interface RouteEntry {
  routes: RouteObject[]
  layout: 'app' | 'bare'
}

export interface RecordTabEntry {
  recordTypes: RecordTypeSelector
  tab: RecordTabContribution
}

export function createModuleApp(key: string): ModuleApp {
  return {
    key,
    routes: (routes, options) => contribute<RouteEntry>('routes', key, { routes, layout: options?.layout ?? 'app' }),
    publicRoutes: (routes) => contribute<RouteObject[]>('publicRoutes', key, routes),
    nav: (item) => contribute('nav', key, item, item.order ?? NAV_ORDER.modules),
    settings: (area, item) => contribute<SettingsEntry>('settings', key, { area, item }, item.order),
    recordTab: (recordType, tab) =>
      contribute<RecordTabEntry>('recordTabs', key, { recordTypes: recordType, tab }, tab.order),
    recordList: (extension) => contribute('recordList', key, extension),
    slot: (name, contribution) => contribute(`slot:${name}`, key, contribution, contribution.order),
    filter: (name, fn, priority = DEFAULT_FILTER_PRIORITY) => contribute(`filter:${name}`, key, { fn }, priority),
    override: (id, component) => contribute(`override:${id}`, key, component),
    fieldType: (type, renderers) => contribute<FieldTypeRenderers>(`fieldType:${type}`, key, renderers),
    timelineRenderer: (kind, renderer) => contribute<TimelineRenderer>(`timeline:${kind}`, key, renderer),
    eventRenderer: (eventType, renderer) => contribute<TimelineRenderer>(`event:${eventType}`, key, renderer),
    widget: (widgetKey, renderer) => contribute<WidgetRenderer>(`widget:${widgetKey}`, key, renderer),
    command: (command) => contribute('commands', key, command),
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
