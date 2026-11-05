import { Braces, Plus, Trash2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { FieldInput } from '@/components/fields'
import { Button } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'
import { activeFields, type FieldDefinition, FieldPicker, useFieldSchema } from '@/sdk'
import { cn } from '@/lib/utils'
import type { TemplateVariable } from '../lib/variables'
import type { JsonObject } from '../types'
import { TemplateInput } from './TemplateInput'

const LOCKED_FIELDS = new Set(['id', 'created_at', 'updated_at', 'deleted_at', 'archived_at', 'created_by'])

export interface FieldValuesEditorProps {
  recordType: string
  value: unknown
  onChange: (value: JsonObject) => void
  variables: TemplateVariable[]
  invalidKeys?: string[]
}

function objectOf(value: unknown): JsonObject {
  return typeof value === 'object' && value !== null && !Array.isArray(value) ? (value as JsonObject) : {}
}

function isTemplate(value: unknown): boolean {
  return typeof value === 'string' && value.includes('{{')
}

function writable(field: FieldDefinition): boolean {
  return (
    field.access === 'editable' &&
    !LOCKED_FIELDS.has(field.key) &&
    (field.field_type as string) !== 'formula'
  )
}

export function FieldValuesEditor({ recordType, value, onChange, variables, invalidKeys = [] }: FieldValuesEditorProps) {
  const { t } = useTranslation('automations')
  const schema = useFieldSchema(recordType)
  const values = objectOf(value)
  const [templateKeys, setTemplateKeys] = useState<Set<string>>(
    () => new Set(Object.entries(values).filter(([, item]) => isTemplate(item)).map(([key]) => key)),
  )

  if (!recordType) return null
  if (schema.isPending) return <Skeleton className="h-9 w-full" />
  if (!schema.data) return null

  const fields = activeFields(schema.data).filter(writable)
  const used = Object.keys(values)
  const available = fields.filter((field) => !used.includes(field.key))

  const setValue = (key: string, next: unknown) => onChange({ ...values, [key]: next })
  const remove = (key: string) => {
    const next = { ...values }
    delete next[key]
    onChange(next)
  }
  const replaceKey = (from: string, field: FieldDefinition) => {
    const next: JsonObject = {}
    for (const [key, item] of Object.entries(values)) next[key === from ? field.key : key] = key === from ? null : item
    onChange(next)
  }
  const toggleTemplate = (key: string) => {
    setTemplateKeys((current) => {
      const next = new Set(current)
      if (next.has(key)) next.delete(key)
      else next.add(key)
      return next
    })
    setValue(key, null)
  }

  return (
    <div className="grid gap-2">
      {used.length === 0 && <p className="text-xs text-muted-foreground">{t('values.empty')}</p>}
      {used.map((key) => {
        const field = fields.find((candidate) => candidate.key === key) ?? schema.data.fields.find((candidate) => candidate.key === key)
        const templated = templateKeys.has(key)
        const invalid = invalidKeys.includes(key)
        return (
          <div key={key} className={cn('grid gap-2 rounded-md border p-2', invalid && 'border-destructive')}>
            <div className="flex min-w-0 items-center gap-1">
              <FieldPicker
                fields={field && !fields.includes(field) ? [field, ...available] : [...(field ? [field] : []), ...available]}
                value={key}
                onChange={(next) => replaceKey(key, next)}
                className="min-w-0 flex-1"
              />
              <Button
                type="button"
                variant={templated ? 'secondary' : 'ghost'}
                size="icon"
                aria-pressed={templated}
                aria-label={t('values.useVariable')}
                title={t('values.useVariable')}
                onClick={() => toggleTemplate(key)}
              >
                <Braces aria-hidden="true" />
              </Button>
              <Button type="button" variant="ghost" size="icon" aria-label={t('values.remove')} onClick={() => remove(key)}>
                <Trash2 aria-hidden="true" />
              </Button>
            </div>
            {templated || !field ? (
              <TemplateInput
                value={typeof values[key] === 'string' ? (values[key] as string) : ''}
                onChange={(next) => setValue(key, next)}
                variables={variables}
                invalid={invalid}
              />
            ) : (
              <FieldInput
                field={field}
                value={values[key] ?? null}
                onChange={(next) => setValue(key, next)}
                invalid={invalid}
                aria-label={field.label}
              />
            )}
          </div>
        )
      })}
      {available.length > 0 && (
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="justify-self-start"
          onClick={() => {
            const first = available[0]
            if (first) setValue(first.key, null)
          }}
        >
          <Plus aria-hidden="true" />
          {t('values.add')}
        </Button>
      )}
    </div>
  )
}
