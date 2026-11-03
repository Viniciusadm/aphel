import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { ArrowDown, ArrowUp, Plus, Trash2 } from 'lucide-react'
import { useId, useMemo } from 'react'
import { useFieldArray, useFormContext, useWatch } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { localizeViolationMessage } from '@/api'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { useActivityTypes } from '@/modules/tasks/api'
import { usePipelines } from '@/modules/workflow/api'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { Slot } from '@/kernel/modules/Slot'
import type { ActionTarget, MatchableField } from './api'
import type { WebhookFormValues } from './form'
import { Field } from '@/components/ui/field'

interface ActionEditorProps {
  index: number
  total: number
  target: ActionTarget | undefined
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

export function ActionEditor({ index, total, target, onMove, onRemove }: ActionEditorProps) {
  const { t } = useTranslation('webhooks', { keyPrefix: 'incoming' })
  const id = useId()
  const { control, register, formState, setValue } = useFormContext<WebhookFormValues>()
  const type = useWatch({ control, name: `actions.${index}.type` })
  const pipelineId = useWatch({ control, name: `actions.${index}.pipeline_id` })
  const matchBy = useWatch({ control, name: `actions.${index}.match_by` })
  const distributionRuleId = useWatch({ control, name: `actions.${index}.distribution_rule_id` })
  const mappings = useFieldArray({ control, name: `actions.${index}.fields` })
  const workflow = usePipelines()
  const activityTypes = useActivityTypes()
  const errors = formState.errors.actions?.[index]

  const stages = useMemo(
    () =>
      (workflow.data ?? [])
        .find((pipeline) => pipeline.id === pipelineId)
        ?.stages.filter((stage) => stage.is_active) ?? [],
    [workflow.data, pipelineId],
  )
  const fieldOptions: MatchableField[] = target?.fields ?? []
  const uniqueFields = fieldOptions.filter((field) => field.is_custom && field.is_unique)

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

      {type === 'upsert_contact' && (
        <Field label={t('actions.matchBy')}>
          {(control) => <NativeSelect {...control} {...register(`actions.${index}.match_by`)}>
            <option value="email">{t('actions.matchEmail')}</option>
            <option value="phone">{t('actions.matchPhone')}</option>
            <option value="field">{t('actions.matchField')}</option>
            <option value="none">{t('actions.matchNone')}</option>
          </NativeSelect>}
        </Field>
      )}

      {type === 'upsert_contact' && matchBy === 'field' && (
        <Field label={t('actions.matchFieldLabel')}>
          {(control) => <>
            <NativeSelect
            {...control}
            aria-invalid={errors?.match_field ? true : undefined}
            aria-describedby={errors?.match_field ? `${id}-match-field-error` : undefined}
            {...register(`actions.${index}.match_field`)}
          >
            <option value="">{t('actions.choose')}</option>
            {uniqueFields.map((field) => (
              <option key={field.key} value={field.key}>
                {field.label}
              </option>
            ))}
          </NativeSelect>
          {uniqueFields.length === 0 && <p className="text-xs text-muted-foreground">{t('actions.noUniqueFields')}</p>}
          <ErrorText id={`${id}-match-field-error`} message={errors?.match_field?.message} />
          </>}
        </Field>
      )}

      {type === 'create_deal' && (
        <div className="grid gap-3 sm:grid-cols-2">
          <Field label={t('actions.pipeline')}>
            {(control) => <>
              <NativeSelect
              {...control}
              aria-invalid={errors?.pipeline_id ? true : undefined}
              {...register(`actions.${index}.pipeline_id`, {
                onChange: () => setValue(`actions.${index}.stage_id`, ''),
              })}
            >
              <option value="">{t('actions.choose')}</option>
              {(workflow.data ?? [])
                .filter((pipeline) => pipeline.is_active)
                .map((pipeline) => (
                  <option key={pipeline.id} value={pipeline.id}>
                    {pipeline.name}
                  </option>
                ))}
            </NativeSelect>
            <ErrorText id={`${id}-pipeline-error`} message={errors?.pipeline_id?.message} />
            </>}
          </Field>
          <Field label={t('actions.stage')}>
            {(control) => <>
              <NativeSelect
              {...control}
              aria-invalid={errors?.stage_id ? true : undefined}
              {...register(`actions.${index}.stage_id`)}
            >
              <option value="">{t('actions.choose')}</option>
              {stages.map((stage) => (
                <option key={stage.id} value={stage.id}>
                  {stage.name}
                </option>
              ))}
            </NativeSelect>
            <ErrorText id={`${id}-stage-error`} message={errors?.stage_id?.message} />
            </>}
          </Field>
        </div>
      )}

      {(type === 'upsert_contact' || type === 'create_deal') && (
        <Slot
          name="leadDistribution.ruleSelect"
          props={{
            recordType: type === 'upsert_contact' ? 'people.person' : 'deals.deal',
            value: distributionRuleId || null,
            onChange: (ruleId) =>
              setValue(`actions.${index}.distribution_rule_id`, ruleId ?? '', { shouldDirty: true }),
          }}
        />
      )}

      {type === 'create_activity' && (
        <Field label={t('actions.activityType')}>
          {(control) => <>
            <NativeSelect
            {...control}
            aria-invalid={errors?.activity_type_id ? true : undefined}
            {...register(`actions.${index}.activity_type_id`)}
          >
            <option value="">{t('actions.choose')}</option>
            {(activityTypes.data ?? [])
              .filter((activityType) => activityType.is_active)
              .map((activityType) => (
                <option key={activityType.id} value={activityType.id}>
                  {activityType.name}
                </option>
              ))}
          </NativeSelect>
          <ErrorText id={`${id}-activity-type-error`} message={errors?.activity_type_id?.message} />
          </>}
        </Field>
      )}

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
              placeholder={t('actions.expressionPlaceholder', { template: '{{ $.nome }}' })}
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
