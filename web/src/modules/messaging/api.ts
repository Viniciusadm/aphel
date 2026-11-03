import type { components } from '@/api/schema'
import i18n from '@/i18n'
import { keepPreviousData, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'

export const MESSAGING_KEY = 'messaging'
export const VIEW_PERMISSION = 'messaging.view'
export const REPLY_PERMISSION = 'messaging.reply'
export const MANAGE_CHANNELS_PERMISSION = 'messaging.manage_channels'
export const CONVERSATIONS_PER_PAGE = 25
export const CONVERSATION_STATUSES = ['open', 'pending', 'resolved'] as const
export const POLL_INTERVAL_MS = 20_000

export type ChannelKind = string
export type ConversationStatus = (typeof CONVERSATION_STATUSES)[number]
export type AssigneeFilter = 'any' | 'me' | 'none'

export interface ListMeta {
  total: number
  page: number
  per_page: number
}

export interface Channel {
  id: string
  kind: ChannelKind
  name: string
  address: string
  is_active: boolean
  status: string
  last_error?: string | null
  last_checked_at?: string | null
  last_polled_at?: string | null
  acting_user_id?: string | null
  created_by: string
  config?: Record<string, unknown> | null
  secrets?: Record<string, boolean> | null
  webhook_url?: string | null
  created_at: string
  updated_at: string
}

export interface ChannelBody {
  kind?: ChannelKind
  name: string
  is_active?: boolean
  acting_user_id?: string | null
  config: Record<string, unknown>
  secrets: Record<string, string>
  clear_secrets?: string[]
}

export interface ChannelTestResult {
  ok: boolean
  detail?: string | null
  error_code?: string | null
  message?: string | null
}

export interface LinkedRecord {
  id: string
  title?: string | null
  accessible: boolean
}

export interface ConversationChannel {
  available: boolean
  uses_references: boolean
  max_body_chars: number
  reply_window_seconds: number | null
  id: string
  name: string
  kind: ChannelKind
  address: string
  is_active: boolean
}

export interface Conversation {
  id: string
  channel: ConversationChannel
  peer: string
  peer_name?: string | null
  subject?: string | null
  status: ConversationStatus
  assignee_user_id?: string | null
  unread_count: number
  last_message_at: string
  last_message_preview?: string | null
  last_message_direction?: string | null
  contact?: LinkedRecord | null
  deal?: LinkedRecord | null
  reply_window_ends_at?: string | null
  created_at: string
}

export interface Message {
  id: string
  direction: 'in' | 'out'
  sender: string
  sender_name?: string | null
  subject?: string | null
  body: string
  message_type: string
  status: string
  error_code?: string | null
  error_message?: string | null
  sent_by_user_id?: string | null
  provider_timestamp?: string | null
  sent_at?: string | null
  delivered_at?: string | null
  read_at?: string | null
  created_at: string
}

export interface ConversationDetail {
  conversation: Conversation
  messages: Message[]
  total_messages: number
}

export interface ConversationFilters {
  channelId: string | null
  status: ConversationStatus | null
  assignee: AssigneeFilter
  unread: boolean
  search: string
  contactId?: string
  dealId?: string
  page: number
}

const BASE = '/v1/messaging'

export const messagingKeys = {
  all: [MESSAGING_KEY] as const,
  channels: () => [...messagingKeys.all, 'channels'] as const,
  conversations: () => [...messagingKeys.all, 'conversations'] as const,
  conversationList: (filters: ConversationFilters) => [...messagingKeys.conversations(), 'list', filters] as const,
  conversation: (id: string) => [...messagingKeys.conversations(), 'detail', id] as const,
}

function conversationPath(id: string, action?: string) {
  const base = `${BASE}/conversations/${encodeURIComponent(id)}`
  return action ? `${base}/${action}` : base
}

function channelPath(id: string, action?: string) {
  const base = `${BASE}/channels/${encodeURIComponent(id)}`
  return action ? `${base}/${action}` : base
}

export const messagingApi = {
  channels: () => api.get<{ data: Channel[]; meta: ListMeta }>(`${BASE}/channels`),
  createChannel: (body: ChannelBody) => api.post<Channel>(`${BASE}/channels`, body),
  updateChannel: (id: string, body: ChannelBody) => api.put<Channel>(channelPath(id), body),
  deleteChannel: (id: string) => api.delete<void>(channelPath(id)),
  testChannel: (id: string) => api.post<ChannelTestResult>(channelPath(id, 'test')),
  conversations: (filters: ConversationFilters) =>
    api.get<{ data: Conversation[]; meta: ListMeta }>(`${BASE}/conversations`, {
      query: {
        channel_id: filters.channelId ?? undefined,
        status: filters.status ?? undefined,
        assignee: filters.assignee === 'any' ? undefined : filters.assignee,
        unread: filters.unread ? 'true' : undefined,
        search: filters.search.trim() || undefined,
        contact_id: filters.contactId,
        deal_id: filters.dealId,
        page: filters.page,
        per_page: CONVERSATIONS_PER_PAGE,
      },
    }),
  conversation: (id: string) => api.get<ConversationDetail>(conversationPath(id)),
  reply: (id: string, body: string) => api.post<Message>(conversationPath(id, 'messages'), { body }),
  assign: (id: string, userId: string | null) =>
    api.put<Conversation>(conversationPath(id, 'assignee'), { user_id: userId }),
  setStatus: (id: string, status: ConversationStatus) =>
    api.put<Conversation>(conversationPath(id, 'status'), { status }),
  link: (id: string, contactId: string | null, dealId: string | null) =>
    api.put<Conversation>(conversationPath(id, 'links'), { contact_id: contactId, deal_id: dealId }),
  markRead: (id: string) => api.post<void>(conversationPath(id, 'read')),
}

export function useChannels(enabled = true) {
  return useQuery({ queryKey: messagingKeys.channels(), queryFn: messagingApi.channels, enabled, staleTime: 30_000 })
}

export function useConversations(filters: ConversationFilters, enabled = true) {
  return useQuery({
    queryKey: messagingKeys.conversationList(filters),
    queryFn: () => messagingApi.conversations(filters),
    enabled,
    placeholderData: keepPreviousData,
    refetchInterval: POLL_INTERVAL_MS,
  })
}

export function useConversation(id: string | null) {
  return useQuery({
    queryKey: messagingKeys.conversation(id ?? ''),
    queryFn: () => messagingApi.conversation(id ?? ''),
    enabled: id !== null,
    refetchInterval: POLL_INTERVAL_MS,
  })
}

export function useInvalidateMessaging() {
  const queryClient = useQueryClient()
  return () => queryClient.invalidateQueries({ queryKey: messagingKeys.all })
}

export function inboxPath(conversationId?: string) {
  return conversationId ? `/messaging?conversation=${encodeURIComponent(conversationId)}` : '/messaging'
}

export function peerLabel(conversation: Pick<Conversation, 'peer' | 'peer_name'>) {
  return conversation.peer_name?.trim() || conversation.peer
}

export type ChannelDefinition = components['schemas']['ChannelDefinition']
export function useChannelProviders(enabled = true) {
  return useQuery({ queryKey: [...messagingKeys.all, 'providers'], queryFn: () => api.get<ChannelDefinition[]>(`${BASE}/providers`), enabled, staleTime: 30_000 })
}
export function channelText(labels: Record<string,string>): string {
  return labels[i18n.resolvedLanguage ?? i18n.language] ?? labels.en ?? Object.values(labels)[0] ?? ''
}
