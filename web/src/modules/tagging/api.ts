import { useEntities, type EntityDescriptor } from '@/sdk/entities'
import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { isCustomRecordType } from '@/sdk'
import { api } from '@/api'
import type { components } from '@/api/schema'

type Schemas = components['schemas']

export type Tag = Schemas['TagResponse']
export type TagSummary = Schemas['TagSummary']
export type TagsPage = Schemas['Page_TagResponse']
export type CreateTagBody = Schemas['CreateTagRequest']
export type UpdateTagBody = Schemas['UpdateTagRequest']
export type TaggingResult = Schemas['TaggingResponse']
export type RecordTagsEntry = Schemas['RecordTagsEntry']
export type Vocabulary = Schemas['TagVocabulary']

export const GENERAL_VOCABULARY = 'general'

export type TaggableRecordType = string
export const DEFAULT_TAG_COLOR = '#64748b'
export const MAX_TAG_NAME_LENGTH = 60
export const TAGS_PER_PAGE = 100
export const MAX_LOOKUP_IDS = 100

function taggableEntity(entity: EntityDescriptor): boolean {
  return Object.hasOwn(entity.traits, 'tagging.taggable') || isCustomRecordType(entity.record_type)
}

export function isTaggable(recordType: string, entities: readonly EntityDescriptor[]): boolean {
  return entities.some((entity) => entity.record_type === recordType && entity.access.read && taggableEntity(entity))
}

export function tagUpdatePermission(recordType: string, entities: readonly EntityDescriptor[]): string | null {
  const entity = entities.find((entity) => entity.record_type === recordType && entity.access.read && taggableEntity(entity))
  return entity?.permissions.update ?? null
}

export function useTaggingEntities() {
  const entities = useEntities()
  return { ...entities, data: entities.data?.filter(taggableEntity) }
}

export function useTaggingEntity(recordType: string) {
  const entities = useEntities().data ?? []
  return { taggable: isTaggable(recordType, entities), permission: tagUpdatePermission(recordType, entities) }
}

export const tagsKeys = {
  all: ['tags'] as const,
  list: (recordType?: string, vocabulary?: string) => [...tagsKeys.all, 'list', recordType ?? null, vocabulary ?? GENERAL_VOCABULARY] as const,
  vocabularies: () => [...tagsKeys.all, 'vocabularies'] as const,
  record: (recordType: string, recordId: string) => [...tagsKeys.all, 'record', recordType, recordId] as const,
  lookups: () => [...tagsKeys.all, 'lookup'] as const,
  lookup: (recordType: string, ids: string[]) => [...tagsKeys.lookups(), recordType, ids] as const,
}

function recordPath(recordType: string, recordId: string, tagId?: string) {
  const base = `/v1/tagging/records/${encodeURIComponent(recordType)}/${encodeURIComponent(recordId)}`
  return tagId ? `${base}/${encodeURIComponent(tagId)}` : base
}

export const tagsApi = {
  list: (recordType?: string, vocabulary?: string) =>
    api.get<TagsPage>('/v1/tagging', { query: { record_type: recordType, vocabulary, per_page: TAGS_PER_PAGE } }),
  vocabularies: () => api.get<Vocabulary[]>('/v1/tagging/vocabularies'),
  create: (body: CreateTagBody) => api.post<Tag>('/v1/tagging', body),
  update: (id: string, body: UpdateTagBody) => api.patch<Tag>(`/v1/tagging/${encodeURIComponent(id)}`, body),
  remove: (id: string) => api.delete<void>(`/v1/tagging/${encodeURIComponent(id)}`),
  recordTags: (recordType: string, recordId: string) => api.get<TagSummary[]>(recordPath(recordType, recordId)),
  lookup: (recordType: string, ids: string[]) =>
    api.get<RecordTagsEntry[]>('/v1/tagging/records', { query: { record_type: recordType, ids: ids.join(',') } }),
  add: (recordType: string, recordId: string, tagId: string) =>
    api.post<TaggingResult>(recordPath(recordType, recordId), { tag_id: tagId }),
  removeFromRecord: (recordType: string, recordId: string, tagId: string) =>
    api.delete<void>(recordPath(recordType, recordId, tagId)),
}

export function useVocabularies() {
  return useQuery({ queryKey: tagsKeys.vocabularies(), queryFn: tagsApi.vocabularies, staleTime: 60_000 })
}

export function useTags(recordType?: string, options: { enabled?: boolean; vocabulary?: string } = {}) {
  return useQuery({
    queryKey: tagsKeys.list(recordType, options.vocabulary),
    queryFn: async () => (await tagsApi.list(recordType, options.vocabulary)).data,
    staleTime: 60_000,
    enabled: options.enabled ?? true,
  })
}

export function useRecordTags(recordType: string, recordId: string) {
  const { taggable } = useTaggingEntity(recordType)
  return useQuery({
    queryKey: tagsKeys.record(recordType, recordId),
    queryFn: () => tagsApi.recordTags(recordType, recordId),
    enabled: taggable && Boolean(recordId),
  })
}

export function useRecordTagsLookup(recordType: string, ids: string[], options: { enabled?: boolean } = {}) {
  const { taggable } = useTaggingEntity(recordType)
  const unique = [...new Set(ids)].slice(0, MAX_LOOKUP_IDS).sort()
  return useQuery({
    queryKey: tagsKeys.lookup(recordType, unique),
    queryFn: async () => {
      const entries = await tagsApi.lookup(recordType, unique)
      return new Map(entries.map((entry) => [entry.record_id, entry.tags]))
    },
    enabled: (options.enabled ?? true) && taggable && unique.length > 0,
    placeholderData: keepPreviousData,
    staleTime: 30_000,
  })
}

export function useInvalidateTags() {
  const queryClient = useQueryClient()
  return () => queryClient.invalidateQueries({ queryKey: tagsKeys.all })
}

export function useRecordTagMutation(recordType: string, recordId: string) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: async ({ tagId, remove }: { tagId: string; remove: boolean }): Promise<TagSummary[]> => {
      if (!remove) return (await tagsApi.add(recordType, recordId, tagId)).tags
      await tagsApi.removeFromRecord(recordType, recordId, tagId)
      const current = queryClient.getQueryData<TagSummary[]>(tagsKeys.record(recordType, recordId)) ?? []
      return current.filter((tag) => tag.id !== tagId)
    },
    onSuccess: (tags) => {
      queryClient.setQueryData(tagsKeys.record(recordType, recordId), tags)
      void queryClient.invalidateQueries({ queryKey: tagsKeys.lookups() })
      void queryClient.invalidateQueries({ queryKey: tagsKeys.list() })
    },
  })
}
