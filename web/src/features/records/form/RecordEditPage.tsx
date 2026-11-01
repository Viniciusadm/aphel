import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useLocation, useNavigate, useParams } from 'react-router'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { PageLoader } from '@/components/ui/page-loader'
import { ErrorState } from '@/components/ui/error-state'
import { EmptyState } from '@/components/ui/empty-state'
import { UnsavedChangesGuard } from '@/components/feedback'
import { RequirePermission } from '@/features/settings/components/RequirePermission'
import { useCustomObjects } from '@/features/custom-records/api'
import { useOfflineRecord } from '@/pwa/offline/records'
import { getRecordResource, recordTitle, type RecordResource } from '../resources'
import { RecordFormDialogBody } from './RecordFormDialog'

export function RecordEditPage() {
  const { recordType = '', recordId = '' } = useParams()
  const { t } = useTranslation('records-form')
  const objects = useCustomObjects()
  const resource = getRecordResource(recordType)
  if (!resource && objects.isPending) return <PageLoader />
  if (!resource) return <EmptyState title={t('dialog.unavailable')} />
  const editor = <RecordEditor key={`${recordType}:${recordId}`} resource={resource} recordId={recordId} />
  return resource.updatePermission ? <RequirePermission permission={resource.updatePermission}>{editor}</RequirePermission> : editor
}

function RecordEditor({ resource, recordId }: { resource: RecordResource; recordId: string }) {
  const { t } = useTranslation('records-form')
  const record = useOfflineRecord(resource.recordType, recordId)
  const location = useLocation()
  const navigate = useNavigate()
  const [dirty, setDirty] = useState(false)
  const finished = useRef(false)
  const state = location.state as { returnTo?: string; listReturnTo?: string } | null
  const target = typeof state?.returnTo === 'string' && state.returnTo.startsWith('/') && !state.returnTo.startsWith('//') ? state.returnTo : resource.detailPath?.(recordId) ?? '/'
  const leave = () => void navigate(target, { state: { returnTo: state?.listReturnTo } })
  if (record.isPending) return <PageLoader />
  if (record.isError || !record.data) return <ErrorState onRetry={() => void record.refetch()} />
  return <Page width="form">
    <PageHeader title={t('dialog.editTitle')} description={recordTitle(resource, record.data)} back={{ to: target }} />
    <div className="rounded-xl border bg-card p-4 [--dialog-padding:1rem] [&_[data-slot=dialog-footer]]:bottom-[calc(4.5rem+env(safe-area-inset-bottom))] sm:p-6 sm:[--dialog-padding:1.5rem] md:[&_[data-slot=dialog-footer]]:bottom-0">
      <RecordFormDialogBody recordType={resource.recordType} record={record.data} onDirtyChange={setDirty} onCancel={leave} onDone={() => { finished.current = true; leave() }} />
    </div>
    <UnsavedChangesGuard when={() => dirty && !finished.current} />
  </Page>
}
