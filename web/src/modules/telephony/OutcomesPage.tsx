import { FieldError as SharedFieldError } from '@/components/ui/field-error'
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
          <Button size="sm" onClick={() => setEditor({ outcome: null })}>
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
              <div className="grid min-w-32 flex-1 gap-0.5">
                <span className="truncate text-sm font-medium">{outcome.name}</span>
                <code className="truncate text-xs text-muted-foreground">{outcome.key}</code>
              </div>
              <div className="flex flex-wrap items-center gap-2">
                {outcome.is_connected && <Badge variant="success">{t('outcomes.connected')}</Badge>}
                {outcome.is_system && <Badge variant="secondary">{t('outcomes.system')}</Badge>}
                {!outcome.is_active && <Badge variant="outline">{t('outcomes.inactive')}</Badge>}
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => setEditor({ outcome })}
                  aria-label={t('outcomes.editLabel', { name: outcome.name })}
                >
                  <Pencil aria-hidden="true" />
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  disabled={busyId === outcome.id}
                  onClick={() => void toggleActive(outcome)}
                >
                  {t(outcome.is_active ? 'outcomes.deactivate' : 'outcomes.activate')}
                </Button>
              </div>
            </div>
          )}
        />
      )}
      <Dialog open={editor !== null} onOpenChange={(open) => !open && setEditor(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t(editor?.outcome ? 'outcomes.editTitle' : 'outcomes.newTitle')}</DialogTitle>
            <DialogDescription>{t('outcomes.dialogDescription')}</DialogDescription>
          </DialogHeader>
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
  const formId = useId()
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
        <Field label={t('outcomes.name')}>
          {(control) => <>
            <Input placeholder={t('placeholders.outcomeName')}
            {...control}
            value={name}
            autoFocus
            aria-invalid={nameError ? true : undefined}
            onChange={(event) => {
              setName(event.target.value)
              setNameError(false)
            }}
          />
          {nameError && <SharedFieldError>{t('outcomes.nameRequired')}</SharedFieldError>}
          {editing && <p className="text-xs text-muted-foreground">{t('outcomes.keyHint', { key: editing.key })}</p>}
          </>}
        </Field>
        <ToggleField label={t('outcomes.isConnected')} description={t('outcomes.isConnectedHint')} checked={connected} onCheckedChange={(value) => setConnected(value === true)} />
      </form>
      <DialogFooter>
        <Button variant="outline" onClick={onClose} disabled={saving}>
          {t('form.cancel')}
        </Button>
        <Button type="submit" form={formId} disabled={saving}>
          {t('outcomes.save')}
        </Button>
      </DialogFooter>
    </>
  )
}
