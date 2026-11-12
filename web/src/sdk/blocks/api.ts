import { useQuery } from '@tanstack/react-query'

import { api } from '@/api'

export interface BlockRecord {
  recordType: string
  recordId: string
}

export interface EmbedOption {
  key: string
  label: string
}

export interface EmbedSource {
  kind: string
  options: EmbedOption[]
}

export interface SiteCollection {
  owner: string
  entity: string
  record_type: string
  label: string
  fields: string[]
  scope_field: string | null
  scopes: { id: string; label: string }[]
}

export const blockSourceKeys = {
  embeds: ['site', 'blocks', 'embeds'] as const,
  collections: ['site', 'blocks', 'collections'] as const,
}

export const blockSourcesApi = {
  embeds: () => api.get<EmbedSource[]>('/v1/site/embeds'),
  collections: () => api.get<SiteCollection[]>('/v1/site/collections'),
}

export function useEmbeds() {
  return useQuery({ queryKey: blockSourceKeys.embeds, queryFn: blockSourcesApi.embeds, staleTime: 60_000 })
}

export function useCollections() {
  return useQuery({ queryKey: blockSourceKeys.collections, queryFn: blockSourcesApi.collections, staleTime: 60_000 })
}
