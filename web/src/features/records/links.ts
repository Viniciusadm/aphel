import { FileText, type LucideIcon } from 'lucide-react'
import { getRecordResource } from './resources'

const ID_PLACEHOLDER = '__aphel_id__'

export function recordListPath(recordType: string): string | undefined {
  const detail = getRecordResource(recordType)?.detailPath?.(ID_PLACEHOLDER)
  if (detail?.endsWith(`/${ID_PLACEHOLDER}`)) return detail.slice(0, -ID_PLACEHOLDER.length - 1)
  return undefined
}

export function recordDetailHref(recordType: string, recordId: string): string | undefined {
  const resource = getRecordResource(recordType)
  if (resource?.detailPath) return resource.detailPath(recordId)
  const list = recordListPath(recordType)
  return list ? `${list}/${encodeURIComponent(recordId)}` : undefined
}

export function viewHref(recordType: string, viewId: string): string | undefined {
  const list = recordListPath(recordType)
  return list ? `${list}?${new URLSearchParams({ view: viewId }).toString()}` : undefined
}

export function recordTypeIcon(recordType: string): LucideIcon {
  return getRecordResource(recordType)?.icon ?? FileText
}
