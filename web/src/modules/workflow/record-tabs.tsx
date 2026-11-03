import type { RecordTabRenderProps } from '@/kernel/modules/define'
import { RecordStageGuidancePanel } from './guidance/RecordStageGuidancePanel'

export function StageGuidanceRecordTab({ recordType, recordId, readOnly }: RecordTabRenderProps) {
  return <RecordStageGuidancePanel recordType={recordType} recordId={recordId} readOnly={readOnly} />
}
