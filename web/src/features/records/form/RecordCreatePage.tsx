import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useLocation, useNavigate, useParams } from 'react-router'
import { RequirePermission } from '@/features/settings/components/RequirePermission'
import { UnsavedChangesGuard } from '@/components/feedback'
import { EmptyState } from '@/components/ui/empty-state'
import { Page } from '@/components/ui/page'
import { PageLoader } from '@/components/ui/page-loader'
import { useCustomObjects } from '@/features/custom-records/api'
import { getRecordResource } from '../resources'
import type { RecordPayload, RecordValues } from '../fields'
import { RecordFormDialogBody } from './RecordFormDialog'
import { PageHeader } from '@/components/ui/page-header'

interface CreateState {
  defaultValues?: RecordValues
  presetValues?: RecordPayload
  returnTo?: string
}

export function RecordCreatePage() {
  const { recordType = '' } = useParams()
  return <CreateRecord key={recordType} recordType={recordType} />
}

function CreateRecord({ recordType }: { recordType: string }) {
  const { t } = useTranslation('records-form')
  const location = useLocation()
  const navigate = useNavigate()
  const objects = useCustomObjects()
  const resource = getRecordResource(recordType)
  const state = location.state as CreateState | null
  const [dirty, setDirty] = useState(false)
  const finished = useRef(false)
  const target = typeof state?.returnTo === 'string' && state.returnTo.startsWith('/') && !state.returnTo.startsWith('//') ? state.returnTo : '/'
  const leave = () => void navigate(target)
  if (!resource && objects.isPending) return <PageLoader />
  if (!resource) return <EmptyState title={t('dialog.unavailable')} />
  const editor = <Page>
    <PageHeader title={t('dialog.title')} back={{ to: target }} />
    <div className="rounded-xl border bg-card p-4 [--dialog-padding:1rem] [&_[data-slot=dialog-footer]]:bottom-[calc(4.5rem+env(safe-area-inset-bottom))] sm:p-6 sm:[--dialog-padding:1.5rem] md:[&_[data-slot=dialog-footer]]:bottom-0">
      <RecordFormDialogBody full recordType={recordType} defaultValues={state?.defaultValues} presetValues={state?.presetValues} onDirtyChange={setDirty} onCancel={leave} onDone={() => { finished.current = true; leave() }} />
    </div>
    <UnsavedChangesGuard when={() => (dirty || Boolean(state?.defaultValues && Object.keys(state.defaultValues).length)) && !finished.current} />
  </Page>
  return resource.createPermission ? <RequirePermission permission={resource.createPermission}>{editor}</RequirePermission> : editor
}
