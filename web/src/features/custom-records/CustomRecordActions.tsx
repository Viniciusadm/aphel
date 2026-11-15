import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Archive, ArchiveRestore, Trash2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { api } from '@/api'
import { useSession } from '@/app/session/session-context'
import { ConfirmDialog, notifySuccess } from '@/components/feedback'
import { useNotifyDeleted } from '@/features/records/useNotifyDeleted'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { recordsKeys } from '@/features/records/api'

import { recordPath, recordTitle, requireRecordResource } from '@/features/records/resources'
import type { RecordItem } from '@/features/records/types'
import { notifyObjectsError } from './errors'
import { customObjectPath, objectPermission, type CustomObject } from './resources'

export interface CustomRecordActionsProps {
  object: CustomObject
  record: RecordItem
}

export function CustomRecordActions({ object, record }: CustomRecordActionsProps) {
  const { t } = useTranslation('objects')
  const { hasPermission } = useSession()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const notifyDeleted = useNotifyDeleted()
  const recordType = object.record_type
  const resource = requireRecordResource(recordType)
  const [confirmingDelete, setConfirmingDelete] = useState(false)
  const archived = Boolean(record.archived_at)
  const name = recordTitle(resource, record)

  const archive = useMutation({
    mutationFn: () =>
      api.post<RecordItem>(`${recordPath(resource, record.id)}/${archived ? 'unarchive' : 'archive'}`),
    onSuccess: (updated) => {
      queryClient.setQueryData(recordsKeys.detail(recordType, record.id), updated)
      void queryClient.invalidateQueries({ queryKey: recordsKeys.list(recordType) })
      notifySuccess(t(archived ? 'detail.unarchived' : 'detail.archived'))
    },
    onError: notifyObjectsError,
  })

  const remove = async () => {
    try {
      await api.delete<void>(recordPath(resource, record.id))
    } catch (error) {
      notifyObjectsError(error)
      throw error
    }
    void queryClient.invalidateQueries({ queryKey: recordsKeys.list(recordType) })
    void queryClient.invalidateQueries({ queryKey: recordsKeys.lookup(recordType) })
    notifyDeleted({ recordType, recordId: record.id, message: t('detail.deleted') })
    navigate(customObjectPath(object.key))
  }

  return (
    <div className="flex flex-wrap items-center gap-2">
      {archived && <Badge variant="warning">{t('detail.archivedBadge')}</Badge>}
      {hasPermission(objectPermission(object, 'archive')) && (
        <Button size="sm" variant="outline" disabled={archive.isPending} onClick={() => archive.mutate()}>
          {archived ? <ArchiveRestore aria-hidden="true" /> : <Archive aria-hidden="true" />}
          {t(archived ? 'detail.unarchive' : 'detail.archive')}
        </Button>
      )}
      {hasPermission(objectPermission(object, 'delete')) && (
        <Button size="sm" variant="outline" onClick={() => setConfirmingDelete(true)}>
          <Trash2 aria-hidden="true" />
          {t('detail.delete')}
        </Button>
      )}
      <ConfirmDialog
        open={confirmingDelete}
        onOpenChange={setConfirmingDelete}
        variant="destructive"
        title={t('detail.deleteTitle', { name })}
        description={t('detail.deleteDescription')}
        confirmationText={name}
        notifyOnError={false}
        onConfirm={remove}
      />
    </div>
  )
}
