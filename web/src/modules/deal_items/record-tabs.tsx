import type { RecordTabRenderProps } from '@/kernel/modules/define'
import { lazyComponent } from '@/lib/lazy-component'
import { DealRevenuePanel } from './recurring/DealRevenuePanel'

const DealItemsTab = lazyComponent(async () => (await import('./DealItemsTab')).DealItemsTab)

export function DealItemsRecordTab({ recordId, readOnly }: RecordTabRenderProps) {
  return <DealItemsTab dealId={recordId} readOnly={readOnly} />
}

export function DealRevenueRecordTab({ recordId }: RecordTabRenderProps) {
  return <DealRevenuePanel dealId={recordId} />
}
