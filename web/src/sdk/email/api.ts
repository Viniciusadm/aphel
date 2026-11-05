import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { API_BASE_URL, api } from '@/api'
import type {
  BlocklistEntry,
  CaptureSettings,
  CreateBlocklistEntryRequest,
  CreateMailboxRequest,
  EmailMessage,
  EmailPreview,
  EmailRecordRef,
  EmailTemplate,
  EmailTemplateInput,
  EmailThreadDetail,
  EmailThreadPage,
  EmailVisibility,
  Mailbox,
  MailboxProviderInfo,
  PreviewEmailRequest,
  SendEmailRequest,
  TemplateScopeFilter,
  TemplateVariable,
  TrackingSettings,
  UpdateMailboxRequest,
} from './types'

export const THREADS_PER_PAGE = 20
export const SYNC_POLL_INTERVAL_MS = 5_000

export const emailKeys = {
  all: ['email'] as const,
  providers: () => [...emailKeys.all, 'providers'] as const,
  mailboxes: () => [...emailKeys.all, 'mailboxes'] as const,
  threads: () => [...emailKeys.all, 'threads'] as const,
  recordThreads: (record: EmailRecordRef, page: number) =>
    [...emailKeys.threads(), 'record', record.record_type, record.record_id, page] as const,
  thread: (id: string) => [...emailKeys.threads(), 'detail', id] as const,
  templates: () => [...emailKeys.all, 'templates'] as const,
  templateList: (scope: TemplateScopeFilter, recordType?: string | null) =>
    [...emailKeys.templates(), scope, recordType ?? null] as const,
  variables: (recordType?: string | null) => [...emailKeys.all, 'variables', recordType ?? null] as const,
  capture: () => [...emailKeys.all, 'settings', 'capture'] as const,
  tracking: () => [...emailKeys.all, 'settings', 'tracking'] as const,
  blocklist: () => [...emailKeys.all, 'blocklist'] as const,
}

const id = encodeURIComponent

export function attachmentDownloadUrl(messageId: string, attachmentId: string): string {
  return `${API_BASE_URL}/v1/email-messages/${id(messageId)}/attachments/${id(attachmentId)}`
}

export const emailApi = {
  providers: () => api.get<MailboxProviderInfo[]>('/v1/mailboxes/providers'),
  mailboxes: () => api.get<Mailbox[]>('/v1/mailboxes'),
  createMailbox: (request: CreateMailboxRequest) => api.post<Mailbox>('/v1/mailboxes', request),
  updateMailbox: (mailboxId: string, request: UpdateMailboxRequest) =>
    api.patch<Mailbox>(`/v1/mailboxes/${id(mailboxId)}`, request),
  testMailbox: (mailboxId: string) => api.post<Mailbox>(`/v1/mailboxes/${id(mailboxId)}/test`),
  syncMailbox: (mailboxId: string) => api.post<unknown>(`/v1/mailboxes/${id(mailboxId)}/sync`),
  removeMailbox: (mailboxId: string) => api.delete<void>(`/v1/mailboxes/${id(mailboxId)}`),

  recordThreads: (record: EmailRecordRef, page: number, perPage = THREADS_PER_PAGE) =>
    api.get<EmailThreadPage>('/v1/email-threads', {
      query: { record_type: record.record_type, record_id: record.record_id, page, per_page: perPage },
    }),
  thread: (threadId: string) => api.get<EmailThreadDetail>(`/v1/email-threads/${id(threadId)}`),
  setThreadVisibility: (threadId: string, visibility: EmailVisibility | null) =>
    api.patch<unknown>(`/v1/email-threads/${id(threadId)}`, { visibility }),
  linkThread: (threadId: string, record: EmailRecordRef) =>
    api.post<unknown>(`/v1/email-threads/${id(threadId)}/links`, record),
  unlinkThread: (threadId: string, record: EmailRecordRef) =>
    api.delete<void>(
      `/v1/email-threads/${id(threadId)}/links/${id(record.record_type)}/${id(record.record_id)}`,
    ),

  send: (request: SendEmailRequest) => api.post<EmailMessage>('/v1/emails', request),
  cancel: (messageId: string) => api.post<EmailMessage>(`/v1/emails/${id(messageId)}/cancel`),
  preview: (request: PreviewEmailRequest) => api.post<EmailPreview>('/v1/emails/preview', request),

  templates: (scope: TemplateScopeFilter, recordType?: string | null) =>
    api.get<EmailTemplate[]>('/v1/email-templates', { query: { scope, record_type: recordType ?? undefined } }),
  createTemplate: (input: EmailTemplateInput) => api.post<EmailTemplate>('/v1/email-templates', input),
  updateTemplate: (templateId: string, input: Partial<EmailTemplateInput>) =>
    api.patch<EmailTemplate>(`/v1/email-templates/${id(templateId)}`, input),
  removeTemplate: (templateId: string) => api.delete<void>(`/v1/email-templates/${id(templateId)}`),
  variables: (recordType?: string | null) =>
    api.get<TemplateVariable[]>('/v1/email-templates/variables', { query: { record_type: recordType ?? undefined } }),

  capture: () => api.get<CaptureSettings>('/v1/email-settings/capture'),
  saveCapture: (settings: CaptureSettings) => api.put<CaptureSettings>('/v1/email-settings/capture', settings),
  tracking: () => api.get<TrackingSettings>('/v1/email-settings/tracking'),
  saveTracking: (settings: TrackingSettings) => api.put<TrackingSettings>('/v1/email-settings/tracking', settings),
  blocklist: () => api.get<BlocklistEntry[]>('/v1/email-blocklist'),
  addBlocklistEntry: (request: CreateBlocklistEntryRequest) => api.post<BlocklistEntry>('/v1/email-blocklist', request),
  removeBlocklistEntry: (entryId: string) => api.delete<void>(`/v1/email-blocklist/${id(entryId)}`),
}

