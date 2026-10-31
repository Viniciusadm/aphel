import { Plus, Trash2 } from 'lucide-react'
import { useId } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import type { FieldSchema, FilterNode } from '@/features/records/types'
import { ConditionEditor } from '../components/ConditionEditor'
import { DurationInput } from '../components/DurationInput'
import { IssueList } from '../components/IssueList'
import { FieldSelect } from '../components/ReferenceSelects'
import { SchemaForm } from '../components/SchemaForm'
import { issuesOutside, issuesUnder, type ValidationIssue } from '../lib/errors'
import { supportsRecordType, useAutomationLabels } from '../lib/labels'
import type { TemplateVariable } from '../lib/variables'
import type { AutomationCatalog, BranchCaseSpec, StepSpec } from '../types'
import { EMPTY_GROUP, actionDefaults, nextCaseKey } from './steps'
import { Field } from '@/components/ui/field'

export interface StepPanelProps {
  step: StepSpec
  recordType: string
  schema: FieldSchema
  catalog: AutomationCatalog
  variables: TemplateVariable[]
  recordTypes: string[]
  issues: ValidationIssue[]
  readOnly?: boolean
  onChange: (step: StepSpec) => void
}

const KNOWN_PREFIXES = ['label', 'filter', 'cases', 'config', 'duration', 'until_field', 'offset', 'recheck', 'timeout']

export function StepPanel({ step, recordType, schema, catalog, variables, recordTypes, issues, readOnly, onChange }: StepPanelProps) {
  const { t } = useTranslation('automations')
  const general = issuesOutside(issues, KNOWN_PREFIXES)

  return (
    <fieldset disabled={readOnly} className="grid min-w-0 gap-5">
      <IssueList issues={general} showPath />
      <Field label={t('panel.label')}>
        {(control) => <>
          <Input
          {...control}
          value={step.label ?? ''}
          maxLength={120}
          placeholder={t('panel.labelPlaceholder')}
          onChange={(event) => onChange({ ...step, label: event.target.value || null })}
        />
        <p className="text-xs text-muted-foreground">
          {t('panel.stepId')} <code>{step.id}</code>
        </p>
        </>}
      </Field>
      {step.type === 'condition' && (
        <ConditionSection
          title={t('panel.conditionTitle')}
          hint={t('panel.conditionHint')}
          recordType={recordType}
          schema={schema}
          value={step.filter}
          onChange={(filter) => onChange({ ...step, filter })}
          issues={issuesUnder(issues, 'filter')}
          stepKey={step.id}
        />
      )}
      {step.type === 'branch' && <BranchEditor step={step} recordType={recordType} schema={schema} issues={issues} onChange={onChange} />}
      {step.type === 'action' && (
        <ActionEditor
          step={step}
          recordType={recordType}
          schema={schema}
          catalog={catalog}
          variables={variables}
          recordTypes={recordTypes}
          issues={issues}
          onChange={onChange}
        />
      )}
      {step.type === 'wait' && <WaitEditor step={step} recordType={recordType} schema={schema} issues={issues} onChange={onChange} />}
      {step.type === 'wait_until' && (
        <>
          <ConditionSection
            title={t('panel.waitUntilTitle')}
            hint={t('panel.waitUntilHint')}
            recordType={recordType}
            schema={schema}
            value={step.filter}
            onChange={(filter) => onChange({ ...step, filter })}
            issues={issuesUnder(issues, 'filter')}
            stepKey={step.id}
            includeTrigger={false}
          />
          <DurationField
            label={t('panel.timeout')}
            hint={t('panel.timeoutHint')}
            value={step.timeout}
            onChange={(timeout) => onChange({ ...step, timeout })}
            issues={issuesUnder(issues, 'timeout')}
          />
        </>
      )}
    </fieldset>
  )
}

function ConditionSection({
  title,
  hint,
  recordType,
  schema,
  value,
  onChange,
  issues,
  stepKey,
  includeTrigger = true,
}: {
  title: string
  hint?: string
  recordType: string
  schema: FieldSchema
  value: FilterNode | null | undefined
  onChange: (value: FilterNode) => void
  issues: ValidationIssue[]
  stepKey: string
  includeTrigger?: boolean
}) {
  return (
    <section className="grid gap-2">
      <div className="grid gap-0.5">
        <h3 className="text-sm font-medium">{title}</h3>
        {hint && <p className="text-xs text-muted-foreground">{hint}</p>}
      </div>
      <ConditionEditor
        key={stepKey}
        recordType={recordType}
        schema={schema}
        value={value}
        onChange={onChange}
        includeTrigger={includeTrigger}
      />
      <IssueList issues={issues} showPath />
    </section>
  )
}

