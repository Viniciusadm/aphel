import type { ParseKeys } from 'i18next'

export interface ReportsTab {
  path: 'conversion' | 'performance' | 'saved'
  labelKey: ParseKeys<'reports'>
  anyPermission: string[]
}

export const RECORD_READ_PERMISSIONS = ['people.read', 'organizations.read', 'deals.read', 'activities.read']

export const REPORTS_TABS: ReportsTab[] = [
  { path: 'conversion', labelKey: 'tabs.conversion', anyPermission: ['deals.read'] },
  { path: 'performance', labelKey: 'tabs.performance', anyPermission: ['deals.read'] },
  { path: 'saved', labelKey: 'tabs.saved', anyPermission: RECORD_READ_PERMISSIONS },
]

export function allowedReportTabs(hasPermission: (permission: string) => boolean): ReportsTab[] {
  return REPORTS_TABS.filter((tab) => tab.anyPermission.some(hasPermission))
}
