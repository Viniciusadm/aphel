import { zodResolver } from '@hookform/resolvers/zod'
import { Controller, useForm, useWatch, type Control } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
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

interface MeetingTypeDialogProps {
  meetingType: MeetingTypeResponse | null
  onClose: () => void
}

function TeamLinkSection({ control }: { control: Control<MeetingTypeFormValues> }) {
  const { t } = useTranslation('scheduling')
  const rules = useDistributionRules()
  if (!rules.data) return null
  return (
    <fieldset className="grid gap-2 rounded-lg border p-3">
      <legend className="px-1 text-sm font-medium">{t('meetingTypes.form.team')}</legend>
      <p className="text-sm text-muted-foreground">{t('meetingTypes.form.teamHint')}</p>
      <div className="grid max-w-sm gap-1">
        <Label htmlFor="meeting-type-rule">{t('meetingTypes.form.distributionRule')}</Label>
        <Controller
          control={control}
          name="distributionRuleId"
          render={({ field }) => (
            <Select value={field.value} onValueChange={field.onChange}>
              <SelectTrigger id="meeting-type-rule">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value={NO_DISTRIBUTION_RULE}>{t('meetingTypes.form.personalLink')}</SelectItem>
                {rules.data?.map((rule) => (
                  <SelectItem key={rule.id} value={rule.id}>
                    {rule.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          )}
        />
      </div>
    </fieldset>
  )
}

export function MeetingTypeDialog({ meetingType, onClose }: MeetingTypeDialogProps) {
  const { t } = useTranslation('scheduling')
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
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>{t(meetingType ? 'meetingTypes.form.editTitle' : 'meetingTypes.form.createTitle')}</DialogTitle>
          <DialogDescription>{t('meetingTypes.form.description')}</DialogDescription>
        </DialogHeader>
        <form className="grid gap-5" onSubmit={onSubmit} noValidate>
          <FieldMessage message={errors.root?.server?.message} />
          <div className="grid gap-1">
            <Label htmlFor="meeting-type-name">{t('meetingTypes.form.name')}</Label>
            <Input placeholder={t('placeholders.name')} id="meeting-type-name" aria-invalid={errors.name ? true : undefined} {...register('name')} />
            <FieldMessage message={errors.name?.message} />
          </div>
          <div className="grid gap-1">
            <Label htmlFor="meeting-type-slug">{t('meetingTypes.form.slug')}</Label>
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
          </div>
          <div className="grid gap-1">
            <Label htmlFor="meeting-type-description">{t('meetingTypes.form.descriptionLabel')}</Label>
            <Textarea placeholder={t('placeholders.description')} id="meeting-type-description" rows={3} {...register('description')} />
            <FieldMessage message={errors.description?.message} />
          </div>
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="grid gap-1">
              <Label htmlFor="meeting-type-duration">{t('meetingTypes.form.duration')}</Label>
              <Input placeholder={t('placeholders.duration')}
                id="meeting-type-duration"
                type="number"
                min={MIN_DURATION}
                max={MAX_DURATION}
                aria-invalid={errors.durationMinutes ? true : undefined}
                {...register('durationMinutes', { valueAsNumber: true })}
              />
              <FieldMessage message={errors.durationMinutes?.message} />
            </div>
            <div className="grid gap-1">
              <Label htmlFor="meeting-type-interval">{t('meetingTypes.form.slotInterval')}</Label>
              <Input placeholder={t('placeholders.interval')}
                id="meeting-type-interval"
                type="number"
                min={MIN_SLOT_INTERVAL}
                max={MAX_SLOT_INTERVAL}
                aria-invalid={errors.slotIntervalMinutes ? true : undefined}
                {...register('slotIntervalMinutes', { valueAsNumber: true })}
              />
              <p className="text-xs text-muted-foreground">{t('meetingTypes.form.slotIntervalHint')}</p>
              <FieldMessage message={errors.slotIntervalMinutes?.message} />
            </div>
            <div className="grid gap-1">
              <Label htmlFor="meeting-type-location-kind">{t('meetingTypes.form.locationKind')}</Label>
              <Controller
                control={control}
                name="locationKind"
                render={({ field }) => (
                  <Select value={field.value} onValueChange={field.onChange}>
                    <SelectTrigger id="meeting-type-location-kind">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {LOCATION_KINDS.map((kind) => (
                        <SelectItem key={kind} value={kind}>
                          {t(`locationKinds.${kind}`)}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                )}
              />
            </div>
            {locationKind !== 'none' && (
              <div className="grid gap-1">
                <Label htmlFor="meeting-type-location-value">{t(`locationValueLabels.${locationKind}`)}</Label>
                <Input placeholder={t('placeholders.locationValue')}
                  id="meeting-type-location-value"
                  aria-invalid={errors.locationValue ? true : undefined}
                  {...register('locationValue')}
                />
                <p className="text-xs text-muted-foreground">{t('meetingTypes.form.locationValueHint')}</p>
                <FieldMessage message={errors.locationValue?.message} />
              </div>
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
          <DialogFooter>
            <Button type="button" variant="outline" onClick={onClose} disabled={isSubmitting}>
              {t('actions.cancel')}
            </Button>
            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting ? t('actions.saving') : t('actions.save')}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
