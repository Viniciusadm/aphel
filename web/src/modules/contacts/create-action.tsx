import { useTranslation } from 'react-i18next'
import { RecordFormDialog } from '@/features/records/form'
import type { CreateActionRenderProps } from '@/kernel/modules/define'
import { CONTACT_CREATE_LAYOUT, CONTACT_RECORD_TYPE } from './resource'

export function CreateContactDialog({ open, onClose }: CreateActionRenderProps) {
  const { t } = useTranslation('contacts')
  return (
    <RecordFormDialog
      recordType={CONTACT_RECORD_TYPE}
      open={open}
      onOpenChange={(next) => !next && onClose()}
      layout={CONTACT_CREATE_LAYOUT}
      title={t('dialog.title')}
      description={t('dialog.description')}
    />
  )
}