function DurationField({
  label,
  hint,
  value,
  onChange,
  issues,
  signed,
}: {
  label: string
  hint?: string
  value: string | null | undefined
  onChange: (value: string | null) => void
  issues: ValidationIssue[]
  signed?: boolean
}) {
  return (
    <Field label={label}>
      {(fieldControl) => <>
        <DurationInput id={fieldControl.id} value={value} onChange={onChange} signed={signed} invalid={issues.length > 0} />
      {hint && <p className="text-xs text-muted-foreground">{hint}</p>}
      <IssueList issues={issues} />
      </>}
    </Field>
  )
}

function BranchEditor({
  step,
  recordType,
  schema,
  issues,
  onChange,
}: {
  step: StepSpec
  recordType: string
  schema: FieldSchema
  issues: ValidationIssue[]
  onChange: (step: StepSpec) => void
}) {
  const { t } = useTranslation('automations')
  const cases = step.cases ?? []
  const setCase = (index: number, next: BranchCaseSpec) =>
    onChange({ ...step, cases: cases.map((item, position) => (position === index ? next : item)) })

  return (
    <section className="grid gap-4">
      <p className="text-xs text-muted-foreground">{t('panel.branchHint')}</p>
      {cases.map((item, index) => (
        <div key={item.key} className="grid gap-3 rounded-md border p-3">
          <div className="flex items-center gap-2">
            <Input
              value={item.label ?? ''}
              placeholder={t('flow.case', { number: index + 1 })}
              aria-label={t('panel.caseLabel', { number: index + 1 })}
              maxLength={80}
              onChange={(event) => setCase(index, { ...item, label: event.target.value || null })}
              className="min-w-0 flex-1"
            />
            {cases.length > 1 && (
              <Button
                type="button"
                variant="ghost"
                size="icon"
                aria-label={t('panel.removeCase')}
                onClick={() => onChange({ ...step, cases: cases.filter((_, position) => position !== index) })}
              >
                <Trash2 aria-hidden="true" />
              </Button>
            )}
          </div>
          <ConditionEditor
            key={item.key}
            recordType={recordType}
            schema={schema}
            value={item.filter}
            onChange={(filter) => setCase(index, { ...item, filter })}
          />
          <IssueList issues={issuesUnder(issues, `cases.${index}`)} showPath />
        </div>
      ))}
      {cases.length < 10 && (
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="justify-self-start"
          onClick={() =>
            onChange({
              ...step,
              cases: [...cases, { key: nextCaseKey(step), label: null, filter: EMPTY_GROUP }],
            })
          }
        >
          <Plus aria-hidden="true" />
          {t('panel.addCase')}
        </Button>
      )}
      <p className="text-xs text-muted-foreground">{t('panel.otherwiseHint')}</p>
    </section>
  )
}

function ActionEditor({
  step,
  recordType,
  schema,
  catalog,
  variables,
  recordTypes,
  issues,
  onChange,
}: {
  step: StepSpec
  recordType: string
  schema: FieldSchema
  catalog: AutomationCatalog
  variables: TemplateVariable[]
  recordTypes: string[]
  issues: ValidationIssue[]
  onChange: (step: StepSpec) => void
}) {
  const { t } = useTranslation('automations')
  const labels = useAutomationLabels(catalog)
  const actions = catalog.actions.filter((action) => supportsRecordType(action, recordType))
  const entry = catalog.actions.find((action) => action.key === step.action)
  const hasOwner = schema.fields.some((field) => field.key === 'owner_id')
  const description = step.action ? labels.actionDescription(step.action) || entry?.description : ''

  return (
    <section className="grid gap-4">
      <Field label={t('panel.action')}>
        {(fieldControl) => <>
          <NativeSelect
          id={fieldControl.id}
          value={step.action ?? ''}
          onChange={(event) => {
            const next = catalog.actions.find((action) => action.key === event.target.value)
            onChange({ ...step, action: event.target.value, config: actionDefaults(next, hasOwner) })
          }}
          className="w-full"
        >
          {!entry && <option value={step.action ?? ''}>{step.action ?? ''}</option>}
          {actions.map((action) => (
            <option key={action.key} value={action.key}>
              {labels.actionLabel(action.key, action)}
            </option>
          ))}
        </NativeSelect>
        {description && <p className="text-xs text-muted-foreground">{description}</p>}
        <IssueList issues={issuesUnder(issues, 'action')} />
        </>}
      </Field>
      {entry ? (
        <SchemaForm
          key={`${step.id}:${step.action}`}
          jsonSchema={entry.config_schema}
          value={step.config ?? {}}
          onChange={(config) => onChange({ ...step, config })}
          context={{
            mode: 'action',
            recordType,
            schema,
            variables,
            recordTypes,
            issues: issuesUnder(issues, 'config'),
          }}
        />
      ) : null}
      <IssueList issues={issuesOutside(issuesUnder(issues, 'config'), Object.keys(entry?.config_schema.properties ?? {}))} showPath />
    </section>
  )
}

