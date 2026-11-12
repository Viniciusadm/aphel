import { api } from '@/api'
import { createContext, useContext, type ComponentType } from 'react'

import type { Block } from '../blocks/blocks'

export interface DocumentRecord {
  id: string
  draft_title: string
  title?: string | null
  slug: string
  publication_state: 'draft' | 'published' | 'withdrawn'
  published_at?: string | null
  revised_at?: string | null
  updated_at?: string | null
  created_at?: string | null
  [field: string]: unknown
}

export interface SiteLanguage {
  code: string
  prefix: string
  default: boolean
}

export type RequestStatus = 'awaiting_approval' | 'scheduled' | 'ready' | 'completed' | 'superseded' | 'cancelled' | 'blocked' | 'failed'

export interface PublicationRequest {
  id: string
  action: 'activate' | 'withdraw'
  revision_id?: string | null
  status: RequestStatus
  run_at?: string | null
  requested_by?: string | null
  reason_code?: string | null
  created_at: string
  completed_at?: string | null
}

export interface PublicationRevision {
  id: string
  number: number
  payload?: { fields?: Record<string, unknown> } | null
  note?: string | null
  title?: string | null
  created_by?: string | null
  created_at: string
}

export interface PublicationOverview {
  generation?: number
  first_published_at?: string | null
  active_revision?: PublicationRevision | null
  requests: PublicationRequest[]
}

export interface RecordStatus {
  record_id: string
  published: boolean
  open_request?: PublicationRequest | null
}

export interface PreviewLink {
  url: string
  page_url: string
  expires_in_minutes: number
}

export interface SiteStructure {
  home: string | null
  menu: string[]
}

export const OPEN_STATUSES: RequestStatus[] = ['awaiting_approval', 'scheduled', 'ready']
const PUBLICATION = '/v1/publication'
const DRAFT = 'draft_'
const TRANSLATIONS = '_translations'

export function textField(record: DocumentRecord, field: string): string {
  const value = record[field]
  return typeof value === 'string' ? value : ''
}

function translationsOf(record: DocumentRecord, base: string): Record<string, string> {
  try {
    const parsed: unknown = JSON.parse(textField(record, `${base}${TRANSLATIONS}`) || '{}')
    return parsed && typeof parsed === 'object' && !Array.isArray(parsed) ? (parsed as Record<string, string>) : {}
  } catch {
    return {}
  }
}

export function readText(record: DocumentRecord, base: string, language: SiteLanguage | null): string {
  if (!language || language.default) return textField(record, base)
  return translationsOf(record, base)[language.code] ?? ''
}

export function writeText(record: DocumentRecord, base: string, language: SiteLanguage | null, value: string): Record<string, string> {
  if (!language || language.default) return { [base]: value }
  const translations = { ...translationsOf(record, base) }
  if (value === '') delete translations[language.code]
  else translations[language.code] = value
  return { [`${base}${TRANSLATIONS}`]: JSON.stringify(translations) }
}

function comparable(value: unknown, field: string): string {
  if (value === null || value === undefined || value === '') return field.endsWith(TRANSLATIONS) ? '{}' : ''
  if (Array.isArray(value)) return JSON.stringify([...value].map(String).sort())
  if (typeof value === 'string' && field.endsWith(TRANSLATIONS)) {
    try {
      const parsed = JSON.parse(value) as Record<string, unknown>
      return JSON.stringify(Object.entries(parsed).filter(([, text]) => text !== '' && text !== null).sort())
    } catch {
      return value
    }
  }
  if (value === '[]') return ''
  return String(value)
}

export function hasUnpublishedChanges(record: DocumentRecord): boolean {
  if (record.publication_state !== 'published') return false
  return Object.keys(record)
    .filter((key) => key.startsWith(DRAFT) && key.slice(DRAFT.length) in record)
    .some((key) => {
      const published = key.slice(DRAFT.length)
      return comparable(record[key], published) !== comparable(record[published], published)
    })
}

export type DocumentState = 'draft' | 'published' | 'changed' | 'withdrawn'

export function documentState(record: DocumentRecord): DocumentState {
  if (record.publication_state === 'withdrawn') return 'withdrawn'
  if (record.publication_state !== 'published') return 'draft'
  return hasUnpublishedChanges(record) ? 'changed' : 'published'
}

