import { zodResolver } from '@hookform/resolvers/zod'
import { useState } from 'react'
import { useForm } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { z } from 'zod'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { consoleApi } from '../api'
import { FormError } from '../components/FormError'
import { consoleErrorMessage, notifyConsoleSuccess } from '../errors'
import { passwordSchema } from '../schemas'

const schema = z
  .object({ current: z.string().min(1, 'validation.required'), password: passwordSchema, confirm: z.string() })
  .refine((values) => values.password === values.confirm, { path: ['confirm'], message: 'validation.passwordsDiffer' })
type Values = z.infer<typeof schema>

export function PasswordPage() {
  const { t } = useTranslation('console')
  const [failure, setFailure] = useState<string | null>(null)
  const {
    register,
    handleSubmit,
    reset,
    formState: { errors, isSubmitting },
  } = useForm<Values>({ resolver: zodResolver(schema), defaultValues: { current: '', password: '', confirm: '' } })

  const onSubmit = handleSubmit(async (values) => {
    setFailure(null)
    try {
      await consoleApi.changePassword(values.current, values.password)
      reset()
      notifyConsoleSuccess(t('password.changed'))
    } catch (error) {
      setFailure(consoleErrorMessage(error))
    }
  })

  const message = (key: string | undefined) => (key ? t(key as 'validation.required') : undefined)

  return (
    <Page width="narrow">
      <PageHeader title={t('password.title')} description={t('password.description')} />
      <Card>
        <CardContent className="pt-6">
          <form className="grid gap-4" onSubmit={onSubmit} noValidate>
            <FormError message={failure} />
            <Field label={t('password.current')} error={message(errors.current?.message)}>
              {(control) => <Input {...control} type="password" autoComplete="current-password" {...register('current')} />}
            </Field>
            <Field label={t('password.new')} error={message(errors.password?.message)}>
              {(control) => <Input {...control} type="password" autoComplete="new-password" {...register('password')} />}
            </Field>
            <Field label={t('password.confirm')} error={message(errors.confirm?.message)}>
              {(control) => <Input {...control} type="password" autoComplete="new-password" {...register('confirm')} />}
            </Field>
            <div>
              <Button type="submit" disabled={isSubmitting}>
                {t('password.submit')}
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </Page>
  )
}
