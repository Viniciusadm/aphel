import type { RecordTab } from '../types'

export const TAB_GROUP_ORDER = ['activity', 'relationships', 'communication', 'insights', 'more'] as const

export type TabGroupKey = (typeof TAB_GROUP_ORDER)[number]

const GROUP_PATTERNS: Record<Exclude<TabGroupKey, 'more'>, RegExp> = {
  activity: /^(timeline|tasks|notes|files|activities|calls)/,
  relationships: /^(related|deals|group|people|organizations|leads|quotes|products)/,
  communication: /^(email|emails|messaging|sequences|sequence)/,
  insights: /^(scoring|score|lifecycle|history|ai|lead_pro|temperature|forecast)/,
}

export function tabGroupOf(key: string): TabGroupKey {
  const match = (Object.entries(GROUP_PATTERNS) as [Exclude<TabGroupKey, 'more'>, RegExp][]).find(([, pattern]) =>
    pattern.test(key),
  )
  return match ? match[0] : 'more'
}

export function groupTabs(tabs: RecordTab[]): { group: TabGroupKey; tabs: RecordTab[] }[] {
  return TAB_GROUP_ORDER.map((group) => ({ group, tabs: tabs.filter((tab) => (tab.group ?? tabGroupOf(tab.key)) === group) })).filter(
    (entry) => entry.tabs.length > 0,
  )
}
