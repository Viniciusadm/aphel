export const EMAILS_READ_PERMISSION = 'emails.read'
export const EMAILS_SEND_PERMISSION = 'emails.send'
export const EMAIL_THREAD_PARAM = 'email_thread'
export const EMAILS_TAB_KEY = 'emails'
export const MAILBOXES_PATH = '/account/mailboxes'
export const EMAIL_EVENT_TYPES = ['email.received', 'email.sent', 'email.opened', 'email.clicked', 'email.bounced'] as const
export type EmailEventType = (typeof EMAIL_EVENT_TYPES)[number]
