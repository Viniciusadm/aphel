import { useQueryClient } from '@tanstack/react-query'
import { Boxes, Pencil, Plus, Settings2, Trash2 } from 'lucide-react'
import { useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate } from 'react-router'
import { notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { ObjectIcon, notifyObjectsError, objectColor } from '@/features/custom-records'
import { RequirePermission } from '@/features/settings/components/RequirePermission'
import { SectionHeader } from '@/features/settings/components/SectionHeader'
import { SortableList } from '@/features/settings/fields/components/SortableList'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import { objectsAdminApi, objectsAdminKeys, useCustomObjectsAdmin, useInvalidateObjects, type CustomObjectAdmin } from './api'
import { ObjectEditorDialog } from './ObjectEditorDialog'
import { ObjectRemovalDialogs, type RemovalState } from './ObjectRemovalDialogs'

export function ObjectsSettingsPage() {
  return (
    <RequirePermission permission={SETTINGS_MANAGE}>
      <ObjectsSection />
    </RequirePermission>
  )
}

function ObjectsSection() {
  const { t } = useTranslation('objects')
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const objects = useCustomObjectsAdmin()
  const invalidate = useInvalidateObjects()
  const [editor, setEditor] = useState<{ object: CustomObjectAdmin | null } | null>(null)
  const [removal, setRemoval] = useState<RemovalState | null>(null)
  const [reordering, setReordering] = useState(false)

  const reorder = async (keys: string[]) => {
    const previous = objects.data ?? []
    const byKey = new Map(previous.map((object) => [object.key, object]))
    queryClient.setQueryData(
      objectsAdminKeys.list(),
      keys.flatMap((key) => {
        const object = byKey.get(key)
        return object ? [object] : []
      }),
    )
    setReordering(true)
    try {
      await objectsAdminApi.reorder(keys)
      notifySuccess(t('settings.reordered'))
    } catch (error) {
      queryClient.setQueryData(objectsAdminKeys.list(), previous)
      notifyObjectsError(error)
    } finally {
      setReordering(false)
      void invalidate()
    }
  }

  return (
    <section className="flex flex-col gap-4">
      <SectionHeader
        title={t('settings.title')}
        description={t('settings.description')}
        action={
          <Button size="sm" onClick={() => setEditor({ object: null })}>
            <Plus aria-hidden="true" />
            {t('settings.new')}
          </Button>
        }
      />
      {objects.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-14 w-full" />
          <Skeleton className="h-14 w-full" />
        </div>
      ) : objects.isError ? (
        <ErrorState title={t('settings.loadError')} onRetry={() => void objects.refetch()} />
      ) : objects.data.length === 0 ? (
        <EmptyState
          icon={Boxes}
          title={t('settings.emptyTitle')}
          description={t('settings.emptyDescription')}
          action={
            <Button size="sm" onClick={() => setEditor({ object: null })}>
              <Plus aria-hidden="true" />
              {t('settings.new')}
            </Button>
          }
        />
      ) : (
        <SortableList
          items={objects.data}
          getId={(object) => object.key}
          getLabel={(object) => object.label_plural}
          onReorder={(keys) => void reorder(keys)}
          disabled={reordering}
          label={t('settings.listLabel')}
          renderItem={(object, handle) => (
            <ObjectRow
              object={object}
              handle={handle}
              onEdit={() => setEditor({ object })}
              onRemove={() => setRemoval({ object, step: object.record_count > 0 ? 'deactivate' : 'delete' })}
            />
          )}
        />
      )}
      <ObjectEditorDialog
        object={editor?.object ?? null}
        open={editor !== null}
        onOpenChange={(open) => !open && setEditor(null)}
        onSaved={(saved) => {
          void invalidate()
          if (!editor?.object) navigate(saved.key)
        }}
      />
      <ObjectRemovalDialogs state={removal} onChange={setRemoval} onDone={() => void invalidate()} />
    </section>
  )
}

interface ObjectRowProps {
  object: CustomObjectAdmin
  handle: ReactNode
  onEdit: () => void
  onRemove: () => void
}

function ObjectRow({ object, handle, onEdit, onRemove }: ObjectRowProps) {
  const { t } = useTranslation('objects')
  return (
    <div className="flex min-w-0 flex-wrap items-center gap-2 rounded-md border bg-card px-2 py-2 sm:flex-nowrap">
      {handle}
      <span
        className="flex size-8 shrink-0 items-center justify-center rounded-md border"
        style={{ color: objectColor(object.color) }}
      >
        <ObjectIcon icon={object.icon} className="size-4" aria-hidden />
      </span>
      <div className="grid min-w-0 flex-1 gap-0.5">
        <Link to={object.key} className="truncate font-medium hover:underline">
          {object.label_plural}
        </Link>
        <span className="truncate text-xs text-muted-foreground">
          <span className="font-mono">{object.record_type}</span>
          {' · '}
          {t('settings.recordCount', { count: object.record_count })}
        </span>
      </div>
      <div className="flex shrink-0 items-center gap-1">
        {!object.is_active && <Badge variant="secondary">{t('settings.inactive')}</Badge>}
        <Button size="sm" variant="ghost" asChild>
          <Link to={object.key} aria-label={t('settings.configureLabel', { label: object.label_plural })}>
            <Settings2 aria-hidden="true" />
          </Link>
        </Button>
        <Button size="sm" variant="ghost" aria-label={t('settings.editLabel', { label: object.label_plural })} onClick={onEdit}>
          <Pencil aria-hidden="true" />
        </Button>
        <Button
          size="sm"
          variant="ghost"
          aria-label={t('settings.deleteLabel', { label: object.label_plural })}
          onClick={onRemove}
        >
          <Trash2 aria-hidden="true" />
        </Button>
      </div>
    </div>
  )
}
