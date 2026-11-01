import { DialogForm } from '@/components/ui/dialog-form'
import { useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError, notifySuccess } from '@/components/feedback'
import {
  Dialog,
  DialogContent,
} from '@/components/ui/dialog'
import {
  INVALID_CURRENT_PASSWORD,
  INVALID_TWO_FACTOR_CODE,
  hasErrorCode,
  twoFactorApi,
  useRefreshAfterTwoFactorChange,
} from './api'
import { codeInputProps } from './code-input'
import { TextField } from './TextField'

interface DisableDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function DisableDialog({ open, onOpenChange }: DisableDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="sm">{open && <DisableForm onClose={() => onOpenChange(false)} />}</DialogContent>
    </Dialog>
  )
}

interface DisableErrors {
  password?: string
  code?: string
}

function DisableForm({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation('two-factor')
  const refresh = useRefreshAfterTwoFactorChange()
  const [password, setPassword] = useState('')
  const [code, setCode] = useState('')
  const [errors, setErrors] = useState<DisableErrors>({})
  const [pending, setPending] = useState(false)

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    const next: DisableErrors = {}
    if (!password) next.password = t('errors.required')
    if (!code.trim()) next.code = t('errors.required')
    setErrors(next)
    if (next.password || next.code) return
    setPending(true)
    try {
      await twoFactorApi.disable({ password, code: code.trim() })
      notifySuccess(t('disable.done'))
      await refresh()
      onClose()
    } catch (cause) {
      if (hasErrorCode(cause, INVALID_CURRENT_PASSWORD)) setErrors({ password: t('errors.invalidPassword') })
      else if (hasErrorCode(cause, INVALID_TWO_FACTOR_CODE)) setErrors({ code: t('errors.invalidCode') })
      else notifyError(cause)
    } finally {
      setPending(false)
    }
  }

  return (
    <DialogForm
      submitVariant="destructive"
      noValidate
      onSubmit={(event) => void submit(event)}
      className="grid gap-4"
      title={t('disable.title')}
      description={t('disable.description')}
      cancelLabel={t('actions.cancel')}
      submitLabel={pending ? t('disable.submitting') : t('disable.submit')}
      onCancel={onClose}
      pending={pending}
      submitDisabled={pending}
    >
      <TextField placeholder={t('fields.passwordPlaceholder')}
        label={t('fields.password')}
        type="password"
        autoComplete="current-password"
        value={password}
        onChange={(event) => setPassword(event.target.value)}
        error={errors.password}
      />
      <TextField placeholder={t('fields.codePlaceholder')}
        label={t('fields.code')}
        hint={t('fields.codeHint')}
        value={code}
        onChange={(event) => setCode(event.target.value)}
        error={errors.code}
        {...codeInputProps}
      />
    </DialogForm>
  )
}
