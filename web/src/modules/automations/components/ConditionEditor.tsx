import { Link2, Plus, Trash2 } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'
import { type DraftGroup, type FieldSchema, FilterBuilder, type FilterNode, type FilterRelated, isFilterGroup, isFilterRelated, NativeSelect, serializeFilter, toDraft, useAggregateSchema, useFieldSchema } from '@/sdk'
import { useConditionSchema } from '../lib/condition-schema'

const EMPTY_FILTER: FilterNode = { op: 'and', conditions: [] }

const ACTIVITY_HOSTS = new Set(['people.person', 'organizations.organization', 'deals.deal'])

interface RelationChoice {
  key: string
  label: string
  target: string
}

interface RelatedDraft {
  uid: string
  relationship: string
  none: boolean
  filter: FilterNode | null
}

let relatedSequence = 0

function relatedUid() {
  relatedSequence += 1
  return `related-${relatedSequence}`
}

function split(node: FilterNode | null | undefined): { base: FilterNode | null; related: RelatedDraft[] } {
  if (!node) return { base: null, related: [] }
  if (isFilterRelated(node)) return { base: null, related: [toRelatedDraft(node)] }
  if (!isFilterGroup(node)) return { base: node, related: [] }
  const related = node.conditions.filter(isFilterRelated).map(toRelatedDraft)
  return { base: { ...node, conditions: node.conditions.filter((child) => !isFilterRelated(child)) }, related }
}

function toRelatedDraft(node: FilterRelated): RelatedDraft {
  return {
    uid: relatedUid(),
    relationship: node.related.relationship,
    none: Boolean(node.related.none),
    filter: node.related.filter ?? null,
  }
}

function relatedNode(draft: RelatedDraft): FilterRelated {
  const related: FilterRelated['related'] = { relationship: draft.relationship, filter: draft.filter ?? EMPTY_FILTER }
  if (draft.none) related.none = true
  return { related }
}

function useRelationChoices(recordType: string): { choices: RelationChoice[]; loading: boolean } {
  const { t } = useTranslation('automations')
  const aggregate = useAggregateSchema(recordType.startsWith('custom.') || ACTIVITY_HOSTS.has(recordType) ? recordType : null)
  return useMemo(() => {
    const choices: RelationChoice[] = (aggregate.data?.relations ?? [])
      .filter((relation) => relation.readable)
      .map((relation) => ({ key: relation.key, label: relation.label, target: relation.target_record_type }))
    if (ACTIVITY_HOSTS.has(recordType) && !choices.some((choice) => choice.key === 'tasks')) {
      choices.push({ key: 'tasks', label: t('conditions.activitiesRelation'), target: 'tasks.task' })
    }
    return { choices, loading: aggregate.isPending && aggregate.fetchStatus !== 'idle' }
  }, [aggregate.data, aggregate.fetchStatus, aggregate.isPending, recordType, t])
}

export interface ConditionEditorProps {
  recordType: string
  schema: FieldSchema
  value: FilterNode | null | undefined
  onChange: (value: FilterNode) => void
  includeTrigger?: boolean
  allowRelated?: boolean
}

