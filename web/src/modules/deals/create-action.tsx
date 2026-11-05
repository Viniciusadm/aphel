import { useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { RecordFormDialog } from '@/sdk'
import type { CreateActionRenderProps } from '@/kernel/modules/define'
import { dealsKeys } from './api'
import { DEAL_CREATE_LAYOUT, DEAL_RECORD_TYPE } from './resource'

export function CreateDealDialog({ open, onClose }: CreateActionRenderProps) {
  const { t } = useTranslation('deals')
  const queryClient = useQueryClient()
  return (
    <RecordFormDialog
      recordType={DEAL_RECORD_TYPE}
      open={open}
      onOpenChange={(next) => !next && onClose()}
      layout={DEAL_CREATE_LAYOUT}
      title={t('dialog.title')}
      description={t('dialog.description')}
      onCreated={() => void queryClient.invalidateQueries({ queryKey: dealsKeys.boards() })}
    />
  )
}
