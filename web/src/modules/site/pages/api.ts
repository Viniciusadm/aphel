import {
  api,
  documentKeys,
  documentPublicationApi as publicationOf,
  documentSiteApi,
  documentSiteKeys,
  documentState,
  isApiError,
  siteTarget,
  type DocumentKind,
  type DocumentRecord,
  type PreviewLink,
  type SiteLanguage,
  fileDownloadUrl,
  uploadFile,
} from '@/sdk'
import { PageSettingsDialog } from './PageSettingsDialog'
import { starterBlocks, type StarterTexts } from './templates'

export { documentSiteApi as siteApi, documentSiteKeys as siteKeys }
export type { PreviewLink, SiteLanguage }

export const PAGE_TYPE = 'site.page'
const RECORDS = `/v1/entities/${PAGE_TYPE}/records`
const MAX_SLUG_ATTEMPTS = 20
const MAX_TITLE = 160

export type PageRecord = DocumentRecord

export const PAGE_TEMPLATES = ['blank', 'about', 'services', 'campaign'] as const
export type PageTemplate = (typeof PAGE_TEMPLATES)[number]

export const pageKeys = documentKeys(PAGE_TYPE)
export const publicationApi = publicationOf(PAGE_TYPE)
export const pageState = documentState

export function pageTarget(id: string): string {
  return siteTarget(PAGE_TYPE, id)
}

export function pagePath(page: Pick<PageRecord, 'slug'>, prefix = ''): string {
  return `${prefix}/${page.slug}`
}

const get = (id: string) => api.get<PageRecord>(`${RECORDS}/${encodeURIComponent(id)}`)
const update = (id: string, changes: Record<string, unknown>) => api.patch<PageRecord>(`${RECORDS}/${encodeURIComponent(id)}`, changes)

function isTaken(error: unknown): boolean {
  return isApiError(error) && error.status === 409
}

async function createWithFreeSlug(base: string, values: Record<string, unknown>, fixed: boolean): Promise<PageRecord> {
  const root = base.slice(0, 60).replace(/-+$/, '')
  for (let attempt = 1; attempt <= MAX_SLUG_ATTEMPTS; attempt += 1) {
    const slug = attempt === 1 ? base : `${root}-${attempt}`
    try {
      return await api.post<PageRecord>(RECORDS, { ...values, slug })
    } catch (error) {
      if (fixed || !isTaken(error)) throw error
    }
  }
  throw new Error('slug_unavailable')
}

export const pagesApi = {
  list: async () => (await api.get<{ data: PageRecord[] }>(RECORDS, { query: { per_page: '100', sort: '-updated_at' } })).data,
  get,
  update,
  remove: (id: string) => api.delete<void>(`${RECORDS}/${encodeURIComponent(id)}`),
  create: (title: string, template: PageTemplate, slug: string, fixedSlug: boolean, texts: StarterTexts) =>
    createWithFreeSlug(
      slug,
      { draft_title: title, draft_blocks: JSON.stringify(starterBlocks(template, title, texts)) },
      fixedSlug,
    ),
  duplicate: async (id: string, copySuffix: { title: string; slug: string }) => {
    const original = await get(id)
    const title = `${original.draft_title.slice(0, MAX_TITLE - copySuffix.title.length)}${copySuffix.title}`
    const created = await createWithFreeSlug(
      `${original.slug}-${copySuffix.slug}`.slice(0, 70),
      { draft_title: title, draft_blocks: '[]', draft_seo_description: original.draft_seo_description ?? '' },
      false,
    )
    let blocks = text(original.draft_blocks) || '[]'
    let translations = text(original.draft_blocks_translations)
    const wanted = new Set([...(blocks.match(UUID) ?? []), ...(translations.match(UUID) ?? [])])
    for (const image of wanted) {
      try {
        const response = await fetch(fileDownloadUrl(image), { credentials: 'include' })
        if (!response.ok) continue
        const body = await response.blob()
        const uploaded = await uploadFile({ file: new File([body], 'image', { type: body.type }), target: { recordType: PAGE_TYPE, recordId: created.id } })
        const copy = (uploaded as { file?: { id?: string }; id?: string }).file?.id ?? (uploaded as { id?: string }).id
        if (copy) {
          blocks = blocks.split(image).join(copy)
          translations = translations.split(image).join(copy)
        }
      } catch {
        continue
      }
    }
    return update(created.id, {
      draft_blocks: blocks,
      ...(translations ? { draft_blocks_translations: translations } : {}),
      ...(original.draft_title_translations ? { draft_title_translations: original.draft_title_translations } : {}),
      ...(original.draft_seo_description_translations ? { draft_seo_description_translations: original.draft_seo_description_translations } : {}),
    })
  },
}

function text(value: unknown): string {
  return typeof value === 'string' ? value : ''
}

const UUID = /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/g

export const pageKind: DocumentKind = {
  recordType: PAGE_TYPE,
  blocks: 'blocks',
  listPath: () => '/site/pages',
  editPath: (id) => `/site/pages/${id}`,
  createPermission: 'site.page.create',
  updatePermission: 'site.page.update',
  deletePermission: 'site.page.delete',
  publishPermission: 'site.page.publish',
  structure: true,
  get,
  update,
  remove: pagesApi.remove,
  duplicate: (id) => pagesApi.duplicate(id, duplicateSuffix()),
  path: (record) => pagePath(record),
  previewFields: {
    blocks: 'draft_blocks',
    title: 'draft_title',
    seo_description: 'draft_seo_description',
    blocks_translations: 'draft_blocks_translations',
    title_translations: 'draft_title_translations',
    seo_description_translations: 'draft_seo_description_translations',
  },
  Settings: PageSettingsDialog,
}

function duplicateSuffix(): { title: string; slug: string } {
  const portuguese = (document.documentElement.lang || 'pt').toLowerCase().startsWith('pt')
  return portuguese ? { title: ' (cópia)', slug: 'copia' } : { title: ' (copy)', slug: 'copy' }
}
