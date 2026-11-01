import { createElement } from 'react'
import type { RecordTab } from '@/features/records/detail'
import type { FilterContext } from '@/kernel/modules/filters'
import { ActivitiesTab } from './ActivitiesTab'

export const ACTIVITIES_TAB_KEY = 'tasks'

export function withActivitiesTab(tabs: RecordTab[], { recordType, record, readOnly }: FilterContext<'records.detailTabs'>) {
  return tabs.map((tab) =>
    tab.key === ACTIVITIES_TAB_KEY
      ? { ...tab, summary: createElement(ActivitiesTab, { recordType, recordId: record.id, readOnly, summary: true }), content: createElement(ActivitiesTab, { recordType, recordId: record.id, readOnly }) }
      : tab,
  )
}
