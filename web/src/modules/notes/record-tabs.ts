import { createElement } from 'react'
import type { RecordTab } from '@/features/records/detail'
import type { FilterContext } from '@/kernel/modules/filters'
import { NotesTab } from './NotesTab'

export const NOTES_TAB_KEY = 'notes'

export function withNotesTab(tabs: RecordTab[], { recordType, record, readOnly }: FilterContext<'records.detailTabs'>) {
  return tabs.map((tab) =>
    tab.key === NOTES_TAB_KEY
      ? { ...tab, content: createElement(NotesTab, { recordType, recordId: record.id, readOnly }) }
      : tab,
  )
}