function WaitEditor({
  step,
  recordType,
  schema,
  issues,
  onChange,
}: {
  step: StepSpec
  recordType: string
  schema: FieldSchema
  issues: ValidationIssue[]
  onChange: (step: StepSpec) => void
}) {
  const { t } = useTranslation('automations')
  const recheckId = useId()
  const mode = step.until_field ? 'field' : 'duration'
  const hasRecheck = step.recheck !== undefined && step.recheck !== null

  const setMode = (next: 'duration' | 'field') => {
    const copy: StepSpec = { ...step }
    if (next === 'duration') {
      delete copy.until_field
      delete copy.offset
      copy.duration = step.duration ?? '1d'
    } else {
      delete copy.duration
      const firstDate = schema.fields.find((field) => field.is_active && (field.field_type === 'date' || field.field_type === 'datetime'))
      copy.until_field = firstDate?.key ?? ''
    }
    onChange(copy)
  }

  return (
    <section className="grid gap-4">
      <div role="radiogroup" aria-label={t('panel.waitMode')} className="grid gap-2 sm:grid-cols-2">
        {(['duration', 'field'] as const).map((option) => (
          <label
            key={option}
            className="flex cursor-pointer items-start gap-2 rounded-md border p-3 text-sm has-[:checked]:border-primary"
          >
            <input
              type="radio"
              name={`${step.id}-wait-mode`}
              checked={mode === option}
              onChange={() => setMode(option)}
              className="mt-0.5 accent-primary"
            />
            <span className="grid gap-0.5">
              <span className="font-medium">{t(`panel.waitModes.${option}`)}</span>
              <span className="text-xs text-muted-foreground">{t(`panel.waitModeHints.${option}`)}</span>
            </span>
          </label>
        ))}
      </div>
      {mode === 'duration' ? (
        <DurationField
          label={t('panel.duration')}
          value={step.duration}
          onChange={(duration) => onChange({ ...step, duration })}
          issues={issuesUnder(issues, 'duration')}
        />
      ) : (
        <>
          <Field label={t('panel.untilField')}>
            {(fieldControl) => <>
              <FieldSelect
              id={fieldControl.id}
              recordType={recordType}
              fieldTypes={['date', 'datetime']}
              value={step.until_field}
              onChange={(until_field) => onChange({ ...step, until_field: until_field ?? '' })}
              invalid={issuesUnder(issues, 'until_field').length > 0}
            />
            <IssueList issues={issuesUnder(issues, 'until_field')} />
            </>}
          </Field>
          <DurationField
            label={t('panel.offset')}
            hint={t('panel.offsetHint')}
            value={step.offset}
            signed
            onChange={(offset) => onChange({ ...step, offset })}
            issues={issuesUnder(issues, 'offset')}
          />
        </>
      )}
      <div className="grid gap-3 rounded-md border p-3">
        <div className="flex items-start gap-2">
          <Checkbox
            id={recheckId}
            checked={hasRecheck}
            onCheckedChange={(checked) => {
              const copy: StepSpec = { ...step }
              if (checked === true) copy.recheck = EMPTY_GROUP
              else delete copy.recheck
              onChange(copy)
            }}
          />
          <div className="grid gap-0.5">
            <Label htmlFor={recheckId}>{t('panel.recheck')}</Label>
            <p className="text-xs text-muted-foreground">{t('panel.recheckHint')}</p>
          </div>
        </div>
        {hasRecheck && (
          <ConditionEditor
            key={`${step.id}-recheck`}
            recordType={recordType}
            schema={schema}
            value={step.recheck}
            onChange={(recheck) => onChange({ ...step, recheck })}
            includeTrigger={false}
          />
        )}
        <IssueList issues={issuesUnder(issues, 'recheck')} showPath />
      </div>
    </section>
  )
}
