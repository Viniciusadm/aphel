import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { api } from '@/api'

export interface EntityDescriptor {
  key: string
  record_type: string
  labels?: Record<string, string>
  permissions: { read: string; create: string | null; update: string | null }
  access: { read: boolean; create: boolean; update: boolean }
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

export function useEntityLabel(recordType: string): string {
  const { i18n } = useTranslation()
  const entities = useEntities()
  return entityLabel(entities.data?.find((entity) => entity.record_type === recordType), i18n.language, recordType)
}
