import { useId } from 'react'
import { useTranslation } from 'react-i18next'
import { Checkbox } from '@/components/ui/checkbox'
import { Label } from '@/components/ui/label'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import type { FieldSchema } from '@/features/records/types'
import { IssueList } from '../components/IssueList'
import { SchemaForm } from '../components/SchemaForm'
import { issuesOutside, issuesUnder, type ValidationIssue } from '../lib/errors'
import { supportsRecordType, useAutomationLabels } from '../lib/labels'
import type { TemplateVariable } from '../lib/variables'
import type { AutomationCatalog, DefinitionOptions, JsonObject, TriggerSpec } from '../types'

export interface TriggerPanelProps {
  trigger: TriggerSpec
  options: DefinitionOptions
  recordType: string
  schema: FieldSchema
  catalog: AutomationCatalog
  variables: TemplateVariable[]
  recordTypes: string[]
  issues: ValidationIssue[]
  readOnly?: boolean
  onChange: (trigger: TriggerSpec) => void
  onOptionsChange: (options: DefinitionOptions) => void
}

const DEFAULT_OPTIONS: Required<DefinitionOptions> = { cancel_waiting_on_pause: true, cancel_waiting_on_publish: false }

export function TriggerPanel({
  trigger,
  options,
  recordType,
  schema,
  catalog,
  variables,
  recordTypes,
  issues,
  readOnly,
  onChange,
  onOptionsChange,
}: TriggerPanelProps) {
  const { t } = useTranslation('automations')
  const id = useId()
  const labels = useAutomationLabels(catalog)
  const triggers = catalog.triggers.filter((entry) => supportsRecordType(entry, recordType))
  const entry = catalog.triggers.find((item) => item.key === trigger.type)
  const { type, ...values } = trigger
  const description = labels.triggerDescription(type) || entry?.description

  return (
    <fieldset disabled={readOnly} className="grid min-w-0 gap-5">
      <div className="grid gap-2.5">
        <Label htmlFor={id}>{t('panel.triggerType')}</Label>
        <NativeSelect id={id} value={type} onChange={(event) => onChange({ type: event.target.value })} className="w-full">
          {!entry && <option value={type}>{type}</option>}
          {triggers.map((item) => (
            <option key={item.key} value={item.key}>
              {labels.triggerLabel(item.key, item)}
            </option>
          ))}
        </NativeSelect>
        {description && <p className="text-xs text-muted-foreground">{description}</p>}
        <IssueList issues={issuesUnder(issues, 'type')} />
      </div>
      {entry && (
        <SchemaForm
          key={type}
          jsonSchema={entry.options_schema}
          value={values as JsonObject}
          onChange={(next) => onChange({ ...next, type })}
          context={{ mode: 'trigger', recordType, schema, variables, recordTypes, issues }}
        />
      )}
      <IssueList
        issues={issuesOutside(issues, ['type', ...Object.keys(entry?.options_schema.properties ?? {})])}
        showPath
      />
      <section className="grid gap-3 border-t pt-4">
        <div className="grid gap-0.5">
          <h3 className="text-sm font-medium">{t('panel.waitingTitle')}</h3>
          <p className="text-xs text-muted-foreground">{t('panel.waitingHint')}</p>
        </div>
        {(['cancel_waiting_on_pause', 'cancel_waiting_on_publish'] as const).map((key) => (
          <div key={key} className="flex items-start gap-2">
            <Checkbox
              id={`${id}-${key}`}
              checked={options[key] ?? DEFAULT_OPTIONS[key]}
              onCheckedChange={(checked) => onOptionsChange({ ...options, [key]: checked === true })}
            />
            <Label htmlFor={`${id}-${key}`} className="leading-snug font-normal">
              {t(`panel.options.${key}`)}
            </Label>
          </div>
        ))}
      </section>
    </fieldset>
  )
}
