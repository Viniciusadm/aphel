import { DialogForm } from '@/components/ui/dialog-form'
import { Controller, useForm } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import {
  Dialog,
  DialogContent,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { toast } from '@/components/ui/toast-store'
import { settingsApi, useSettingsMutation, type InvitedMember, type Role } from '../api'
import { FieldError, FormAlert } from '../components/FieldError'
import { RoleSelect } from '../components/RoleSelect'
import { applyApiError } from '../lib/errors'
import { inviteMemberSchema, type InviteMemberValues } from '../lib/schemas'
import { zodResolver } from '@hookform/resolvers/zod'
import { Field } from '@/components/ui/field'

interface InviteMemberDialogProps {
  open: boolean
  roles: Role[]
  onOpenChange: (open: boolean) => void
  onInvited: (member: InvitedMember) => void
}

function defaultRoleId(roles: Role[]): string {
  return (roles.find((role) => role.is_system && role.key === 'member') ?? roles[0])?.id ?? ''
}

export function InviteMemberDialog({ open, roles, onOpenChange, onInvited }: InviteMemberDialogProps) {
  const { t } = useTranslation('settings')
  const invite = useSettingsMutation(settingsApi.inviteMember)
  const {
    register,
    control,
    handleSubmit,
    reset,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<InviteMemberValues, unknown, InviteMemberValues>({
    resolver: zodResolver(inviteMemberSchema),
    values: { email: '', roleId: defaultRoleId(roles) },
    resetOptions: { keepDirtyValues: true },
  })

  const close = (next: boolean) => {
    if (!next) reset()
    onOpenChange(next)
  }

  const onSubmit = handleSubmit(async (values) => {
    try {
      const member = await invite.mutateAsync({ email: values.email, role_id: values.roleId })
      if (!member.invitation_link) toast({ title: t('users.toast.invited', { email: member.email }), variant: 'success' })
      close(false)
      onInvited(member)
    } catch (error) {
      applyApiError(error, setError, {
        fields: { email: 'email', role_id: 'roleId' },
        codes: { already_member: 'email' },
      })
    }
  })

  return (
    <Dialog open={open} onOpenChange={close}>
      <DialogContent>
        <DialogForm
          onSubmit={onSubmit}
          noValidate
          title={t('users.inviteDialog.title')}
          description={t('users.inviteDialog.description')}
          cancelLabel={t('actions.cancel')}
          submitLabel={isSubmitting ? t('actions.saving') : t('users.inviteDialog.submit')}
          onCancel={() => close(false)}
          pending={isSubmitting}
          submitDisabled={isSubmitting}
        >
          <FormAlert message={errors.root?.server?.message} />
          <Field label={t('users.inviteDialog.email')}>
            {(control) => <>
              <Input placeholder={t('users.inviteDialog.emailPlaceholder')}
              {...control}
              type="email"
              autoComplete="off"
              aria-invalid={errors.email ? true : undefined}
              aria-describedby={errors.email ? 'invite-email-error' : undefined}
              {...register('email')}
            />
            <FieldError id="invite-email-error" message={errors.email?.message} />
            </>}
          </Field>
          <Field label={t('users.inviteDialog.role')}>
            {(fieldControl) => <>
              <Controller
              control={control}
              name="roleId"
              render={({ field }) => (
                <RoleSelect
                  id={fieldControl.id}
                  roles={roles}
                  value={field.value}
                  onChange={field.onChange}
                  placeholder={t('users.inviteDialog.rolePlaceholder')}
                  invalid={Boolean(errors.roleId)}
                  describedBy={errors.roleId ? 'invite-role-error' : undefined}
                />
              )}
            />
            <FieldError id="invite-role-error" message={errors.roleId?.message} />
            </>}
          </Field>
        </DialogForm>
      </DialogContent>
    </Dialog>
  )
}
