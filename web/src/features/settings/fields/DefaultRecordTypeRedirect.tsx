import { Database } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Navigate } from 'react-router'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { useEntities } from '@/sdk/entities'
import { ListSkeleton } from '../components/ListSkeleton'

export function DefaultRecordTypeRedirect() {
  const { t } = useTranslation('fields')
  const entities = useEntities()
  const first = entities.data?.[0]
  if (first) return <Navigate to={first.record_type} replace />

  return (
    <Page>
      <PageHeader title={t('title')} description={t('description')} />
      {entities.isPending ? (
        <ListSkeleton rows={5} />
      ) : entities.isError ? (
        <ErrorState onRetry={() => void entities.refetch()} />
      ) : (
        <EmptyState icon={Database} title={t('list.noRecordTypes')} description={t('list.noRecordTypesDescription')} />
      )}
    </Page>
  )
}
