import { Link2 } from 'lucide-react'
import type { ModuleApp, RecordTabVisibilityProps } from '@/kernel/modules/define'
import { useRecordRelations } from './api'
import { RelationsTab } from './RelationsTab'
import { isCustomRecordType } from './resources'

function useHasRelations({ recordType, recordId, enabled }: RecordTabVisibilityProps): boolean {
  const relations = useRecordRelations(recordType, recordId, { enabled })
  return (relations.data?.length ?? 0) > 0
}

export function registerCustomRecordContributions(app: ModuleApp) {
  app.recordTab((recordType) => !isCustomRecordType(recordType), {
    key: 'relations',
    label: 'objects:relations.tab',
    icon: Link2,
    order: 0,
    useVisible: useHasRelations,
    render: RelationsTab,
  })
}
