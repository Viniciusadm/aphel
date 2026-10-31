import { useQueryClient } from '@tanstack/react-query'
import { CircleAlert } from 'lucide-react'
import { useId, useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useSearchParams } from 'react-router'
import { NETWORK_ERROR_CODE, isApiError } from '@/api'
import { storeSession } from '@/app/session/session-query'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { logout } from '../api'
import { AuthCard } from '../components/AuthCard'
import { SubmitButton } from '../components/SubmitButton'
import { REDIRECT_PARAM, withRedirect } from '../lib/redirect'
import { INVALID_TWO_FACTOR_CODE, submitTwoFactorChallenge } from './api'
import { Field } from '@/components/ui/field'

type ChallengeMode = 'totp' | 'recovery'
type ChallengeError = 'required' | 'totpFormat' | 'invalidCode' | 'rateLimited' | 'network' | 'generic'

const TOTP_PATTERN = /^\d{6}$/

function challengeError(error: unknown): ChallengeError | 'expired' {
  if (!isApiError(error)) return 'generic'
  if (error.code === NETWORK_ERROR_CODE) return 'network'
  if (error.code === INVALID_TWO_FACTOR_CODE) return 'invalidCode'
  if (error.status === 401) return 'expired'
  if (error.status === 429) return 'rateLimited'
  return 'generic'
}

export function TwoFactorChallengePage() {
  const { t } = useTranslation('two-factor')
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const [searchParams] = useSearchParams()
  const redirect = searchParams.get(REDIRECT_PARAM)
  const loginPath = withRedirect('/login', redirect)
  const inputId = useId()
  const errorId = `${inputId}-error`
  const [mode, setMode] = useState<ChallengeMode>('totp')
  const [code, setCode] = useState('')
  const [error, setError] = useState<ChallengeError | null>(null)
  const [expired, setExpired] = useState(false)
  const [pending, setPending] = useState(false)

  const switchMode = () => {
    setMode((current) => (current === 'totp' ? 'recovery' : 'totp'))
    setCode('')
    setError(null)
  }

  const cancel = async () => {
    await logout().catch(() => undefined)
    void navigate(loginPath, { replace: true })
  }

  const onSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    const value = code.trim()
    if (!value) {
      setError('required')
      return
    }
    if (mode === 'totp' && !TOTP_PATTERN.test(value.replace(/\s/g, ''))) {
      setError('totpFormat')
      return
    }
    setPending(true)
    setError(null)
    try {
      storeSession(queryClient, await submitTwoFactorChallenge(value))
    } catch (cause) {
      const next = challengeError(cause)
      if (next === 'expired') setExpired(true)
      else setError(next)
      setCode('')
    } finally {
      setPending(false)
    }
  }

  if (expired) {
    return (
      <AuthCard title={t('challenge.expiredTitle')} description={t('challenge.expiredDescription')}>
        <Button asChild className="w-full">
          <Link to={loginPath} replace>
            {t('challenge.backToLogin')}
          </Link>
        </Button>
      </AuthCard>
    )
  }

  const recovery = mode === 'recovery'

  return (
    <AuthCard
      title={t('challenge.title')}
      description={recovery ? t('challenge.recoveryDescription') : t('challenge.description')}
      footer={
        <Button type="button" variant="link" size="sm" onClick={() => void cancel()}>
          {t('challenge.cancel')}
        </Button>
      }
    >
      <form noValidate onSubmit={(event) => void onSubmit(event)} className="grid gap-4">
        <Field label={recovery ? t('challenge.recoveryLabel') : t('challenge.codeLabel')}>
          {(control) => <>
            <Input
            {...control}
            key={mode}
            value={code}
            onChange={(event) => setCode(event.target.value)}
            autoFocus
            autoComplete={recovery ? 'off' : 'one-time-code'}
            inputMode={recovery ? 'text' : 'numeric'}
            spellCheck={false}
            autoCapitalize="none"
            maxLength={recovery ? 16 : 7}
            placeholder={recovery ? 'xxxxx-xxxxx' : '123456'}
            className="font-mono tracking-widest"
            aria-invalid={error ? true : undefined}
            aria-describedby={error ? errorId : undefined}
          />
          {error && (
            <p id={errorId} role="alert" className="flex items-start gap-1.5 text-sm text-destructive">
              <CircleAlert className="mt-0.5 size-4 shrink-0" aria-hidden />
              <span>{t(`challenge.errors.${error}`)}</span>
            </p>
          )}
          </>}
        </Field>
        <SubmitButton pending={pending} pendingLabel={t('challenge.submitting')}>
          {t('challenge.submit')}
        </SubmitButton>
        <Button type="button" variant="ghost" size="sm" className="justify-self-center" onClick={switchMode}>
          {recovery ? t('challenge.useAuthenticator') : t('challenge.useRecoveryCode')}
        </Button>
      </form>
    </AuthCard>
  )
}
