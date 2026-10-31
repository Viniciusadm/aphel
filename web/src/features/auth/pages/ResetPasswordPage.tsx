import { useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useForm } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { Link, useSearchParams } from 'react-router'
import { clearSession } from '@/app/session/session-query'
import { Button } from '@/components/ui/button'
import { resetPassword } from '../api'
import { AuthCard } from '../components/AuthCard'
import { FormAlert } from '../components/FormAlert'
import { FormField } from '../components/FormField'
import { InvalidTokenNotice } from '../components/InvalidTokenNotice'
import { SubmitButton } from '../components/SubmitButton'
import { applyApiError, isInvalidTokenError } from '../lib/api-errors'
import { resetPasswordSchema, type ResetPasswordValues } from '../lib/schemas'
import { zodResolver } from '@hookform/resolvers/zod'

type ResetState = 'form' | 'done' | 'invalid'

export function ResetPasswordPage() {
  const { t } = useTranslation('auth')
  const queryClient = useQueryClient()
  const [searchParams] = useSearchParams()
  const token = searchParams.get('token')
  const [state, setState] = useState<ResetState>('form')
  const {
    register,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<ResetPasswordValues, unknown, ResetPasswordValues>({
    resolver: zodResolver(resetPasswordSchema),
    defaultValues: { password: '' },
  })

  const requestNewLink = (
    <Button asChild variant="outline" className="w-full">
      <Link to="/forgot-password">{t('invalidToken.requestNewLink')}</Link>
    </Button>
  )

  if (!token) return <InvalidTokenNotice description={t('invalidToken.missing')} action={requestNewLink} />
  if (state === 'invalid') {
    return <InvalidTokenNotice description={t('invalidToken.resetDescription')} action={requestNewLink} />
  }

  if (state === 'done') {
    return (
      <AuthCard title={t('resetPassword.successTitle')} description={<span role="status">{t('resetPassword.successDescription')}</span>}>
        <Button asChild className="w-full">
          <Link to="/login">{t('resetPassword.goToLogin')}</Link>
        </Button>
      </AuthCard>
    )
  }

  const onSubmit = handleSubmit(async (values) => {
    try {
      await resetPassword({ token, password: values.password })
      clearSession(queryClient)
      setState('done')
    } catch (error) {
      if (isInvalidTokenError(error)) {
        setState('invalid')
        return
      }
      applyApiError(error, setError, { fields: { password: 'password' } })
    }
  })

  return (
    <AuthCard title={t('resetPassword.title')} description={t('resetPassword.description')}>
      <form noValidate onSubmit={onSubmit} className="grid gap-4">
        <FormAlert message={errors.root?.server?.message} />
        <FormField placeholder={t('placeholders.newPassword')}
          id="reset-password-password"
          type="password"
          autoComplete="new-password"
          label={t('fields.newPassword')}
          hint={t('hints.password')}
          error={errors.password?.message}
          {...register('password')}
        />
        <SubmitButton pending={isSubmitting} pendingLabel={t('resetPassword.submitting')}>
          {t('resetPassword.submit')}
        </SubmitButton>
      </form>
    </AuthCard>
  )
}
