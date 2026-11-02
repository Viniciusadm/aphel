import type { NotificationChannel } from '@/modules/notifications/ui/api'

export const NOTIFICATION_KINDS: readonly string[] = [
  'task_assigned',
  'task_due',
  'deal_assigned',
  'deal_stage_changed',
  'note_mention',
  'daily_digest',
  'export_ready',
  'export_failed',
  'import_finished',
  'import_failed',
  'webhook_endpoint_disabled',
  'reminder',
  'automation_message',
  'automation_email',
  'automation_skipped',
  'approval_requested',
  'approval_decided',
  'approval_overdue',
  'followed_record_activity',
]

export type NotificationKind = string

export function orderedNotificationKinds(kinds: Iterable<string>): NotificationKind[] {
  const unique = [...new Set(kinds)]
  const rank = (kind: string) => {
    const position = NOTIFICATION_KINDS.indexOf(kind)
    return position === -1 ? NOTIFICATION_KINDS.length : position
  }
  return unique
    .map((kind, index) => ({ kind, index }))
    .sort((left, right) => rank(left.kind) - rank(right.kind) || left.index - right.index)
    .map(({ kind }) => kind)
}

export const NOTIFICATION_CHANNELS: NotificationChannel[] = ['in_app', 'email', 'push']
