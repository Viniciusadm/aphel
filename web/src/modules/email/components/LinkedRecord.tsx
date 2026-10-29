import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router'
import { api } from '@/api'
import { recordsKeys } from '@/features/records/api'
import { getRecordResource, recordPath, recordTitle } from '@/features/records/resources'
import type { RecordItem } from '@/features/records/types'
import type { EmailRecordRef } from '../types'

export function LinkedRecord({ record_type, record_id }: EmailRecordRef) {
  const resource = getRecordResource(record_type)
  const detail = useQuery({
    queryKey: recordsKeys.detail(record_type, record_id),
    queryFn: () => api.get<RecordItem>(recordPath(resource!, record_id)),
    enabled: Boolean(resource),
    staleTime: 60_000,
    retry: false,
  })
  const title = detail.data && resource ? recordTitle(resource, detail.data) : record_id.slice(0, 8)
  const href = resource?.detailPath?.(record_id)
  if (!href || detail.isError) return <span className="truncate">{title}</span>
  return (
    <Link to={href} className="truncate font-medium underline-offset-2 hover:underline">
      {title}
    </Link>
  )
}
