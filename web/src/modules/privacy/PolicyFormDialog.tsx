import { DialogForm } from '@/components/ui/dialog-form'
import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { useId, useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { notifySuccess } from '@/components/feedback'
import {
  Dialog,
  DialogContent,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { NativeSelect } from '@/sdk'
import {
  MAX_THRESHOLD_DAYS,
  MIN_THRESHOLD_DAYS,
  RETENTION_ACTIONS,
  usePrivacyCatalog,
  isRetentionAction,
  needsRecordType,
  usePolicyMutations,
  type RetentionAction,
  type RetentionPolicy,
} from './api'
import { localizedLabel, usePrivacyErrors } from './lib/shared'
import { Field } from '@/components/ui/field'

interface PolicyFormDialogProps {
  policy: RetentionPolicy | null
  onClose: () => void
}

const NAME_MAX_LENGTH = 140

export function PolicyFormDialog({ policy, onClose }: PolicyFormDialogProps) {
  const { t, i18n } = useTranslation('privacy')
  const { create, update } = usePolicyMutations()
  const notifyFailure = usePrivacyErrors()
  const ids = { name: useId(), action: useId(), recordType: useId(), days: useId(), daysHint: useId(), error: useId() }
  const [name, setName] = useState(policy?.name ?? '')
  const [action, setAction] = useState<RetentionAction>(
    policy && isRetentionAction(policy.action) ? policy.action : 'archive_inactive',
  )
  const [selectedRecordType, setRecordType] = useState(policy?.record_type ?? '')
  const catalog = usePrivacyCatalog()
  const recordTypeOptions = (catalog.data?.entities ?? []).filter((entity) => entity.retention)
    .map((entity) => ({ value: entity.record_type, label: localizedLabel(entity.labels, i18n.language, entity.record_type) }))
  const recordType = selectedRecordType || recordTypeOptions[0]?.value || ''
  if (recordType && !recordTypeOptions.some((option) => option.value === recordType)) {
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
        <DialogForm
          onSubmit={submit}
          noValidate
          title={t(policy ? 'policyForm.editTitle' : 'policyForm.createTitle')}
          description={t('policyForm.description')}
          cancelLabel={t('actions.cancel')}
          submitLabel={pending ? t('actions.saving') : t('actions.save')}
          onCancel={onClose}
          pending={pending}
          submitDisabled={pending || (needsRecordType(action) && !recordType)}
        >
          {error && (
            <SharedFieldError id={ids.error}>
              {error}
            </SharedFieldError>
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
          <Field label={t('policyForm.action')}>
            {(control) => <>
              <NativeSelect
              {...control}
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
            </>}
          </Field>
          {needsRecordType(action) && (
            <Field label={t('policyForm.recordType')}>
              {(control) => <NativeSelect
                {...control}
                value={recordType}
                disabled={Boolean(policy)}
                onChange={(event) => setRecordType(event.target.value)}
              >
                {recordTypeOptions.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </NativeSelect>}
            </Field>
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
        </DialogForm>
      </DialogContent>
    </Dialog>
  )
}
