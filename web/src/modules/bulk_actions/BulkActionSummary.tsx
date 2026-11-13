import { useTranslation } from 'react-i18next'
import { useReferenceOptions } from '@/kernel/modules/reference-options'
import { useFieldSchema, useOrganizationUsers } from '@/sdk'
import type { BulkOperation } from './api'

interface ActionParams {
  action?: string
  owner_id?: string
  tag_id?: string
  stage_id?: string
  sequence_id?: string
  field?: string
  value?: unknown
  custom?: boolean
}

function valueText(value: unknown): string | null {
  if (value === null || value === undefined || value === '') return null
  if (typeof value === 'string') return value
  if (typeof value === 'number' || typeof value === 'boolean') return String(value)
  return JSON.stringify(value)
}

function useActionSummary(operation: BulkOperation): string | null {
  const { t } = useTranslation('bulk_actions')
  const params = operation.params as ActionParams
  const action = operation.action
  const users = useOrganizationUsers()
  const schema = useFieldSchema(operation.record_type)
  const tags = useReferenceOptions('tag', {
    recordType: operation.record_type,
    includeInactive: true,
    enabled: action === 'add_tag' || action === 'remove_tag',
  })
  const stages = useReferenceOptions('pipeline_stage', { enabled: action === 'move_stage' })
  const sequences = useReferenceOptions('sequence', { enabled: action === 'enroll_in_sequence' })
  const unknown = t('history.summary.unknown')
  const labelOf = (options: { value: string; label: string }[], id: string | undefined) =>
    options.find((option) => option.value === id)?.label ?? unknown

  switch (action) {
    case 'assign_owner':
      return t('history.summary.assign_owner', {
        name: users.data?.find((user) => user.id === params.owner_id)?.display_name ?? unknown,
      })
    case 'add_tag':
    case 'remove_tag':
      return t(`history.summary.${action}`, { name: labelOf(tags.options, params.tag_id) })
    case 'move_stage':
      return t('history.summary.move_stage', { name: labelOf(stages.options, params.stage_id) })
    case 'enroll_in_sequence':
      return t('history.summary.enroll_in_sequence', { name: labelOf(sequences.options, params.sequence_id) })
    case 'set_field': {
      const fields = schema.data?.fields ?? []
      const field = fields.find((item) => item.key === params.field)?.label ?? params.field ?? unknown
      const value = valueText(params.value)
      return value === null
        ? t('history.summary.set_field_cleared', { field })
        : t('history.summary.set_field', { field, value })
    }
    default:
      return null
  }
}

export function BulkActionSummary({ operation, className }: { operation: BulkOperation; className?: string }) {
  const summary = useActionSummary(operation)
  if (!summary) return null
  return <span className={className ?? 'text-xs text-muted-foreground'}>{summary}</span>
}
