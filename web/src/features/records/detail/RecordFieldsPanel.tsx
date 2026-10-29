import { useMemo, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'
import { getFieldValue } from '../fields'
import { InlineFieldEditor } from '../form/InlineFieldEditor'
import { resolveLayout, type FormLayout } from '../form/lib/layout'
import { layoutRuleStates, recordRuleValues, useConditionContext } from '../form/use-form-rules'
import type { FieldDefinition, RecordItem } from '../types'

export interface RecordFieldsPanelProps {
  fields: FieldDefinition[]
  record: RecordItem
  onSaveField: (field: FieldDefinition, value: unknown) => Promise<void>
  readOnly?: boolean
  readOnlyFieldKeys?: string[]
  layout?: FormLayout | null
  title?: string
  className?: string
  children?: ReactNode
}

interface PanelGroup {
  id: string
  title: string
  subtitle: string
  fields: FieldDefinition[]
}

export function RecordFieldsPanel({
  fields,
  record,
  onSaveField,
  readOnly = false,
  readOnlyFieldKeys = [],
  layout,
  title,
  className,
  children,
}: RecordFieldsPanelProps) {
  const { t } = useTranslation('records-detail')
  const context = useConditionContext(fields)
  const groups = useMemo<PanelGroup[]>(() => {
    if (!layout) return [{ id: 'all', title: '', subtitle: '', fields }]
    const resolved = resolveLayout(layout, fields)
    const states = layoutRuleStates(resolved, recordRuleValues(fields, record), context)
    const showTabs = resolved.tabs.length > 1
    return resolved.tabs.flatMap((tab) =>
      tab.sections.map((section, index) => ({
        id: `${tab.id}:${section.id}`,
        title: showTabs && index === 0 ? tab.label : '',
        subtitle: section.label,
        fields: section.columns
          .flatMap((column) => column.fields)
          .filter((item) => states[item.field.key]?.visible !== false)
          .map((item) => item.field),
      })),
    )
  }, [layout, fields, record, context])
  const empty = groups.every((group) => group.fields.length === 0)

  return (
    <section aria-label={t('fields.label')} className={cn('grid content-start gap-3 rounded-lg border bg-card p-4', className)}>
      <h2 className="text-sm font-semibold">{title ?? t('fields.title')}</h2>
      {empty ? (
        <p className="text-sm text-muted-foreground">{t('fields.empty')}</p>
      ) : (
        groups.map((group) =>
          group.fields.length === 0 && !group.title ? null : (
            <div key={group.id} className="grid min-w-0 gap-3" data-section={group.id}>
              {group.title && <h3 className="border-t pt-3 text-xs font-semibold tracking-wide text-muted-foreground uppercase">{group.title}</h3>}
              {group.subtitle && <h4 className="text-sm font-medium">{group.subtitle}</h4>}
              <dl className="grid gap-3">
                {group.fields.map((field) => (
                  <div key={field.key} data-field={field.key} className="grid min-w-0 gap-1">
                    <dt className="text-xs font-medium text-muted-foreground">{field.label}</dt>
                    <dd className="min-w-0 text-sm">
                      <InlineFieldEditor
                        field={field}
                        value={getFieldValue(record, field)}
                        readOnly={readOnly || readOnlyFieldKeys.includes(field.key)}
                        knownFields={fields}
                        onSave={(value) => onSaveField(field, value)}
                      />
                    </dd>
                  </div>
                ))}
              </dl>
            </div>
          ),
        )
      )}
      {children}
    </section>
  )
}
