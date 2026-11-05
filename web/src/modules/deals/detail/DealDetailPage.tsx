import { useCallback, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useParams } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { RecordDetailPage, type RecordTabsConfig } from '@/sdk'
import { Slot } from '@/kernel/modules/Slot'
import { DealCurrencyControl } from '../DealCurrencyControl'
import { DEAL_READ_ONLY_KEYS, DEAL_RECORD_TYPE, DEALS_PATH } from '../resource'
import { DealActions } from './DealActions'
import { useDealTimeline } from './timeline'

export interface DealDetailPageProps {
  tabs?: RecordTabsConfig
}

function sameFields(left: string[], right: string[]) {
  return left.length === right.length && left.every((field, index) => field === right[index])
}

function DealDetail({ id, tabs }: DealDetailPageProps & { id: string }) {
  const { t } = useTranslation('deals')
  const { hasPermission } = useSession()
  const timeline = useDealTimeline(id)
  const readOnly = !hasPermission('deals.update')
  const [locks, setLocks] = useState<Record<string, string[]>>({})
  const lockFields = useCallback(
    (source: string, fields: string[]) =>
      setLocks((current) => (sameFields(current[source] ?? [], fields) ? current : { ...current, [source]: fields })),
    [],
  )
  const lockedKeys = useMemo(() => Object.values(locks).flat(), [locks])
  const readOnlyFieldKeys = useMemo(
    () => (lockedKeys.length > 0 ? [...DEAL_READ_ONLY_KEYS, ...lockedKeys] : DEAL_READ_ONLY_KEYS),
    [lockedKeys],
  )
  const amountLocked = lockedKeys.includes('amount')

  return (
    <>
      <Slot name="deals.detail.fieldLocks" props={{ dealId: id, lockFields }} />
      <RecordDetailPage
        recordType={DEAL_RECORD_TYPE}
        recordId={id}
        back={{ to: DEALS_PATH, label: t('detail.back') }}
        readOnly={readOnly}
        readOnlyFieldKeys={readOnlyFieldKeys}
        timeline={timeline}
        tabs={tabs}
        actions={(record) => (
          <>
            <DealCurrencyControl record={record} readOnly={readOnly || amountLocked} />
            <DealActions record={record} />
          </>
        )}
      />
    </>
  )
}

export function DealDetailPage({ tabs }: DealDetailPageProps) {
  const { id = '' } = useParams()
  return <DealDetail key={id} id={id} tabs={tabs} />
}
