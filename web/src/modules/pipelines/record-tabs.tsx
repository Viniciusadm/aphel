import type { RecordTabRenderProps } from '@/kernel/modules/define'
import { DealStageGuidancePanel } from './guidance/DealStageGuidancePanel'

export function StageGuidanceRecordTab({ recordId, readOnly }: RecordTabRenderProps) {
  return <DealStageGuidancePanel dealId={recordId} readOnly={readOnly} />
}
