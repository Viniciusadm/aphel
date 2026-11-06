import { createElement } from 'react'
import { type RecordTab } from '@/sdk'
import type { FilterContext } from '@/kernel/modules/filters'
import { ActivitiesTab } from './ActivitiesTab'

export const TASKS_TAB_KEY = 'tasks'

export function withActivitiesTab(tabs: RecordTab[], { recordType, record, readOnly }: FilterContext<'records.detailTabs'>) {
  return tabs.map((tab) =>
    tab.key === TASKS_TAB_KEY
      ? { ...tab, source: { module: 'tasks', key: TASKS_TAB_KEY }, permission: 'tasks.read', summary: createElement(ActivitiesTab, { recordType, recordId: record.id, readOnly, summary: true }), content: createElement(ActivitiesTab, { recordType, recordId: record.id, readOnly }) }
      : tab,
  )
}
