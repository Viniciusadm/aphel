import { useState } from 'react'
import { useForm } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { forgotPassword } from '../api'
import { AuthCard } from '../components/AuthCard'
import { FormAlert } from '../components/FormAlert'
import { FormField } from '../components/FormField'
import { SubmitButton } from '../components/SubmitButton'
import { applyApiError } from '../lib/api-errors'
import { forgotPasswordSchema, type ForgotPasswordValues } from '../lib/schemas'
import { zodResolver } from '@hookform/resolvers/zod'

export function ForgotPasswordPage() {
  const { t } = useTranslation('auth')
  const [sentTo, setSentTo] = useState<string | null>(null)
  const {
    register,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<ForgotPasswordValues, unknown, ForgotPasswordValues>({
    resolver: zodResolver(forgotPasswordSchema),
    defaultValues: { email: '' },
  })

  const onSubmit = handleSubmit(async (values) => {
    try {
      await forgotPassword(values)
      setSentTo(values.email)
    } catch (error) {
      applyApiError(error, setError, { fields: { email: 'email' } })
    }
  })

  const backToLogin = (
    <Link to="/login" className="font-medium text-primary hover:underline">
      {t('forgotPassword.backToLogin')}
    </Link>
  )

  if (sentTo) {
    return (
      <AuthCard
        title={t('forgotPassword.sentTitle')}
        description={<span role="status">{t('forgotPassword.sentDescription', { email: sentTo })}</span>}
        footer={backToLogin}
      />
    )
  }

  return (
    <AuthCard title={t('forgotPassword.title')} description={t('forgotPassword.description')} footer={backToLogin}>
      <form noValidate onSubmit={onSubmit} className="grid gap-4">
        <FormAlert message={errors.root?.server?.message} />
        <FormField
          id="forgot-password-email"
          type="email"
          autoComplete="email"
          label={t('fields.email')}
          error={errors.email?.message}
          {...register('email')}
        />
        <SubmitButton pending={isSubmitting} pendingLabel={t('forgotPassword.submitting')}>
          {t('forgotPassword.submit')}
        </SubmitButton>
      </form>
    </AuthCard>
  )
}
