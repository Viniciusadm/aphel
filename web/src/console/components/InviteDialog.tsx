import { zodResolver } from '@hookform/resolvers/zod'
import { useState } from 'react'
import { useForm } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { z } from 'zod'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import { DialogForm } from '@/components/ui/dialog-form'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { consoleApi, type Invitation } from '../api'
import { consoleErrorMessage } from '../errors'
import { emailSchema, requiredText } from '../schemas'

const schema = z.object({ email: emailSchema, name: requiredText })
type Values = z.infer<typeof schema>

interface InviteDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  onInvited: (invitation: Invitation) => void
}

export function InviteDialog({ open, onOpenChange, onInvited }: InviteDialogProps) {
  const { t } = useTranslation('console')
  const [failure, setFailure] = useState<string | null>(null)
  const {
    register,
    handleSubmit,
    reset,
    formState: { errors, isSubmitting },
  } = useForm<Values>({ resolver: zodResolver(schema), defaultValues: { email: '', name: '' } })

  const close = (next: boolean) => {
    if (!next) {
      reset()
      setFailure(null)
    }
    onOpenChange(next)
  }

  const onSubmit = handleSubmit(async (values) => {
    setFailure(null)
    try {
      const invitation = await consoleApi.invite(values.email, values.name)
      reset()
      onInvited(invitation)
    } catch (error) {
      setFailure(consoleErrorMessage(error))
    }
  })

  const message = (key: string | undefined) => (key ? t(key as 'validation.required') : undefined)

  return (
    <Dialog open={open} onOpenChange={close}>
      <DialogContent>
        <DialogForm
          title={t('operators.invite')}
          description={t('operators.inviteDescription')}
          error={failure}
          pending={isSubmitting}
          submitLabel={t('operators.send')}
          cancelLabel={t('operators.cancel')}
          onCancel={() => close(false)}
          onSubmit={(event) => void onSubmit(event)}
          noValidate
        >
          <Field label={t('operators.name')} required error={message(errors.name?.message)}>
            {(field) => <Input {...field} autoFocus autoComplete="off" {...register('name')} />}
          </Field>
          <Field label={t('operators.email')} required error={message(errors.email?.message)}>
            {(field) => <Input {...field} type="email" autoComplete="off" {...register('email')} />}
          </Field>
        </DialogForm>
      </DialogContent>
    </Dialog>
  )
}
