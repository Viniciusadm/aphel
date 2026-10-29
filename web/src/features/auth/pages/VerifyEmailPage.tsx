import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useSearchParams } from 'react-router'
import { refreshSession, useSessionQuery } from '@/app/session/session-query'
import { Button } from '@/components/ui/button'
import { ErrorState } from '@/components/ui/error-state'
import { verifyEmail } from '../api'
import { AuthCard } from '../components/AuthCard'
import { InlineLoader } from '../components/InlineLoader'
import { InvalidTokenNotice } from '../components/InvalidTokenNotice'
import { ResendVerificationButton } from '../components/ResendVerificationButton'
import { generalErrorMessage, isInvalidTokenError } from '../lib/api-errors'

export function VerifyEmailPage() {
  const { t } = useTranslation('auth')
  const queryClient = useQueryClient()
  const [searchParams] = useSearchParams()
  const token = searchParams.get('token')
  const session = useSessionQuery()
  const requestedToken = useRef<string | null>(null)
  const { mutate, status, error } = useMutation({
    mutationFn: verifyEmail,
    onSuccess: () => refreshSession(queryClient),
  })

  useEffect(() => {
    if (!token || requestedToken.current === token) return
    requestedToken.current = token
    mutate(token)
  }, [token, mutate])

  const signedIn = Boolean(session.data)
  const recoveryAction = signedIn ? (
    session.data?.user.email_verified ? null : <ResendVerificationButton className="w-full" />
  ) : (
    <Button asChild variant="outline" className="w-full">
      <Link to="/login">{t('invalidToken.goToLogin')}</Link>
    </Button>
  )

  if (!token) return <InvalidTokenNotice description={t('invalidToken.missing')} action={recoveryAction} />

  if (status === 'error') {
    if (isInvalidTokenError(error)) {
      return <InvalidTokenNotice description={t('invalidToken.verifyDescription')} action={recoveryAction} />
    }
    return <ErrorState description={t(generalErrorMessage(error))} onRetry={() => mutate(token)} />
  }

  if (status === 'success') {
    return (
      <AuthCard title={t('verifyEmail.successTitle')} description={<span role="status">{t('verifyEmail.successDescription')}</span>}>
        <Button asChild className="w-full">
          <Link to={signedIn ? '/' : '/login'}>{t('verifyEmail.continue')}</Link>
        </Button>
      </AuthCard>
    )
  }

  return (
    <AuthCard title={t('verifyEmail.verifying')}>
      <InlineLoader label={t('verifyEmail.verifying')} />
    </AuthCard>
  )
}
