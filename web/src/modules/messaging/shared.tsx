import { Mail, MessageCircle } from 'lucide-react'
import { Link } from 'react-router'
import { Badge } from '@/components/ui/badge'
import { getRecordResource } from '@/features/records/resources'
import { cn } from '@/lib/utils'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { MESSAGING_KEY, type ChannelKind, type ConversationStatus, type LinkedRecord } from './api'

export function ChannelIcon({ kind, className }: { kind: ChannelKind; className?: string }) {
  const Icon = kind === 'whatsapp' ? MessageCircle : Mail
  return <Icon className={cn('size-4 shrink-0', className)} aria-hidden="true" />
}

export function StatusBadge({ status }: { status: ConversationStatus }) {
  const t = useModuleTranslation(MESSAGING_KEY)
  const variant = status === 'open' ? 'default' : status === 'pending' ? 'warning' : 'secondary'
  return <Badge variant={variant}>{t(`status.${status}`)}</Badge>
}

export function LinkedRecordLabel({ recordType, record }: { recordType: 'contact' | 'deal'; record: LinkedRecord }) {
  const t = useModuleTranslation(MESSAGING_KEY)
  if (!record.accessible) {
    return <span className="text-muted-foreground">{t(`links.${recordType}Hidden`)}</span>
  }
  const href = getRecordResource(recordType)?.detailPath?.(record.id)
  const label = record.title || t(`links.${recordType}Untitled`)
  return href ? (
    <Link to={href} className="truncate text-primary underline-offset-4 hover:underline">
      {label}
    </Link>
  ) : (
    <span className="truncate">{label}</span>
  )
}
