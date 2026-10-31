import { Plus, Trash2 } from 'lucide-react'
import { useId, useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { useFieldSchema } from '@/features/records/api'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import type { FieldSchema, FieldType, FilterNode } from '@/features/records/types'
import { issuesUnder, type ValidationIssue } from '../lib/errors'
import { useAutomationLabels } from '../lib/labels'
import type { TemplateVariable } from '../lib/variables'
import type { JsonObject, JsonSchema } from '../types'
import { ConditionEditor } from './ConditionEditor'
import { DurationInput } from './DurationInput'
import { FieldValuesEditor } from './FieldValuesEditor'
import { IssueList } from './IssueList'
import {
  ActivityTypeSelect,
  FieldSelect,
  IncomingWebhookSelect,
  LeadRuleSelect,
  RecordTypeSelect,
  RelationshipSelect,
  SegmentSelect,
  SequenceSelect,
  StageSelect,
  TagSelect,
} from './ReferenceSelects'
import { TemplateInput } from './TemplateInput'
import { UserSpecInput } from './UserSpecInput'

const DURATION_NAMES = new Set(['due_in', 'duration', 'offset', 'timeout'])
const SIGNED_DURATIONS = new Set(['offset'])
const STAGE_NAMES = new Set(['stage_id', 'to_stage_id', 'from_stage_id'])
const MULTILINE_NAMES = new Set(['description', 'body', 'message'])
const PROPERTY_ORDER = [
  'title',
  'subject',
  'recipients',
  'body',
  'description',
  'assignee',
  'owner',
  'due_in',
  'due_at',
  'priority',
  'activity_type_id',
  'link_to_record',
  'field',
  'offset',
  'at',
  'mode',
  'days',
  'cron',
  'filter',
  'segment_id',
  'from_stage_id',
  'to_stage_id',
]

function orderOf(name: string): number {
  const index = PROPERTY_ORDER.indexOf(name)
  return index === -1 ? PROPERTY_ORDER.length : index
}

const CRON_PRESETS = ['0 0 9 * * *', '0 0 9 * * MON-FRI', '0 0 9 * * MON', '0 0 9 1 * *']

export type SchemaFormMode = 'trigger' | 'action'

export interface SchemaFormContext {
  mode: SchemaFormMode
  recordType: string
  schema: FieldSchema | undefined
  variables: TemplateVariable[]
  recordTypes: string[]
  issues: ValidationIssue[]
}

export interface SchemaFormProps {
  jsonSchema: JsonSchema
  value: JsonObject
  onChange: (value: JsonObject) => void
  context: SchemaFormContext
}

function isUserSpec(schema: JsonSchema | undefined): boolean {
  return Boolean(
    schema?.oneOf?.some((option) => {
      const type = option.properties?.type
      return type?.const === 'record_owner' || type?.const === 'user'
    }),
  )
}

function typeOf(schema: JsonSchema): string | undefined {
  if (Array.isArray(schema.type)) return schema.type.find((type) => type !== 'null')
  if (schema.type) return schema.type
  if (schema.enum) return 'enum'
  return undefined
}

export function SchemaForm({ jsonSchema, value, onChange, context }: SchemaFormProps) {
  const { t } = useTranslation('automations')
  const properties = Object.entries(jsonSchema.properties ?? {}).filter(([, property]) => !property.readOnly)
    .sort(([left], [right]) => orderOf(left) - orderOf(right))
  if (properties.length === 0) {
    return <p className="text-sm text-muted-foreground">{t('form.noOptions')}</p>
  }
  const required = new Set(jsonSchema.required ?? [])
  const set = (name: string, next: unknown) => {
    const copy = { ...value }
    if (next === undefined) delete copy[name]
    else copy[name] = next
    onChange(copy)
  }

  return (
    <div className="grid gap-4">
      {properties.map(([name, property]) => (
        <SchemaProperty
          key={name}
          name={name}
          property={property}
          required={required.has(name)}
          value={value[name]}
          values={value}
          onChange={(next) => set(name, next)}
          context={context}
        />
      ))}
    </div>
  )
}

interface SchemaPropertyProps {
  name: string
  property: JsonSchema
  required: boolean
  value: unknown
  values: JsonObject
  onChange: (value: unknown) => void
  context: SchemaFormContext
}

function SchemaProperty({ name, property, required, value, values, onChange, context }: SchemaPropertyProps) {
  const { t } = useTranslation('automations')
  const labels = useAutomationLabels()
  const id = useId()
  const issues = issuesUnder(context.issues, name)
  const invalid = issues.length > 0
  const describedBy = invalid ? `${id}-issues` : undefined
  const type = typeOf(property)
  const label = labels.propertyLabel(name, property.title)
  const hint = t(`hints.${name.replace(/[.:]/g, '_')}` as 'hints.due_in', { defaultValue: '' })

  if (type === 'boolean') {
    return (
      <div className="grid gap-1">
        <div className="flex items-center gap-2">
          <Checkbox
            id={id}
            checked={value === undefined ? property.default === true : value === true}
            onCheckedChange={(checked) => onChange(checked === true)}
            aria-describedby={describedBy}
          />
          <Label htmlFor={id}>{label}</Label>
        </div>
        {hint && <p className="text-xs text-muted-foreground">{hint}</p>}
        <IssueList id={describedBy} issues={issues} />
      </div>
    )
  }

  let control: ReactNode
  const base = { id, invalid, 'aria-describedby': describedBy }
  const text = typeof value === 'string' ? value : ''

  if (property.writeOnly) {
    control = <SecretInput {...base} configured={values[`${name}_configured`] === true} onChange={onChange} />
  } else if (isUserSpec(property)) {
    control = <UserSpecInput id={id} value={value} onChange={onChange} schema={context.schema} invalid={invalid} />
  } else if (type === 'array' && isUserSpec(property.items)) {
    control = <UserSpecList value={value} onChange={onChange} schema={context.schema} max={property.maxItems} />
  } else if (name === 'values') {
    const target = typeof values.record_type === 'string' && values.record_type ? values.record_type : context.recordType
    control = (
      <FieldValuesEditor
        key={target}
        recordType={target}
        value={value}
        onChange={onChange}
        variables={context.variables}
        invalidKeys={issues.map((issue) => issue.path.split('.')[0] ?? '').filter(Boolean)}
      />
    )
  } else if (name === 'filter' && context.schema) {
    control = (
      <ConditionEditor
        recordType={context.recordType}
        schema={context.schema}
        value={(value as FilterNode | null) ?? null}
        onChange={onChange}
        includeTrigger={false}
      />
    )
  } else if (STAGE_NAMES.has(name)) {
    control = <StageSelect {...base} value={value} onChange={(next) => onChange(next ?? undefined)} optional={!required} />
  } else if (name === 'tag_id') {
    control = <TagSelect {...base} recordType={context.recordType} value={value} onChange={(next) => onChange(next ?? undefined)} />
  } else if (name === 'activity_type_id') {
    control = <ActivityTypeSelect {...base} value={value} onChange={(next) => onChange(next ?? undefined)} />
  } else if (name === 'incoming_webhook_id') {
    control = <IncomingWebhookSelect {...base} value={value} onChange={(next) => onChange(next ?? undefined)} />
  } else if (name === 'segment_id') {
    control = (
      <SegmentSelect {...base} recordType={context.recordType} value={value} onChange={(next) => onChange(next ?? undefined)} optional />
    )
  } else if (name === 'record_type') {
    control = (
      <RecordTypeSelect
        {...base}
        recordTypes={context.recordTypes}
        labelOf={labels.recordTypeLabel}
        value={value}
        onChange={(next) => onChange(next ?? undefined)}
      />
    )
  } else if (name === 'relationship') {
    control = (
      <RelationshipSelect {...base} recordType={context.recordType} value={value} onChange={(next) => onChange(next ?? undefined)} optional />
    )
  } else if (name === 'sequence_id') {
    control = <SequenceSelect {...base} value={value} onChange={(next) => onChange(next ?? undefined)} />
  } else if (name === 'rule_id') {
    control = (
      <LeadRuleSelect {...base} recordType={context.recordType} value={value} onChange={(next) => onChange(next ?? undefined)} />
    )
  } else if (name === 'field' || name === 'until_field') {
    const hinted = property['x-field-types']
    control =
      hinted || context.mode === 'trigger' ? (
        <FieldSelect
          {...base}
          recordType={context.recordType}
          fieldTypes={(hinted as FieldType[] | undefined) ?? ['date', 'datetime']}
          value={value}
          onChange={(next) => onChange(next ?? undefined)}
        />
      ) : (
        <FieldSelect
          {...base}
          recordType={context.recordType}
          fieldTypes={['user']}
          value={value}
          onChange={(next) => onChange(next ?? undefined)}
          optional
        />
      )
  } else if (name === 'fields' && type === 'array') {
    control = <FieldChecklist recordType={context.recordType} value={value} onChange={onChange} />
  } else if (DURATION_NAMES.has(name)) {
    control = (
      <DurationInput
        {...base}
        value={text}
        signed={SIGNED_DURATIONS.has(name)}
        onChange={(next) => onChange(next ?? undefined)}
      />
    )
  } else if (name === 'cron') {
    control = <CronInput {...base} value={text} onChange={(next) => onChange(next || undefined)} />
  } else if (name === 'at') {
    control = (
      <Input
        id={id}
        type="time"
        value={text}
        onChange={(event) => onChange(event.target.value || undefined)}
        aria-invalid={invalid ? true : undefined}
        aria-describedby={describedBy}
        className="w-36"
      />
    )
  } else if (type === 'enum' || property.enum) {
    control = (
      <NativeSelect
        id={id}
        value={value === undefined || value === null ? '' : String(value)}
        onChange={(event) => onChange(event.target.value === '' ? undefined : event.target.value)}
        aria-invalid={invalid ? true : undefined}
        aria-describedby={describedBy}
        className="w-full"
      >
        <option value="" disabled={required}>
          {required ? t('form.choose') : t('form.default')}
        </option>
        {(property.enum ?? []).map((option) => (
          <option key={String(option)} value={String(option)}>
            {labels.enumLabel(name, option)}
          </option>
        ))}
      </NativeSelect>
    )
  } else if (type === 'integer' || type === 'number') {
    control = (
      <Input placeholder={t('placeholders.number')}
        id={id}
        type="number"
        inputMode="numeric"
        min={property.minimum}
        max={property.maximum}
        value={typeof value === 'number' ? String(value) : ''}
        onChange={(event) => {
          const raw = event.target.value
          if (raw === '') onChange(undefined)
          else onChange(type === 'integer' ? Math.trunc(Number(raw)) : Number(raw))
        }}
        aria-invalid={invalid ? true : undefined}
        aria-describedby={describedBy}
        className="w-36"
      />
    )
  } else if (type === undefined || type === 'object' || type === 'array') {
    control = <JsonOrTextInput {...base} value={value} onChange={onChange} variables={context.variables} />
  } else {
    control = (
      <TemplateInput
        {...base}
        value={text}
        onChange={(next) => onChange(next === '' ? undefined : next)}
        variables={context.variables}
        multiline={MULTILINE_NAMES.has(name)}
        maxLength={property.maxLength}
        placeholder={typeof property.examples?.[0] === 'string' ? property.examples[0] : undefined}
      />
    )
  }

  return (
    <div className="grid min-w-0 gap-1.5">
      <Label htmlFor={id}>
        {label}
        {required && <span aria-hidden="true" className="text-destructive"> *</span>}
      </Label>
      {control}
      {hint && <p className="text-xs text-muted-foreground">{hint}</p>}
      <IssueList id={describedBy} issues={issues} />
    </div>
  )
}

function SecretInput({
  id,
  configured,
  onChange,
  invalid,
  'aria-describedby': describedBy,
}: {
  id: string
  configured: boolean
  onChange: (value: unknown) => void
  invalid: boolean
  'aria-describedby'?: string
}) {
  const { t } = useTranslation('automations')
  const [text, setText] = useState('')
  return (
    <div className="grid gap-1">
      <div className="flex min-w-0 gap-1">
        <Input
          id={id}
          type="password"
          autoComplete="new-password"
          value={text}
          placeholder={configured ? t('form.secretConfigured') : t('form.secretEmpty')}
          onChange={(event) => {
            setText(event.target.value)
            onChange(event.target.value === '' ? undefined : event.target.value)
          }}
          aria-invalid={invalid ? true : undefined}
          aria-describedby={describedBy}
          className="min-w-0 flex-1"
        />
        {configured && (
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="h-9"
            onClick={() => {
              setText('')
              onChange(null)
            }}
          >
            {t('form.secretRemove')}
          </Button>
        )}
      </div>
    </div>
  )
}

function UserSpecList({
  value,
  onChange,
  schema,
  max,
}: {
  value: unknown
  onChange: (value: unknown) => void
  schema: FieldSchema | undefined
  max?: number
}) {
  const { t } = useTranslation('automations')
  const items = Array.isArray(value) ? value : []
  const hasOwner = Boolean(schema?.fields.some((field) => field.key === 'owner_id'))
  return (
    <div className="grid gap-2">
      {items.map((item, index) => (
        <div key={index} className="flex min-w-0 items-start gap-1">
          <div className="min-w-0 flex-1">
            <UserSpecInput
              value={item}
              schema={schema}
              onChange={(next) => onChange(items.map((current, position) => (position === index ? next : current)))}
            />
          </div>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            aria-label={t('userSpec.remove')}
            onClick={() => onChange(items.filter((_, position) => position !== index))}
          >
            <Trash2 aria-hidden="true" />
          </Button>
        </div>
      ))}
      {(max === undefined || items.length < max) && (
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="justify-self-start"
          onClick={() => onChange([...items, hasOwner ? { type: 'record_owner' } : { type: 'user', user_id: '' }])}
        >
          <Plus aria-hidden="true" />
          {t('userSpec.add')}
        </Button>
      )}
    </div>
  )
}

function FieldChecklist({
  recordType,
  value,
  onChange,
}: {
  recordType: string
  value: unknown
  onChange: (value: unknown) => void
}) {
  const { t } = useTranslation('automations')
  const selected = Array.isArray(value) ? value.filter((item): item is string => typeof item === 'string') : []
  const [adding, setAdding] = useState('')
  return (
    <div className="grid gap-2">
      {selected.length === 0 && <p className="text-xs text-muted-foreground">{t('form.anyField')}</p>}
      <div className="flex flex-wrap gap-1">
        {selected.map((key) => (
          <span key={key} className="inline-flex items-center gap-1 rounded-md border bg-muted px-2 py-0.5 text-xs">
            <FieldName recordType={recordType} fieldKey={key} />
            <button
              type="button"
              aria-label={t('form.removeField')}
              className="text-muted-foreground hover:text-foreground"
              onClick={() => {
                const next = selected.filter((item) => item !== key)
                onChange(next.length > 0 ? next : undefined)
              }}
            >
              <Trash2 className="size-3" aria-hidden="true" />
            </button>
          </span>
        ))}
      </div>
      <FieldSelect
        recordType={recordType}
        value={adding}
        filter={(field) => !selected.includes(field.key)}
        onChange={(next) => {
          setAdding('')
          if (next) onChange([...selected, next])
        }}
        optional
      />
    </div>
  )
}

function FieldName({ recordType, fieldKey }: { recordType: string; fieldKey: string }) {
  const schema = useFieldSchema(recordType)
  return <>{schema.data?.fields.find((field) => field.key === fieldKey)?.label ?? fieldKey}</>
}

function CronInput({
  id,
  value,
  onChange,
  invalid,
  'aria-describedby': describedBy,
}: {
  id: string
  value: string
  onChange: (value: string) => void
  invalid: boolean
  'aria-describedby'?: string
}) {
  const { t } = useTranslation('automations')
  return (
    <div className="flex min-w-0 flex-wrap gap-2">
      <Input
        id={id}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder="0 0 9 * * MON"
        aria-invalid={invalid ? true : undefined}
        aria-describedby={describedBy}
        className="min-w-0 flex-1 font-mono"
      />
      <NativeSelect
        value={CRON_PRESETS.includes(value) ? value : ''}
        onChange={(event) => event.target.value && onChange(event.target.value)}
        aria-label={t('cron.presets')}
        className="w-full sm:w-56"
      >
        <option value="">{t('cron.custom')}</option>
        {CRON_PRESETS.map((preset, index) => (
          <option key={preset} value={preset}>
            {t(`cron.preset${index}` as 'cron.preset0')}
          </option>
        ))}
      </NativeSelect>
    </div>
  )
}

function JsonOrTextInput({
  id,
  value,
  onChange,
  variables,
  invalid,
  'aria-describedby': describedBy,
}: {
  id: string
  value: unknown
  onChange: (value: unknown) => void
  variables: TemplateVariable[]
  invalid: boolean
  'aria-describedby'?: string
}) {
  const [text, setText] = useState(() => {
    if (value === undefined || value === null) return ''
    if (typeof value === 'string') return value
    return JSON.stringify(value, null, 2)
  })
  const parse = (raw: string): unknown => {
    const trimmed = raw.trim()
    if (trimmed === '') return undefined
    if (trimmed.startsWith('{') || trimmed.startsWith('[')) {
      try {
        return JSON.parse(trimmed)
      } catch {
        return raw
      }
    }
    return raw
  }
  return (
    <TemplateInput
      id={id}
      value={text}
      onChange={(next) => {
        setText(next)
        onChange(parse(next))
      }}
      variables={variables}
      multiline
      invalid={invalid}
      aria-describedby={describedBy}
    />
  )
}
