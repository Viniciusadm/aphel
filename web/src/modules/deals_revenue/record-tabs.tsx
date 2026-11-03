import type { RecordTabRenderProps } from '@/kernel/modules/define'
import { DealRevenuePanel } from './DealRevenuePanel'
export function RevenueTab({ recordId }: RecordTabRenderProps) { return <DealRevenuePanel dealId={recordId} /> }
