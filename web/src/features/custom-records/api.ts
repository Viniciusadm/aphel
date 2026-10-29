import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { components } from '@/api/schema'
import { recordsKeys } from '@/features/records/api'
import type { Page } from '@/features/records/types'
import { registerCustomObjectResources, type CustomObject } from './resources'

type Schemas = components['schemas']

export type ObjectSchema = Schemas['ObjectSchemaResponse']
export type ObjectRelation = Schemas['ObjectRelationResponse']
export type RelationInfo = Schemas['RelationInfoResponse']
export type RelatedRecord = Schemas['RelatedRecordResponse']
export type Relationship = Schemas['RelationshipResponse']
export type RelationshipCardinality = Schemas['RelationshipCardinality']
export type RelationshipOnDelete = Schemas['RelationshipOnDelete']

export interface RequiredRelation {
  key: string
  label: string
  targetType: string
}

export const RELATED_RECORDS_PAGE_SIZE = 20

export const customRecordsKeys = {
  all: ['custom-objects'] as const,
  objects: () => [...customRecordsKeys.all, 'navigation'] as const,
  schema: (key: string) => [...customRecordsKeys.all, 'schema', key] as const,
  relations: (recordType: string, recordId: string) =>
    [...recordsKeys.detail(recordType, recordId), 'relations'] as const,
  related: (recordType: string, recordId: string, relation: string, page: number) =>
    [...customRecordsKeys.relations(recordType, recordId), relation, page] as const,
  relationships: (recordType?: string) => [...customRecordsKeys.all, 'relationships', recordType ?? 'all'] as const,
}

function relationsPath(recordType: string, recordId: string, suffix = '') {
  return `/v1/records/${encodeURIComponent(recordType)}/${encodeURIComponent(recordId)}/relations${suffix}`
}

export const customRecordsApi = {
  objects: async () => {
    const objects = await api.get<CustomObject[]>('/v1/objects')
    registerCustomObjectResources(objects)
    return objects
  },
  schema: (key: string) => api.get<ObjectSchema>(`/v1/objects/${encodeURIComponent(key)}/schema`),
  relationships: (recordType?: string) =>
    api.get<Relationship[]>('/v1/relationships', { query: { record_type: recordType } }),
  relations: (recordType: string, recordId: string) =>
    api.get<RelationInfo[]>(relationsPath(recordType, recordId)),
  related: (recordType: string, recordId: string, relation: string, page: number) =>
    api.get<Page<RelatedRecord>>(relationsPath(recordType, recordId, `/${encodeURIComponent(relation)}`), {
      query: { page, per_page: RELATED_RECORDS_PAGE_SIZE },
    }),
  link: (recordType: string, recordId: string, relation: string, targetId: string) =>
    api.post<{ linked: boolean }>(relationsPath(recordType, recordId, `/${encodeURIComponent(relation)}`), {
      record_id: targetId,
    }),
  unlink: (recordType: string, recordId: string, relation: string, targetId: string) =>
    api.delete<void>(
      relationsPath(recordType, recordId, `/${encodeURIComponent(relation)}/${encodeURIComponent(targetId)}`),
    ),
}

export function useCustomObjects() {
  return useQuery({
    queryKey: customRecordsKeys.objects(),
    queryFn: customRecordsApi.objects,
    staleTime: 60_000,
  })
}

export function useCustomObjectLabel(recordType: string): string | undefined {
  const objects = useCustomObjects()
  return objects.data?.find((object) => object.record_type === recordType)?.label_plural
}

export function useObjectSchema(key: string) {
  return useQuery({
    queryKey: customRecordsKeys.schema(key),
    queryFn: () => customRecordsApi.schema(key),
    staleTime: 60_000,
  })
}

export function useRelationships(recordType?: string) {
  return useQuery({
    queryKey: customRecordsKeys.relationships(recordType),
    queryFn: () => customRecordsApi.relationships(recordType),
  })
}

export function requiredRelations(relationships: Relationship[], recordType: string): RequiredRelation[] {
  return relationships
    .filter((relationship) => relationship.required && relationship.to_type === recordType)
    .map((relationship) => ({
      key: relationship.inverse_key,
      label: relationship.to_label,
      targetType: relationship.from_type,
    }))
}

export function useRecordRelations(recordType: string, recordId: string, options: { enabled?: boolean } = {}) {
  return useQuery({
    queryKey: customRecordsKeys.relations(recordType, recordId),
    queryFn: () => customRecordsApi.relations(recordType, recordId),
    enabled: (options.enabled ?? true) && Boolean(recordId),
  })
}

export function useRelatedRecords(recordType: string, recordId: string, relation: string, page: number) {
  return useQuery({
    queryKey: customRecordsKeys.related(recordType, recordId, relation, page),
    queryFn: () => customRecordsApi.related(recordType, recordId, relation, page),
    placeholderData: keepPreviousData,
  })
}

export function useRelationMutations(recordType: string, recordId: string, relation: RelationInfo) {
  const queryClient = useQueryClient()
  const invalidate = (_result: unknown, targetId: string) => {
    void queryClient.invalidateQueries({ queryKey: customRecordsKeys.relations(recordType, recordId) })
    void queryClient.invalidateQueries({ queryKey: customRecordsKeys.relations(relation.target_type, targetId) })
    void queryClient.invalidateQueries({ queryKey: recordsKeys.list(relation.target_type) })
    void queryClient.invalidateQueries({ queryKey: recordsKeys.list(recordType) })
  }
  const link = useMutation({
    mutationFn: (targetId: string) => customRecordsApi.link(recordType, recordId, relation.key, targetId),
    onSuccess: invalidate,
  })
  const unlink = useMutation({
    mutationFn: (targetId: string) => customRecordsApi.unlink(recordType, recordId, relation.key, targetId),
    onSuccess: invalidate,
  })
  return { link, unlink }
}
