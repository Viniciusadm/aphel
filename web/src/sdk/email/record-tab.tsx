import { Mail } from 'lucide-react'
import { type RecordTab } from '@/sdk'
import type { RecordTabContribution, RecordTabRenderProps } from '@/kernel/modules/define'
import { lazyComponent } from '@/lib/lazy-component'
import { EMAILS_READ_PERMISSION, EMAILS_TAB_KEY } from './constants'

export const EmailsTab = lazyComponent<RecordTabRenderProps>(async () => (await import('./record/EmailsTab')).EmailsTab)

const TABS_BEFORE_EMAILS = ['notes', 'tasks', 'timeline']

export const EMAILS_RECORD_TAB: RecordTabContribution = {
  key: EMAILS_TAB_KEY,
  group: 'communication',
  label: 'tab.label',
  icon: Mail,
  permission: EMAILS_READ_PERMISSION,
  render: EmailsTab,
}

export function placeTabAfter(tabs: RecordTab[], key: string, anchors: string[]): RecordTab[] {
  const tab = tabs.find((item) => item.key === key)
  if (!tab) return tabs
  const rest = tabs.filter((item) => item !== tab)
  const anchor = anchors.map((name) => rest.findIndex((item) => item.key === name)).find((index) => index >= 0)
  if (anchor === undefined) return tabs
  return [...rest.slice(0, anchor + 1), tab, ...rest.slice(anchor + 1)]
}

export function withEmailsTabPlacement(tabs: RecordTab[]): RecordTab[] {
  return placeTabAfter(tabs, EMAILS_TAB_KEY, TABS_BEFORE_EMAILS)
}
