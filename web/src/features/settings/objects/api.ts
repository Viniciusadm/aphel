import { recordResources } from '@/features/records/resources'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { components } from '@/api/schema'
import { customRecordsKeys, type Relationship } from '@/features/custom-records'

type Schemas = components['schemas']

export type CustomObjectAdmin = Schemas['CustomObjectResponse'] & { record_count: number }
export type CreateCustomObjectRequest = Schemas['CreateCustomObjectRequest']
export type UpdateCustomObjectRequest = Schemas['UpdateCustomObjectRequest']
export type CreateRelationshipRequest = Schemas['CreateRelationshipRequest']
export type UpdateRelationshipRequest = Schemas['UpdateRelationshipRequest']

export const OBJECT_KEY_MAX_LENGTH = 40
export const OBJECT_KEY_PATTERN = /^[a-z](?:_?[a-z])*$/
export const RELATION_KEY_PATTERN = /^[a-z](?:_?[a-z0-9])*$/
export function relationshipRecordTypes(): string[] {
  return recordResources().filter((resource) => resource.relatable).map((resource) => resource.recordType)
}

export const objectsAdminKeys = {
  all: ['custom-objects-admin'] as const,
  list: () => [...objectsAdminKeys.all, 'list'] as const,
  detail: (key: string) => [...objectsAdminKeys.all, 'detail', key] as const,
}

function objectPath(key: string, suffix = '') {
  return `/v1/custom-objects/${encodeURIComponent(key)}${suffix}`
}

function relationshipPath(key: string) {
  return `/v1/relationships/${encodeURIComponent(key)}`
}

function ifMatch(version?: string | null) {
  return version ? { 'If-Match': version } : undefined
}

export const objectsAdminApi = {
  list: () => api.get<CustomObjectAdmin[]>('/v1/custom-objects'),
  get: (key: string) => api.get<CustomObjectAdmin>(objectPath(key)),
  create: (body: CreateCustomObjectRequest) => api.post<CustomObjectAdmin>('/v1/custom-objects', body),
  update: (key: string, body: UpdateCustomObjectRequest, version?: string | null) =>
    api.patch<CustomObjectAdmin>(objectPath(key), body, { headers: ifMatch(version) }),
  remove: (key: string) => api.delete<void>(objectPath(key)),
  reorder: (keys: string[]) => api.put<void>('/v1/custom-objects/order', { keys }),
  createRelationship: (body: CreateRelationshipRequest) => api.post<Relationship>('/v1/relationships', body),
  updateRelationship: (key: string, body: UpdateRelationshipRequest, version?: string | null) =>
    api.patch<Relationship>(relationshipPath(key), body, { headers: ifMatch(version) }),
  removeRelationship: (key: string) => api.delete<void>(relationshipPath(key)),
}

export function useCustomObjectsAdmin() {
  return useQuery({ queryKey: objectsAdminKeys.list(), queryFn: objectsAdminApi.list })
}

export function useCustomObjectAdmin(key: string) {
  return useQuery({ queryKey: objectsAdminKeys.detail(key), queryFn: () => objectsAdminApi.get(key) })
}

export function useInvalidateObjects() {
  const queryClient = useQueryClient()
  return () =>
    Promise.all([
      queryClient.invalidateQueries({ queryKey: objectsAdminKeys.all }),
      queryClient.invalidateQueries({ queryKey: customRecordsKeys.all }),
    ])
}
