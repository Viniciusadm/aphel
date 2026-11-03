import { useQuery } from '@tanstack/react-query'
import { api } from '@/api'

export interface EntityDescriptor {
  key: string
  record_type: string
  labels?: Record<string, string>
  traits: Record<string, unknown>
}

export function useEntities() {
  return useQuery({
    queryKey: ['entities', 'catalog'],
    queryFn: () => api.get<EntityDescriptor[]>('/v1/entities'),
  })
}

export function entityLabel(entity: EntityDescriptor | undefined, locale: string, fallback: string): string {
  const value = entity?.labels ?? entity?.traits['ui.plural_labels']
  if (!value || typeof value !== 'object' || Array.isArray(value)) return fallback
  const labels = value as Record<string, unknown>
  const label = labels[locale] ?? labels[locale.split('-')[0]] ?? labels.en ?? Object.values(labels)[0]
  return typeof label === 'string' ? label : fallback
}
