import { Pencil, Plus, Trash2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { ConfirmDialog } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { toast } from '@/components/ui/toast-store'
import { settingsApi, usePermissionCatalog, useRoles, useSettingsMutation, type Role } from '../api'
import { ListSkeleton } from '../components/ListSkeleton'
import { RequirePermission } from '../components/RequirePermission'
import { errorMessageKey } from '../lib/errors'
import { SETTINGS_MANAGE } from '../lib/permissions'
import { RoleEditorDialog } from './RoleEditorDialog'
import { PageHeader } from '@/components/ui/page-header'

export function RolesPage() {
  return (
    <RequirePermission permission={SETTINGS_MANAGE}>
      <RolesSection />
    </RequirePermission>
  )
}

function RolesSection() {
  const { t } = useTranslation('settings')
  const roles = useRoles()
  const catalog = usePermissionCatalog()
  const [editing, setEditing] = useState<{ role: Role | null } | null>(null)
  const [deleting, setDeleting] = useState<Role | null>(null)
  const remove = useSettingsMutation((role: Role) => settingsApi.deleteRole(role.id))

  const confirmDelete = async (role: Role) => {
    try {
      await remove.mutateAsync(role)
      toast({ title: t('roles.toast.deleted'), variant: 'success' })
    } catch (error) {
      toast({ title: t(errorMessageKey(error)), variant: 'destructive' })
    } finally {
      setDeleting(null)
    }
  }

  const loading = roles.isPending || catalog.isPending
  const failed = roles.isError || catalog.isError

  return (
    <section className="grid gap-4">
      <PageHeader
        title={t('roles.title')}
        description={t('roles.description')}
        actions={
          <Button onClick={() => setEditing({ role: null })} disabled={!catalog.data}>
            <Plus />
            {t('roles.create')}
          </Button>
        }
      />
      {loading ? (
        <ListSkeleton />
      ) : failed ? (
        <ErrorState
          onRetry={() => {
            void roles.refetch()
            void catalog.refetch()
          }}
        />
      ) : roles.data.length === 0 ? (
        <EmptyState title={t('roles.empty')} />
      ) : (
        <ul className="grid gap-2">
          {roles.data.map((role) => (
            <li key={role.id} className="flex items-center justify-between gap-3 rounded-lg border px-4 py-3">
              <div className="grid gap-1">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="font-medium">{role.name}</span>
                  <code className="text-xs text-muted-foreground">{role.key}</code>
                  {role.is_system && <Badge variant="secondary">{t('roles.system')}</Badge>}
                </div>
                <p className="text-sm text-muted-foreground">
                  {t('roles.members', { count: role.member_count })} ·{' '}
                  {role.grants_everything
                    ? t('roles.fullAccess')
                    : t('roles.permissionsCount', { count: role.permissions.length })}
                </p>
              </div>
              <div className="flex gap-1">
                <Button
                  variant="ghost"
                  size="icon"
                  aria-label={`${t('actions.edit')}: ${role.name}`}
                  onClick={() => setEditing({ role })}
                >
                  <Pencil />
                </Button>
                {!role.is_system && (
                  <Button
                    variant="ghost"
                    size="icon"
                    aria-label={`${t('actions.delete')}: ${role.name}`}
                    onClick={() => setDeleting(role)}
                  >
                    <Trash2 />
                  </Button>
                )}
              </div>
            </li>
          ))}
        </ul>
      )}
      {editing && catalog.data && (
        <RoleEditorDialog role={editing.role} catalog={catalog.data} onClose={() => setEditing(null)} />
      )}
      {deleting && (
        <ConfirmDialog
          open
          title={t('roles.confirmDelete.title', { name: deleting.name })}
          description={t('roles.confirmDelete.description')}
          confirmLabel={t('actions.delete')}
          pending={remove.isPending}
          variant="destructive"
          onConfirm={() => confirmDelete(deleting)}
          onOpenChange={(open) => !open && setDeleting(null)}
        />
      )}
    </section>
  )
}
