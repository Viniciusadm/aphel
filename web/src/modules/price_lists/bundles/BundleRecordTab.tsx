import type { RecordTabRenderProps } from '@/kernel/modules/define'
import { lazyComponent } from '@/lib/lazy-component'

const BundlePanel = lazyComponent(async () => (await import('./BundlePanel')).BundlePanel)

export function BundleRecordTab({ recordId, readOnly }: RecordTabRenderProps) {
  return <BundlePanel productId={recordId} readOnly={readOnly} />
}
