import { Plus } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { Button } from '@/components/ui/button'
import { RecordFormDialog } from '@/features/records/form'
import { RecordListView } from '@/features/records/table'
import { CONTACT_CREATE_LAYOUT, CONTACT_RECORD_TYPE, contactResource } from './resource'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

export function ContactsPage() {
  const { t } = useTranslation('people')
  const navigate = useNavigate()
  const { hasPermission } = useSession()
  const [creating, setCreating] = useState(false)
  const canCreate = hasPermission('people.create')
  const newButton = canCreate ? (
    <Button size="sm" onClick={() => setCreating(true)}>
      <Plus aria-hidden="true" />
      {t('list.new')}
    </Button>
  ) : undefined

  return (
    <Page>
      <PageHeader title={t('list.title')} actions={newButton} />
      <RecordListView
        recordType={CONTACT_RECORD_TYPE}
        emptyAction={newButton}
        onRowClick={(record) => navigate(contactResource.detailPath!(record.id))}
        archivedFilter
      />
      {canCreate && (
        <RecordFormDialog
          recordType={CONTACT_RECORD_TYPE}
          open={creating}
          onOpenChange={setCreating}
          layout={CONTACT_CREATE_LAYOUT}
          title={t('dialog.title')}
          description={t('dialog.description')}
          onCreated={(record) => {
            navigate(contactResource.detailPath!(record.id))
          }}
        />
      )}
    </Page>
  )
}
