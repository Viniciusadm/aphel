import { useQuery } from '@tanstack/react-query'
import { useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { INVALID_TWO_FACTOR_CODE, hasErrorCode, twoFactorApi, twoFactorKeys, useRefreshAfterTwoFactorChange } from './api'
import { RecoveryCodesView } from './RecoveryCodesView'
import { codeInputProps } from './code-input'
import { TextField } from './TextField'

const TOTP_PATTERN = /^\d{6}$/

interface SetupDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function SetupDialog({ open, onOpenChange }: SetupDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-lg">
        {open && <SetupFlow onClose={() => onOpenChange(false)} />}
      </DialogContent>
    </Dialog>
  )
}

function formatSecret(secret: string): string {
  return secret.match(/.{1,4}/g)?.join(' ') ?? secret
}

function SetupFlow({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation('two-factor')
  const refresh = useRefreshAfterTwoFactorChange()
  const [code, setCode] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [pending, setPending] = useState(false)
  const [recoveryCodes, setRecoveryCodes] = useState<string[] | null>(null)
  const setup = useQuery({
    queryKey: twoFactorKeys.setup(),
    queryFn: twoFactorApi.setup,
    gcTime: 0,
    staleTime: Infinity,
    retry: false,
    refetchOnWindowFocus: false,
  })

  const confirm = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    const value = code.replace(/\s/g, '')
    if (!TOTP_PATTERN.test(value)) {
      setError(t('errors.totpFormat'))
      return
    }
    setPending(true)
    setError(null)
    try {
      const result = await twoFactorApi.confirm(value)
      setRecoveryCodes(result.recovery_codes)
      notifySuccess(t('setup.enabled'))
      await refresh()
    } catch (cause) {
      if (hasErrorCode(cause, INVALID_TWO_FACTOR_CODE)) setError(t('errors.invalidCode'))
      else notifyError(cause)
    } finally {
      setPending(false)
    }
  }

  if (recoveryCodes) {
    return (
      <>
        <DialogHeader>
          <DialogTitle>{t('recoveryCodes.title')}</DialogTitle>
          <DialogDescription>{t('recoveryCodes.saveNow')}</DialogDescription>
        </DialogHeader>
        <RecoveryCodesView codes={recoveryCodes} />
        <DialogFooter>
          <Button type="button" onClick={onClose}>
            {t('recoveryCodes.done')}
          </Button>
        </DialogFooter>
      </>
    )
  }

  return (
    <>
      <DialogHeader>
        <DialogTitle>{t('setup.title')}</DialogTitle>
        <DialogDescription>{t('setup.description')}</DialogDescription>
      </DialogHeader>
      {setup.isPending ? (
        <div className="grid justify-items-center gap-3" aria-busy="true">
          <Skeleton className="size-56" />
          <Skeleton className="h-4 w-48" />
        </div>
      ) : setup.isError ? (
        <ErrorState onRetry={() => void setup.refetch()} />
      ) : (
        <form noValidate onSubmit={(event) => void confirm(event)} className="grid gap-4">
          <ol className="grid list-decimal gap-1 pl-5 text-sm text-muted-foreground">
            <li>{t('setup.stepScan')}</li>
            <li>{t('setup.stepConfirm')}</li>
          </ol>
          <div className="grid justify-items-center gap-2">
            <img
              src={`data:image/svg+xml;utf8,${encodeURIComponent(setup.data.qr_code_svg)}`}
              alt={t('setup.qrAlt')}
              width={220}
              height={220}
              className="size-56 max-w-full rounded-md border bg-white p-2"
            />
          </div>
          <details className="rounded-md border p-3 text-sm">
            <summary className="cursor-pointer font-medium">{t('setup.manualTitle')}</summary>
            <p className="mt-2 text-muted-foreground">{t('setup.manualDescription')}</p>
            <code className="mt-2 block rounded-md bg-muted p-2 font-mono text-xs break-all" aria-label={t('setup.secretLabel')}>
              {formatSecret(setup.data.secret)}
            </code>
          </details>
          <TextField
            label={t('setup.codeLabel')}
            value={code}
            onChange={(event) => setCode(event.target.value)}
            error={error}
            inputMode="numeric"
            placeholder="123456"
            {...codeInputProps}
          />
          <DialogFooter>
            <Button type="button" variant="outline" onClick={onClose}>
              {t('actions.cancel')}
            </Button>
            <Button type="submit" disabled={pending} aria-busy={pending || undefined}>
              {pending ? t('setup.confirming') : t('setup.confirm')}
            </Button>
          </DialogFooter>
        </form>
      )}
    </>
  )
}
