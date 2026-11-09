import { useQuery } from '@tanstack/react-query'
import { X } from 'lucide-react'
import { useId, useState, type KeyboardEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { api } from '@/api'
import { RecordFieldInput } from '@/components/fields/RecordFieldInput'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { MaskedInput } from '@/components/ui/masked-input'
import { isTextMaskKind } from '@/lib/text-mask'
import { recordsKeys, useOrganizationUsers } from '../api'
import { getRecordResource, recordPath, recordTitle } from '../resources'
import type { RecordItem } from '../types'
import { fieldConfig, valueFieldType } from '../fields'
import type { FieldDefinition, OperandKind } from '../types'
import { CURRENT_USER_VALUE, MAX_LIST_VALUES, MAX_RELATIVE_DAYS } from './model'
import { NativeSelect } from './NativeSelect'

export interface ValueInputProps {
  field: FieldDefinition
  operand: OperandKind
  value: unknown
  onChange: (value: unknown) => void
  label: string
}

interface Choice {
  value: string
  label: string
}

const CHOICE_TYPES = new Set(['select', 'multi_select', 'user'])

function textOf(value: unknown): string {
  if (value === null || value === undefined) return ''
  return String(value)
}

function dateOf(value: unknown): string {
  const text = textOf(value)
  return /^\d{4}-\d{2}-\d{2}/.test(text) ? text.slice(0, 10) : ''
}

function useChoices(field: FieldDefinition): Choice[] {
  const { t } = useTranslation('records-table')
  const users = useOrganizationUsers()
  if (field.field_type === 'user') {
    return [
      { value: CURRENT_USER_VALUE, label: t('filters.me') },
      ...(users.data ?? []).map((user) => ({ value: user.id, label: user.display_name })),
    ]
  }
  return field.options
    .filter((option) => option.is_active)
    .sort((left, right) => left.position - right.position)
    .map((option) => ({ value: option.value, label: option.label }))
}

export function ValueInput({ field, operand, value, onChange, label }: ValueInputProps) {
  const { t } = useTranslation('records-table')
  if (operand === 'none') return null
  if (operand === 'days') {
    return (
      <span className="inline-flex min-w-0 items-center gap-2">
        <Input placeholder={t('filters.daysPlaceholder')}
          type="number"
          inputMode="numeric"
          min={0}
          max={MAX_RELATIVE_DAYS}
          step={1}
          aria-label={label}
          className="w-24"
          value={textOf(value)}
          onChange={(event) => onChange(event.target.value === '' ? null : Number(event.target.value))}
        />
        <span className="text-sm text-muted-foreground">{t('filters.days')}</span>
      </span>
    )
  }
  if (operand === 'pair') {
    const [from, to] = Array.isArray(value) ? value : [null, null]
    return (
      <span className="flex min-w-0 flex-wrap items-center gap-2">
        <ScalarInput field={field} value={from} label={t('filters.valueFrom')} onChange={(next) => onChange([next, to])} />
        <span className="text-sm text-muted-foreground">{t('filters.and')}</span>
        <ScalarInput field={field} value={to} label={t('filters.valueTo')} onChange={(next) => onChange([from, next])} />
      </span>
    )
  }
  if (operand === 'list') {
    const items = Array.isArray(value) ? value : []
    if (valueFieldType(field) === 'record' && fieldConfig(field).target_record_type) {
      return <RecordListInput field={field} values={items} label={label} onChange={onChange} />
    }
    return CHOICE_TYPES.has(field.field_type) ? (
      <ChoiceList field={field} values={items} label={label} onChange={onChange} />
    ) : (
      <TagListInput field={field} values={items} label={label} onChange={onChange} />
    )
  }
  return <ScalarInput field={field} value={value} label={label} onChange={onChange} />
}

interface ScalarInputProps {
  field: FieldDefinition
  value: unknown
  label: string
  onChange: (value: unknown) => void
}

function ScalarInput({ field, value, label, onChange }: ScalarInputProps) {
  const { t } = useTranslation('records-table')
  const choices = useChoices(field)
  switch (valueFieldType(field)) {
    case 'checkbox':
      return (
        <NativeSelect
          aria-label={label}
          className="w-32"
          value={value === false ? 'false' : 'true'}
          onChange={(event) => onChange(event.target.value === 'true')}
        >
          <option value="true">{t('filters.yes')}</option>
          <option value="false">{t('filters.no')}</option>
        </NativeSelect>
      )
    case 'select':
    case 'multi_select':
    case 'user':
      return (
        <NativeSelect
          aria-label={label}
          className="w-full sm:w-48"
          value={textOf(value)}
          onChange={(event) => onChange(event.target.value || null)}
        >
          <option value="">{t('filters.choose')}</option>
          {choices.map((choice) => (
            <option key={choice.value} value={choice.value}>
              {choice.label}
            </option>
          ))}
        </NativeSelect>
      )
    case 'number':
    case 'currency':
      return (
        <Input placeholder={t('filters.numberPlaceholder')}
          type="number"
          step="any"
          inputMode="decimal"
          aria-label={label}
          className="w-full sm:w-36"
          value={textOf(value)}
          onChange={(event) => onChange(event.target.value === '' ? null : event.target.value)}
        />
      )
    case 'date':
    case 'datetime':
      return (
        <Input
          type="date"
          aria-label={label}
          className="w-full sm:w-40"
          value={dateOf(value)}
          onChange={(event) => onChange(event.target.value || null)}
        />
      )
    case 'record':
      if (fieldConfig(field).target_record_type) {
        return (
          <RecordFieldInput
            targetRecordType={fieldConfig(field).target_record_type}
            aria-label={label}
            className="w-full sm:w-56"
            value={textOf(value) || null}
            onChange={(next) => onChange(next ?? null)}
          />
        )
      }
      return (
        <Input
          aria-label={label}
          className="w-full sm:w-48"
          placeholder={t('filters.recordPlaceholder')}
          value={textOf(value)}
          onChange={(event) => onChange(event.target.value)}
        />
      )
    default: {
      const mask = valueFieldType(field) === 'text' ? fieldConfig(field).mask : null
      if (isTextMaskKind(mask)) {
        return (
          <MaskedInput
            mask={mask}
            aria-label={label}
            className="w-full sm:w-48"
            value={textOf(value)}
            onValueChange={onChange}
          />
        )
      }
      return (
        <Input placeholder={t('filters.valuePlaceholder')}
          aria-label={label}
          className="w-full sm:w-48"
          value={textOf(value)}
          onChange={(event) => onChange(event.target.value)}
        />
      )
    }
  }
}

interface ListInputProps {
  field: FieldDefinition
  values: unknown[]
  label: string
  onChange: (value: unknown[]) => void
}

function ChoiceList({ field, values, label, onChange }: ListInputProps) {
  const baseId = useId()
  const choices = useChoices(field)
  const selected = new Set(values.map(textOf))
  const toggle = (choice: string, checked: boolean) => {
    if (checked) {
      if (selected.size < MAX_LIST_VALUES) onChange([...values, choice])
    } else onChange(values.filter((item) => textOf(item) !== choice))
  }
  return (
    <div
      role="group"
      aria-label={label}
      className="grid max-h-40 w-full min-w-0 gap-1 overflow-y-auto rounded-md border p-2 sm:w-56"
    >
      {choices.map((choice, index) => {
        const id = `${baseId}-${index}`
        return (
          <span key={choice.value} className="flex items-center gap-2 text-sm">
            <Checkbox
              id={id}
              checked={selected.has(choice.value)}
              onCheckedChange={(checked) => toggle(choice.value, checked === true)}
            />
            <label htmlFor={id} className="min-w-0 truncate">
              {choice.label}
            </label>
          </span>
        )
      })}
    </div>
  )
}

function RecordChipLabel({ recordType, id }: { recordType: string; id: string }) {
  const resource = getRecordResource(recordType)
  const record = useQuery({
    queryKey: recordsKeys.detail(recordType, id),
    queryFn: () => api.get<RecordItem>(recordPath(resource!, id)),
    enabled: Boolean(resource),
    staleTime: 60_000,
  })
  return <span className="truncate">{resource && record.data ? recordTitle(resource, record.data) : id}</span>
}

function RecordListInput({ field, values, label, onChange }: ListInputProps) {
  const { t } = useTranslation('records-table')
  const recordType = fieldConfig(field).target_record_type ?? ''
  const ids = values.map(textOf).filter(Boolean)
  return (
    <div className="flex w-full min-w-0 flex-col gap-1 sm:w-64">
      {ids.length > 0 && (
        <div className="flex flex-wrap gap-1">
          {ids.map((id) => (
            <span key={id} className="inline-flex max-w-full items-center gap-1 rounded bg-muted px-1.5 py-0.5 text-xs">
              <RecordChipLabel recordType={recordType} id={id} />
              <button
                type="button"
                className="rounded-sm text-muted-foreground hover:text-foreground"
                aria-label={t('filters.removeValue', { value: id })}
                onClick={() => onChange(values.filter((existing) => textOf(existing) !== id))}
              >
                <X className="size-3" aria-hidden />
              </button>
            </span>
          ))}
        </div>
      )}
      <RecordFieldInput
        targetRecordType={recordType}
        aria-label={label}
        clearable={false}
        value={null}
        disabled={ids.length >= MAX_LIST_VALUES}
        onChange={(next) => {
          if (next && !ids.includes(next)) onChange([...values, next])
        }}
      />
    </div>
  )
}

function TagListInput({ field, values, label, onChange }: ListInputProps) {
  const { t } = useTranslation('records-table')
  const [draft, setDraft] = useState('')
  const fieldType = valueFieldType(field)
  const numeric = fieldType === 'number' || fieldType === 'currency'

  const commit = () => {
    const text = draft.trim()
    if (!text) return
    const next = text
      .split(',')
      .map((item) => item.trim())
      .filter(Boolean)
      .filter((item) => !values.some((existing) => textOf(existing) === item))
    if (next.length > 0) onChange([...values, ...next].slice(0, MAX_LIST_VALUES))
    setDraft('')
  }

  const handleKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'Enter' || event.key === ',') {
      event.preventDefault()
      commit()
    } else if (event.key === 'Backspace' && draft === '' && values.length > 0) {
      onChange(values.slice(0, -1))
    }
  }

  return (
    <div className="flex w-full min-w-0 flex-wrap items-center gap-1 rounded-md border border-input bg-background p-1 sm:w-64">
      {values.map((item) => (
        <span
          key={textOf(item)}
          className="inline-flex max-w-full items-center gap-1 rounded bg-muted px-1.5 py-0.5 text-xs"
        >
          <span className="truncate">{textOf(item)}</span>
          <button
            type="button"
            className="rounded-sm text-muted-foreground hover:text-foreground"
            aria-label={t('filters.removeValue', { value: textOf(item) })}
            onClick={() => onChange(values.filter((existing) => existing !== item))}
          >
            <X className="size-3" aria-hidden />
          </button>
        </span>
      ))}
      <input
        aria-label={label}
        inputMode={numeric ? 'decimal' : undefined}
        placeholder={values.length === 0 ? t('filters.listPlaceholder') : undefined}
        className="h-7 min-w-24 flex-1 bg-transparent px-1 text-sm outline-none placeholder:text-muted-foreground"
        value={draft}
        onChange={(event) => setDraft(event.target.value)}
        onKeyDown={handleKeyDown}
        onBlur={commit}
      />
    </div>
  )
}
