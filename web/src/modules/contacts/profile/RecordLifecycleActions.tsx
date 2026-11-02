import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Archive, ArchiveRestore, Trash2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { api } from '@/api'
import { useSession } from '@/app/session/session-context'
import { ConfirmDialog, notifyError, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { recordsKeys } from '@/features/records/api'

import { recordPath, recordTitle, requireRecordResource } from '@/features/records/resources'
import type { RecordItem } from '@/features/records/types'

export interface RecordLifecycleActionsProps {
  recordType: string
  record: RecordItem
  listPath: string
  archivePermission: string
  deletePermission: string
}

export function RecordLifecycleActions({
  recordType,
  record,
  listPath,
  archivePermission,
  deletePermission,
}: RecordLifecycleActionsProps) {
  const { t } = useTranslation('contacts')
  const { hasPermission } = useSession()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
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
      notifySuccess(t(archived ? 'profile.actions.unarchived' : 'profile.actions.archived'))
    },
    onError: (error) => notifyError(error),
  })

  const remove = async () => {
    await api.delete<void>(recordPath(resource, record.id))
    void queryClient.invalidateQueries({ queryKey: recordsKeys.list(recordType) })
    void queryClient.invalidateQueries({ queryKey: recordsKeys.lookup(recordType) })
    notifySuccess(t('profile.actions.deleted'))
    navigate(listPath)
  }

  return (
    <div className="flex flex-wrap items-center gap-2">
      {archived && <Badge variant="warning">{t('profile.actions.archivedBadge')}</Badge>}
      {hasPermission(archivePermission) && (
        <Button size="sm" variant="outline" disabled={archive.isPending} onClick={() => archive.mutate()}>
          {archived ? <ArchiveRestore aria-hidden="true" /> : <Archive aria-hidden="true" />}
          {t(archived ? 'profile.actions.unarchive' : 'profile.actions.archive')}
        </Button>
      )}
      {hasPermission(deletePermission) && (
        <Button size="sm" variant="outline" onClick={() => setConfirmingDelete(true)}>
          <Trash2 aria-hidden="true" />
          {t('profile.actions.delete')}
        </Button>
      )}
      <ConfirmDialog
        open={confirmingDelete}
        onOpenChange={setConfirmingDelete}
        variant="destructive"
        title={t('profile.actions.deleteTitle', { name })}
        description={t('profile.actions.deleteDescription')}
        confirmationText={name}
        onConfirm={remove}
      />
    </div>
  )
}
