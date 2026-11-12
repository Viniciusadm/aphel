import { api } from '@/api'
import type { Block } from '@/sdk/blocks'

export type DesignKind = 'template' | 'part' | 'pattern' | 'menu' | 'font'

export interface DesignContent {
  blocks: Block[]
  name?: string
  header?: string
  footer?: string
  category?: string
  css?: string
}

export interface MenuItem {
  label: string
  url: string
  children?: MenuItem[]
}

export interface TemplateEntry {
  key: string
  owner: string
  owner_name: string
  page: string
  entity: string | null
  listing: boolean
  prefixed: boolean
  prefix_entity: string | null
  path: string
  default: DesignContent | null
  variants: { variant: string; default: DesignContent | null }[]
}

export interface DesignCatalog {
  templates: TemplateEntry[]
  parts: Record<string, DesignContent>
  patterns: Record<string, DesignContent>
}

export interface DesignRecord {
  id: string
  target: string
  kind: DesignKind
  name: string | null
  draft_content: string | null
  content: string | null
  publication_state: 'draft' | 'published' | 'withdrawn'
  updated_at: string
}

export interface SiteTarget {
  target: string
  label: string
  group: string
  path: string
}

const RECORDS = '/v1/entities/site.design/records'

export const designKeys = {
  catalog: ['site', 'designs', 'catalog'] as const,
  records: ['site', 'designs', 'records'] as const,
  targets: ['site', 'targets'] as const,
}

export const designApi = {
  catalog: () => api.get<DesignCatalog>('/v1/site/designs/catalog'),
  records: async () => (await api.get<{ data: DesignRecord[] }>(RECORDS, { query: { per_page: '100', sort: 'name' } })).data,
  targets: () => api.get<SiteTarget[]>('/v1/site/targets'),
  create: (values: { target: string; kind: DesignKind; name: string; draft_content: string }) => api.post<DesignRecord>(RECORDS, values),
  save: (record: DesignRecord, values: Partial<Pick<DesignRecord, 'name' | 'draft_content'>>) =>
    api.patch<DesignRecord>(`${RECORDS}/${encodeURIComponent(record.id)}`, { ...values, _expected_updated_at: record.updated_at }),
  remove: (record: DesignRecord) => api.delete<void>(`${RECORDS}/${encodeURIComponent(record.id)}`),
  load: (id: string) => api.get<DesignRecord>(`${RECORDS}/${encodeURIComponent(id)}`),
  publish: async (record: DesignRecord) => {
    const revision = await api.post<{ id: string }>('/v1/publication/revisions', { record_type: 'site.design', record_id: record.id, expected_version: record.updated_at })
    return api.post<{ status: string; reason_code: string | null }>('/v1/publication/requests', { revision_id: revision.id, idempotency_key: crypto.randomUUID() })
  },
  preview: async (target: string, content: string, path: string) => (await api.post<{ html: string }>('/v1/site/designs/preview', { target, content, path })).html,
}

export function parseContent(text: string | null | undefined): DesignContent {
  if (!text) return { blocks: [] }
  try {
    const parsed = JSON.parse(text) as Partial<DesignContent>
    return { ...parsed, blocks: Array.isArray(parsed.blocks) ? parsed.blocks : [] }
  } catch {
    return { blocks: [] }
  }
}

export function parseMenu(text: string | null | undefined): MenuItem[] {
  if (!text) return []
  try {
    const parsed: unknown = JSON.parse(text)
    return Array.isArray(parsed) ? (parsed as MenuItem[]) : []
  } catch {
    return []
  }
}
