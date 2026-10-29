import { useEffect } from 'react'
import { useNavigate, useParams } from 'react-router'
import { getRecordResource } from '@/features/records/resources'
import type { RecordSlotProps } from '@/kernel/modules/slots'

export function MergedRecordRedirect({ recordType, record }: RecordSlotProps) {
  const navigate = useNavigate()
  const { id: requestedId } = useParams()
  const target =
    requestedId && record.id !== requestedId ? getRecordResource(recordType)?.detailPath?.(record.id) : undefined

  useEffect(() => {
    if (target) navigate(target, { replace: true })
  }, [navigate, target])

  return null
}
