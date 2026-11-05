import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { api } from '@/api'
import { activeFields, type FieldDefinition, type FieldType, NativeSelect, useAggregateSchema, useFieldSchema } from '@/sdk'
import type { ReferenceOption, ReferenceOptionsContext } from '@/kernel/modules/define'
import { ReferenceOptions } from '@/kernel/modules/ReferenceOptions'

export interface ReferenceSelectProps {
  id?: string
  value: unknown
  onChange: (value: string | null) => void
  invalid?: boolean
  optional?: boolean
  'aria-describedby'?: string
}

function textValue(value: unknown): string {
  return typeof value === 'string' ? value : ''
}

function Placeholder({ optional }: { optional?: boolean }) {
  const { t } = useTranslation('automations')
  return (
    <option value="" disabled={!optional}>
      {optional ? t('form.any') : t('form.choose')}
    </option>
  )
}

function selectProps(props: ReferenceSelectProps) {
  return {
    id: props.id,
    value: textValue(props.value),
    onChange: (event: { target: { value: string } }) => props.onChange(event.target.value || null),
    'aria-invalid': props.invalid ? true : undefined,
    'aria-describedby': props['aria-describedby'],
    className: 'w-full',
  }
}

function groupOptions(options: ReferenceOption[]): [string | undefined, ReferenceOption[]][] {
  const groups = new Map<string | undefined, ReferenceOption[]>()
  for (const option of options) groups.set(option.group, [...(groups.get(option.group) ?? []), option])
  return [...groups.entries()]
}

function OptionList({ options }: { options: ReferenceOption[] }) {
  return groupOptions(options).map(([group, items]) => {
    const rendered = items.map((option) => (
      <option key={option.value} value={option.value}>
        {option.label}
      </option>
    ))
    return group === undefined ? (
      rendered
    ) : (
      <optgroup key={group} label={group}>
        {rendered}
      </optgroup>
    )
  })
}

function ReferenceSelect({
  kind,
  context,
  forceOptional,
  ...props
}: ReferenceSelectProps & { kind: string; context?: ReferenceOptionsContext; forceOptional?: boolean }) {
  return (
    <ReferenceOptions kind={kind} context={context}>
      {({ options }) => (
        <NativeSelect {...selectProps(props)}>
          <Placeholder optional={forceOptional || props.optional} />
          <OptionList options={options} />
        </NativeSelect>
      )}
    </ReferenceOptions>
  )
}

const INACTIVE_CONTEXT: ReferenceOptionsContext = { includeInactive: true }

export function StageSelect(props: ReferenceSelectProps) {
  return <ReferenceSelect kind="pipeline_stage" context={INACTIVE_CONTEXT} {...props} />
}


export function TagSelect({ recordType, ...props }: ReferenceSelectProps & { recordType: string }) {
  return <ReferenceSelect kind="tag" context={{ recordType }} {...props} />
}


export function ActivityTypeSelect(props: ReferenceSelectProps) {
  return <ReferenceSelect kind="activity_type" forceOptional {...props} />
}


interface IncomingWebhookSummary {
  id: string
  name: string
}

export function IncomingWebhookSelect(props: ReferenceSelectProps) {
  const webhooks = useQuery({
    queryKey: ['automations', 'incoming-webhooks'],
    queryFn: () =>
      api.get<{ data: IncomingWebhookSummary[] }>('/v1/incoming-webhooks', { query: { per_page: 100 } }),
    staleTime: 60_000,
  })
  const items = webhooks.data?.data ?? []
  return (
    <NativeSelect {...selectProps(props)}>
      <Placeholder optional={props.optional} />
      {items.map((webhook) => (
        <option key={webhook.id} value={webhook.id}>
          {webhook.name}
        </option>
      ))}
    </NativeSelect>
  )
}

export function SegmentSelect({ recordType, ...props }: ReferenceSelectProps & { recordType: string }) {
  return <ReferenceSelect kind="segment" context={{ recordType }} {...props} />
}


export function RelationshipSelect(props: ReferenceSelectProps & { recordType: string }) {
  const aggregate = useAggregateSchema(props.recordType)
  return (
    <NativeSelect {...selectProps(props)}>
      <Placeholder optional={props.optional} />
      {(aggregate.data?.relations ?? []).map((relation) => (
        <option key={relation.key} value={relation.key}>
          {relation.label}
        </option>
      ))}
    </NativeSelect>
  )
}

export function RecordTypeSelect(
  props: ReferenceSelectProps & { recordTypes: string[]; labelOf: (recordType: string) => string },
) {
  return (
    <NativeSelect {...selectProps(props)}>
      <Placeholder optional={props.optional} />
      {props.recordTypes.map((recordType) => (
        <option key={recordType} value={recordType}>
          {props.labelOf(recordType)}
        </option>
      ))}
    </NativeSelect>
  )
}

export function FieldSelect(
  props: ReferenceSelectProps & { recordType: string; fieldTypes?: FieldType[]; filter?: (field: FieldDefinition) => boolean },
) {
  const schema = useFieldSchema(props.recordType)
  const fields = schema.data
    ? activeFields(schema.data).filter(
        (field) =>
          (!props.fieldTypes || props.fieldTypes.includes(field.field_type)) && (!props.filter || props.filter(field)),
      )
    : []
  return (
    <NativeSelect {...selectProps(props)}>
      <Placeholder optional={props.optional} />
      {fields.map((field) => (
        <option key={field.key} value={field.key}>
          {field.label}
        </option>
      ))}
    </NativeSelect>
  )
}

interface LeadRuleSummary {
  id: string
  name: string
  is_active?: boolean
}

export function LeadRuleSelect(props: ReferenceSelectProps & { recordType: string }) {
  const rules = useQuery({
    queryKey: ['automations', 'assignment-rules', props.recordType],
    queryFn: () =>
      api.get<{ data: LeadRuleSummary[] } | LeadRuleSummary[]>('/v1/assignment/rules', {
        query: { record_type: props.recordType, per_page: 100 },
      }),
    staleTime: 60_000,
  })
  const items = Array.isArray(rules.data) ? rules.data : (rules.data?.data ?? [])
  return (
    <NativeSelect {...selectProps(props)}>
      <Placeholder optional={props.optional} />
      {items.map((rule) => (
        <option key={rule.id} value={rule.id}>
          {rule.name}
        </option>
      ))}
    </NativeSelect>
  )
}

export function SequenceSelect(props: ReferenceSelectProps) {
  return <ReferenceSelect kind="sequence" {...props} />
}