export function ConditionEditor({
  recordType,
  schema,
  value,
  onChange,
  includeTrigger = true,
  allowRelated = true,
}: ConditionEditorProps) {
  const { t } = useTranslation('automations')
  const conditionSchema = useConditionSchema(schema, { includeTrigger }) ?? schema
  const [initial] = useState(() => split(value))
  const [draft, setDraft] = useState<DraftGroup>(() => toDraft(initial.base))
  const [related, setRelated] = useState<RelatedDraft[]>(initial.related)
  const relations = useRelationChoices(recordType)

  const emit = (nextDraft: DraftGroup, nextRelated: RelatedDraft[]) => {
    const base = serializeFilter(nextDraft, conditionSchema)
    const conditions = [
      ...(base && isFilterGroup(base) ? base.conditions : base ? [base] : []),
      ...nextRelated.filter((item) => item.relationship).map(relatedNode),
    ]
    onChange({ op: nextDraft.op, conditions })
  }

  const changeDraft = (next: DraftGroup) => {
    setDraft(next)
    emit(next, related)
  }

  const changeRelated = (next: RelatedDraft[]) => {
    setRelated(next)
    emit(draft, next)
  }

  return (
    <div className="grid gap-4">
      <FilterBuilder schema={conditionSchema} value={draft} onChange={changeDraft} />
      {allowRelated && (relations.choices.length > 0 || related.length > 0) && (
        <section className="grid gap-2" aria-label={t('conditions.relatedTitle')}>
          <div className="flex flex-wrap items-center justify-between gap-2">
            <h4 className="text-sm font-medium">{t('conditions.relatedTitle')}</h4>
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={relations.choices.length === 0}
              onClick={() =>
                changeRelated([
                  ...related,
                  { uid: relatedUid(), relationship: relations.choices[0]?.key ?? '', none: true, filter: null },
                ])
              }
            >
              <Plus aria-hidden="true" />
              {t('conditions.addRelated')}
            </Button>
          </div>
          {related.length === 0 ? (
            <p className="text-xs text-muted-foreground">{t('conditions.relatedHint')}</p>
          ) : (
            related.map((item) => (
              <RelatedConditionEditor
                key={item.uid}
                value={item}
                choices={relations.choices}
                onChange={(next) => changeRelated(related.map((entry) => (entry.uid === item.uid ? next : entry)))}
                onRemove={() => changeRelated(related.filter((entry) => entry.uid !== item.uid))}
              />
            ))
          )}
        </section>
      )}
    </div>
  )
}

interface RelatedConditionEditorProps {
  value: RelatedDraft
  choices: RelationChoice[]
  onChange: (value: RelatedDraft) => void
  onRemove: () => void
}

function RelatedConditionEditor({ value, choices, onChange, onRemove }: RelatedConditionEditorProps) {
  const { t } = useTranslation('automations')
  const choice = choices.find((item) => item.key === value.relationship)
  const known = choice ?? (value.relationship ? { key: value.relationship, label: value.relationship, target: '' } : undefined)

  return (
    <div className="grid gap-3 rounded-md border bg-muted/40 p-3">
      <div className="flex flex-wrap items-center gap-2">
        <Link2 className="size-4 text-muted-foreground" aria-hidden="true" />
        <NativeSelect
          value={value.none ? 'none' : 'exists'}
          onChange={(event) => onChange({ ...value, none: event.target.value === 'none' })}
          aria-label={t('conditions.relatedMode')}
          className="w-44"
        >
          <option value="none">{t('conditions.relatedNone')}</option>
          <option value="exists">{t('conditions.relatedExists')}</option>
        </NativeSelect>
        <NativeSelect
          value={value.relationship}
          onChange={(event) => onChange({ ...value, relationship: event.target.value, filter: null })}
          aria-label={t('conditions.relationship')}
          className="min-w-40 flex-1"
        >
          {!choice && known && <option value={known.key}>{known.label}</option>}
          {choices.map((item) => (
            <option key={item.key} value={item.key}>
              {item.label}
            </option>
          ))}
        </NativeSelect>
        <Button type="button" variant="ghost" size="icon" aria-label={t('conditions.removeRelated')} onClick={onRemove}>
          <Trash2 aria-hidden="true" />
        </Button>
      </div>
      {choice?.target ? (
        <RelatedFilter key={`${value.relationship}`} target={choice.target} value={value.filter} onChange={(filter) => onChange({ ...value, filter })} />
      ) : null}
    </div>
  )
}

interface RelatedFilterProps {
  target: string
  value: FilterNode | null
  onChange: (value: FilterNode | null) => void
}

function RelatedFilter({ target, value, onChange }: RelatedFilterProps) {
  const schema = useFieldSchema(target)
  if (schema.isPending) return <Skeleton className="h-9 w-full" />
  if (!schema.data) return null
  return <RelatedFilterBuilder schema={schema.data} value={value} onChange={onChange} />
}

function RelatedFilterBuilder({
  schema,
  value,
  onChange,
}: {
  schema: FieldSchema
  value: FilterNode | null
  onChange: (value: FilterNode | null) => void
}) {
  const { t } = useTranslation('automations')
  const [draft, setDraft] = useState<DraftGroup>(() => toDraft(value))
  return (
    <div className="grid gap-1">
      <p className="text-xs text-muted-foreground">{t('conditions.relatedFilter')}</p>
      <FilterBuilder
        schema={schema}
        value={draft}
        onChange={(next) => {
          setDraft(next)
          onChange(serializeFilter(next, schema))
        }}
      />
    </div>
  )
}
