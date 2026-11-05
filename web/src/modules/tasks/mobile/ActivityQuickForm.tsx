import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { OptionGroup } from '@/components/ui/option-group'
import { useQueryClient } from '@tanstack/react-query'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { zonedInputToUtc } from '@/components/fields/lib/datetime'
import { notifyError } from '@/components/feedback/notify'
import { Input } from '@/components/ui/input'
import {activitiesKeys, useActivityTypes} from '@/sdk'
import { ActivityTypeIcon } from '../components/ActivityTypeIcon'
import { DictationField, FormActions, FormBody, type PickedRecord, type QuickFormProps, RecordPicker, useOutboxFeedback, useRecordFormatting } from '@/sdk'
import { submitActivity } from './api'
import { Field } from '@/components/ui/field'

type ActivityWhen = 'done' | 'due'

export interface ActivityQuickFormProps extends QuickFormProps {
  initialRecord: PickedRecord | null
}

export function ActivityQuickForm({ onDone, initialRecord }: ActivityQuickFormProps) {
  const { t } = useTranslation('tasks', { keyPrefix: 'quickAdd' })
  const { timezone } = useRecordFormatting()
  const queryClient = useQueryClient()
  const feedback = useOutboxFeedback()
  const types = useActivityTypes()
  const activeTypes = (types.data ?? []).filter((type) => type.is_active)
  const ids = { title: useId(), due: useId(), description: useId() }
  const [title, setTitle] = useState('')
  const [typeId, setTypeId] = useState('')
  const [when, setWhen] = useState<ActivityWhen>('due')
  const [dueAt, setDueAt] = useState('')
  const [description, setDescription] = useState('')
  const [record, setRecord] = useState<PickedRecord | null>(initialRecord)
  const [touched, setTouched] = useState(false)
  const [pending, setPending] = useState(false)
  const selectedTypeId = typeId || activeTypes[0]?.id || ''
  const titleMissing = title.trim() === ''

  const submit = async () => {
    setTouched(true)
    if (titleMissing || !selectedTypeId) return
    setPending(true)
    try {
      const result = await submitActivity(
        {
          title: title.trim(),
          activity_type_id: selectedTypeId,
          status: when === 'done' ? 'done' : 'open',
          due_at: when === 'due' && dueAt ? zonedInputToUtc(dueAt, timezone) : null,
          description: description.trim() || null,
          record_type: record?.recordType ?? null,
          record_id: record?.recordId ?? null,
        },
        title.trim(),
      )
      feedback(result, t('activity.created', { title: title.trim() }))
      void queryClient.invalidateQueries({ queryKey: activitiesKeys.all })
      onDone()
    } catch (error) {
      notifyError(error)
    } finally {
      setPending(false)
    }
  }

  return (
    <FormBody onSubmit={() => void submit()}>
      <Field label={t('activity.title')}>
        {(control) => <>
          <Input placeholder={t('placeholders.title')}
          {...control}
          value={title}
          onChange={(event) => setTitle(event.target.value)}
          className="h-11 text-base"
          maxLength={255}
          aria-invalid={touched && titleMissing}
        />
        {touched && titleMissing && <SharedFieldError>{t('activity.titleRequired')}</SharedFieldError>}
        </>}
      </Field>
      <fieldset className="grid gap-2">
        <legend className="mb-2 text-sm font-medium">{t('activity.type')}</legend>
        <OptionGroup
          size="touch"
          value={selectedTypeId}
          onValueChange={(next) => next && setTypeId(next)}
          options={activeTypes.map((type) => ({
            value: type.id,
            label: type.name,
            icon: <ActivityTypeIcon icon={type.icon} color={type.color} className="size-6" />,
          }))}
        />
      </fieldset>
      <fieldset className="grid gap-2">
        <legend className="mb-2 text-sm font-medium">{t('activity.when')}</legend>
        <OptionGroup
          size="touch"
          value={when}
          onValueChange={(next) => next && setWhen(next)}
          options={(['due', 'done'] as const).map((option) => ({ value: option, label: t(`activity.whenOptions.${option}`) }))}
        />
        {when === 'due' && (
          <Field label={t('activity.dueAt')} hideLabel>
            {(fieldControl) => <Input
              {...fieldControl}
              type="datetime-local"
              value={dueAt}
              onChange={(event) => setDueAt(event.target.value)}
              className="h-11 text-base"
            />}
          </Field>
        )}
      </fieldset>
      <RecordPicker value={record} onChange={setRecord} />
      <DictationField
        id={ids.description}
        label={t('activity.description')}
        value={description}
        onChange={setDescription}
        maxLength={20_000}
      />
      <FormActions pending={pending} disabled={!selectedTypeId} label={t('activity.submit')} />
    </FormBody>
  )
}
