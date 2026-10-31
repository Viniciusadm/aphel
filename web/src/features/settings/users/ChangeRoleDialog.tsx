import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { toast } from '@/components/ui/toast-store'
import { settingsApi, useSettingsMutation, versionOf, type Member, type Role } from '../api'
import { FormAlert } from '../components/FieldError'
import { RoleSelect } from '../components/RoleSelect'
import { errorMessageKey, type SettingsMessageKey } from '../lib/errors'
import { Field } from '@/components/ui/field'

interface ChangeRoleDialogProps {
  member: Member
  roles: Role[]
  onClose: () => void
}

export function ChangeRoleDialog({ member, roles, onClose }: ChangeRoleDialogProps) {
  const { t } = useTranslation('settings')
  const [roleId, setRoleId] = useState(member.role.id)
  const [error, setError] = useState<SettingsMessageKey>()
  const change = useSettingsMutation((value: string) => settingsApi.changeMemberRole(member.id, value, versionOf(member)))

  const submit = async () => {
    setError(undefined)
    try {
      await change.mutateAsync(roleId)
      toast({ title: t('users.toast.roleChanged'), variant: 'success' })
      onClose()
    } catch (caught) {
      setError(errorMessageKey(caught))
    }
  }

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t('users.roleDialog.title')}</DialogTitle>
          <DialogDescription>{t('users.roleDialog.description', { name: member.display_name })}</DialogDescription>
        </DialogHeader>
        <FormAlert message={error} />
        <Field label={t('users.roleDialog.role')}>
          {(fieldControl) => <RoleSelect
            id={fieldControl.id}
            roles={roles}
            value={roleId}
            onChange={setRoleId}
            placeholder={t('users.inviteDialog.rolePlaceholder')}
          />}
        </Field>
        <DialogFooter>
          <Button variant="outline" onClick={onClose} disabled={change.isPending}>
            {t('actions.cancel')}
          </Button>
          <Button onClick={submit} disabled={change.isPending || roleId === member.role.id}>
            {change.isPending ? t('actions.saving') : t('actions.save')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
