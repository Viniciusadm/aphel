import { SearchX } from 'lucide-react'
import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { PageLoader } from '@/components/feedback/PageLoader'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import type { RecordItem } from '../types'
import { RecordPageLayout, type RecordPageLayoutProps } from './RecordPageLayout'
import { useRecordDetailData } from './use-record-detail-data'

export interface RecordDetailPageProps
  extends Omit<RecordPageLayoutProps, 'recordType' | 'record' | 'schema' | 'actions'> {
  recordType: string
  recordId: string
  actions?: ReactNode | ((record: RecordItem) => ReactNode)
}

export function RecordDetailPage({ recordType, recordId, actions, back, ...layout }: RecordDetailPageProps) {
  const { t } = useTranslation('records-detail')
  const data = useRecordDetailData(recordType, recordId)

  if (data.status === 'not-found') {
    return (
      <EmptyState
        icon={SearchX}
        title={t('states.notFoundTitle')}
        description={t('states.notFoundDescription')}
        action={
          back && (
            <Button variant="outline" asChild>
              <Link to={back.to}>{back.label ?? t('header.back')}</Link>
            </Button>
          )
        }
      />
    )
  }

  if (data.status === 'error') {
    return (
      <ErrorState
        title={t('states.errorTitle')}
        onRetry={data.retry}
      />
    )
  }

  if (data.status === 'loading') return <PageLoader label={t('states.loading')} />

  return (
    <RecordPageLayout
      {...layout}
      recordType={recordType}
      record={data.record}
      schema={data.schema}
      back={back}
      actions={typeof actions === 'function' ? actions(data.record) : actions}
    />
  )
}
