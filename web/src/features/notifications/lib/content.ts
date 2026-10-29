import type { TFunction } from 'i18next'
import type { Notification } from '../api'

type NotificationData = Record<string, unknown>

function data(notification: Notification): NotificationData {
  const value: unknown = notification.data
  return value && typeof value === 'object' ? (value as NotificationData) : {}
}

export function isRestricted(notification: Notification): boolean {
  return notification.kind === 'note_mention' && data(notification).restricted === true
}

export function notificationHeadline(notification: Notification, t: TFunction<'notifications'>): string {
  const actor = notification.actor?.display_name?.trim() || t('someone')
  const kind = notification.kind
  switch (kind) {
    case 'task_due':
      if (data(notification).window === 'reminder') return t('headline.task_reminder')
      return data(notification).window === 'today' ? t('headline.task_due_today') : t('headline.task_due_soon')
    case 'task_assigned':
    case 'deal_assigned':
    case 'deal_stage_changed':
    case 'note_mention':
      return t(`headline.${kind}`, { actor })
    default:
      return t(`headline.${kind}` as never, { actor, defaultValue: notification.title })
  }
}

export function notificationSubject(notification: Notification, t: TFunction<'notifications'>): string {
  if (isRestricted(notification)) return t('restrictedRecord')
  return notification.title
}

export function isDownloadLink(link: string): boolean {
  return link.startsWith('/api/')
}
