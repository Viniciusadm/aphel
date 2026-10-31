import { useId, useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
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
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { useCustomObjectsAdmin } from '@/features/settings/objects/api'
import {
  MAX_THRESHOLD_DAYS,
  MIN_THRESHOLD_DAYS,
  RETENTION_ACTIONS,
  RETENTION_RECORD_TYPES,
  isRetentionAction,
  needsRecordType,
  usePolicyMutations,
  type RetentionAction,
  type RetentionPolicy,
} from './api'
import { usePrivacyErrors } from './lib/shared'
import { Field } from '@/components/ui/field'

interface PolicyFormDialogProps {
  policy: RetentionPolicy | null
  onClose: () => void
}

const NAME_MAX_LENGTH = 140

export function PolicyFormDialog({ policy, onClose }: PolicyFormDialogProps) {
  const { t } = useTranslation('privacy')
  const { create, update } = usePolicyMutations()
  const notifyFailure = usePrivacyErrors()
  const ids = { name: useId(), action: useId(), recordType: useId(), days: useId(), daysHint: useId(), error: useId() }
  const [name, setName] = useState(policy?.name ?? '')
  const [action, setAction] = useState<RetentionAction>(
    policy && isRetentionAction(policy.action) ? policy.action : 'archive_inactive',
  )
  const [recordType, setRecordType] = useState(policy?.record_type ?? 'contact')
  const customObjects = useCustomObjectsAdmin()
  const recordTypeOptions = [
    ...RETENTION_RECORD_TYPES.map((value) => ({ value: value as string, label: t(`recordTypesPlural.${value}`) })),
    ...(customObjects.data ?? [])
      .filter((object) => object.is_active || object.record_type === recordType)
      .map((object) => ({ value: object.record_type, label: object.label_plural })),
  ]
  if (!recordTypeOptions.some((option) => option.value === recordType)) {
    recordTypeOptions.push({ value: recordType, label: recordType })
  }
  const [days, setDays] = useState(String(policy?.threshold_days ?? 1095))
  const [error, setError] = useState<string | null>(null)
  const pending = create.isPending || update.isPending

  const submit = (event: FormEvent) => {
    event.preventDefault()
    const threshold = Number(days)
    if (!name.trim() || name.trim().length > NAME_MAX_LENGTH) {
      setError(t('policyForm.nameInvalid'))
      return
    }
    if (!Number.isInteger(threshold) || threshold < MIN_THRESHOLD_DAYS || threshold > MAX_THRESHOLD_DAYS) {
      setError(t('policyForm.daysInvalid', { min: MIN_THRESHOLD_DAYS, max: MAX_THRESHOLD_DAYS }))
      return
    }
    setError(null)
    const done = () => {
      notifySuccess(t(policy ? 'policyForm.updated' : 'policyForm.created'))
      onClose()
    }
    if (policy) {
      update.mutate(
        { id: policy.id, body: { name: name.trim(), threshold_days: threshold } },
        { onSuccess: done, onError: notifyFailure },
      )
    } else {
      create.mutate(
        {
          name: name.trim(),
          action,
          record_type: needsRecordType(action) ? recordType : undefined,
          threshold_days: threshold,
        },
        { onSuccess: done, onError: notifyFailure },
      )
    }
  }

  return (
    <Dialog open onOpenChange={(open) => !open && !pending && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t(policy ? 'policyForm.editTitle' : 'policyForm.createTitle')}</DialogTitle>
          <DialogDescription>{t('policyForm.description')}</DialogDescription>
        </DialogHeader>
        <form className="grid gap-4" onSubmit={submit} noValidate>
          {error && (
            <p id={ids.error} role="alert" className="text-sm text-destructive">
              {error}
            </p>
          )}
          <Field label={t('policyForm.name')}>
            {(control) => <Input placeholder={t('placeholders.name')}
              {...control}
              value={name}
              maxLength={NAME_MAX_LENGTH}
              required
              onChange={(event) => setName(event.target.value)}
            />}
          </Field>
          <div className="grid gap-2">
            <Label htmlFor={ids.action}>{t('policyForm.action')}</Label>
            <NativeSelect
              id={ids.action}
              value={action}
              disabled={Boolean(policy)}
              onChange={(event) => isRetentionAction(event.target.value) && setAction(event.target.value)}
            >
              {RETENTION_ACTIONS.map((value) => (
                <option key={value} value={value}>
                  {t(`retentionActions.${value}`)}
                </option>
              ))}
            </NativeSelect>
            <p className="text-xs text-muted-foreground">{t(`actionHints.${action}`)}</p>
          </div>
          {needsRecordType(action) && (
            <div className="grid gap-2">
              <Label htmlFor={ids.recordType}>{t('policyForm.recordType')}</Label>
              <NativeSelect
                id={ids.recordType}
                value={recordType}
                disabled={Boolean(policy)}
                onChange={(event) => setRecordType(event.target.value)}
              >
                {recordTypeOptions.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </NativeSelect>
            </div>
          )}
          <Field label={t('policyForm.days')}>
            {(control) => <>
              <Input placeholder={t('placeholders.days')}
              {...control}
              type="number"
              inputMode="numeric"
              min={MIN_THRESHOLD_DAYS}
              max={MAX_THRESHOLD_DAYS}
              value={days}
              required
              aria-describedby={ids.daysHint}
              onChange={(event) => setDays(event.target.value)}
            />
            <p id={ids.daysHint} className="text-xs text-muted-foreground">
              {t(policy ? 'policyForm.daysHintEdit' : 'policyForm.daysHint')}
            </p>
            </>}
          </Field>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={onClose} disabled={pending}>
              {t('actions.cancel')}
            </Button>
            <Button type="submit" disabled={pending} aria-busy={pending}>
              {pending ? t('actions.saving') : t('actions.save')}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
