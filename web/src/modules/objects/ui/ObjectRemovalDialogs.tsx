import { useTranslation } from 'react-i18next'
import { ConfirmDialog, notifySuccess } from '@/components/feedback'
import { notifyObjectsError } from '@/features/custom-records'
import { hasErrorCode } from '@/features/custom-records/errors'
import { objectsAdminApi, type CustomObjectAdmin } from '@/modules/objects/ui/api'

export interface RemovalState {
  object: CustomObjectAdmin
  step: 'delete' | 'deactivate'
}

export interface ObjectRemovalDialogsProps {
  state: RemovalState | null
  onChange: (state: RemovalState | null) => void
  onDone: () => void
}

export function ObjectRemovalDialogs({ state, onChange, onDone }: ObjectRemovalDialogsProps) {
  const { t } = useTranslation('objects')
  const object = state?.object
  const label = object?.label_plural ?? ''

  const remove = async () => {
    if (!object) return
    try {
      await objectsAdminApi.remove(object.key)
    } catch (error) {
      if (hasErrorCode(error, 'custom_object_has_records')) {
        onChange({ object, step: 'deactivate' })
        throw error
      }
      notifyObjectsError(error)
      throw error
    }
    notifySuccess(t('removal.deleted', { label }))
    onDone()
  }

  const deactivate = async () => {
    if (!object) return
    if (!object.is_active) return
    try {
      await objectsAdminApi.update(object.key, { is_active: false }, object.updated_at)
    } catch (error) {
      notifyObjectsError(error)
      throw error
    }
    notifySuccess(t('removal.deactivated', { label }))
    onDone()
  }

  return (
    <>
      <ConfirmDialog
        open={state?.step === 'delete'}
        onOpenChange={(open) => !open && state?.step === 'delete' && onChange(null)}
        variant="destructive"
        title={t('removal.deleteTitle', { label })}
        description={t('removal.deleteDescription')}
        confirmationText={object?.key}
        confirmLabel={t('removal.delete')}
        notifyOnError={false}
        onConfirm={remove}
      />
      <ConfirmDialog
        open={state?.step === 'deactivate'}
        onOpenChange={(open) => !open && onChange(null)}
        title={t('removal.hasRecordsTitle', { label })}
        description={
          object?.is_active
            ? t('removal.hasRecordsDescription', { count: object.record_count })
            : t('removal.hasRecordsInactive', { count: object?.record_count ?? 0 })
        }
        confirmLabel={object?.is_active ? t('removal.deactivate') : t('removal.understood')}
        notifyOnError={false}
        onConfirm={deactivate}
      />
    </>
  )
}
