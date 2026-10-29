import { useTranslation } from 'react-i18next'
import { RecordFormDialog } from '@/features/records/form'
import type { CreateActionRenderProps } from '@/kernel/modules/define'
import { COMPANY_CREATE_LAYOUT, COMPANY_RECORD_TYPE } from './resource'

export function CreateCompanyDialog({ open, onClose }: CreateActionRenderProps) {
  const { t } = useTranslation('companies')
  return (
    <RecordFormDialog
      recordType={COMPANY_RECORD_TYPE}
      open={open}
      onOpenChange={(next) => !next && onClose()}
      layout={COMPANY_CREATE_LAYOUT}
      title={t('dialog.title')}
      description={t('dialog.description')}
    />
  )
}
