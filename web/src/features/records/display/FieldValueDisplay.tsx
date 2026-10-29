import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { api } from '@/api'
import { Avatar } from '@/components/ui/avatar'
import { cn } from '@/lib/utils'
import { recordsKeys, useOrganizationUsers } from '../api'
import { useFieldTypeRenderers, type FieldDisplayProps } from '@/kernel/modules/field-types'
import { asValueField, isFormulaField } from '../fields'
import { useRecordFormatting } from '../formatting'
import { getRecordResource, recordPath, recordTitle } from '../resources'
import type { FieldDefinition, FieldOption, RecordItem } from '../types'
import { formatFieldText, isEmptyValue } from './values'

export interface FieldValueDisplayProps {
  field: FieldDefinition
  value: unknown
  className?: string
}

export function OptionChip({ option }: { option: FieldOption }) {
  return (
    <span className="inline-flex max-w-full items-center gap-1.5 rounded-md border px-1.5 py-0.5 text-xs font-medium">
      <span
        aria-hidden="true"
        className="size-2 shrink-0 rounded-full bg-muted-foreground"
        style={option.color ? { backgroundColor: option.color } : undefined}
      />
      <span className="truncate">{option.label}</span>
    </span>
  )
}

export function UserValue({ userId }: { userId: string }) {
  const { t } = useTranslation('records')
  const users = useOrganizationUsers()
  const name = users.data?.find((user) => user.id === userId)?.display_name ?? t('values.unknownUser')
  return (
    <span className="inline-flex min-w-0 items-center gap-2">
      <Avatar name={name} />
      <span className="truncate">{name}</span>
    </span>
  )
}

export function RecordValue({ recordType, recordId }: { recordType: string; recordId: string }) {
  const resource = getRecordResource(recordType)
  const query = useQuery({
    queryKey: [...recordsKeys.detail(recordType, recordId)],
    queryFn: () => api.get<RecordItem>(recordPath(resource!, recordId)),
    enabled: Boolean(resource),
    staleTime: 60_000,
  })
  const label = resource && query.data ? recordTitle(resource, query.data) : recordId
  const href = resource?.detailPath?.(recordId)
  if (!href) return <span className="truncate">{label}</span>
  return (
    <Link to={href} className="truncate text-primary underline-offset-4 hover:underline">
      {label}
    </Link>
  )
}

export function FieldValueDisplay({ field, value, className }: FieldValueDisplayProps) {
  const { t } = useTranslation('records')
  const target = isFormulaField(field) ? asValueField(field) : field
  const { Display = TextValue } = useFieldTypeRenderers(target.field_type, target.key)
  const empty = target.field_type !== 'checkbox' && isEmptyValue(value)
  return (
    <span className={cn('min-w-0', className)}>
      {empty ? <span className="text-muted-foreground">{t('values.empty')}</span> : <Display field={target} value={value} />}
    </span>
  )
}

export function TextValue({ field, value }: FieldDisplayProps) {
  const formatting = useRecordFormatting()
  return <span className="truncate">{formatFieldText(field, value, formatting)}</span>
}
