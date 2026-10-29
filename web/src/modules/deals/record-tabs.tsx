import type { RecordTabRenderProps } from '@/kernel/modules/define'
import { lazyComponent } from '@/lib/lazy-component'
import { RecordDealsTab, type RecordDealsTabProps } from './RecordDealsTab'

const DealTeamTab = lazyComponent(async () => (await import('./team/DealTeamTab')).DealTeamTab)

export function RelatedDealsTab({ recordType, recordId }: RecordTabRenderProps) {
  return <RecordDealsTab recordType={recordType as RecordDealsTabProps['recordType']} recordId={recordId} />
}

export function DealTeamRecordTab({ recordId, readOnly }: RecordTabRenderProps) {
  return <DealTeamTab dealId={recordId} readOnly={readOnly} />
}
