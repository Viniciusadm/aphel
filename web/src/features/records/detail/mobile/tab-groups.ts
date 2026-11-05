import type { RecordTab } from '../types'

export const TAB_GROUP_ORDER = ['activity', 'relationships', 'communication', 'insights', 'more'] as const

export type TabGroupKey = (typeof TAB_GROUP_ORDER)[number]

export function tabGroupOf(tab: Pick<RecordTab, 'group'>): TabGroupKey {
  return TAB_GROUP_ORDER.find((group) => group === tab.group) ?? 'more'
}

export function groupTabs(tabs: RecordTab[]): { group: TabGroupKey; tabs: RecordTab[] }[] {
  return TAB_GROUP_ORDER.map((group) => ({ group, tabs: tabs.filter((tab) => tabGroupOf(tab) === group) })).filter(
    (entry) => entry.tabs.length > 0,
  )
}
