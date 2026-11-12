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

export interface StyleToken {
  slug: string
  name?: string
}

export interface PaletteToken extends StyleToken {
  light?: string
  dark?: string
}

export interface SiteStylesDocument {
  mode?: 'system' | 'light' | 'dark'
  palette?: PaletteToken[]
  gradients?: (StyleToken & { angle?: number; stops?: string[] })[]
  families?: (StyleToken & { stack?: string })[]
  sizes?: (StyleToken & { min?: string; max?: string })[]
  spacing?: { scale?: string[]; content?: string; wide?: string }
  shape?: { radius?: string; shadow?: string }
  elements?: Record<string, Record<string, unknown>>
  blocks?: Record<string, { style?: Record<string, unknown>; variations?: (StyleToken & { style?: Record<string, unknown> })[] }>
}

export function useSiteStyles() {
  return useQuery({
    queryKey: ['site', 'styles'] as const,
    queryFn: async () => (await api.get<{ styles: SiteStylesDocument }>('/v1/site/styles')).styles,
    staleTime: 60_000,
  })
}

export type LocalizedText = string | Record<string, string> | null | undefined

export interface PluginBlockField {
  key: string
  kind: 'text' | 'long_text' | 'select' | 'flag' | 'number'
  label?: LocalizedText
  description?: LocalizedText
  default?: unknown
  options?: { value: string; label?: LocalizedText }[]
}

export interface PluginBlockDefinition {
  type: `${string}:${string}`
  owner: string
  label?: LocalizedText
  description?: LocalizedText
  icon?: string
  context?: string | null
  group?: string
  fields: PluginBlockField[]
}

export function usePluginBlocks() {
  return useQuery({
    queryKey: ['site', 'blocks', 'plugins'] as const,
    queryFn: async () => (await api.get<{ blocks: PluginBlockDefinition[] }>('/v1/site/blocks')).blocks,
    staleTime: 60_000,
  })
}

export interface SiteDesign {
  id: string
  target: string
  kind: 'template' | 'part' | 'pattern' | 'menu'
  name?: string | null
}

export function useSiteDesigns(kind: SiteDesign['kind']) {
  return useQuery({
    queryKey: ['site', 'designs', kind] as const,
    queryFn: async () => (await api.get<{ data: SiteDesign[] }>('/v1/entities/site.design/records', { query: { 'filter[kind][eq]': kind, per_page: '100' } })).data,
    staleTime: 30_000,
    retry: false,
  })
}

export function localized(text: LocalizedText, language: string): string {
  if (!text) return ''
  if (typeof text === 'string') return text
  return text[language] ?? text[language.split('-')[0]] ?? text['pt-BR'] ?? text.en ?? Object.values(text)[0] ?? ''
}