export function publicationApi(recordType: string) {
  return {
    status: (id: string) => api.get<PublicationOverview>(`${PUBLICATION}/status`, { query: { record_type: recordType, record_id: id } }),
    statuses: (ids: string[]) => api.get<RecordStatus[]>(`${PUBLICATION}/statuses`, { query: { record_type: recordType, record_ids: ids.join(',') } }),
    revisions: async (id: string) =>
      (await api.get<{ data: PublicationRevision[] }>(`${PUBLICATION}/revisions`, { query: { record_type: recordType, record_id: id, per_page: '50' } })).data,
    revision: (id: string) => api.get<PublicationRevision>(`${PUBLICATION}/revisions/${encodeURIComponent(id)}`),
    publish: async (record: DocumentRecord, at?: string) => {
      const revision = await api.post<PublicationRevision>(`${PUBLICATION}/revisions`, {
        record_type: recordType,
        record_id: record.id,
        expected_version: record.updated_at,
      })
      return api.post<PublicationRequest>(`${PUBLICATION}/requests`, { revision_id: revision.id, at: at ?? null, idempotency_key: crypto.randomUUID() })
    },
    withdraw: (id: string) =>
      api.post<PublicationRequest>(`${PUBLICATION}/withdrawals`, { record_type: recordType, record_id: id, idempotency_key: crypto.randomUUID() }),
    cancel: (requestId: string) => api.post<PublicationRequest>(`${PUBLICATION}/requests/${encodeURIComponent(requestId)}/cancel`),
    restore: (revisionId: string, expectedVersion?: string | null) =>
      api.post<DocumentRecord>(`${PUBLICATION}/revisions/${encodeURIComponent(revisionId)}/restore`, { expected_version: expectedVersion ?? null }),
  }
}

export const siteKeys = {
  languages: ['content', 'site', 'languages'] as const,
  structure: ['content', 'site', 'structure'] as const,
}

export const siteApi = {
  languages: () => api.get<SiteLanguage[]>('/v1/site/languages'),
  structure: () => api.get<SiteStructure>('/v1/site/structure'),
  setHome: (target: string | null) => api.put<SiteStructure>('/v1/site/home', { target }),
  setMenu: (target: string, shown: boolean) => api.put<SiteStructure>('/v1/site/menu', { target, shown }),
}

export function siteTarget(recordType: string, id: string): string {
  return `record:${recordType}:${id}`
}

export function documentKeys(recordType: string) {
  const all = ['content', 'documents', recordType] as const
  return {
    all,
    list: () => [...all, 'list'] as const,
    detail: (id: string) => [...all, 'detail', id] as const,
    publication: (id: string) => [...all, 'publication', id] as const,
    revisions: (id: string) => [...all, 'revisions', id] as const,
    statuses: (ids: string[]) => [...all, 'statuses', ids.join(',')] as const,
  }
}

export interface SettingsProps {
  open: boolean
  record: DocumentRecord
  language: SiteLanguage | null
  onOpenChange: (open: boolean) => void
  onSaved: (record: DocumentRecord) => void
}

export interface DocumentBodyProps {
  record: DocumentRecord
  recordType: string
  title: string
  blocks: Block[]
  disabled: boolean
  translating: boolean
  onTitleChange: (title: string) => void
  onBlocksChange: (blocks: Block[]) => void
  focus: boolean
  onFocusChange: (focus: boolean) => void
  preview: boolean
  onPreviewChange: (preview: boolean) => void
}

export interface DocumentKind {
  recordType: string
  blocks: string
  listPath: (record: DocumentRecord) => string
  editPath: (id: string) => string
  createPermission: string
  updatePermission: string
  deletePermission: string
  publishPermission: string
  structure: boolean
  get: (id: string) => Promise<DocumentRecord>
  update: (id: string, changes: Record<string, unknown>) => Promise<DocumentRecord>
  remove: (id: string) => Promise<void>
  duplicate?: (id: string) => Promise<DocumentRecord>
  path: (record: DocumentRecord) => string | null
  previewFields: Record<string, string>
  Settings: ComponentType<SettingsProps>
  Body?: ComponentType<DocumentBodyProps>
  untitled?: string
  texts?: { namespace: string; scope: string }
}

export const DocumentKindContext = createContext<DocumentKind | null>(null)

export function useDocumentKind(): DocumentKind {
  const kind = useContext(DocumentKindContext)
  if (!kind) throw new Error('DocumentKindContext is missing')
  return kind
}

export function previewApi(kind: DocumentKind) {
  return {
    render: (record: DocumentRecord, values: Record<string, string>, path: string) =>
      api.post<{ html: string }>('/v1/site/preview/render', { record: { record_type: kind.recordType, record_id: record.id, values }, path }),
    link: (record: DocumentRecord, path: string) =>
      api.post<PreviewLink>('/v1/site/preview', { record: { record_type: kind.recordType, record_id: record.id, fields: kind.previewFields }, path }),
  }
}
