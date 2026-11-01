import { FolderPlus, Link2, Plus, Trash2, X } from 'lucide-react'
import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { cn } from '@/lib/utils'
import { useFieldSchema, useOrganizationUsers } from '../api'
import { useAggregateSchema, type AggregateRelation } from '../aggregates'
import { valueFieldType } from '../fields'
import { useRecordFormatting } from '../formatting'
import type { FieldDefinition, FieldSchema, FilterOperator, LogicalOperator } from '../types'
import { findExtension, type FilterFieldExtension } from './extensions'
import { FieldPicker } from './FieldPicker'
import {
  MAX_EDITOR_DEPTH,
  MAX_FILTER_CONDITIONS,
  appendDraftNode,
  countConditions,
  emptyCondition,
  emptyGroup,
  emptyRelated,
  filterableFields,
  operandOf,
  removeDraftNode,
  updateDraftNode,
  withField,
  withOperator,
  type DraftCondition,
  type DraftGroup,
  type DraftRelated,
} from './model'
import { NativeSelect } from './NativeSelect'
import { describeNode, operatorLabel } from './summary'
import { ValueInput } from './ValueInput'

export interface FilterBuilderProps {
  schema: FieldSchema
  value: DraftGroup
  onChange: (value: DraftGroup) => void
  extensions?: FilterFieldExtension[]
  allowRelated?: boolean
  className?: string
}

interface EditorContext {
  schema: FieldSchema
  fields: FieldDefinition[]
  canAdd: boolean
  root: DraftGroup
  onChange: (value: DraftGroup) => void
  extensions: FilterFieldExtension[]
  allowRelated: boolean
  relations: AggregateRelation[]
}

const NO_EXTENSIONS: FilterFieldExtension[] = []
const NO_RELATIONS: AggregateRelation[] = []

function hasRelated(group: DraftGroup): boolean {
  return group.children.some((child) => child.kind === 'related' || (child.kind === 'group' && hasRelated(child)))
}

export function FilterBuilder({
  schema,
  value,
  onChange,
  extensions = NO_EXTENSIONS,
  allowRelated = false,
  className,
}: FilterBuilderProps) {
  const { t } = useTranslation('records-table')
  const fields = useMemo(() => filterableFields(schema), [schema])
  const total = countConditions(value)
  const aggregate = useAggregateSchema(allowRelated || hasRelated(value) ? schema.record_type : null)
  const relations = useMemo(
    () => aggregate.data?.relations.filter((relation) => relation.readable) ?? NO_RELATIONS,
    [aggregate.data],
  )
  const context: EditorContext = {
    schema,
    fields,
    canAdd: total < MAX_FILTER_CONDITIONS,
    root: value,
    onChange,
    extensions,
    allowRelated,
    relations,
  }
  return (
    <div className={cn('grid gap-3', className)}>
      <GroupEditor group={value} depth={1} context={context} />
      {!context.canAdd && (
        <p role="status" className="text-xs text-muted-foreground">
          {t('filters.limitReached', { max: MAX_FILTER_CONDITIONS })}
        </p>
      )}
    </div>
  )
}

interface GroupEditorProps {
  group: DraftGroup
  depth: number
  context: EditorContext
  onRemove?: () => void
}

