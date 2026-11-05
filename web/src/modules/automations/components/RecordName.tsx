import { useQuery } from '@tanstack/react-query'
import { ExternalLink } from 'lucide-react'
import { Link } from 'react-router'
import { api } from '@/api'
import { getRecordResource, type RecordItem, recordPath, recordsKeys, recordTitle } from '@/sdk'
export function RecordName({ recordType, recordId, link = true }: { recordType: string; recordId: string; link?: boolean }) {
  const resource = getRecordResource(recordType)
  const record = useQuery({
    queryKey: recordsKeys.detail(recordType, recordId),
    queryFn: () => api.get<RecordItem>(recordPath(resource!, recordId)),
    enabled: Boolean(resource),
    staleTime: 60_000,
    retry: false,
  })
  const label = record.data && resource ? recordTitle(resource, record.data) : `${recordId.slice(0, 8)}…`
  const href = resource?.detailPath?.(recordId)
  if (!link || !href) return <span className="truncate">{label}</span>
  return (
    <Link to={href} className="inline-flex min-w-0 items-center gap-1 underline-offset-4 hover:underline">
      <span className="truncate">{label}</span>
      <ExternalLink className="size-3 shrink-0" aria-hidden="true" />
    </Link>
  )
}
