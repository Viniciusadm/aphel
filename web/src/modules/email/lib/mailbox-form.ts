import { IMAP_PROVIDER, type ImapSettings, type MailSecurity, type Mailbox, type EmailVisibility } from '../types'

export const DEFAULT_HISTORY_DAYS = 90
export const MAX_HISTORY_DAYS = 3650

const IMAP_PORTS: Record<MailSecurity, number> = { tls: 993, starttls: 143, none: 143 }
const SMTP_PORTS: Record<MailSecurity, number> = { tls: 465, starttls: 587, none: 25 }

export interface MailboxFormValues {
  display_name: string
  history_days: string
  default_visibility: EmailVisibility
  imap_host: string
  imap_port: string
  imap_security: MailSecurity
  smtp_host: string
  smtp_port: string
  smtp_security: MailSecurity
  username: string
  password: string
}

export function emptyMailboxForm(): MailboxFormValues {
  return {
    display_name: '',
    history_days: String(DEFAULT_HISTORY_DAYS),
    default_visibility: 'metadata',
    imap_host: '',
    imap_port: String(IMAP_PORTS.tls),
    imap_security: 'tls',
    smtp_host: '',
    smtp_port: String(SMTP_PORTS.starttls),
    smtp_security: 'starttls',
    username: '',
    password: '',
  }
}

function text(value: unknown, fallback: string): string {
  return typeof value === 'string' || typeof value === 'number' ? String(value) : fallback
}

function security(value: unknown, fallback: MailSecurity): MailSecurity {
  return value === 'tls' || value === 'starttls' || value === 'none' ? value : fallback
}

export function mailboxFormFrom(mailbox: Mailbox): MailboxFormValues {
  const empty = emptyMailboxForm()
  const settings = mailbox.settings
  return {
    display_name: mailbox.display_name ?? '',
    history_days: String(mailbox.history_days),
    default_visibility: mailbox.default_visibility,
    imap_host: text(settings.imap_host, ''),
    imap_port: text(settings.imap_port, empty.imap_port),
    imap_security: security(settings.imap_security, empty.imap_security),
    smtp_host: text(settings.smtp_host, ''),
    smtp_port: text(settings.smtp_port, empty.smtp_port),
    smtp_security: security(settings.smtp_security, empty.smtp_security),
    username: text(settings.username, ''),
    password: '',
  }
}

export function portFor(kind: 'imap' | 'smtp', value: MailSecurity): string {
  return String((kind === 'imap' ? IMAP_PORTS : SMTP_PORTS)[value])
}

export function isDefaultPort(kind: 'imap' | 'smtp', port: string): boolean {
  const ports = kind === 'imap' ? IMAP_PORTS : SMTP_PORTS
  return Object.values(ports).some((value) => String(value) === port.trim())
}

function validPort(value: string): boolean {
  const port = Number(value)
  return Number.isInteger(port) && port > 0 && port <= 65535
}

export function validHistoryDays(value: string): boolean {
  const days = Number(value)
  return Number.isInteger(days) && days >= 1 && days <= MAX_HISTORY_DAYS
}

export function isImapProvider(provider: string): boolean {
  return provider === IMAP_PROVIDER
}

export function mailboxFormErrors(values: MailboxFormValues, withServer: boolean, requirePassword: boolean) {
  return {
    history_days: !validHistoryDays(values.history_days),
    imap_host: withServer && values.imap_host.trim() === '',
    imap_port: withServer && !validPort(values.imap_port),
    smtp_host: withServer && values.smtp_host.trim() === '',
    smtp_port: withServer && !validPort(values.smtp_port),
    username: withServer && values.username.trim() === '',
    password: withServer && requirePassword && values.password === '',
  }
}

export function imapSettings(values: MailboxFormValues): ImapSettings {
  return {
    imap_host: values.imap_host.trim(),
    imap_port: Number(values.imap_port),
    imap_security: values.imap_security,
    smtp_host: values.smtp_host.trim(),
    smtp_port: Number(values.smtp_port),
    smtp_security: values.smtp_security,
    username: values.username.trim(),
  }
}
