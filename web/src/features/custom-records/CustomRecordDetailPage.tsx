import { useCallback, useMemo } from 'react'
import { useParams } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { RecordDetailPage, type RecordTab } from '@/features/records/detail'
import { useModuleTimelineSource } from '@/kernel/modules/timeline-source'
import { CustomObjectGate } from './CustomObjectGate'
import { CustomRecordActions } from './CustomRecordActions'
import { useRelationTabs } from './relation-tabs'
import { CUSTOM_RECORD_READ_ONLY_KEYS, customObjectPath, objectPermission, type CustomObject } from './resources'

const CUSTOM_RECORD_TABS = ['timeline', 'notes', 'files']

export function CustomRecordDetailPage() {
  return <CustomObjectGate>{(object) => <CustomRecordDetail key={object.key} object={object} />}</CustomObjectGate>
}

function CustomRecordDetail({ object }: { object: CustomObject }) {
  const { id = '' } = useParams()
  const { hasPermission } = useSession()
  const recordType = object.record_type
  const readOnly = !hasPermission(objectPermission(object, 'update'))
  const timelineSource = useModuleTimelineSource()
  const timeline = useMemo(() => timelineSource?.(recordType, id), [timelineSource, recordType, id])
  const relationTabs = useRelationTabs(recordType, id, readOnly)

  const tabs = useCallback(
    (defaults: RecordTab[]): RecordTab[] => {
      const base = defaults.filter((tab) => CUSTOM_RECORD_TABS.includes(tab.key))
      const [timelineTab, ...rest] = base
      return timelineTab ? [timelineTab, ...relationTabs, ...rest] : [...relationTabs, ...rest]
    },
    [relationTabs],
  )

  return (
    <RecordDetailPage
      key={id}
      recordType={recordType}
      recordId={id}
      back={{ to: customObjectPath(object.key), label: object.label_plural }}
      readOnly={readOnly}
      readOnlyFieldKeys={CUSTOM_RECORD_READ_ONLY_KEYS}
      ownerField="owner_id"
      timeline={timeline}
      tabs={tabs}
      actions={(record) => <CustomRecordActions object={object} record={record} />}
    />
  )
}
