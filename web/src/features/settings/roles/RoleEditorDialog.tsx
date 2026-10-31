import { useState } from 'react'
import { useForm } from 'react-hook-form'
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
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { toast } from '@/components/ui/toast-store'
import { settingsApi, useSettingsMutation, versionOf, type PermissionDefinition, type Role } from '../api'
import { FieldError, FormAlert } from '../components/FieldError'
import { applyApiError } from '../lib/errors'
import { roleSchema, type RoleValues } from '../lib/schemas'
import { zodResolver } from '@hookform/resolvers/zod'
import { PermissionMatrix } from './PermissionMatrix'
import { grantsFromMatrix, matrixFromRole } from './matrix'

interface RoleEditorDialogProps {
  role: Role | null
  catalog: PermissionDefinition[]
  onClose: () => void
}

export function RoleEditorDialog({ role, catalog, onClose }: RoleEditorDialogProps) {
  const { t } = useTranslation('settings')
  const [matrix, setMatrix] = useState(() => matrixFromRole(role))
  const locked = role?.grants_everything ?? false
  const save = useSettingsMutation(async (values: RoleValues) => {
    const permissions = locked ? undefined : grantsFromMatrix(matrix)
    const key = values.key || undefined
    if (role) return settingsApi.updateRole(role.id, { name: values.name, key, permissions }, versionOf(role))
    return settingsApi.createRole({ name: values.name, key, permissions: permissions ?? [] })
  })
  const {
    register,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<RoleValues, unknown, RoleValues>({
    resolver: zodResolver(roleSchema),
    defaultValues: { name: role?.name ?? '', key: role?.key ?? '' },
  })

  const onSubmit = handleSubmit(async (values) => {
    try {
      await save.mutateAsync(values)
      toast({ title: t(role ? 'roles.toast.updated' : 'roles.toast.created'), variant: 'success' })
      onClose()
    } catch (error) {
      applyApiError(error, setError, {
        fields: { name: 'name', key: 'key' },
        codes: { role_key_taken: 'key' },
      })
    }
  })

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent size="2xl">
        <DialogHeader>
          <DialogTitle>{t(role ? 'roles.editor.editTitle' : 'roles.editor.createTitle')}</DialogTitle>
          <DialogDescription>{t('roles.editor.description')}</DialogDescription>
        </DialogHeader>
        <form className="grid gap-4" onSubmit={onSubmit} noValidate>
          <FormAlert message={errors.root?.server?.message} />
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="grid gap-2">
              <Label htmlFor="role-name">{t('roles.editor.name')}</Label>
              <Input placeholder={t('roles.editor.namePlaceholder')}
                id="role-name"
                aria-invalid={errors.name ? true : undefined}
                aria-describedby={errors.name ? 'role-name-error' : undefined}
                {...register('name')}
              />
              <FieldError id="role-name-error" message={errors.name?.message} />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="role-key">{t('roles.editor.key')}</Label>
              <Input placeholder={t('roles.editor.keyPlaceholder')}
                id="role-key"
                disabled={role?.is_system}
                aria-invalid={errors.key ? true : undefined}
                aria-describedby={errors.key ? 'role-key-error' : 'role-key-hint'}
                {...register('key')}
              />
              {errors.key ? (
                <FieldError id="role-key-error" message={errors.key.message} />
              ) : (
                <p id="role-key-hint" className="text-xs text-muted-foreground">
                  {t(role?.is_system ? 'roles.editor.systemKeyHint' : 'roles.editor.keyHint')}
                </p>
              )}
            </div>
          </div>
          {locked && (
            <p className="rounded-md border bg-muted/40 px-3 py-2 text-sm text-muted-foreground">
              {t('roles.editor.adminNotice')}
            </p>
          )}
          <PermissionMatrix catalog={catalog} value={matrix} readOnly={locked} onChange={setMatrix} />
          <DialogFooter>
            <Button variant="outline" onClick={onClose} disabled={isSubmitting}>
              {t('actions.cancel')}
            </Button>
            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting ? t('actions.saving') : t('actions.save')}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
