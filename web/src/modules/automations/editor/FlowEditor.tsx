import {
  AlertCircle,
  ArrowDownToLine,
  CornerDownRight,
  Filter,
  GitBranch,
  Hourglass,
  Plus,
  Sparkles as ActionIcon,
  Timer,
  X,
  Zap,
  type LucideIcon,
} from 'lucide-react'
import type { TFunction } from 'i18next'
import { useLayoutEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { type FieldSchema, type FilterNode } from '@/sdk'
import type { ReferenceOptionsContext } from '@/kernel/modules/define'
import { useReferenceOptions } from '@/kernel/modules/reference-options'
import { cn } from '@/lib/utils'
import type { ValidationReport } from '../lib/errors'
import { DEFAULT_SLOT, MATCHED_SLOT, TIMEOUT_SLOT, type FlowNode, type InsertionPoint, type SequencePath, type StepNode } from '../lib/flow'
import { supportsRecordType, useAutomationLabels } from '../lib/labels'
import type { ActionCatalogEntry, AutomationCatalog, JsonSchema, StepSpec, StepType, TriggerSpec } from '../types'
import type { NewStepKind } from './steps'
import { useDurationText, useFilterText, useStepSummary } from './summary'

export type FlowSelection = { kind: 'trigger' } | { kind: 'step'; id: string }

const STEP_ICONS: Record<StepType, LucideIcon> = {
  condition: Filter,
  branch: GitBranch,
  action: ActionIcon,
  wait: Timer,
  wait_until: Hourglass,
}

export interface FlowEditorProps {
  recordType: string
  trigger: TriggerSpec
  nodes: FlowNode[]
  catalog: AutomationCatalog
  schema: FieldSchema | undefined
  report: ValidationReport
  selection: FlowSelection | null
  readOnly?: boolean
  onSelect: (selection: FlowSelection) => void
  onInsert: (point: InsertionPoint, kind: NewStepKind) => void
  onRemoveJump: (uid: string) => void
}

interface FlowContext extends FlowEditorProps {
  nested?: boolean
  summary: ReturnType<typeof useStepSummary>
  actions: ActionCatalogEntry[]
  stepTitle: (id: string) => string
}

const TRIGGER_SUMMARY_LIMIT = 4
const DURATION_OPTIONS = new Set(['due_in', 'duration', 'offset', 'timeout'])
const SIGNED_DURATION_OPTIONS = new Set(['offset'])

function isFilterValue(value: unknown): value is FilterNode {
  return (
    typeof value === 'object' &&
    value !== null &&
    !Array.isArray(value) &&
    ('conditions' in value || 'related' in value || ('field' in value && 'operator' in value))
  )
}

function isBlankOption(value: unknown): boolean {
  if (value === null || value === undefined || value === '') return true
  if (Array.isArray(value)) return value.length === 0
  return false
}

const ALL_STAGES: ReferenceOptionsContext = { includeInactive: true }

function useTriggerSummary(trigger: TriggerSpec, catalog: AutomationCatalog, schema: FieldSchema | undefined) {
  const { t } = useTranslation('records-table')
  const labels = useAutomationLabels(catalog)
  const durationText = useDurationText()
  const filterText = useFilterText(schema)
  const stages = useReferenceOptions('pipeline_stage', ALL_STAGES)
  const entry = catalog.triggers.find((item) => item.key === trigger.type)
  const properties: Record<string, JsonSchema> = entry?.options_schema.properties ?? {}
  const stageName = (id: string) => stages.options.find((option) => option.value === id)?.label
  const fieldName = (key: string) => schema?.fields.find((field) => field.key === key)?.label

  const describe = (name: string, value: unknown): string => {
    if (Array.isArray(value)) return value.map((item) => describe(name, item)).join(', ')
    if (typeof value === 'boolean') return value ? t('filters.yes') : t('filters.no')
    if (isFilterValue(value)) return filterText(value)
    if (typeof value === 'string') {
      if (DURATION_OPTIONS.has(name)) return durationText(value, SIGNED_DURATION_OPTIONS.has(name))
      if (properties[name]?.enum || properties[name]?.items?.enum) return labels.enumLabel(name, value)
      return stageName(value) ?? fieldName(value) ?? value
    }
    if (typeof value === 'number') return String(value)
    return ''
  }

  const parts: string[] = []
  for (const [name, value] of Object.entries(trigger)) {
    if (name === 'type' || name.endsWith('_configured') || isBlankOption(value)) continue
    const property = properties[name]
    if (property?.writeOnly || property?.readOnly) continue
    const text = describe(name, value)
    if (text) parts.push(`${labels.propertyLabel(name)}: ${text}`)
  }
  return parts.length > TRIGGER_SUMMARY_LIMIT
    ? [...parts.slice(0, TRIGGER_SUMMARY_LIMIT), `+${parts.length - TRIGGER_SUMMARY_LIMIT}`]
    : parts
}

function shapeOf(node: FlowNode): unknown {
  if (node.kind === 'jump') return node.target
  return [node.step.id, node.children.map((child) => child.nodes.map(shapeOf))]
}

export function FlowEditor(props: FlowEditorProps) {
  const { t } = useTranslation('automations')
  const labels = useAutomationLabels(props.catalog)
  const summary = useStepSummary(props.schema, props.catalog)
  const actions = props.catalog.actions.filter((action) => supportsRecordType(action, props.recordType))
  const titles = new Map<string, string>()
  const collect = (nodes: FlowNode[]) => {
    for (const node of nodes) {
      if (node.kind === 'jump') continue
      titles.set(node.step.id, summary(node.step).title)
      node.children.forEach((child) => collect(child.nodes))
    }
  }
  collect(props.nodes)
  const context: FlowContext = { ...props, summary, actions, stepTitle: (id) => titles.get(id) ?? id }
  const triggerIssues = props.report.trigger.length
  const scrollerRef = useRef<HTMLDivElement>(null)
  const layoutKey = JSON.stringify(props.nodes.map(shapeOf))
  useLayoutEffect(() => {
    const element = scrollerRef.current
    if (element) element.scrollLeft = Math.max(0, (element.scrollWidth - element.clientWidth) / 2)
  }, [layoutKey])
  const selected = props.selection?.kind === 'trigger'
  const triggerOptions = useTriggerSummary(props.trigger, props.catalog, props.schema)

  return (
    <div ref={scrollerRef} className="overflow-x-auto pb-4">
      <div className="mx-auto flex w-max min-w-full flex-col items-center px-1">
        <button
          type="button"
          onClick={() => props.onSelect({ kind: 'trigger' })}
          aria-pressed={selected}
          className={cn(
            'grid w-[min(22rem,calc(100vw-3rem))] gap-1 rounded-lg border-2 bg-card p-3 text-left shadow-xs transition-colors hover:border-primary/60 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none',
            selected ? 'border-primary' : 'border-primary/30',
            triggerIssues > 0 && 'border-destructive',
          )}
        >
          <span className="flex items-center gap-2 text-xs font-medium text-primary uppercase">
            <Zap className="size-4" aria-hidden="true" />
            {t('flow.trigger')}
            {triggerIssues > 0 && <IssueBadge count={triggerIssues} />}
          </span>
          <span className="font-medium">{labels.triggerLabel(props.trigger.type)}</span>
          {triggerOptions.length > 0 ? (
            <ul className="grid gap-0.5 text-xs text-muted-foreground">
              {triggerOptions.map((option) => (
                <li key={option} className="line-clamp-2 break-words">
                  {option}
                </li>
              ))}
            </ul>
          ) : (
            <span className="line-clamp-2 text-xs text-muted-foreground">
              {labels.triggerDescription(props.trigger.type) || labels.recordTypeLabel(props.recordType)}
            </span>
          )}
        </button>
        <Sequence nodes={props.nodes} path={[]} context={context} />
      </div>
    </div>
  )
}

function Connector({ short = false }: { short?: boolean }) {
  return <div aria-hidden="true" className={cn('w-px bg-border', short ? 'h-3' : 'h-5')} />
}

function Sequence({ nodes, path, context }: { nodes: FlowNode[]; path: SequencePath; context: FlowContext }) {
  const { t } = useTranslation('automations')
  const last = nodes[nodes.length - 1]
  const endsOpen = !last || (last.kind === 'step' && last.children.length === 0)
  return (
    <div className="flex flex-col items-center">
      <Connector />
      <AddStepButton point={{ path, index: 0 }} context={context} />
      {nodes.map((node, index) => (
        <div key={node.kind === 'jump' ? node.uid : node.step.id} className="flex flex-col items-center">
          <Connector short />
          {node.kind === 'jump' ? (
            <JumpBlock target={node.target} uid={node.uid} context={context} />
          ) : (
            <StepBlock node={node} context={context} />
          )}
          {node.kind === 'step' && node.children.length > 0 ? (
            <BranchColumns node={node} path={path} context={context} />
          ) : node.kind === 'step' ? (
            <>
              <Connector />
              <AddStepButton point={{ path, index: index + 1 }} context={context} />
            </>
          ) : null}
        </div>
      ))}
      {endsOpen && (
        <>
          <Connector short />
          <span className="flex items-center gap-1 rounded-full border px-3 py-0.5 text-xs text-muted-foreground">
            <ArrowDownToLine className="size-3" aria-hidden="true" />
            {t('flow.end')}
          </span>
        </>
      )}
    </div>
  )
}

function slotLabel(t: TFunction<'automations'>, node: StepNode, slot: string): string {
  if (slot === DEFAULT_SLOT) return t('flow.otherwise')
  if (slot === MATCHED_SLOT) return t('flow.matched')
  if (slot === TIMEOUT_SLOT) return t('flow.timedOut')
  const index = (node.step.cases ?? []).findIndex((item) => item.key === slot)
  const item = node.step.cases?.[index]
  return item?.label?.trim() || t('flow.case', { number: index + 1 })
}

function BranchColumns({ node, path, context }: { node: StepNode; path: SequencePath; context: FlowContext }) {
  const { t } = useTranslation('automations')
  return (
    <div className="flex flex-col items-center">
      <Connector short />
      <div className="flex items-start gap-3 border-t pt-0">
        {node.children.map((child) => (
          <div
            key={child.slot}
            role="group"
            aria-label={slotLabel(t, node, child.slot)}
            className="flex min-w-60 flex-col items-center sm:min-w-72"
          >
            <Connector short />
            <span
              className={cn(
                'max-w-56 truncate rounded-full px-3 py-0.5 text-xs font-medium',
                child.slot === DEFAULT_SLOT || child.slot === TIMEOUT_SLOT
                  ? 'bg-muted text-muted-foreground'
                  : 'bg-primary/10 text-primary',
              )}
            >
              {slotLabel(t, node, child.slot)}
            </span>
            <Sequence
              nodes={child.nodes}
              path={[...path, { stepId: node.step.id, slot: child.slot }]}
              context={{ ...context, nested: true }}
            />
          </div>
        ))}
      </div>
    </div>
  )
}

function IssueBadge({ count }: { count: number }) {
  const { t } = useTranslation('automations')
  return (
    <span className="ml-auto inline-flex items-center gap-1 rounded-full bg-destructive-solid px-1.5 py-0.5 text-[10px] font-semibold text-destructive-foreground normal-case">
      <AlertCircle className="size-3" aria-hidden="true" />
      <span>{t('flow.issues', { count })}</span>
    </span>
  )
}

function StepBlock({ node, context }: { node: StepNode; context: FlowContext }) {
  const { t } = useTranslation('automations')
  const step: StepSpec = node.step
  const Icon = STEP_ICONS[step.type]
  const { title, detail } = context.summary(step)
  const issues = context.report.steps[step.id]?.length ?? 0
  const selected = context.selection?.kind === 'step' && context.selection.id === step.id
  return (
    <button
      type="button"
      onClick={() => context.onSelect({ kind: 'step', id: step.id })}
      aria-pressed={selected}
      className={cn(
        'grid gap-1 rounded-lg border bg-card p-3 text-left shadow-xs transition-colors hover:border-primary/60 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none',
        context.nested ? 'w-56 sm:w-64' : 'w-[min(22rem,calc(100vw-3rem))]',
        selected && 'border-primary ring-1 ring-primary',
        issues > 0 && 'border-destructive',
      )}
    >
      <span className="flex items-center gap-2 text-xs font-medium text-muted-foreground uppercase">
        <Icon className="size-4" aria-hidden="true" />
        {t(`steps.${step.type}`)}
        {issues > 0 && <IssueBadge count={issues} />}
      </span>
      <span className="truncate font-medium">{title}</span>
      {detail && <span className="line-clamp-2 text-xs break-words text-muted-foreground">{detail}</span>}
    </button>
  )
}

function JumpBlock({ target, uid, context }: { target: string; uid: string; context: FlowContext }) {
  const { t } = useTranslation('automations')
  return (
    <div
      className={cn(
        'flex items-center gap-2 rounded-lg border border-dashed p-2 text-sm',
        context.nested ? 'w-56 sm:w-64' : 'w-[min(22rem,calc(100vw-3rem))]',
      )}
    >
      <CornerDownRight className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
      <button
        type="button"
        className="min-w-0 flex-1 truncate text-left underline-offset-4 hover:underline"
        onClick={() => context.onSelect({ kind: 'step', id: target })}
      >
        {t('flow.jump', { step: context.stepTitle(target) })}
      </button>
      {!context.readOnly && (
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="size-7"
          aria-label={t('flow.removeJump')}
          onClick={() => context.onRemoveJump(uid)}
        >
          <X aria-hidden="true" />
        </Button>
      )}
    </div>
  )
}

function AddStepButton({ point, context }: { point: InsertionPoint; context: FlowContext }) {
  const { t } = useTranslation('automations')
  const labels = useAutomationLabels(context.catalog)
  if (context.readOnly) return null
  const pick = (kind: NewStepKind) => context.onInsert(point, kind)
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          type="button"
          variant="outline"
          size="icon"
          className="size-7 rounded-full bg-background"
          aria-label={t('flow.addStep')}
          title={t('flow.addStep')}
        >
          <Plus aria-hidden="true" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="center" className="max-h-[70vh] w-64 overflow-y-auto">
        <DropdownMenuLabel>{t('flow.logic')}</DropdownMenuLabel>
        <DropdownMenuItem onSelect={() => pick('condition')}>
          <Filter aria-hidden="true" />
          {t('steps.condition')}
        </DropdownMenuItem>
        <DropdownMenuItem onSelect={() => pick('branch')}>
          <GitBranch aria-hidden="true" />
          {t('steps.branch')}
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuLabel>{t('flow.time')}</DropdownMenuLabel>
        <DropdownMenuItem onSelect={() => pick('wait')}>
          <Timer aria-hidden="true" />
          {t('steps.wait')}
        </DropdownMenuItem>
        <DropdownMenuItem onSelect={() => pick('wait_until')}>
          <Hourglass aria-hidden="true" />
          {t('steps.wait_until')}
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuLabel>{t('flow.actions')}</DropdownMenuLabel>
        {context.actions.map((action) => (
          <DropdownMenuItem key={action.key} onSelect={() => pick({ action: action.key })}>
            <ActionIcon aria-hidden="true" />
            {labels.actionLabel(action.key, action)}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
