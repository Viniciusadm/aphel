import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { ArrowDown, ArrowUp, Plus, Trash2 } from 'lucide-react'
import { useId } from 'react'
import { useFieldArray, useFormContext, useWatch } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { localizeViolationMessage } from '@/api'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { NativeSelect } from '@/sdk'
import { useEntities, entityLabel } from '@/sdk/entities'
import { Slot } from '@/kernel/modules/Slot'
import { useActionTargets, type MatchableField } from './api'
import type { WebhookFormValues } from './form'
import { Field } from '@/components/ui/field'

interface ActionEditorProps {
  index: number
  total: number
  onMove: (from: number, to: number) => void
  onRemove: () => void
}

function ErrorText({ id, message }: { id: string; message?: string }) {
  const { t } = useTranslation('webhooks', { keyPrefix: 'incoming' })
  if (!message) return null
  return (
    <SharedFieldError id={id}>
      {t(message, { defaultValue: localizeViolationMessage(message) })}
    </SharedFieldError>
  )
}

export function ActionEditor({ index, total, onMove, onRemove }: ActionEditorProps) {
  const { t, i18n } = useTranslation('webhooks', { keyPrefix: 'incoming' })
  const entities = useEntities()
  const id = useId()
  const { control, register, formState, setValue } = useFormContext<WebhookFormValues>()
  const type = useWatch({ control, name: `actions.${index}.type` })
  const recordType = useWatch({ control, name: `actions.${index}.record_type` })
  const assignmentRuleId = useWatch({ control, name: `actions.${index}.assignment_rule_id` })
  const mappings = useFieldArray({ control, name: `actions.${index}.fields` })
  const targets = useActionTargets()
  const target = targets.data?.find((item) => item.type === type && item.record_type === recordType)
  const errors = formState.errors.actions?.[index]
  const fieldOptions: MatchableField[] = target?.fields ?? []
  const uniqueFields = fieldOptions.filter((field) => field.is_unique)

  return (
    <li className="grid gap-3 rounded-md border bg-card p-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <span className="text-sm font-medium">
          {t('actions.heading', { position: index + 1, type: t(`actionTypes.${type}`) })}
        </span>
        <div className="flex gap-1">
          <Button
            type="button"
            size="icon"
            variant="ghost"
            aria-label={t('actions.moveUp')}
            disabled={index === 0}
            onClick={() => onMove(index, index - 1)}
          >
            <ArrowUp aria-hidden="true" />
          </Button>
          <Button
            type="button"
            size="icon"
            variant="ghost"
            aria-label={t('actions.moveDown')}
            disabled={index === total - 1}
            onClick={() => onMove(index, index + 1)}
          >
            <ArrowDown aria-hidden="true" />
          </Button>
          <Button type="button" size="icon" variant="ghost" aria-label={t('actions.remove')} onClick={onRemove}>
            <Trash2 aria-hidden="true" />
          </Button>
        </div>
      </div>
      <p className="text-xs text-muted-foreground">{t(`actionHelp.${type}`)}</p>

      <Field label={t('actions.entity')}>
        {(control) => <NativeSelect {...control} {...register(`actions.${index}.record_type`, { onChange: () => { setValue(`actions.${index}.fields`, []); setValue(`actions.${index}.match_field`, ''); setValue(`actions.${index}.assignment_rule_id`, '') } })}>
          <option value="">{t('actions.choose')}</option>
          {(targets.data ?? []).filter((item) => item.type === type).map((item) => <option key={item.record_type} value={item.record_type}>{entityLabel(entities.data?.find((entity) => entity.record_type === item.record_type), i18n.language, item.record_type)}</option>)}
        </NativeSelect>}
      </Field>
      {type === 'upsert_record' && <Field label={t('actions.matchFieldLabel')}>
        {(control) => <NativeSelect {...control} {...register(`actions.${index}.match_field`)}>
          <option value="">{t('actions.choose')}</option>
          {uniqueFields.map((field) => <option key={field.key} value={field.key}>{field.label}</option>)}
        </NativeSelect>}
      </Field>}
      {recordType && fieldOptions.some((field) => field.key === 'owner_id') && <Slot name="assignment.ruleSelect" props={{ recordType, value: assignmentRuleId || null, onChange: (ruleId) => setValue(`actions.${index}.assignment_rule_id`, ruleId ?? '', { shouldDirty: true }) }} />}

      <fieldset className="grid gap-2">
        <legend className="mb-1 text-sm font-medium">{t('actions.mappings')}</legend>
        {mappings.fields.length === 0 && <p className="text-xs text-muted-foreground">{t('actions.noMappings')}</p>}
        {mappings.fields.map((mapping, mappingIndex) => (
          <div key={mapping.id} className="grid gap-2 sm:grid-cols-[minmax(0,14rem)_minmax(0,1fr)_auto] sm:items-start">
            <NativeSelect
              aria-label={t('actions.crmField')}
              {...register(`actions.${index}.fields.${mappingIndex}.key`)}
            >
              <option value="">{t('actions.chooseField')}</option>
              {fieldOptions.map((field) => (
                <option key={field.key} value={field.key}>
                  {field.label}
                  {field.is_custom ? ` (${t('actions.custom')})` : ''}
                  {field.is_required ? ' *' : ''}
                </option>
              ))}
            </NativeSelect>
            <Input
              aria-label={t('actions.expression')}
              placeholder={t('actions.expressionPlaceholder', { template: '{{ $.payload.name }}' })}
              className="font-mono text-xs"
              {...register(`actions.${index}.fields.${mappingIndex}.expression`)}
            />
            <Button
              type="button"
              size="icon"
              variant="ghost"
              aria-label={t('actions.removeField')}
              onClick={() => mappings.remove(mappingIndex)}
            >
              <Trash2 aria-hidden="true" />
            </Button>
          </div>
        ))}
        <ErrorText id={`${id}-fields-error`} message={errors?.fields?.message ?? errors?.fields?.root?.message} />
        <Button
          type="button"
          size="sm"
          variant="outline"
          className="self-start"
          onClick={() => mappings.append({ key: '', expression: '' })}
        >
          <Plus aria-hidden="true" />
          {t('actions.addField')}
        </Button>
        <p className="text-xs text-muted-foreground">{t('actions.expressionHelp', { template: '{{ $.caminho }}' })}</p>
      </fieldset>
    </li>
  )
}
