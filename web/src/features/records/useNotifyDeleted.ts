import { useQueryClient } from '@tanstack/react-query'
import { useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { api } from '@/api'
import { notifyError, notifySuccess } from '@/components/feedback'
import { toast } from '@/components/ui/toast-store'
import { useIsModuleActive } from '@/kernel/modules/registry'

export interface DeletedRecord {
  recordType: string
  recordId: string
  message: string
}

const UNDO_DURATION_MS = 10_000

export function useNotifyDeleted() {
  const { t } = useTranslation('records-detail')
  const queryClient = useQueryClient()
  const trashActive = useIsModuleActive('trash')
  return useCallback(
    ({ recordType, recordId, message }: DeletedRecord) => {
      if (!trashActive) {
        notifySuccess(message)
        return
      }
      toast({
        title: message,
        description: t('undo.description'),
        variant: 'success',
        duration: UNDO_DURATION_MS,
        action: {
          label: t('undo.label'),
          onClick: () => {
            api
              .post(`/v1/trash/${encodeURIComponent(recordType)}/${encodeURIComponent(recordId)}/restore`)
              .then(() => {
                notifySuccess(t('undo.restored'))
                return queryClient.invalidateQueries()
              })
              .catch((error: unknown) => notifyError(error))
          },
        },
      })
    },
    [queryClient, t, trashActive],
  )
}
