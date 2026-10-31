import { Database, Plus, ShieldCheck } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useParams } from 'react-router'
import { ConfirmDialog } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { toast } from '@/components/ui/toast-store'
import { ListSkeleton } from '../components/ListSkeleton'
import { RecordTypeSelect } from '../components/RecordTypeSelect'
import { RequirePermission } from '../components/RequirePermission'
import { SETTINGS_MANAGE } from '../lib/permissions'
import { fieldsApi, useFieldMutation, useFields, useReorderFields, type Field } from './api'
import { FieldRow, type FieldAction } from './components/FieldRow'
import { SortableList } from './components/SortableList'
import { FieldEditorDialog } from './FieldEditorDialog'
import { errorMessageKey, isRecordTypeNotFound } from './lib/errors'
import { DEFAULT_RECORD_TYPE } from './lib/field-types'
import { mergeVisibleOrder, moveId } from './lib/order'
import { useRecordTypeLabel } from './lib/useRecordTypeLabel'
import { PageHeader } from '@/components/ui/page-header'

export function FieldsPage() {
  const { recordType = DEFAULT_RECORD_TYPE } = useParams()
  return (
    <RequirePermission permission={SETTINGS_MANAGE}>
      <FieldsSection key={recordType} recordType={recordType} />
    </RequirePermission>
  )
}

function FieldsSection({ recordType }: { recordType: string }) {
  const { t } = useTranslation('fields')
  const { t: tp } = useTranslation('field-permissions')
  const fields = useFields(recordType)
  const [showInactive, setShowInactive] = useState(true)
  const [editing, setEditing] = useState<{ fieldId: string | null } | null>(null)
  const [deactivating, setDeactivating] = useState<Field | null>(null)
  const reorder = useReorderFields(recordType)
  const deactivate = useFieldMutation(recordType, (field: Field) => fieldsApi.deactivate(recordType, field.id))
  const activate = useFieldMutation(recordType, (field: Field) => fieldsApi.activate(recordType, field.id))

  const all = fields.data ?? []
  const visible = showInactive ? all : all.filter((field) => field.is_active)
  const unavailable = fields.isError && isRecordTypeNotFound(fields.error)
  const recordTypeLabel = useRecordTypeLabel(recordType)
  const editingField = editing?.fieldId ? (all.find((field) => field.id === editing.fieldId) ?? null) : null

  const saveOrder = (visibleOrder: string[]) => {
    reorder.mutate(mergeVisibleOrder(all, visibleOrder), {
      onError: (error) => toast({ title: t(errorMessageKey(error)), variant: 'destructive' }),
    })
  }

  const confirmDeactivate = async (field: Field) => {
    try {
      await deactivate.mutateAsync(field)
      toast({ title: t('toast.deactivated'), variant: 'success' })
    } catch (error) {
      toast({ title: t(errorMessageKey(error)), variant: 'destructive' })
    } finally {
      setDeactivating(null)
    }
  }

  const activateField = async (field: Field) => {
    try {
      await activate.mutateAsync(field)
      toast({ title: t('toast.activated'), variant: 'success' })
    } catch (error) {
      toast({ title: t(errorMessageKey(error)), variant: 'destructive' })
    }
  }

  const handleAction = (action: FieldAction, field: Field) => {
    const ids = visible.map((item) => item.id)
    const index = ids.indexOf(field.id)
    switch (action) {
      case 'edit':
        setEditing({ fieldId: field.id })
        break
      case 'moveUp':
        if (index > 0) saveOrder(moveId(ids, index, index - 1))
        break
      case 'moveDown':
        if (index >= 0 && index < ids.length - 1) saveOrder(moveId(ids, index, index + 1))
        break
      case 'deactivate':
        setDeactivating(field)
        break
      case 'activate':
        void activateField(field)
        break
    }
  }

  const openCreate = () => setEditing({ fieldId: null })

  return (
    <section className="grid grid-cols-1 gap-4">
      <PageHeader
        title={t('title')}
        description={t('description')}
        actions={
          <div className="flex flex-wrap gap-2">
            <RecordTypeSelect
              value={recordType}
              label={t('recordTypes.label')}
              coreLabel={(type) => t(`recordTypes.${type}`)}
              to={(type) => `/settings/fields/${type}`}
            />
            <Button variant="outline" asChild>
              <Link to={`/settings/fields/${recordType}/permissions`}>
                <ShieldCheck />
                {tp('link')}
              </Link>
            </Button>
            <Button onClick={openCreate} disabled={!fields.isSuccess}>
              <Plus />
              {t('create')}
            </Button>
          </div>
        }
      />
      {fields.isSuccess && all.length > 0 && (
        <label className="flex w-fit items-center gap-2 text-sm text-muted-foreground">
          <input
            type="checkbox"
            className="size-4 accent-primary"
            checked={showInactive}
            onChange={(event) => setShowInactive(event.target.checked)}
          />
          {t('list.showInactive')}
        </label>
      )}
      {fields.isPending ? (
        <ListSkeleton rows={5} />
      ) : unavailable ? (
        <EmptyState
          icon={Database}
          title={t('list.unavailableTitle')}
          description={t('list.unavailableDescription')}
        />
      ) : fields.isError ? (
        <ErrorState onRetry={() => void fields.refetch()} />
      ) : all.length === 0 ? (
        <EmptyState
          title={t('list.empty')}
          description={t('list.emptyDescription')}
          action={
            <Button onClick={openCreate}>
              <Plus />
              {t('create')}
            </Button>
          }
        />
      ) : visible.length === 0 ? (
        <EmptyState title={t('list.allInactive')} description={t('list.allInactiveDescription')} />
      ) : (
        <SortableList
          items={visible}
          getId={(field) => field.id}
          getLabel={(field) => field.label}
          onReorder={(ids) => saveOrder(ids)}
          label={t('list.label', { recordType: recordTypeLabel })}
          renderItem={(field, handle, index) => (
            <FieldRow
              field={field}
              handle={handle}
              canMoveUp={index > 0}
              canMoveDown={index < visible.length - 1}
              onAction={handleAction}
            />
          )}
        />
      )}
      {editing && (editing.fieldId === null || editingField) && (
        <FieldEditorDialog recordType={recordType} field={editingField} onClose={() => setEditing(null)} />
      )}
      {deactivating && (
        <ConfirmDialog
          open
          title={t('confirmDeactivate.title', { label: deactivating.label })}
          description={t('confirmDeactivate.description')}
          confirmLabel={t('actions.deactivate')}
          pending={deactivate.isPending}
          variant="destructive"
          onConfirm={() => confirmDeactivate(deactivating)}
          onOpenChange={(open) => !open && setDeactivating(null)}
        />
      )}
    </section>
  )
}
