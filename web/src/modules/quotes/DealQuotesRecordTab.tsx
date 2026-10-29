import type { RecordTabRenderProps } from '@/kernel/modules/define'
import { DealQuotesTab } from './DealQuotesTab'

export function DealQuotesRecordTab({ recordId }: RecordTabRenderProps) {
  return <DealQuotesTab dealId={recordId} />
}
