import { useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import {
  INVALID_CURRENT_PASSWORD,
  INVALID_TWO_FACTOR_CODE,
  hasErrorCode,
  twoFactorApi,
  useRefreshAfterTwoFactorChange,
} from './api'
import { RecoveryCodesView } from './RecoveryCodesView'
import { codeInputProps } from './code-input'
import { TextField } from './TextField'

type ConfirmWith = 'password' | 'code'

interface RegenerateDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function RegenerateDialog({ open, onOpenChange }: RegenerateDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        {open && <RegenerateFlow onClose={() => onOpenChange(false)} />}
      </DialogContent>
    </Dialog>
  )
}

function RegenerateFlow({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation('two-factor')
  const refresh = useRefreshAfterTwoFactorChange()
  const [confirmWith, setConfirmWith] = useState<ConfirmWith>('password')
  const [value, setValue] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [pending, setPending] = useState(false)
  const [codes, setCodes] = useState<string[] | null>(null)

  const toggle = () => {
    setConfirmWith((current) => (current === 'password' ? 'code' : 'password'))
    setValue('')
    setError(null)
  }

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (!value.trim()) {
      setError(t('errors.required'))
      return
    }
    setPending(true)
    setError(null)
    try {
      const result = await twoFactorApi.regenerate(
        confirmWith === 'password' ? { password: value } : { code: value.trim() },
      )
      setCodes(result.recovery_codes)
      await refresh()
    } catch (cause) {
      if (hasErrorCode(cause, INVALID_CURRENT_PASSWORD)) setError(t('errors.invalidPassword'))
      else if (hasErrorCode(cause, INVALID_TWO_FACTOR_CODE)) setError(t('errors.invalidCode'))
      else notifyError(cause)
    } finally {
      setPending(false)
    }
  }

  if (codes) {
    return (
      <>
        <DialogHeader>
          <DialogTitle>{t('recoveryCodes.title')}</DialogTitle>
          <DialogDescription>{t('recoveryCodes.saveNow')}</DialogDescription>
        </DialogHeader>
        <RecoveryCodesView codes={codes} />
        <DialogFooter>
          <Button type="button" onClick={onClose}>
            {t('recoveryCodes.done')}
          </Button>
        </DialogFooter>
      </>
    )
  }

  const usingPassword = confirmWith === 'password'

  return (
    <form noValidate onSubmit={(event) => void submit(event)} className="grid gap-4">
      <DialogHeader>
        <DialogTitle>{t('regenerate.title')}</DialogTitle>
        <DialogDescription>{t('regenerate.description')}</DialogDescription>
      </DialogHeader>
      {usingPassword ? (
        <TextField
          key="password"
          label={t('fields.password')}
          type="password"
          autoComplete="current-password"
          value={value}
          onChange={(event) => setValue(event.target.value)}
          error={error}
        />
      ) : (
        <TextField
          key="code"
          label={t('fields.code')}
          hint={t('fields.codeHint')}
          value={value}
          onChange={(event) => setValue(event.target.value)}
          error={error}
          {...codeInputProps}
        />
      )}
      <Button type="button" variant="link" size="sm" className="justify-self-start px-0" onClick={toggle}>
        {usingPassword ? t('regenerate.useCode') : t('regenerate.usePassword')}
      </Button>
      <DialogFooter>
        <Button type="button" variant="outline" onClick={onClose}>
          {t('actions.cancel')}
        </Button>
        <Button type="submit" disabled={pending} aria-busy={pending || undefined}>
          {pending ? t('regenerate.submitting') : t('regenerate.submit')}
        </Button>
      </DialogFooter>
    </form>
  )
}
