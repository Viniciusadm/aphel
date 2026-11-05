import { ListOrdered } from 'lucide-react'
import { createElement } from 'react'
import { type RecordTab } from '@/sdk'
import type { RecordTabContribution, RecordTabRenderProps } from '@/kernel/modules/define'
import { EMAILS_TAB_KEY, placeTabAfter } from '@/modules/email'
import { SEQUENCES_MANAGE_PERMISSION, SEQUENCES_READ_PERMISSION, SEQUENCES_TAB_KEY } from './constants'
import { SequencesTab } from './record/SequencesTab'

const TABS_BEFORE_SEQUENCES = [EMAILS_TAB_KEY, 'notes', 'tasks', 'timeline']

function SequencesRecordTab({ recordId, readOnly }: RecordTabRenderProps) {
  return createElement(SequencesTab, { contactId: recordId, readOnly })
}

export const SEQUENCES_RECORD_TAB: RecordTabContribution = {
  key: SEQUENCES_TAB_KEY,
  group: 'communication',
  label: 'record.tab',
  icon: ListOrdered,
  permission: [SEQUENCES_READ_PERMISSION, SEQUENCES_MANAGE_PERMISSION],
  render: SequencesRecordTab,
}

export function withSequencesTabPlacement(tabs: RecordTab[]): RecordTab[] {
  return placeTabAfter(tabs, SEQUENCES_TAB_KEY, TABS_BEFORE_SEQUENCES)
}
