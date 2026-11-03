import type { RecordTabRenderProps } from '@/kernel/modules/define'
import { lazyComponent } from '@/lib/lazy-component'
const LineItemsTab = lazyComponent(async () => (await import('./LineItemsTab')).LineItemsTab)
export function LineItemsRecordTab({ recordType, recordId, readOnly }: RecordTabRenderProps) {
  return <LineItemsTab recordType={recordType} recordId={recordId} readOnly={readOnly} />
}
