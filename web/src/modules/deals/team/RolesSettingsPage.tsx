import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { Pencil, Plus, Trash2 } from 'lucide-react'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ConfirmDialog, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Skeleton } from '@/components/ui/skeleton'
import { toast } from '@/components/ui/toast-store'
import { RequirePermission, SETTINGS_MANAGE, SortableList } from '@/sdk'
import { cn } from '@/lib/utils'
import { dealTeamApi, useDealMemberRoles, useInvalidateDealMemberRoles } from './api'
import { dealTeamErrorMessage } from './errors'
import type { DealMemberRole } from './types'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'
import { Field } from '@/components/ui/field'

interface EditorState {
  role: DealMemberRole | null
}

interface PendingOrder {
  source: DealMemberRole[] | undefined
  items: DealMemberRole[]
}

export function RolesSettingsPage() {
  return (
    <RequirePermission permission={SETTINGS_MANAGE}>
      <RolesSection />
    </RequirePermission>
  )
}

function RolesSection() {
  const { t } = useTranslation('deals', { keyPrefix: 'team' })
  const roles = useDealMemberRoles(true)
  const invalidate = useInvalidateDealMemberRoles()
  const [order, setOrder] = useState<PendingOrder | null>(null)
  const [editor, setEditor] = useState<EditorState | null>(null)
  const [deleting, setDeleting] = useState<DealMemberRole | null>(null)
  const [busyId, setBusyId] = useState<string | null>(null)
  const items = order && order.source === roles.data ? order.items : (roles.data ?? [])
  const fail = (error: unknown) => toast({ title: dealTeamErrorMessage(error, t), variant: 'destructive' })

  const saveOrder = async (ids: string[]) => {
    const byId = new Map(items.map((item) => [item.id, item]))
    setOrder({ source: roles.data, items: ids.flatMap((id) => byId.get(id) ?? []) })
    try {
      await dealTeamApi.reorderRoles(ids)
      await invalidate()
    } catch (error) {
      setOrder(null)
      fail(error)
    }
  }

  const toggleActive = async (role: DealMemberRole) => {
    setBusyId(role.id)
    try {
      await dealTeamApi.updateRole(role.id, { is_active: !role.is_active }, role.updated_at)
      await invalidate()
    } catch (error) {
      fail(error)
    } finally {
      setBusyId(null)
    }
  }

  const remove = async () => {
    if (!deleting) return
    try {
      await dealTeamApi.deleteRole(deleting.id)
      notifySuccess(t('roles.deleted', { name: deleting.name }))
      setDeleting(null)
      await invalidate()
    } catch (error) {
      fail(error)
    }
  }

  return (
    <Page>
      <PageHeader
        title={t('roles.title')}
        description={t('roles.description')}
        actions={
          <Button size="sm" onClick={() => setEditor({ role: null })}>
            <Plus aria-hidden="true" />
            {t('roles.new')}
          </Button>
        }
      />
      {roles.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-12 w-full" />
          <Skeleton className="h-12 w-full" />
        </div>
      ) : roles.isError ? (
        <ErrorState onRetry={() => void roles.refetch()} />
      ) : items.length === 0 ? (
        <EmptyState
          title={t('roles.empty')}
          description={t('roles.emptyDescription')}
          action={
            <Button size="sm" onClick={() => setEditor({ role: null })}>
              <Plus aria-hidden="true" />
              {t('roles.new')}
            </Button>
          }
        />
      ) : (
        <SortableList
          items={items}
          getId={(role) => role.id}
          getLabel={(role) => role.name}
          onReorder={(ids) => void saveOrder(ids)}
          label={t('roles.listLabel')}
          renderItem={(role, handle) => (
            <div
              className={cn(
                'flex flex-wrap items-center gap-3 rounded-md border bg-card px-3 py-2',
                !role.is_active && 'opacity-70',
              )}
            >
              {handle}
              <div className="grid min-w-32 flex-1 gap-0.5">
                <span className="truncate text-sm font-medium">{role.name}</span>
              </div>
              <div className="flex flex-wrap items-center gap-2">
                {!role.is_active && <Badge variant="outline">{t('roles.inactive')}</Badge>}
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => setEditor({ role })}
                  aria-label={t('roles.editLabel', { name: role.name })}
                >
                  <Pencil aria-hidden="true" />
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => setDeleting(role)}
                  aria-label={t('roles.deleteLabel', { name: role.name })}
                >
                  <Trash2 aria-hidden="true" />
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  disabled={busyId === role.id}
                  onClick={() => void toggleActive(role)}
                >
                  {t(role.is_active ? 'roles.deactivate' : 'roles.activate')}
                </Button>
              </div>
            </div>
          )}
        />
      )}
      <Dialog open={editor !== null} onOpenChange={(open) => !open && setEditor(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t(editor?.role ? 'roles.editTitle' : 'roles.newTitle')}</DialogTitle>
            <DialogDescription>{t('roles.dialogDescription')}</DialogDescription>
          </DialogHeader>
          {editor && (
            <RoleForm
              key={editor.role?.id ?? 'new'}
              editing={editor.role}
              onClose={() => setEditor(null)}
              onSaved={() => void invalidate()}
            />
          )}
        </DialogContent>
      </Dialog>
      <ConfirmDialog
        open={deleting !== null}
        onOpenChange={(open) => !open && setDeleting(null)}
        title={t('roles.deleteTitle')}
        description={t('roles.deleteDescription', { name: deleting?.name ?? '' })}
        confirmLabel={t('roles.deleteConfirm')}
        variant="destructive"
        notifyOnError={false}
        onConfirm={remove}
      />
    </Page>
  )
}

function RoleForm({
  editing,
  onClose,
  onSaved,
}: {
  editing: DealMemberRole | null
  onClose: () => void
  onSaved: () => void
}) {
  const { t } = useTranslation('deals', { keyPrefix: 'team' })
  const formId = useId()
  const [name, setName] = useState(editing?.name ?? '')
  const [nameError, setNameError] = useState(false)
  const [saving, setSaving] = useState(false)

  const submit = async () => {
    if (!name.trim()) {
      setNameError(true)
      return
    }
    setSaving(true)
    try {
      if (editing) {
        await dealTeamApi.updateRole(editing.id, { name: name.trim() }, editing.updated_at)
      } else {
        await dealTeamApi.createRole({ name: name.trim() })
      }
      notifySuccess(t('roles.saved', { name: name.trim() }))
      onSaved()
      onClose()
    } catch (error) {
      toast({ title: dealTeamErrorMessage(error, t), variant: 'destructive' })
    } finally {
      setSaving(false)
    }
  }

  return (
    <>
      <form
        id={formId}
        className="grid gap-4"
        noValidate
        onSubmit={(event) => {
          event.preventDefault()
          void submit()
        }}
      >
        <Field label={t('roles.name')}>
          {(control) => <>
            <Input placeholder={t('placeholders.roleName')}
            {...control}
            value={name}
            autoFocus
            maxLength={140}
            aria-invalid={nameError ? true : undefined}
            onChange={(event) => {
              setName(event.target.value)
              setNameError(false)
            }}
          />
          {nameError && <SharedFieldError>{t('roles.nameRequired')}</SharedFieldError>}
          {editing && <code className="text-xs text-muted-foreground">{editing.key}</code>}
          </>}
        </Field>
      </form>
      <DialogFooter>
        <Button variant="outline" onClick={onClose} disabled={saving}>
          {t('roles.cancel')}
        </Button>
        <Button type="submit" form={formId} disabled={saving}>
          {t('roles.save')}
        </Button>
      </DialogFooter>
    </>
  )
}
