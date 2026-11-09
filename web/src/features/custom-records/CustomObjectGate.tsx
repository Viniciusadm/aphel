import { SearchX } from 'lucide-react'
import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useParams } from 'react-router'
import { PageLoader } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { useEntities } from '@/sdk/entities'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { useCustomObjects } from './api'
import { registerCustomObjectResources, type CustomObject } from './resources'

export interface CustomObjectGateProps {
  children: (object: CustomObject) => ReactNode
}

export function CustomObjectGate({ children }: CustomObjectGateProps) {
  const { t } = useTranslation('objects')
  const { key = '' } = useParams()
  const objects = useCustomObjects()
  const entities = useEntities()

  if (objects.isPending || entities.isPending) return <PageLoader label={t('records.loading')} />
  if (objects.isError) return <ErrorState title={t('records.loadError')} onRetry={() => void objects.refetch()} />

  registerCustomObjectResources(objects.data, entities.data ?? [])
  const object = objects.data.find((item) => item.key === key)
  if (!object) {
    return (
      <EmptyState
        icon={SearchX}
        title={t('records.notFoundTitle')}
        description={t('records.notFoundDescription')}
        action={
          <Button variant="outline" asChild>
            <Link to="/">{t('records.goHome')}</Link>
          </Button>
        }
      />
    )
  }
  return <>{children(object)}</>
}
