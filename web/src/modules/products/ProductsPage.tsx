import { Plus } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { Button } from '@/components/ui/button'
import { RecordFormDialog } from '@/features/records/form'
import { useRecordFormatting } from '@/features/records/formatting'
import { RecordListView } from '@/features/records/table'
import { Slot } from '@/kernel/modules/Slot'
import { PRODUCT_CREATE_LAYOUT, PRODUCT_RECORD_TYPE, productPath } from './resource'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

export function ProductsPage() {
  const { t } = useTranslation('products')
  const navigate = useNavigate()
  const { hasPermission } = useSession()
  const [creating, setCreating] = useState(false)
  const { currency } = useRecordFormatting()
  const defaults = useMemo(() => ({ currency, recurrence: 'none', is_active: true }), [currency])
  const canCreate = hasPermission('products.create')
  const newButton = canCreate ? (
    <Button size="sm" onClick={() => setCreating(true)}>
      <Plus aria-hidden="true" />
      {t('list.new')}
    </Button>
  ) : undefined

  return (
    <Page>
<PageHeader title={t('list.title')} actions={<>
        <Slot name="products.list.header" props={{}} />
      
      </>} />
      <RecordListView
        recordType={PRODUCT_RECORD_TYPE}
        toolbarActions={newButton}
        emptyAction={newButton}
        onRowClick={(record) => navigate(productPath(record.id))}
      />
      {canCreate && (
        <RecordFormDialog
          recordType={PRODUCT_RECORD_TYPE}
          open={creating}
          onOpenChange={setCreating}
          layout={PRODUCT_CREATE_LAYOUT}
          defaultValues={defaults}
          title={t('dialog.title')}
          description={t('dialog.description')}
          onCreated={(record) => navigate(productPath(record.id))}
        />
      )}
    </Page>
  )
}
