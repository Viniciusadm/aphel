import { zodResolver } from '@hookform/resolvers/zod'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useForm } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useParams } from 'react-router'
import { z } from 'zod'
import { Button } from '@/components/ui/button'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { PageLoader } from '@/components/ui/page-loader'
import { consoleApi, consoleKeys } from '../api'
import { FormError } from '../components/FormError'
import { PublicCard } from '../components/PublicCard'
import { consoleErrorMessage } from '../errors'
import { SIGN_IN_PATH, TENANTS_PATH } from '../paths'
import { passwordSchema } from '../schemas'

const schema = z
  .object({ password: passwordSchema, confirm: z.string() })
  .refine((values) => values.password === values.confirm, { path: ['confirm'], message: 'validation.passwordsDiffer' })
type Values = z.infer<typeof schema>

export function InvitationPage() {
  const { t } = useTranslation('console')
  const { token = '' } = useParams()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [failure, setFailure] = useState<string | null>(null)
  const invitation = useQuery({ queryKey: consoleKeys.invitation(token), queryFn: () => consoleApi.invitation(token), retry: false })
  const {
    register,
    handleSubmit,
    formState: { errors, isSubmitting },
  } = useForm<Values>({ resolver: zodResolver(schema), defaultValues: { password: '', confirm: '' } })

  const onSubmit = handleSubmit(async (values) => {
    setFailure(null)
    try {
      const operator = await consoleApi.acceptInvitation(token, values.password)
      queryClient.setQueryData(consoleKeys.session, operator)
      void navigate(TENANTS_PATH, { replace: true })
    } catch (error) {
      setFailure(consoleErrorMessage(error))
    }
  })

  if (invitation.isPending) return <PageLoader fullScreen />
  if (invitation.isError) {
    return (
      <PublicCard title={t('invitation.invalidTitle')} description={t('invitation.invalidDescription')}>
        <Button asChild variant="outline" className="w-full">
          <Link to={SIGN_IN_PATH}>{t('invitation.signIn')}</Link>
        </Button>
      </PublicCard>
    )
  }

  return (
    <PublicCard
      title={t('invitation.title')}
      description={t('invitation.description', { name: invitation.data.display_name, email: invitation.data.email })}
    >
      <form className="grid gap-4" onSubmit={onSubmit} noValidate>
        <FormError message={failure} />
        <Field label={t('invitation.password')} error={errors.password?.message && t(errors.password.message as 'validation.passwordLength')}>
          {(control) => <Input {...control} type="password" autoComplete="new-password" autoFocus {...register('password')} />}
        </Field>
        <Field label={t('invitation.confirm')} error={errors.confirm?.message && t(errors.confirm.message as 'validation.passwordsDiffer')}>
          {(control) => <Input {...control} type="password" autoComplete="new-password" {...register('confirm')} />}
        </Field>
        <Button type="submit" disabled={isSubmitting}>
          {t('invitation.submit')}
        </Button>
      </form>
    </PublicCard>
  )
}
