import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { activeFields } from '@/features/records/fields'
import type { FieldDefinition, FieldOption, FieldSchema, FilterOperator } from '@/features/records/types'

const ACTOR_TYPES = ['user', 'system', 'api_token', 'automation'] as const

function option(value: string, label: string, position: number): FieldOption {
  return { value, label, position, is_active: true } as FieldOption
}

function synthetic(
  key: string,
  label: string,
  base: Partial<FieldDefinition> & Pick<FieldDefinition, 'field_type' | 'operators'>,
  position: number,
): FieldDefinition {
  return {
    access: 'read_only',
    config: {} as FieldDefinition['config'],
    id: key,
    is_active: true,
    is_filterable: true,
    is_personal: false,
    is_required: false,
    is_searchable: false,
    is_sensitive: false,
    is_sortable: false,
    is_system: true,
    is_unique: false,
    key,
    label,
    options: [],
    position,
    record_type: 'trigger',
    ...base,
  } as FieldDefinition
}

export function useConditionSchema(
  schema: FieldSchema | undefined,
  options: { includeTrigger: boolean },
): FieldSchema | undefined {
  const { t } = useTranslation('automations')
  return useMemo(() => {
    if (!schema) return undefined
    if (!options.includeTrigger) return schema
    const fields = activeFields(schema).filter((field) => field.is_filterable && field.operators.length > 0)
    let position = 100_000
    const extra: FieldDefinition[] = []
    const equality: FilterOperator[] = ['eq', 'in']
    extra.push(
      synthetic(
        'trigger.changed_fields',
        t('conditions.changedFields'),
        {
          field_type: 'select',
          operators: equality,
          options: fields.map((field, index) => option(field.key, field.label, index)),
        },
        (position += 1),
      ),
    )
    extra.push(
      synthetic(
        'trigger.actor.type',
        t('conditions.actorType'),
        {
          field_type: 'select',
          operators: ['eq', 'neq', 'in', 'not_in'],
          options: ACTOR_TYPES.map((value, index) => option(value, t(`actorTypes.${value}`), index)),
        },
        (position += 1),
      ),
    )
    for (const field of fields.filter((candidate) => candidate.is_system)) {
      extra.push({
        ...field,
        id: `trigger.previous.${field.key}`,
        key: `trigger.previous.${field.key}`,
        label: t('conditions.previous', { field: field.label }),
        position: (position += 1),
      })
    }
    return { ...schema, fields: [...schema.fields, ...extra] }
  }, [options.includeTrigger, schema, t])
}
