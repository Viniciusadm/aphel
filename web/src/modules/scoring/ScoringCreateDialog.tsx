import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError } from '@/components/feedback'
import { DialogForm } from '@/components/ui/dialog-form'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { NativeSelect } from '@/sdk'
import { scoringApi } from './api'
import { useScoringRecordTypeLabel, useScoringRecordTypes } from './labels'
import { NAME_MAX_LENGTH } from './scoring-form'
import { DEFAULT_HOT_FROM, DEFAULT_WARM_FROM, SCORING_CORE_RECORD_TYPES, type ScoringModel } from './types'

interface ScoringCreateDialogProps {
  open: boolean
  hasPrimary: (recordType: string) => boolean
  onOpenChange: (open: boolean) => void
  onCreated: (model: ScoringModel) => void
}

export function ScoringCreateDialog({ open, onOpenChange, ...props }: ScoringCreateDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="md">{open && <CreateForm onOpenChange={onOpenChange} {...props} />}</DialogContent>
    </Dialog>
  )
}

function CreateForm({ hasPrimary, onOpenChange, onCreated }: Omit<ScoringCreateDialogProps, 'open'>) {
  const { t } = useTranslation('scoring')
  const [name, setName] = useState('')
  const [recordType, setRecordType] = useState<string>(SCORING_CORE_RECORD_TYPES[0])
  const [error, setError] = useState<string | null>(null)
  const [saving, setSaving] = useState(false)
  const recordTypes = useScoringRecordTypes()
  const recordTypeLabel = useScoringRecordTypeLabel()

  const submit = async () => {
    if (!name.trim()) {
      setError(t('dialog.errors.required'))
      return
    }
    setError(null)
    setSaving(true)
    try {
      const model = await scoringApi.create({
        record_type: recordType,
        name: name.trim(),
        is_primary: !hasPrimary(recordType),
        is_active: true,
        warm_from: DEFAULT_WARM_FROM,
        hot_from: DEFAULT_HOT_FROM,
        criteria: [],
      })
      onCreated(model)
    } catch (failure) {
      notifyError(failure)
    } finally {
      setSaving(false)
    }
  }

  return (
    <DialogForm
      noValidate
      onSubmit={(event) => {
        event.preventDefault()
        void submit()
      }}
      title={t('create.title')}
      description={t('create.description')}
      cancelLabel={t('dialog.cancel')}
      submitLabel={saving ? t('create.saving') : t('create.submit')}
      onCancel={() => onOpenChange(false)}
      submitDisabled={saving}
    >
      <Field label={t('dialog.name')} error={error}>
        {(control) => (
          <Input
            {...control}
            autoFocus
            value={name}
            maxLength={NAME_MAX_LENGTH}
            placeholder={t('placeholders.name')}
            onChange={(event) => setName(event.target.value)}
          />
        )}
      </Field>
      <Field label={t('dialog.recordType')}>
        {(control) => (
          <NativeSelect {...control} value={recordType} onChange={(event) => setRecordType(event.target.value)}>
            {recordTypes.map((type) => (
              <option key={type} value={type}>
                {recordTypeLabel(type)}
              </option>
            ))}
          </NativeSelect>
        )}
      </Field>
    </DialogForm>
  )
}
