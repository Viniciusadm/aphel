import { DialogForm } from '@/components/ui/dialog-form'
import { NativeSelect } from '@/components/ui/native-select'
import { zodResolver } from '@hookform/resolvers/zod'
import { Controller, useForm, useWatch, type Control } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { notifySuccess } from '@/components/feedback'
import { Checkbox } from '@/components/ui/checkbox'
import {
  Dialog,
  DialogContent,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { PhoneFieldInput } from '@/components/fields/LazyPhoneFieldInput'
import { useRecordFormatting } from '@/sdk'
import { Textarea } from '@/components/ui/textarea'
import { useDistributionRules, useSaveMeetingType } from '../api'
import { matchServerField, serverFieldViolations } from '../lib/errors'
import { FieldMessage } from '../lib/FieldMessage'
import { LOCATION_KINDS, type MeetingTypeResponse } from '../types'
import {
  MAX_DURATION,
  MAX_SLOT_INTERVAL,
  MIN_DURATION,
  MIN_SLOT_INTERVAL,
  NO_DISTRIBUTION_RULE,
  defaultValues,
  meetingTypeFormSchema,
  serverFieldPaths,
  toRequest,
  type MeetingTypeFormValues,
} from './form'
import { QuestionsEditor } from './QuestionsEditor'
import { Field } from '@/components/ui/field'

interface MeetingTypeDialogProps {
  meetingType: MeetingTypeResponse | null
  onClose: () => void
}

function TeamLinkSection({ control }: { control: Control<MeetingTypeFormValues> }) {
  const { t } = useTranslation('booking')
  const rules = useDistributionRules()
  if (!rules.data) return null
  return (
    <fieldset className="grid gap-2 rounded-lg border p-3">
      <legend className="px-1 text-sm font-medium">{t('meetingTypes.form.team')}</legend>
      <p className="text-sm text-muted-foreground">{t('meetingTypes.form.teamHint')}</p>
      <Field label={t('meetingTypes.form.distributionRule')} className="max-w-sm">
        {(fieldControl) => <Controller
          control={control}
          name="distributionRuleId"
          render={({ field }) => (
            <NativeSelect value={field.value} onValueChange={field.onChange} id={fieldControl.id}>
                <option value={NO_DISTRIBUTION_RULE}>{t('meetingTypes.form.personalLink')}</option>
                {rules.data?.map((rule) => (
                  <option key={rule.id} value={rule.id}>
                    {rule.name}
                  </option>
                ))}
              </NativeSelect>
          )}
        />}
      </Field>
    </fieldset>
  )
}

export function MeetingTypeDialog({ meetingType, onClose }: MeetingTypeDialogProps) {
  const { t } = useTranslation('booking')
  const save = useSaveMeetingType(meetingType)
  const rules = useDistributionRules()
  const {
    control,
    register,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<MeetingTypeFormValues>({
    resolver: zodResolver(meetingTypeFormSchema),
    defaultValues: defaultValues(meetingType),
  })
  const locationKind = useWatch({ control, name: 'locationKind' })
  const formatting = useRecordFormatting()

  const onSubmit = handleSubmit(async (values) => {
    try {
      await save.mutateAsync(toRequest(values, Boolean(rules.data)))
      notifySuccess(t(meetingType ? 'meetingTypes.toast.updated' : 'meetingTypes.toast.created'))
      onClose()
    } catch (error) {
      let message = 'errors.generic'
      if (isApiError(error)) {
        if (error.code === 'meeting_type_slug_taken') {
          setError('slug', { type: 'server', message: 'errors.slugTaken' })
          return
        }
        if (error.code === 'version_conflict') message = 'errors.versionConflict'
        for (const violation of serverFieldViolations(error)) {
          const path = matchServerField(violation.field, serverFieldPaths)
          if (path) setError(path, { type: 'server', message: 'errors.invalid' })
          message = 'errors.invalid'
        }
      }
      setError('root.server', { type: 'server', message })
    }
  })

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent size="xl">
        <DialogForm
          onSubmit={onSubmit}
          noValidate
          title={t(meetingType ? 'meetingTypes.form.editTitle' : 'meetingTypes.form.createTitle')}
          description={t('meetingTypes.form.description')}
          cancelLabel={t('actions.cancel')}
          submitLabel={isSubmitting ? t('actions.saving') : t('actions.save')}
          onCancel={onClose}
          pending={isSubmitting}
          submitDisabled={isSubmitting}
        >
          <FieldMessage message={errors.root?.server?.message} />
          <Field label={t('meetingTypes.form.name')}>
            {(fieldControl) => <>
              <Input placeholder={t('placeholders.name')} id={fieldControl.id} aria-invalid={errors.name ? true : undefined} {...register('name')} />
            <FieldMessage message={errors.name?.message} />
            </>}
          </Field>
          <Field id="meeting-type-slug" label={t('meetingTypes.form.slug')}>
            <div className="flex min-w-0 items-center rounded-md border border-input bg-muted/40 shadow-xs">
              <span className="truncate pl-3 text-sm text-muted-foreground">{`${window.location.origin}/book/`}</span>
              <Input
                id="meeting-type-slug"
                className="min-w-24 flex-1 border-0 shadow-none"
                placeholder={meetingType ? undefined : t('meetingTypes.form.slugPlaceholder')}
                aria-invalid={errors.slug ? true : undefined}
                {...register('slug')}
              />
            </div>
            <p className="text-xs text-muted-foreground">{t('meetingTypes.form.slugHint')}</p>
            <FieldMessage message={errors.slug?.message} />
          </Field>
          <Field label={t('meetingTypes.form.descriptionLabel')}>
            {(fieldControl) => <>
              <Textarea placeholder={t('placeholders.description')} {...fieldControl} rows={3} {...register('description')} />
            <FieldMessage message={errors.description?.message} />
            </>}
          </Field>
          <div className="grid gap-4 sm:grid-cols-2">
            <Field label={t('meetingTypes.form.duration')}>
              {(control) => <>
                <Input placeholder={t('placeholders.duration')}
                {...control}
                type="number"
                min={MIN_DURATION}
                max={MAX_DURATION}
                aria-invalid={errors.durationMinutes ? true : undefined}
                {...register('durationMinutes', { valueAsNumber: true })}
              />
              <FieldMessage message={errors.durationMinutes?.message} />
              </>}
            </Field>
            <Field label={t('meetingTypes.form.slotInterval')}>
              {(control) => <>
                <Input placeholder={t('placeholders.interval')}
                {...control}
                type="number"
                min={MIN_SLOT_INTERVAL}
                max={MAX_SLOT_INTERVAL}
                aria-invalid={errors.slotIntervalMinutes ? true : undefined}
                {...register('slotIntervalMinutes', { valueAsNumber: true })}
              />
              <p className="text-xs text-muted-foreground">{t('meetingTypes.form.slotIntervalHint')}</p>
              <FieldMessage message={errors.slotIntervalMinutes?.message} />
              </>}
            </Field>
            <Field label={t('meetingTypes.form.locationKind')}>
              {(fieldControl) => <Controller
                control={control}
                name="locationKind"
                render={({ field }) => (
                  <NativeSelect value={field.value} onValueChange={field.onChange} id={fieldControl.id}>
                      {LOCATION_KINDS.map((kind) => (
                        <option key={kind} value={kind}>
                          {t(`locationKinds.${kind}`)}
                        </option>
                      ))}
                    </NativeSelect>
                )}
              />}
            </Field>
            {locationKind !== 'none' && (
              <Field label={t(`locationValueLabels.${locationKind}`)}>
                {(fieldControl) => <>
                  {locationKind === 'phone' ? (
                    <Controller
                      control={control}
                      name="locationValue"
                      render={({ field }) => (
                        <PhoneFieldInput
                          id={fieldControl.id}
                          locale={formatting.locale}
                          fieldLabel={t('locationValueLabels.phone')}
                          invalid={Boolean(errors.locationValue)}
                          value={field.value || null}
                          onBlur={field.onBlur}
                          onChange={(next) => field.onChange(next ?? '')}
                        />
                      )}
                    />
                  ) : (
                    <Input placeholder={t('placeholders.locationValue')}
                      id={fieldControl.id}
                      aria-invalid={errors.locationValue ? true : undefined}
                      {...register('locationValue')}
                    />
                  )}
                <p className="text-xs text-muted-foreground">{t('meetingTypes.form.locationValueHint')}</p>
                <FieldMessage message={errors.locationValue?.message} />
                </>}
              </Field>
            )}
          </div>
          <div className="grid gap-3">
            <Controller
              control={control}
              name="spamProtection"
              render={({ field }) => (
                <label className="flex items-center gap-2 text-sm">
                  <Checkbox checked={field.value} onCheckedChange={(checked) => field.onChange(checked === true)} />
                  {t('meetingTypes.form.spamProtection')}
                </label>
              )}
            />
            <Controller
              control={control}
              name="isActive"
              render={({ field }) => (
                <label className="flex items-center gap-2 text-sm">
                  <Checkbox checked={field.value} onCheckedChange={(checked) => field.onChange(checked === true)} />
                  {t('meetingTypes.form.active')}
                </label>
              )}
            />
          </div>
          <TeamLinkSection control={control} />
          <QuestionsEditor control={control} register={register} errors={errors} />
        </DialogForm>
      </DialogContent>
    </Dialog>
  )
}
