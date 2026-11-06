import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { ErrorBoundary } from '@/components/feedback/ErrorBoundary'
import { ErrorState } from '@/components/ui/error-state'
import type { RecordSectionSource } from './types'

export function RecordSectionBoundary({ children, recordType, recordId, source, label }: {
  children?: ReactNode
  recordType: string
  recordId: string
  source: RecordSectionSource
  label: string
}) {
  const { t } = useTranslation('records-detail')
  const identity = `${source.module}:${source.key}`

  return (
    <ErrorBoundary
      resetKeys={[recordType, recordId, identity]}
      fallback={({ reset }) => (
        <ErrorState
          title={t('composition.sectionFailed', { label })}
          description={t('composition.sectionSource', { identity })}
          onRetry={reset}
        />
      )}
    >
      {children}
    </ErrorBoundary>
  )
}
