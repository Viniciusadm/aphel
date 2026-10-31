import { useQuery } from '@tanstack/react-query'
import { Pencil, Plus } from 'lucide-react'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
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
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { toast } from '@/components/ui/toast-store'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { RequirePermission } from '@/features/settings/components/RequirePermission'
import { SortableList } from '@/features/settings/fields/components/SortableList'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import { cn } from '@/lib/utils'
import { activitiesApi, activitiesKeys, useInvalidateActivityTypes, type ActivityType } from '../api'
import { ActivityTypeIcon } from '../components/ActivityTypeIcon'
import { activityErrorKey } from '../lib/errors'
import { ACTIVITY_ICON_NAMES, DEFAULT_ACTIVITY_COLOR } from '../lib/icons'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

const HEX_COLOR = /^#[0-9a-f]{6}$/i

interface EditorState {
  type: ActivityType | null
}

interface PendingOrder {
  source: ActivityType[] | undefined
  items: ActivityType[]
}

export function ActivityTypesPage() {
  return (
    <RequirePermission permission={SETTINGS_MANAGE}>
      <ActivityTypesSection />
    </RequirePermission>
  )
}

function ActivityTypesSection() {
  const { t } = useTranslation('activities')
  const types = useQuery({
    queryKey: activitiesKeys.types(true),
    queryFn: () => activitiesApi.listTypes(true),
    select: (page) => page.data,
  })
  const invalidate = useInvalidateActivityTypes()
  const [order, setOrder] = useState<PendingOrder | null>(null)
  const [editor, setEditor] = useState<EditorState | null>(null)
  const [busyId, setBusyId] = useState<string | null>(null)
  const items = order && order.source === types.data ? order.items : (types.data ?? [])

  const saveOrder = async (ids: string[]) => {
    const byId = new Map(items.map((item) => [item.id, item]))
    setOrder({
      source: types.data,
      items: ids.flatMap((id) => {
        const item = byId.get(id)
        return item ? [item] : []
      }),
    })
    try {
      await activitiesApi.reorderTypes(ids)
      await invalidate()
    } catch (error) {
      setOrder(null)
      toast({ title: t(activityErrorKey(error)), variant: 'destructive' })
    }
  }

  const toggleActive = async (type: ActivityType) => {
    setBusyId(type.id)
    try {
      await activitiesApi.setTypeActive(type.id, !type.is_active)
      await invalidate()
      toast({
        title: t(type.is_active ? 'types.deactivated' : 'types.activated', { name: type.name }),
        variant: 'success',
      })
    } catch (error) {
      toast({ title: t(activityErrorKey(error)), variant: 'destructive' })
    } finally {
      setBusyId(null)
    }
  }

  return (
    <Page>
      <PageHeader
        title={t('types.title')}
        description={t('types.description')}
        actions={
          <Button size="sm" onClick={() => setEditor({ type: null })}>
            <Plus aria-hidden="true" />
            {t('types.new')}
          </Button>
        }
      />
      {types.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-12 w-full" />
          <Skeleton className="h-12 w-full" />
          <Skeleton className="h-12 w-full" />
        </div>
      ) : types.isError ? (
        <ErrorState onRetry={() => void types.refetch()} />
      ) : (
        <SortableList
          items={items}
          getId={(type) => type.id}
          getLabel={(type) => type.name}
          onReorder={(ids) => void saveOrder(ids)}
          label={t('types.listLabel')}
          renderItem={(type, handle) => (
            <div
              className={cn(
                'flex flex-wrap items-center gap-3 rounded-md border bg-card px-3 py-2',
                !type.is_active && 'opacity-70',
              )}
            >
              {handle}
              <ActivityTypeIcon icon={type.icon} color={type.color} />
              <div className="grid min-w-32 flex-1 gap-0.5">
                <span className="truncate text-sm font-medium">{type.name}</span>
                <code className="truncate text-xs text-muted-foreground">{type.key}</code>
              </div>
              <div className="flex flex-wrap items-center gap-2">
                {type.is_system && <Badge variant="secondary">{t('types.system')}</Badge>}
                {!type.is_active && <Badge variant="outline">{t('types.inactive')}</Badge>}
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => setEditor({ type })}
                  aria-label={t('types.editLabel', { name: type.name })}
                >
                  <Pencil aria-hidden="true" />
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  disabled={busyId === type.id}
                  onClick={() => void toggleActive(type)}
                >
                  {t(type.is_active ? 'types.deactivate' : 'types.activate')}
                </Button>
              </div>
            </div>
          )}
        />
      )}
      <ActivityTypeDialog editor={editor} onClose={() => setEditor(null)} onSaved={() => void invalidate()} />
    </Page>
  )
}

