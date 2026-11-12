import { useQuery } from '@tanstack/react-query'
import { api, ApiError } from '@/api'
import { uploadFile } from '@/features/files'
import type { RecordTarget } from '@/features/files/api'

export const MEDIA_ASSET = 'media.asset'
export const MEDIA_READ = 'media.assets.read'
export const MEDIA_MANAGE = 'media.assets.manage'
const RECORDS = `/v1/entities/${MEDIA_ASSET}/records`
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
  created_at: string
  updated_at: string
}

export interface MediaUsage {
  record_type: string
  record_id: string
  type_label: string
  label: string | null
}

export type MediaChanges = Partial<Pick<MediaAsset, 'name' | 'alt' | 'caption' | 'credit' | 'focal_x' | 'focal_y'>>

export const mediaKeys = {
  all: ['media'] as const,
  list: (search: string, images: boolean) => [...mediaKeys.all, 'list', search, images] as const,
  usage: (id: string) => [...mediaKeys.all, 'usage', id] as const,
}

export const mediaApi = {
  list: async (search: string, images: boolean) => {
    const query: Record<string, string> = { per_page: '100', sort: '-created_at' }
    if (search.trim()) query.search = search.trim()
    if (images) query['filter[content_type][starts_with]'] = 'image/'
    return (await api.get<{ data: MediaAsset[] }>(RECORDS, { query })).data
  },
  update: (id: string, changes: MediaChanges) => api.patch<MediaAsset>(`${RECORDS}/${encodeURIComponent(id)}`, changes),
  remove: (id: string) => api.delete<void>(`${RECORDS}/${encodeURIComponent(id)}`),
  usage: (id: string) => api.get<MediaUsage[]>(`/v1/media/assets/${encodeURIComponent(id)}/usage`),
  create: async (file: File): Promise<MediaAsset> => {
    const created = await api.post<MediaAsset>(RECORDS, { name: file.name.slice(0, NAME_MAX) || 'arquivo' })
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
