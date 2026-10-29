import { Briefcase, Building2, CheckSquare, FileText, Package, Users, type LucideIcon } from 'lucide-react'
import { getRecordResource } from './resources'

const LIST_PATHS: Record<string, string> = {
  contact: '/contacts',
  company: '/companies',
  deal: '/deals',
  activity: '/tasks',
  product: '/products',
}

const RECORD_ICONS: Record<string, LucideIcon> = {
  contact: Users,
  company: Building2,
  deal: Briefcase,
  activity: CheckSquare,
  product: Package,
}

const ID_PLACEHOLDER = '__aphel_id__'

export function recordListPath(recordType: string): string | undefined {
  const detail = getRecordResource(recordType)?.detailPath?.(ID_PLACEHOLDER)
  if (detail?.endsWith(`/${ID_PLACEHOLDER}`)) return detail.slice(0, -ID_PLACEHOLDER.length - 1)
  return LIST_PATHS[recordType]
}

export function recordDetailHref(recordType: string, recordId: string): string | undefined {
  const resource = getRecordResource(recordType)
  if (resource?.detailPath) return resource.detailPath(recordId)
  const list = LIST_PATHS[recordType]
  return list ? `${list}/${encodeURIComponent(recordId)}` : undefined
}

export function viewHref(recordType: string, viewId: string): string | undefined {
  const list = recordListPath(recordType)
  return list ? `${list}?${new URLSearchParams({ view: viewId }).toString()}` : undefined
}

export function recordTypeIcon(recordType: string): LucideIcon {
  return RECORD_ICONS[recordType] ?? FileText
}
