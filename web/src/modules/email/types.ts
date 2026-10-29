import type { components } from '@/api/schema'

type Schemas = components['schemas']
type Narrow<T, O> = Omit<T, keyof O> & O

export type EmailVisibility = 'private' | 'metadata' | 'shared'
export type EmailAccess = 'metadata' | 'full'
export type MailboxStatus = 'active' | 'syncing' | 'error' | 'disconnected'
export type MailSecurity = 'tls' | 'starttls' | 'none'
export type EmailDirection = 'inbound' | 'outbound'
export type EmailKind = 'message' | 'bounce' | 'auto_reply'
export type EmailStatus = 'received' | 'draft' | 'scheduled' | 'sending' | 'sent' | 'failed' | 'cancelled'
export type ParticipantRole = 'from' | 'to' | 'cc' | 'bcc' | 'reply_to'
export type LinkOrigin = 'automatic' | 'manual'
export type TemplateScope = 'organization' | 'personal'
export type TemplateScopeFilter = 'all' | TemplateScope
export type BlocklistKind = 'domain' | 'address'

export type MailboxConnection = { kind: 'credentials' } | { kind: 'oauth'; start_path: string }

export interface MailboxProviderInfo {
  key: string
  label: string
  connection: MailboxConnection
}

export interface ImapSettings {
  imap_host: string
  imap_port: number
  imap_security: MailSecurity
  smtp_host: string
  smtp_port: number
  smtp_security: MailSecurity
  username: string
}

export type Mailbox = Narrow<Schemas['MailboxResponse'], { status: MailboxStatus; default_visibility: EmailVisibility }>

export interface CreateMailboxRequest {
  provider: string
  display_name?: string | null
  history_days?: number
  default_visibility?: EmailVisibility
  settings: Record<string, unknown>
  credentials: Record<string, unknown>
}

export interface UpdateMailboxRequest {
  display_name?: string | null
  default_visibility?: EmailVisibility
  history_days?: number
  settings?: Record<string, unknown>
  credentials?: Record<string, unknown>
}

export type Participant = Narrow<Schemas['EmailParticipantResponse'], { role: ParticipantRole }>

export interface EmailRecordRef {
  record_type: string
  record_id: string
}

export type ThreadRecordLink = Narrow<Schemas['EmailThreadRecordResponse'], { origin: LinkOrigin }>

export type EmailOwner = Schemas['EmailThreadOwnerResponse']

export type EmailThread = Narrow<
  Schemas['EmailThreadResponse'],
  {
    visibility: EmailVisibility
    access: EmailAccess
    last_direction: EmailDirection | null
    participants: Participant[]
    records: ThreadRecordLink[]
  }
>

export type EmailBody = Schemas['EmailBodyResponse']

export type EmailAttachment = Schemas['EmailAttachmentResponse']

export type EmailTracking = Schemas['EmailTrackingResponse']

export type EmailMessage = Narrow<
  Schemas['EmailMessageResponse'],
  { direction: EmailDirection; kind: EmailKind; status: EmailStatus; participants: Participant[] }
>

export type EmailThreadDetail = EmailThread & { messages: EmailMessage[] }

export interface PageMeta {
  total: number
  page: number
  per_page: number
}

export interface EmailThreadPage {
  data: EmailThread[]
  meta: PageMeta
}

export type CaptureSettings = Schemas['EmailCaptureSettingsDto']

export type TrackingSettings = Schemas['EmailTrackingSettingsDto']

export type BlocklistEntry = Narrow<Schemas['EmailBlockResponse'], { kind: BlocklistKind }>

export interface CreateBlocklistEntryRequest {
  kind: BlocklistKind
  value: string
  reason?: string | null
}

export interface EmailAddress {
  address: string
  name?: string | null
}

export interface SendEmailRequest {
  mailbox_id?: string
  to: EmailAddress[]
  cc?: EmailAddress[]
  bcc?: EmailAddress[]
  subject: string
  body_html?: string
  body_text?: string
  template_id?: string
  record?: EmailRecordRef
  reply_to_message_id?: string
  attachment_file_ids?: string[]
  scheduled_at?: string
  tracking?: boolean
}

export interface PreviewEmailRequest {
  template_id?: string
  subject?: string
  body_html?: string
  record?: EmailRecordRef
}

export interface EmailPreview {
  subject: string
  html: string
  text: string
  missing_variables: string[]
}

export type EmailTemplate = Narrow<Schemas['EmailTemplateResponse'], { scope: TemplateScope }>

export interface EmailTemplateInput {
  key: string
  name: string
  scope: TemplateScope
  record_type: string | null
  subject: string
  body_html: string
}

export type TemplateVariable = Schemas['EmailTemplateVariableResponse']

export interface TimelineEmail {
  thread_id: string
  subject: string | null
  direction: EmailDirection
  access: EmailAccess
  snippet: string | null
  from: Participant | null
}

export const EMAIL_VISIBILITIES: EmailVisibility[] = ['private', 'metadata', 'shared']
export const MAIL_SECURITIES: MailSecurity[] = ['tls', 'starttls', 'none']
export const BLOCKLIST_KINDS: BlocklistKind[] = ['domain', 'address']
export const IMAP_PROVIDER = 'imap'