interface ActivityTypeDialogProps {
  editor: EditorState | null
  onClose: () => void
  onSaved: () => void
}

function ActivityTypeDialog({ editor, onClose, onSaved }: ActivityTypeDialogProps) {
  const { t } = useTranslation('activities')
  const editing = editor?.type ?? null
  return (
    <Dialog open={editor !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t(editing ? 'types.editTitle' : 'types.newTitle')}</DialogTitle>
          <DialogDescription>{t('types.dialogDescription')}</DialogDescription>
        </DialogHeader>
        {editor && (
          <ActivityTypeForm key={editing?.id ?? 'new'} editing={editing} onClose={onClose} onSaved={onSaved} />
        )}
      </DialogContent>
    </Dialog>
  )
}

interface ActivityTypeFormProps {
  editing: ActivityType | null
  onClose: () => void
  onSaved: () => void
}

function ActivityTypeForm({ editing, onClose, onSaved }: ActivityTypeFormProps) {
  const { t } = useTranslation('activities')
  const formId = useId()
  const [name, setName] = useState(editing?.name ?? '')
  const [icon, setIcon] = useState(editing?.icon ?? 'check-square')
  const [color, setColor] = useState(editing?.color ?? DEFAULT_ACTIVITY_COLOR)
  const [saving, setSaving] = useState(false)
  const [nameError, setNameError] = useState(false)

  const submit = async () => {
    if (!name.trim()) {
      setNameError(true)
      return
    }
    setSaving(true)
    const validColor = HEX_COLOR.test(color) ? color : null
    try {
      if (editing) {
        await activitiesApi.updateType(editing.id, { name: name.trim(), icon: icon || null, color: validColor })
      } else {
        await activitiesApi.createType({ name: name.trim(), icon: icon || null, color: validColor })
      }
      toast({ title: t(editing ? 'types.updated' : 'types.created', { name: name.trim() }), variant: 'success' })
      onSaved()
      onClose()
    } catch (error) {
      toast({ title: t(activityErrorKey(error)), variant: 'destructive' })
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
        <div className="grid gap-2.5">
          <Label htmlFor={`${formId}-name`}>{t('types.name')}</Label>
          <Input placeholder={t('placeholders.typeName')}
            id={`${formId}-name`}
            value={name}
            autoFocus
            aria-invalid={nameError ? true : undefined}
            onChange={(event) => {
              setName(event.target.value)
              setNameError(false)
            }}
          />
          {nameError && <p className="text-sm text-destructive">{t('types.nameRequired')}</p>}
          {editing && <p className="text-xs text-muted-foreground">{t('types.keyHint', { key: editing.key })}</p>}
        </div>
        <div className="grid gap-4 sm:grid-cols-2">
          <div className="grid gap-2.5">
            <Label htmlFor={`${formId}-icon`}>{t('types.icon')}</Label>
            <div className="flex items-center gap-2">
              <ActivityTypeIcon icon={icon} color={color} />
              <NativeSelect
                id={`${formId}-icon`}
                className="flex-1"
                value={icon}
                onChange={(event) => setIcon(event.target.value)}
              >
                {ACTIVITY_ICON_NAMES.map((iconName) => (
                  <option key={iconName} value={iconName}>
                    {iconName}
                  </option>
                ))}
              </NativeSelect>
            </div>
          </div>
          <div className="grid gap-2.5">
            <Label htmlFor={`${formId}-color`}>{t('types.color')}</Label>
            <div className="flex items-center gap-2">
              <input
                id={`${formId}-color`}
                type="color"
                className="h-9 w-12 shrink-0 cursor-pointer rounded-md border border-input bg-background p-1"
                value={HEX_COLOR.test(color) ? color : DEFAULT_ACTIVITY_COLOR}
                onChange={(event) => setColor(event.target.value)}
              />
              <Input placeholder={t('placeholders.colorHex')} aria-label={t('types.colorHex')} value={color} onChange={(event) => setColor(event.target.value)} />
            </div>
          </div>
        </div>
      </form>
      <DialogFooter>
        <Button variant="outline" onClick={onClose} disabled={saving}>
          {t('form.cancel')}
        </Button>
        <Button type="submit" form={formId} disabled={saving}>
          {t('types.save')}
        </Button>
      </DialogFooter>
    </>
  )
}
