import { useQueryClient } from '@tanstack/react-query'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { zonedInputToUtc } from '@/components/fields/lib/datetime'
import { notifyError } from '@/components/feedback/notify'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { activitiesKeys, useActivityTypes } from '../api'
import { ActivityTypeIcon } from '../components/ActivityTypeIcon'
import { useRecordFormatting } from '@/features/records/formatting'
import { cn } from '@/lib/utils'
import { submitActivity } from './api'
import { DictationField } from '@/features/mobile/components/DictationField'
import { RecordPicker, type PickedRecord } from '@/features/mobile/components/RecordPicker'
import { useOutboxFeedback } from '@/features/mobile/lib/outbox-feedback'
import { FormActions, FormBody, type QuickFormProps } from '@/features/mobile/components/QuickForm'

type ActivityWhen = 'done' | 'due'

export interface ActivityQuickFormProps extends QuickFormProps {
  initialRecord: PickedRecord | null
}

export function ActivityQuickForm({ onDone, initialRecord }: ActivityQuickFormProps) {
  const { t } = useTranslation('activities', { keyPrefix: 'quickAdd' })
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
      <div className="grid gap-2">
        <Label htmlFor={ids.title}>{t('activity.title')}</Label>
        <Input
          id={ids.title}
          value={title}
          onChange={(event) => setTitle(event.target.value)}
          className="h-11 text-base"
          maxLength={255}
          aria-invalid={touched && titleMissing}
        />
        {touched && titleMissing && <p className="text-sm text-destructive">{t('activity.titleRequired')}</p>}
      </div>
      <fieldset className="grid gap-2">
        <legend className="mb-2 text-sm font-medium">{t('activity.type')}</legend>
        <div className="flex flex-wrap gap-2">
          {activeTypes.map((type) => (
            <button
              key={type.id}
              type="button"
              aria-pressed={selectedTypeId === type.id}
              onClick={() => setTypeId(type.id)}
              className={cn(
                'inline-flex min-h-11 items-center gap-2 rounded-full border px-3 text-sm',
                selectedTypeId === type.id ? 'border-primary bg-primary/10 font-medium' : 'bg-background',
              )}
            >
              <ActivityTypeIcon icon={type.icon} color={type.color} className="size-6" />
              {type.name}
            </button>
          ))}
        </div>
      </fieldset>
      <fieldset className="grid gap-2">
        <legend className="mb-2 text-sm font-medium">{t('activity.when')}</legend>
        <div className="grid grid-cols-2 gap-2">
          {(['due', 'done'] as const).map((option) => (
            <button
              key={option}
              type="button"
              aria-pressed={when === option}
              onClick={() => setWhen(option)}
              className={cn(
                'min-h-11 rounded-md border px-3 text-sm',
                when === option ? 'border-primary bg-primary/10 font-medium' : 'bg-background',
              )}
            >
              {t(`activity.whenOptions.${option}`)}
            </button>
          ))}
        </div>
        {when === 'due' && (
          <div className="grid gap-2">
            <Label htmlFor={ids.due} className="sr-only">
              {t('activity.dueAt')}
            </Label>
            <Input
              id={ids.due}
              type="datetime-local"
              value={dueAt}
              onChange={(event) => setDueAt(event.target.value)}
              className="h-11 text-base"
            />
          </div>
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
