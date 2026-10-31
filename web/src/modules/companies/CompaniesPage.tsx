import { Plus } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { Button } from '@/components/ui/button'
import { RecordFormDialog } from '@/features/records/form'
import { RecordListView } from '@/features/records/table'
import { COMPANY_CREATE_LAYOUT, COMPANY_RECORD_TYPE, companyResource } from './resource'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

export function CompaniesPage() {
  const { t } = useTranslation('companies')
  const navigate = useNavigate()
  const { hasPermission } = useSession()
  const [creating, setCreating] = useState(false)
  const canCreate = hasPermission('companies.create')
  const newButton = canCreate ? (
    <Button size="sm" onClick={() => setCreating(true)}>
      <Plus aria-hidden="true" />
      {t('list.new')}
    </Button>
  ) : undefined

  return (
    <Page>
      <PageHeader title={t('list.title')} description={t('list.description')} />
      <RecordListView
        recordType={COMPANY_RECORD_TYPE}
        toolbarActions={newButton}
        emptyAction={newButton}
        onRowClick={(record) => navigate(companyResource.detailPath!(record.id))}
        archivedFilter
      />
      {canCreate && (
        <RecordFormDialog
          recordType={COMPANY_RECORD_TYPE}
          open={creating}
          onOpenChange={setCreating}
          layout={COMPANY_CREATE_LAYOUT}
          title={t('dialog.title')}
          description={t('dialog.description')}
          onCreated={(record) => {
            navigate(companyResource.detailPath!(record.id))
          }}
        />
      )}
    </Page>
  )
}
