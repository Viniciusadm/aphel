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
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { toast } from '@/components/ui/toast-store'
import { SortableList } from '@/features/settings/fields/components/SortableList'
import { cn } from '@/lib/utils'
import { lifecycleApi, useDisqualificationReasons, useInvalidateDisqualificationReasons } from '../api'
import { MAX_REASON_NAME_LENGTH } from '../constants'
import { lifecycleErrorMessage } from '../errors'
import type { DisqualificationReason } from '../types'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

interface EditorState {
  reason: DisqualificationReason | null
}

interface PendingOrder {
  source: DisqualificationReason[] | undefined
  items: DisqualificationReason[]
}

export function DisqualificationReasonsSection() {
  const { t } = useTranslation('leads')
  const reasons = useDisqualificationReasons(true)
  const invalidate = useInvalidateDisqualificationReasons()
  const [order, setOrder] = useState<PendingOrder | null>(null)
  const [editor, setEditor] = useState<EditorState | null>(null)
  const [deleting, setDeleting] = useState<DisqualificationReason | null>(null)
  const [busyId, setBusyId] = useState<string | null>(null)
  const items = order && order.source === reasons.data ? order.items : (reasons.data ?? [])
  const fail = (error: unknown) => toast({ title: lifecycleErrorMessage(error, t), variant: 'destructive' })

  const saveOrder = async (ids: string[]) => {
    const byId = new Map(items.map((item) => [item.id, item]))
    setOrder({ source: reasons.data, items: ids.flatMap((id) => byId.get(id) ?? []) })
    try {
      await lifecycleApi.reorderReasons(ids)
      await invalidate()
    } catch (error) {
      setOrder(null)
      fail(error)
    }
  }

  const toggleActive = async (reason: DisqualificationReason) => {
    setBusyId(reason.id)
    try {
      await lifecycleApi.updateReason(reason.id, { is_active: !reason.is_active }, reason.updated_at)
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
      await lifecycleApi.deleteReason(deleting.id)
      notifySuccess(t('reasons.deleted', { name: deleting.name }))
      setDeleting(null)
      await invalidate()
    } catch (error) {
      fail(error)
    }
  }

  return (
    <Page>
      <PageHeader level="section"
        title={t('reasons.title')}
        description={t('reasons.description')}
        actions={
          <Button size="sm" onClick={() => setEditor({ reason: null })}>
            <Plus aria-hidden="true" />
            {t('reasons.new')}
          </Button>
        }
      />
      {reasons.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-12 w-full" />
          <Skeleton className="h-12 w-full" />
        </div>
      ) : reasons.isError ? (
        <ErrorState onRetry={() => void reasons.refetch()} />
      ) : items.length === 0 ? (
        <EmptyState title={t('reasons.empty')} />
      ) : (
        <SortableList
          items={items}
          getId={(reason) => reason.id}
          getLabel={(reason) => reason.name}
          onReorder={(ids) => void saveOrder(ids)}
          label={t('reasons.listLabel')}
          renderItem={(reason, handle) => (
            <div
              className={cn(
                'flex flex-wrap items-center gap-3 rounded-md border bg-card px-3 py-2',
                !reason.is_active && 'opacity-70',
              )}
            >
              {handle}
              <div className="grid min-w-32 flex-1 gap-0.5">
                <span className="truncate text-sm font-medium">{reason.name}</span>
                <code className="truncate text-xs text-muted-foreground">{reason.key}</code>
              </div>
              <div className="flex flex-wrap items-center gap-2">
                {!reason.is_active && <Badge variant="outline">{t('reasons.inactive')}</Badge>}
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => setEditor({ reason })}
                  aria-label={t('reasons.editLabel', { name: reason.name })}
                >
                  <Pencil aria-hidden="true" />
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => setDeleting(reason)}
                  aria-label={t('reasons.deleteLabel', { name: reason.name })}
                >
                  <Trash2 aria-hidden="true" />
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  disabled={busyId === reason.id}
                  onClick={() => void toggleActive(reason)}
                >
                  {t(reason.is_active ? 'reasons.deactivate' : 'reasons.activate')}
                </Button>
              </div>
            </div>
          )}
        />
      )}
      <Dialog open={editor !== null} onOpenChange={(open) => !open && setEditor(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t(editor?.reason ? 'reasons.editTitle' : 'reasons.newTitle')}</DialogTitle>
            <DialogDescription>{t('reasons.dialogDescription')}</DialogDescription>
          </DialogHeader>
          {editor && (
            <ReasonForm
              key={editor.reason?.id ?? 'new'}
              editing={editor.reason}
              onClose={() => setEditor(null)}
              onSaved={() => void invalidate()}
            />
          )}
        </DialogContent>
      </Dialog>
      <ConfirmDialog
        open={deleting !== null}
        onOpenChange={(open) => !open && setDeleting(null)}
        title={t('reasons.deleteTitle')}
        description={t('reasons.deleteDescription', { name: deleting?.name ?? '' })}
        confirmLabel={t('reasons.deleteConfirm')}
        variant="destructive"
        notifyOnError={false}
        onConfirm={remove}
      />
    </Page>
  )
}

function ReasonForm({
  editing,
  onClose,
  onSaved,
}: {
  editing: DisqualificationReason | null
  onClose: () => void
  onSaved: () => void
}) {
  const { t } = useTranslation('leads')
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
        await lifecycleApi.updateReason(editing.id, { name: name.trim() }, editing.updated_at)
      } else {
        await lifecycleApi.createReason({ name: name.trim() })
      }
      notifySuccess(t('reasons.saved', { name: name.trim() }))
      onSaved()
      onClose()
    } catch (error) {
      toast({ title: lifecycleErrorMessage(error, t), variant: 'destructive' })
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
        <div className="grid gap-1.5">
          <Label htmlFor={`${formId}-name`}>{t('reasons.name')}</Label>
          <Input placeholder={t('placeholders.reasonName')}
            id={`${formId}-name`}
            value={name}
            autoFocus
            maxLength={MAX_REASON_NAME_LENGTH}
            aria-invalid={nameError ? true : undefined}
            onChange={(event) => {
              setName(event.target.value)
              setNameError(false)
            }}
          />
          {nameError && <p className="text-xs text-destructive">{t('reasons.nameRequired')}</p>}
          {editing && <code className="text-xs text-muted-foreground">{editing.key}</code>}
        </div>
      </form>
      <DialogFooter>
        <Button variant="outline" onClick={onClose} disabled={saving}>
          {t('actions.cancel')}
        </Button>
        <Button type="submit" form={formId} disabled={saving}>
          {t('reasons.save')}
        </Button>
      </DialogFooter>
    </>
  )
}
