import { Link2 } from 'lucide-react'
import { useMemo } from 'react'
import type { RecordTab } from '@/features/records/detail'
import { useRecordRelations } from './api'
import { RelationPanel } from './RelationsTab'

export function useRelationTabs(recordType: string, recordId: string, readOnly = false): RecordTab[] {
  const relations = useRecordRelations(recordType, recordId)
  return useMemo(
    () =>
      (relations.data ?? []).map((relation) => ({
        key: `relation-${relation.key}`,
        label: relation.total != null ? `${relation.label} (${relation.total})` : relation.label,
        icon: Link2,
        content: (
          <RelationPanel recordType={recordType} recordId={recordId} relation={relation} readOnly={readOnly} />
        ),
      })),
    [relations.data, recordType, recordId, readOnly],
  )
}
