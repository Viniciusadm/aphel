import { useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useForm } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useSearchParams } from 'react-router'
import { isApiError } from '@/api'
import { storeSession } from '@/app/session/session-query'
import { Button } from '@/components/ui/button'
import { discoverSso, isTwoFactorRequired, login, type SsoOption } from '../api'
import { AuthCard } from '../components/AuthCard'
import { FormAlert } from '../components/FormAlert'
import { FormField } from '../components/FormField'
import { SsoOptions } from '../components/SsoOptions'
import { SubmitButton } from '../components/SubmitButton'
import { applyApiError, isLoginLockedError, retryAfterSeconds } from '../lib/api-errors'
import type { AuthMessageKey } from '../lib/messages'
import { REDIRECT_PARAM, withRedirect } from '../lib/redirect'
import { loginSchema, type LoginValues } from '../lib/schemas'
import { TWO_FACTOR_CHALLENGE_PATH } from '../two-factor/paths'
import { zodResolver } from '@hookform/resolvers/zod'

const SSO_REQUIRED_CODE = 'sso_required'
const SSO_ERROR_PARAM = 'sso_error'

function ssoErrorMessage(code: string | null): AuthMessageKey | undefined {
  if (!code) return undefined
  return code === 'no_account' ? 'errors.ssoNoAccount' : 'errors.ssoFailed'
}

export function LoginPage() {
  const { t } = useTranslation('auth')
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const [searchParams] = useSearchParams()
  const redirect = searchParams.get(REDIRECT_PARAM)
  const ssoError = ssoErrorMessage(searchParams.get(SSO_ERROR_PARAM))
  const [ssoOptions, setSsoOptions] = useState<SsoOption[] | null>(null)
  const [ssoPending, setSsoPending] = useState(false)
  const [lockedMinutes, setLockedMinutes] = useState<number | null | undefined>(undefined)
  const {
    register,
    handleSubmit,
    setError,
    getValues,
    formState: { errors, isSubmitting },
  } = useForm<LoginValues, unknown, LoginValues>({
    resolver: zodResolver(loginSchema),
    defaultValues: { email: '', password: '' },
  })

  const lookupSso = async () => {
    const email = getValues('email').trim()
    if (!email) {
      setError('email', { message: 'validation.email' })
      return
    }
    setSsoPending(true)
    try {
      setSsoOptions(await discoverSso(email))
    } catch {
      setSsoOptions([])
    } finally {
      setSsoPending(false)
    }
  }

  const onSubmit = handleSubmit(async (values) => {
    setLockedMinutes(undefined)
    try {
      const response = await login(values)
      if (isTwoFactorRequired(response)) {
        void navigate(withRedirect(TWO_FACTOR_CHALLENGE_PATH, redirect))
        return
      }
      storeSession(queryClient, response)
    } catch (error) {
      if (isApiError(error) && error.code === SSO_REQUIRED_CODE) {
        setError('root.server', { message: 'errors.ssoRequired' })
        setSsoOptions(await discoverSso(values.email).catch(() => []))
        return
      }
      if (isLoginLockedError(error)) {
        const seconds = retryAfterSeconds(error)
        setLockedMinutes(seconds === null ? null : Math.max(1, Math.ceil(seconds / 60)))
        return
      }
      applyApiError(error, setError, {
        fields: { email: 'email', password: 'password' },
        codes: { unauthorized: { message: 'errors.invalidCredentials' } },
      })
    }
  })

  return (
    <AuthCard
      title={t('login.title')}
      description={t('login.description')}
    >
      <form noValidate onSubmit={onSubmit} className="grid gap-4">
        {lockedMinutes === undefined ? (
          <FormAlert message={errors.root?.server?.message ?? ssoError} />
        ) : (
          <FormAlert
            message={lockedMinutes === null ? 'errors.loginLockedUnknown' : 'errors.loginLocked'}
            values={lockedMinutes === null ? undefined : { count: lockedMinutes }}
          />
        )}
        <FormField
          id="login-email"
          type="email"
          autoComplete="email"
          label={t('fields.email')}
          error={errors.email?.message}
          {...register('email')}
        />
        <div className="grid gap-2">
          <FormField
            id="login-password"
            type="password"
            autoComplete="current-password"
            label={t('fields.password')}
            error={errors.password?.message}
            {...register('password')}
          />
          <Link to="/forgot-password" className="justify-self-end text-sm text-primary hover:underline">
            {t('login.forgotPassword')}
          </Link>
        </div>
        <SubmitButton pending={isSubmitting} pendingLabel={t('login.submitting')}>
          {t('login.submit')}
        </SubmitButton>
        <div className="flex items-center gap-3 text-xs text-muted-foreground">
          <span className="h-px flex-1 bg-border" />
          {t('login.or')}
          <span className="h-px flex-1 bg-border" />
        </div>
        {ssoOptions === null ? (
          <Button type="button" variant="outline" disabled={ssoPending} onClick={() => void lookupSso()}>
            {t('login.sso')}
          </Button>
        ) : ssoOptions.length === 0 ? (
          <p className="text-center text-sm text-muted-foreground">{t('login.ssoNone')}</p>
        ) : (
          <SsoOptions options={ssoOptions} redirect={redirect} />
        )}
      </form>
    </AuthCard>
  )
}