function GroupEditor({ group, depth, context, onRemove }: GroupEditorProps) {
  const { t } = useTranslation('records-table')
  const { root, onChange, canAdd } = context
  const nested = depth > 1
  const setOp = (op: LogicalOperator) =>
    onChange(updateDraftNode(root, group.id, (node) => (node.kind === 'group' ? { ...node, op } : node)))

  return (
    <div
      role="group"
      aria-label={nested ? t('filters.group') : t('filters.title')}
      className={cn('grid gap-2', nested && 'rounded-md border bg-muted/40 p-2 sm:p-3')}
    >
      <div className="flex flex-wrap items-center justify-between gap-2">
        <label className="flex flex-wrap items-center gap-2 text-sm text-muted-foreground">
          <span>{t('filters.logic')}</span>
          <NativeSelect
            className="w-auto"
            value={group.op}
            onChange={(event) => setOp(event.target.value as LogicalOperator)}
          >
            <option value="and">{t('filters.logicAnd')}</option>
            <option value="or">{t('filters.logicOr')}</option>
          </NativeSelect>
        </label>
        {onRemove && (
          <Button variant="ghost" size="sm" onClick={onRemove}>
            <Trash2 aria-hidden />
            {t('filters.removeGroup')}
          </Button>
        )}
      </div>
      {group.children.length === 0 && <p className="text-sm text-muted-foreground">{t('filters.empty')}</p>}
      <ul className="grid gap-2">
        {group.children.map((child, index) => (
          <li key={child.id} className="grid gap-2">
            {index > 0 && (
              <span className="text-xs font-medium tracking-wide text-muted-foreground uppercase">
                {group.op === 'and' ? t('filters.and') : t('filters.or')}
              </span>
            )}
            {child.kind === 'condition' ? (
              <ConditionEditor
                condition={child}
                context={context}
                onRemove={() => onChange(removeDraftNode(root, child.id))}
              />
            ) : child.kind === 'related' ? (
              <RelatedEditor
                node={child}
                context={context}
                onRemove={() => onChange(removeDraftNode(root, child.id))}
              />
            ) : (
              <GroupEditor
                group={child}
                depth={depth + 1}
                context={context}
                onRemove={() => onChange(removeDraftNode(root, child.id))}
              />
            )}
          </li>
        ))}
      </ul>
      <div className="flex flex-wrap gap-2">
        <Button
          variant="outline"
          size="sm"
          disabled={!canAdd}
          onClick={() => onChange(appendDraftNode(root, group.id, emptyCondition()))}
        >
          <Plus aria-hidden />
          {t('filters.addCondition')}
        </Button>
        {depth < MAX_EDITOR_DEPTH && (
          <Button
            variant="ghost"
            size="sm"
            disabled={!canAdd}
            onClick={() => onChange(appendDraftNode(root, group.id, emptyGroup('or', [emptyCondition()])))}
          >
            <FolderPlus aria-hidden />
            {t('filters.addGroup')}
          </Button>
        )}
        {context.allowRelated && context.relations.length > 0 && (
          <Button
            variant="ghost"
            size="sm"
            disabled={!canAdd}
            onClick={() => onChange(appendDraftNode(root, group.id, emptyRelated(context.relations[0].key)))}
          >
            <Link2 aria-hidden />
            {t('filters.addRelated')}
          </Button>
        )}
      </div>
    </div>
  )
}

interface ConditionEditorProps {
  condition: DraftCondition
  context: EditorContext
  onRemove: () => void
}

function ConditionEditor({ condition, context, onRemove }: ConditionEditorProps) {
  const { t } = useTranslation('records-table')
  const { schema, fields, root, onChange, extensions } = context
  const field = fields.find((candidate) => candidate.key === condition.field)
  const replace = (next: DraftCondition) => onChange(updateDraftNode(root, condition.id, () => next))
  const operand = condition.operator ? operandOf(schema, condition.operator) : null
  const extension = field ? findExtension(extensions, field.key) : undefined
  const ExtensionInput = extension?.ValueInput

  return (
    <div className="grid min-w-0 gap-2 rounded-lg border p-3 sm:flex sm:flex-wrap sm:items-start sm:border-0 sm:p-0" data-testid="filter-condition">
      <FieldPicker fields={fields} value={condition.field} onChange={(next) => replace(withField(condition, next, schema))} />
      {field && (
        <NativeSelect
          aria-label={t('filters.operator')}
          className="w-full sm:w-44"
          value={condition.operator ?? ''}
          onChange={(event) => replace(withOperator(condition, event.target.value as FilterOperator, schema))}
        >
          {field.operators.map((operator) => (
            <option key={operator} value={operator}>
              {operatorLabel(t, operator, valueFieldType(field))}
            </option>
          ))}
        </NativeSelect>
      )}
      {field && operand && ExtensionInput && (
        <ExtensionInput
          recordType={schema.record_type}
          value={condition.value}
          label={t('filters.value')}
          onChange={(value) => replace({ ...condition, value })}
        />
      )}
      {field && operand && !ExtensionInput && (
        <ValueInput
          field={field}
          operand={operand}
          value={condition.value}
          label={t('filters.value')}
          onChange={(value) => replace({ ...condition, value })}
        />
      )}
      <Button variant="ghost" size="icon" className="ml-auto size-9" aria-label={t('filters.removeCondition')} onClick={onRemove}>
        <X aria-hidden />
      </Button>
    </div>
  )
}

