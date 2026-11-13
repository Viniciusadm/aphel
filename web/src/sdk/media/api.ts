import { useQuery } from '@tanstack/react-query'
import { api, ApiError } from '@/api'
import { uploadFile } from '@/features/files'
import type { RecordTarget } from '@/features/files/api'

export const MEDIA_ASSET = 'media.asset'
export const MEDIA_READ = 'media.assets.read'
export const MEDIA_MANAGE = 'media.assets.manage'
export const MEDIA_FOLDER = 'media.folder'
const RECORDS = `/v1/entities/${MEDIA_ASSET}/records`
const FOLDER_RECORDS = `/v1/entities/${MEDIA_FOLDER}/records`
export const ROOT_FOLDER = 'none'
const NAME_MAX = 160

export interface MediaAsset {
  id: string
  name: string
  file: string | null
  content_type: string | null
  alt: string | null
  caption: string | null
  credit: string | null
  focal_x: number | null
  focal_y: number | null
  folder_id: string | null
  created_at: string
  updated_at: string
}

export interface MediaFolder {
  id: string
  name: string
  parent_id: string | null
}

export type MediaKind = 'all' | 'images' | 'documents'
export type MediaSort = '-created_at' | 'created_at' | 'name'
export type MediaFolderFilter = 'all' | string

export interface MediaQuery {
  search: string
  kind: MediaKind
  sort: MediaSort
  folder: MediaFolderFilter
}

export interface MediaUsage {
  record_type: string
  record_id: string
  type_label: string
  label: string | null
}

export type MediaChanges = Partial<Pick<MediaAsset, 'name' | 'alt' | 'caption' | 'credit' | 'focal_x' | 'focal_y' | 'folder_id'>>

export const mediaKeys = {
  all: ['media'] as const,
  list: (search: string, images: boolean) => [...mediaKeys.all, 'list', search, images] as const,
  library: (query: MediaQuery) => [...mediaKeys.all, 'library', query] as const,
  folders: () => [...mediaKeys.all, 'folders'] as const,
  usage: (id: string) => [...mediaKeys.all, 'usage', id] as const,
}

export const mediaApi = {
  list: async (search: string, images: boolean) => {
    const query: Record<string, string> = { per_page: '100', sort: '-created_at' }
    if (search.trim()) query.search = search.trim()
    if (images) query['filter[content_type][starts_with]'] = 'image/'
    return (await api.get<{ data: MediaAsset[] }>(RECORDS, { query })).data
  },
  library: async (query: MediaQuery) => {
    const params: Record<string, string> = { per_page: '100', sort: query.sort }
    if (query.search.trim()) params.search = query.search.trim()
    if (query.kind === 'images') params['filter[content_type][starts_with]'] = 'image/'
    if (query.folder === ROOT_FOLDER) params['filter[folder_id][is_empty]'] = 'true'
    else if (query.folder !== 'all') params['filter[folder_id][eq]'] = query.folder
    const rows = (await api.get<{ data: MediaAsset[] }>(RECORDS, { query: params })).data
    return query.kind === 'documents' ? rows.filter((row) => !row.content_type?.startsWith('image/')) : rows
  },
  folders: async () => (await api.get<{ data: MediaFolder[] }>(FOLDER_RECORDS, { query: { per_page: '100', sort: 'name' } })).data,
  createFolder: (name: string, parentId: string | null) => api.post<MediaFolder>(FOLDER_RECORDS, { name, parent_id: parentId }),
  renameFolder: (id: string, name: string) => api.patch<MediaFolder>(`${FOLDER_RECORDS}/${encodeURIComponent(id)}`, { name }),
  removeFolder: (id: string) => api.delete<void>(`${FOLDER_RECORDS}/${encodeURIComponent(id)}`),
  update: (id: string, changes: MediaChanges) => api.patch<MediaAsset>(`${RECORDS}/${encodeURIComponent(id)}`, changes),
  remove: (id: string) => api.delete<void>(`${RECORDS}/${encodeURIComponent(id)}`),
  usage: (id: string) => api.get<MediaUsage[]>(`/v1/media/assets/${encodeURIComponent(id)}/usage`),
  create: async (file: File, folderId: string | null = null): Promise<MediaAsset> => {
    const created = await api.post<MediaAsset>(RECORDS, { name: file.name.slice(0, NAME_MAX) || 'arquivo', folder_id: folderId })
    try {
      const uploaded = await uploadFile({ file, target: { recordType: MEDIA_ASSET, recordId: created.id } })
      return await api.patch<MediaAsset>(`${RECORDS}/${encodeURIComponent(created.id)}`, {
        file: uploaded.file.id,
        content_type: uploaded.file.mime_type,
      })
    } catch (failure) {
      await api.delete(`${RECORDS}/${encodeURIComponent(created.id)}`).catch(() => undefined)
      throw failure
    }
  },
  attachTo: async (asset: MediaAsset, target: RecordTarget): Promise<string> => {
    if (!asset.file) throw new Error('the asset has no file')
    try {
      await api.post('/v1/attachments', { file_id: asset.file, record_type: target.recordType, record_id: target.recordId })
    } catch (failure) {
      if (!(failure instanceof ApiError && failure.code === 'already_attached')) throw failure
    }
    return asset.file
  },
}

export function useMediaAssets(search: string, images = false, enabled = true) {
  return useQuery({ queryKey: mediaKeys.list(search, images), queryFn: () => mediaApi.list(search, images), enabled })
}

export function useMediaUsage(id: string | null) {
  return useQuery({ queryKey: mediaKeys.usage(id ?? ''), queryFn: () => mediaApi.usage(id ?? ''), enabled: id !== null })
}

export function useMediaLibrary(query: MediaQuery) {
  return useQuery({ queryKey: mediaKeys.library(query), queryFn: () => mediaApi.library(query) })
}

export function useMediaFolders() {
  return useQuery({ queryKey: mediaKeys.folders(), queryFn: mediaApi.folders })
}
