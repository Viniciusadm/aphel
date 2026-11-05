import { Mail, MailCheck, MailOpen, MailWarning, MousePointerClick, type LucideIcon } from 'lucide-react'
import { type TimelineRenderer } from '@/sdk'
import type {EmailEventType} from '@/sdk'
import { EmailEventContent } from './EmailEventContent'

const ICONS: Record<EmailEventType, LucideIcon> = {
  'email.received': Mail,
  'email.sent': MailCheck,
  'email.opened': MailOpen,
  'email.clicked': MousePointerClick,
  'email.bounced': MailWarning,
}

export const EMAIL_TIMELINE_RENDERERS: Record<EmailEventType, TimelineRenderer> = Object.fromEntries(
  Object.entries(ICONS).map(([eventType, icon]) => [eventType, { icon, render: (item) => <EmailEventContent item={item} /> }]),
) as Record<EmailEventType, TimelineRenderer>
