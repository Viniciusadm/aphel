import { zodResolver } from '@hookform/resolvers/zod'
import { useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useForm } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { useLocation, useNavigate } from 'react-router'
import { z } from 'zod'
import { Button } from '@/components/ui/button'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { consoleApi, consoleKeys } from '../api'
import { FormError } from '../components/FormError'
import { PublicCard } from '../components/PublicCard'
import { consoleErrorMessage } from '../errors'
import { TENANTS_PATH } from '../paths'
import { emailSchema } from '../schemas'

const schema = z.object({ email: emailSchema, password: z.string().min(1, 'validation.required') })
type Values = z.infer<typeof schema>

export function SignInPage() {
  const { t } = useTranslation('console')
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const location = useLocation()
  const [failure, setFailure] = useState<string | null>(null)
  const {
    register,
    handleSubmit,
    formState: { errors, isSubmitting },
  } = useForm<Values>({ resolver: zodResolver(schema), defaultValues: { email: '', password: '' } })

  const onSubmit = handleSubmit(async (values) => {
    setFailure(null)
    try {
      const operator = await consoleApi.signIn(values.email, values.password)
      queryClient.setQueryData(consoleKeys.session, operator)
      const from = (location.state as { from?: string } | null)?.from
      void navigate(from && from !== location.pathname ? from : TENANTS_PATH, { replace: true })
    } catch (error) {
      setFailure(consoleErrorMessage(error))
    }
  })

  return (
    <PublicCard title={t('signIn.title')} description={t('signIn.description')}>
      <form className="grid gap-4" onSubmit={onSubmit} noValidate>
        <FormError message={failure} />
        <Field label={t('signIn.email')} error={errors.email?.message && t(errors.email.message as 'validation.email')}>
          {(control) => <Input {...control} type="email" autoComplete="username" autoFocus {...register('email')} />}
        </Field>
        <Field label={t('signIn.password')} error={errors.password?.message && t(errors.password.message as 'validation.required')}>
          {(control) => <Input {...control} type="password" autoComplete="current-password" {...register('password')} />}
        </Field>
        <Button type="submit" disabled={isSubmitting}>
          {isSubmitting ? t('signIn.submitting') : t('signIn.submit')}
        </Button>
      </form>
    </PublicCard>
  )
}
