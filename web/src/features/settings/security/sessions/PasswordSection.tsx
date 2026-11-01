import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { zodResolver } from '@hookform/resolvers/zod'
import { useId } from 'react'
import { useForm } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { z } from 'zod'
import { isApiError } from '@/api'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  MAX_PASSWORD_LENGTH,
  MIN_PASSWORD_LENGTH,
  securitySessionsApi,
  useInvalidateSecuritySessions,
} from './api'
import { PageHeader } from '@/components/ui/page-header'
import { PageSection } from '@/components/ui/page'
import { Field } from '@/components/ui/field'

const passwordSchema = z
  .object({
    currentPassword: z.string().min(1, 'password.errors.currentRequired'),
    newPassword: z
      .string()
      .min(MIN_PASSWORD_LENGTH, 'password.errors.length')
      .max(MAX_PASSWORD_LENGTH, 'password.errors.length'),
    confirmation: z.string(),
  })
  .refine((values) => values.newPassword === values.confirmation, {
    path: ['confirmation'],
    message: 'password.errors.mismatch',
  })
  .refine((values) => values.newPassword !== values.currentPassword, {
    path: ['newPassword'],
    message: 'password.errors.same',
  })

type PasswordValues = z.infer<typeof passwordSchema>
type PasswordMessageKey =
  | 'password.errors.currentRequired'
  | 'password.errors.length'
  | 'password.errors.mismatch'
  | 'password.errors.same'
  | 'password.errors.wrongCurrent'

const EMPTY_VALUES: PasswordValues = { currentPassword: '', newPassword: '', confirmation: '' }

export function PasswordSection() {
  const { t } = useTranslation('security')
  const formId = useId()
  const invalidateSessions = useInvalidateSecuritySessions()
  const {
    register,
    handleSubmit,
    reset,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<PasswordValues>({ resolver: zodResolver(passwordSchema), defaultValues: EMPTY_VALUES })

  const onSubmit = handleSubmit(async (values) => {
    try {
      const result = await securitySessionsApi.changePassword({
        current_password: values.currentPassword,
        new_password: values.newPassword,
      })
      reset(EMPTY_VALUES)
      await invalidateSessions()
      notifySuccess(t('password.changed', { count: result.revoked_sessions }))
    } catch (error) {
      if (isApiError(error) && error.code === 'invalid_current_password') {
        setError('currentPassword', { message: 'password.errors.wrongCurrent' }, { shouldFocus: true })
        return
      }
      if (isApiError(error) && error.status === 422) {
        setError('newPassword', { message: 'password.errors.length' }, { shouldFocus: true })
        return
      }
      notifyError(error)
    }
  })

  const fieldError = (name: keyof PasswordValues) => {
    const message = errors[name]?.message as PasswordMessageKey | undefined
    if (!message) return null
    return (
      <SharedFieldError id={`${formId}-${name}-error`}>
        {t(message, { min: MIN_PASSWORD_LENGTH, max: MAX_PASSWORD_LENGTH })}
      </SharedFieldError>
    )
  }

  const describedBy = (name: keyof PasswordValues) => (errors[name] ? `${formId}-${name}-error` : undefined)

  return (
    <PageSection>
      <PageHeader level="section" title={t('password.title')} description={t('password.description')} />
      <form className="grid max-w-md gap-4" noValidate onSubmit={(event) => void onSubmit(event)}>
        <Field label={t('password.current')}>
          {(control) => <>
            <Input placeholder={t('password.currentPlaceholder')}
            {...control}
            type="password"
            autoComplete="current-password"
            aria-invalid={errors.currentPassword ? true : undefined}
            aria-describedby={describedBy('currentPassword')}
            {...register('currentPassword')}
          />
          {fieldError('currentPassword')}
          </>}
        </Field>
        <Field label={t('password.new')}>
          {(control) => <>
            <Input placeholder={t('password.newPlaceholder')}
            {...control}
            type="password"
            autoComplete="new-password"
            maxLength={MAX_PASSWORD_LENGTH}
            aria-invalid={errors.newPassword ? true : undefined}
            aria-describedby={describedBy('newPassword') ?? `${formId}-newPassword-hint`}
            {...register('newPassword')}
          />
          {fieldError('newPassword') ?? (
            <p id={`${formId}-newPassword-hint`} className="text-xs text-muted-foreground">
              {t('password.hint', { min: MIN_PASSWORD_LENGTH })}
            </p>
          )}
          </>}
        </Field>
        <Field label={t('password.confirmation')}>
          {(control) => <>
            <Input placeholder={t('password.confirmationPlaceholder')}
            {...control}
            type="password"
            autoComplete="new-password"
            maxLength={MAX_PASSWORD_LENGTH}
            aria-invalid={errors.confirmation ? true : undefined}
            aria-describedby={describedBy('confirmation')}
            {...register('confirmation')}
          />
          {fieldError('confirmation')}
          </>}
        </Field>
        <Button type="submit" className="justify-self-start" disabled={isSubmitting}>
          {t('password.submit')}
        </Button>
      </form>
    </PageSection>
  )
}
