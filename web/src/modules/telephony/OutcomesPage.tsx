import { MoreHorizontal, Pencil, Phone, Plus, Power } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import { DialogForm } from '@/components/ui/dialog-form'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Skeleton } from '@/components/ui/skeleton'
import { toast } from '@/components/ui/toast-store'
import { RequirePermission, SETTINGS_MANAGE, SortableList } from '@/sdk'
import { cn } from '@/lib/utils'
import { callsApi, useCallOutcomes, useInvalidateCalls } from './api'
import { callErrorKey } from './lib/errors'
import type { CallOutcome } from './types'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'
import { Field } from '@/components/ui/field'
import { ToggleField } from '@/components/ui/toggle-field'

interface EditorState {
  outcome: CallOutcome | null
}

interface PendingOrder {
  source: CallOutcome[] | undefined
  items: CallOutcome[]
}

export function OutcomesPage() {
  return (
    <RequirePermission permission={SETTINGS_MANAGE}>
      <OutcomesSection />
    </RequirePermission>
  )
}

function OutcomesSection() {
  const { t } = useTranslation('telephony')
  const outcomes = useCallOutcomes(true)
  const invalidate = useInvalidateCalls()
  const [order, setOrder] = useState<PendingOrder | null>(null)
  const [editor, setEditor] = useState<EditorState | null>(null)
  const [busyId, setBusyId] = useState<string | null>(null)
  const items = order && order.source === outcomes.data ? order.items : (outcomes.data ?? [])

  const saveOrder = async (ids: string[]) => {
    const byId = new Map(items.map((item) => [item.id, item]))
    setOrder({ source: outcomes.data, items: ids.flatMap((id) => byId.get(id) ?? []) })
    try {
      await callsApi.reorderOutcomes(ids)
      await invalidate()
    } catch (error) {
      setOrder(null)
      toast({ title: t(callErrorKey(error)), variant: 'destructive' })
    }
  }

  const toggleActive = async (outcome: CallOutcome) => {
    setBusyId(outcome.id)
    try {
      await callsApi.updateOutcome(outcome.id, { is_active: !outcome.is_active })
      await invalidate()
    } catch (error) {
      toast({ title: t(callErrorKey(error)), variant: 'destructive' })
    } finally {
      setBusyId(null)
    }
  }

  return (
    <Page>
      <PageHeader
        title={t('outcomes.title')}
        description={t('outcomes.description')}
        actions={
          <Button onClick={() => setEditor({ outcome: null })}>
            <Plus aria-hidden="true" />
            {t('outcomes.new')}
          </Button>
        }
      />
      {outcomes.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-12 w-full" />
          <Skeleton className="h-12 w-full" />
        </div>
      ) : outcomes.isError ? (
        <ErrorState onRetry={() => void outcomes.refetch()} />
      ) : items.length === 0 ? (
        <EmptyState
          icon={Phone}
          title={t('outcomes.emptyTitle')}
          description={t('outcomes.emptyDescription')}
          action={
            <Button onClick={() => setEditor({ outcome: null })}>
              <Plus aria-hidden="true" />
              {t('outcomes.new')}
            </Button>
          }
        />
      ) : (
        <SortableList
          items={items}
          getId={(outcome) => outcome.id}
          getLabel={(outcome) => outcome.name}
          onReorder={(ids) => void saveOrder(ids)}
          label={t('outcomes.listLabel')}
          renderItem={(outcome, handle) => (
            <div
              className={cn(
                'flex flex-wrap items-center gap-3 rounded-md border bg-card px-3 py-2',
                !outcome.is_active && 'opacity-70',
              )}
            >
              {handle}
              <span className="min-w-32 flex-1 truncate text-sm font-medium">{outcome.name}</span>
              <div className="flex flex-wrap items-center gap-2">
                {outcome.is_connected && <Badge variant="success">{t('outcomes.connected')}</Badge>}
                {outcome.is_system && <Badge variant="secondary">{t('outcomes.system')}</Badge>}
                {!outcome.is_active && <Badge variant="outline">{t('outcomes.inactive')}</Badge>}
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => setEditor({ outcome })}
                  aria-label={t('outcomes.editLabel', { name: outcome.name })}
                >
                  <Pencil aria-hidden="true" />
                  {t('outcomes.edit')}
                </Button>
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button size="icon" variant="ghost" aria-label={t('outcomes.actions', { name: outcome.name })}>
                      <MoreHorizontal aria-hidden="true" />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
                    <DropdownMenuItem disabled={busyId === outcome.id} onSelect={() => void toggleActive(outcome)}>
                      <Power aria-hidden="true" />
                      {t(outcome.is_active ? 'outcomes.deactivate' : 'outcomes.activate')}
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              </div>
            </div>
          )}
        />
      )}
      <Dialog open={editor !== null} onOpenChange={(open) => !open && setEditor(null)}>
        <DialogContent>
          {editor && (
            <OutcomeForm
              key={editor.outcome?.id ?? 'new'}
              editing={editor.outcome}
              onClose={() => setEditor(null)}
              onSaved={() => void invalidate()}
            />
          )}
        </DialogContent>
      </Dialog>
    </Page>
  )
}

function OutcomeForm({
  editing,
  onClose,
  onSaved,
}: {
  editing: CallOutcome | null
  onClose: () => void
  onSaved: () => void
}) {
  const { t } = useTranslation('telephony')
  const [name, setName] = useState(editing?.name ?? '')
  const [connected, setConnected] = useState(editing?.is_connected ?? false)
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
        await callsApi.updateOutcome(editing.id, { name: name.trim(), is_connected: connected })
      } else {
        await callsApi.createOutcome({ name: name.trim(), is_connected: connected })
      }
      toast({ title: t('outcomes.saved', { name: name.trim() }), variant: 'success' })
      onSaved()
      onClose()
    } catch (error) {
      toast({ title: t(callErrorKey(error)), variant: 'destructive' })
    } finally {
      setSaving(false)
    }
  }

  return (
    <DialogForm
      noValidate
      onSubmit={(event) => {
        event.preventDefault()
        void submit()
      }}
      title={t(editing ? 'outcomes.editTitle' : 'outcomes.newTitle')}
      description={t('outcomes.dialogDescription')}
      cancelLabel={t('form.cancel')}
      submitLabel={t('outcomes.save')}
      onCancel={onClose}
      pending={saving}
      submitDisabled={saving}
    >
      <Field label={t('outcomes.name')} error={nameError ? t('outcomes.nameRequired') : undefined}>
        {(control) => (
          <Input
            {...control}
            autoFocus
            value={name}
            placeholder={t('placeholders.outcomeName')}
            onChange={(event) => {
              setName(event.target.value)
              setNameError(false)
            }}
          />
        )}
      </Field>
      <ToggleField
        type="switch"
        label={t('outcomes.isConnected')}
        description={t('outcomes.isConnectedHint')}
        checked={connected}
        onCheckedChange={setConnected}
      />
    </DialogForm>
  )
}
