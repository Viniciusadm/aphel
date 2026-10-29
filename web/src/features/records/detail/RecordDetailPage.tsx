import { SearchX } from 'lucide-react'
import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { isApiError } from '@/api'
import { PageLoader } from '@/components/feedback/PageLoader'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { useOfflineFieldSchema, useOfflineRecord } from '@/pwa/offline/records'
import type { RecordItem } from '../types'
import { RecordPageLayout, type RecordPageLayoutProps } from './RecordPageLayout'

export interface RecordDetailPageProps
  extends Omit<RecordPageLayoutProps, 'recordType' | 'record' | 'schema' | 'actions'> {
  recordType: string
  recordId: string
  actions?: ReactNode | ((record: RecordItem) => ReactNode)
}

export function RecordDetailPage({ recordType, recordId, actions, back, ...layout }: RecordDetailPageProps) {
  const { t } = useTranslation('records-detail')
  const schema = useOfflineFieldSchema(recordType)
  const record = useOfflineRecord(recordType, recordId)

  if (record.isError && isApiError(record.error) && record.error.status === 404) {
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

  if (record.isError || schema.isError) {
    return (
      <ErrorState
        title={t('states.errorTitle')}
        onRetry={() => {
          if (record.isError) void record.refetch()
          if (schema.isError) void schema.refetch()
        }}
      />
    )
  }

  if (!record.data || !schema.data) return <PageLoader label={t('states.loading')} />

  return (
    <RecordPageLayout
      {...layout}
      recordType={recordType}
      record={record.data}
      schema={schema.data}
      back={back}
      actions={typeof actions === 'function' ? actions(record.data) : actions}
    />
  )
}
