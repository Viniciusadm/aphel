import { Plus } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { Button } from '@/components/ui/button'
import { type FilterNode, type RecordListPreset, RecordFormDialog, RecordListView, useRecordFormatting, useRecordList } from '@/sdk'
import { Slot } from '@/kernel/modules/Slot'
import { PRODUCT_CREATE_LAYOUT, PRODUCT_RECORD_TYPE, productPath } from './resource'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

const FILTERS: Record<'active' | 'inactive' | 'all', FilterNode | null> = {
  active: { field: 'is_active', operator: 'eq', value: true },
  inactive: { field: 'is_active', operator: 'eq', value: false },
  all: null,
}
const PRESET_KEYS = ['active', 'inactive', 'all'] as const
const COLUMNS = ['name', 'code', 'price', 'unit', 'recurrence', 'is_active']

function usePresetCounts() {
  const active = useRecordList(PRODUCT_RECORD_TYPE, { page: 1, perPage: 1, filter: FILTERS.active })
  const inactive = useRecordList(PRODUCT_RECORD_TYPE, { page: 1, perPage: 1, filter: FILTERS.inactive })
  const all = useRecordList(PRODUCT_RECORD_TYPE, { page: 1, perPage: 1, filter: FILTERS.all })
  return { active: active.data?.meta.total, inactive: inactive.data?.meta.total, all: all.data?.meta.total }
}

export function ProductsPage() {
  const { t } = useTranslation('catalog_items')
  const navigate = useNavigate()
  const { hasPermission } = useSession()
  const [creating, setCreating] = useState(false)
  const { currency } = useRecordFormatting()
  const defaults = useMemo(() => ({ currency, recurrence: 'none', is_active: true }), [currency])
  const counts = usePresetCounts()
  const presets: RecordListPreset[] = PRESET_KEYS.map((key) => ({
    key,
    label: t(`list.presets.${key}`),
    filter: FILTERS[key],
    emptyTitle: t(`list.empty.${key}.title`),
    emptyDescription: t(`list.empty.${key}.description`),
  }))
  const canCreate = hasPermission('catalog_items.create')
  const newButton = canCreate ? (
    <Button size="sm" onClick={() => setCreating(true)}>
      <Plus aria-hidden="true" />
      {t('list.new')}
    </Button>
  ) : undefined

  return (
    <Page>
<PageHeader title={t('list.title')} description={t('list.description')} actions={<>
        <Slot name="catalog_items.list.header" props={{}} />
        {newButton}
      </>} />
      <RecordListView
        recordType={PRODUCT_RECORD_TYPE}
        presets={presets}
        presetCounts={counts}
        defaultColumns={COLUMNS}
        calendar={false}
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