interface RelatedEditorProps {
  node: DraftRelated
  context: EditorContext
  onRemove: () => void
}

function RelatedEditor({ node, context, onRemove }: RelatedEditorProps) {
  const { t } = useTranslation('records-table')
  const { root, onChange, relations } = context
  const relation = relations.find((candidate) => candidate.key === node.relationship)
  const target = useFieldSchema(relation?.target_record_type ?? '', Boolean(relation))
  const options = relation || !node.relationship ? relations : [...relations, null]
  const replace = (next: DraftRelated) => onChange(updateDraftNode(root, node.id, () => next))

  const changeRelationship = (relationship: string) =>
    replace({ ...emptyRelated(relationship), id: node.id, none: node.none })

  return (
    <div role="group" aria-label={t('filters.related')} className="grid gap-2 rounded-md border border-dashed p-2 sm:p-3">
      <div className="flex flex-wrap items-center gap-2">
        <Link2 className="size-4 text-muted-foreground" aria-hidden />
        <NativeSelect
          aria-label={t('filters.relationship')}
          className="w-full sm:w-56"
          value={node.relationship}
          onChange={(event) => changeRelationship(event.target.value)}
        >
          {options.map((option) =>
            option ? (
              <option key={option.key} value={option.key}>
                {option.label}
              </option>
            ) : (
              <option key={node.relationship} value={node.relationship}>
                {node.relationship}
              </option>
            ),
          )}
        </NativeSelect>
        <label className="flex items-center gap-2 text-sm">
          <Checkbox checked={node.none} onCheckedChange={(checked) => replace({ ...node, none: checked === true })} />
          {t('filters.relatedNone')}
        </label>
        <Button variant="ghost" size="sm" className="ml-auto" onClick={onRemove}>
          <Trash2 aria-hidden />
          {t('filters.removeRelated')}
        </Button>
      </div>
      <p className="text-xs text-muted-foreground">
        {node.none ? t('filters.relatedNoneHint') : t('filters.relatedHint')}
      </p>
      {target.data ? (
        <FilterBuilder
          schema={target.data}
          value={node.filter}
          onChange={(filter) => replace({ ...node, filter, schema: target.data })}
        />
      ) : relation && target.isPending ? (
        <p className="text-sm text-muted-foreground">{t('filters.relatedLoading')}</p>
      ) : (
        <RelatedSummary node={node} />
      )}
    </div>
  )
}

function RelatedSummary({ node }: { node: DraftRelated }) {
  const { t } = useTranslation('records-table')
  const formatting = useRecordFormatting()
  const users = useOrganizationUsers()
  const filter = node.original?.related.filter
  if (!filter) return <p className="text-sm text-muted-foreground">{t('filters.relatedUnavailable')}</p>
  const summary = describeNode(filter, {
    schema: { record_type: '', fields: [], operators: [] },
    t,
    formatting,
    users: users.data ?? [],
  })
  return (
    <p className="text-sm text-muted-foreground">
      {t('filters.relatedReadOnly', { summary })}
    </p>
  )
}
