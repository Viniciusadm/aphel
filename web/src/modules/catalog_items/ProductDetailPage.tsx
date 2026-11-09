import { useCallback, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { useParams } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { useModuleTimelineSource } from '@/kernel/modules/timeline-source'
import { RecordDetailPage, type RecordTab } from '@/sdk'
import { ProductActions } from './ProductActions'
import { PRODUCT_READ_ONLY_KEYS, PRODUCT_RECORD_TYPE, PRODUCTS_PATH } from './resource'

const PRODUCT_TABS = ['timeline', 'notes', 'files']

export function ProductDetailPage() {
  const { t } = useTranslation('catalog_items')
  const { id = '' } = useParams()
  const { hasPermission } = useSession()
  const timelineSource = useModuleTimelineSource()
  const timeline = useMemo(() => timelineSource?.(PRODUCT_RECORD_TYPE, id), [timelineSource, id])
  const readOnly = !hasPermission('catalog_items.update')

  const tabs = useCallback(
    (defaults: RecordTab[]): RecordTab[] =>
      defaults.filter((tab) => PRODUCT_TABS.includes(tab.key) && (tab.key !== 'timeline' || timeline)),
    [timeline],
  )

  return (
    <RecordDetailPage
      key={id}
      recordType={PRODUCT_RECORD_TYPE}
      recordId={id}
      back={{ to: PRODUCTS_PATH, label: t('detail.back') }}
      readOnly={readOnly}
      readOnlyFieldKeys={PRODUCT_READ_ONLY_KEYS}
      timeline={timeline}
      tabs={tabs}
      actions={(record) => <ProductActions record={record} />}
    />
  )
}