export function useMailboxProviders() {
  return useQuery({ queryKey: emailKeys.providers(), queryFn: emailApi.providers, staleTime: 60_000 })
}

export function useMailboxes() {
  return useQuery({
    queryKey: emailKeys.mailboxes(),
    queryFn: emailApi.mailboxes,
    refetchInterval: (query) =>
      query.state.data?.some((mailbox) => mailbox.status === 'syncing') ? SYNC_POLL_INTERVAL_MS : false,
  })
}

export function useInvalidateMailboxes() {
  const queryClient = useQueryClient()
  return () => queryClient.invalidateQueries({ queryKey: emailKeys.mailboxes() })
}

export function useRecordThreads(record: EmailRecordRef, page: number) {
  return useQuery({
    queryKey: emailKeys.recordThreads(record, page),
    queryFn: () => emailApi.recordThreads(record, page),
    placeholderData: keepPreviousData,
  })
}

export function useThread(threadId: string | null) {
  return useQuery({
    queryKey: emailKeys.thread(threadId ?? ''),
    queryFn: () => emailApi.thread(threadId!),
    enabled: Boolean(threadId),
  })
}

export function useInvalidateThreads() {
  const queryClient = useQueryClient()
  return () => queryClient.invalidateQueries({ queryKey: emailKeys.threads() })
}

export function useEmailTemplates(scope: TemplateScopeFilter, recordType?: string | null, enabled = true) {
  return useQuery({
    queryKey: emailKeys.templateList(scope, recordType),
    queryFn: () => emailApi.templates(scope, recordType),
    enabled,
  })
}

export function useTemplateVariables(recordType?: string | null) {
  return useQuery({
    queryKey: emailKeys.variables(recordType),
    queryFn: () => emailApi.variables(recordType),
    staleTime: 60_000,
  })
}

export function useSaveTemplate() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ templateId, input }: { templateId: string | null; input: EmailTemplateInput }) =>
      templateId ? emailApi.updateTemplate(templateId, input) : emailApi.createTemplate(input),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: emailKeys.templates() }),
  })
}

export function useRemoveTemplate() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: emailApi.removeTemplate,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: emailKeys.templates() }),
  })
}

export function useCaptureSettings() {
  return useQuery({ queryKey: emailKeys.capture(), queryFn: emailApi.capture })
}

export function useTrackingSettings(enabled = true) {
  return useQuery({ queryKey: emailKeys.tracking(), queryFn: emailApi.tracking, enabled, staleTime: 60_000 })
}

export function useBlocklist() {
  return useQuery({ queryKey: emailKeys.blocklist(), queryFn: emailApi.blocklist })
}
