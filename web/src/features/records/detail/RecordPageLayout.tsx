import { useState } from 'react'
import { ExperienceNotice } from '@/features/experiences/ExperienceNotice'
import { useRecordExperience } from '@/features/experiences/use-record-experience'
import { useTranslation } from 'react-i18next'
import { ErrorBoundary } from '@/components/feedback/ErrorBoundary'
import { Alert } from '@/components/ui/alert'
import { DisplayRecordProvider } from '../display/display-record'
import { DefaultRecordPresentation } from './DefaultRecordPresentation'
import { useRecordPageModel, type RecordPageModelOptions } from './use-record-page-model'

export interface RecordPageLayoutProps extends Omit<RecordPageModelOptions, 'composition'> {
  defaultTab?: string
  className?: string
}

export function RecordPageLayout({ defaultTab, className, ...options }: RecordPageLayoutProps) {
  const experience = useRecordExperience(options.recordType)
  const failureKey = `${options.recordType}:${options.record.id}:${experience.profileKey ?? ''}:${experience.requested}`
  const [failed, setFailed] = useState<{ key: string; provider: typeof experience.Presentation; detail: typeof experience.detail } | null>(null)
  const hasFailed = failed?.key === failureKey && failed.provider === experience.Presentation && failed.detail === experience.detail
  const model = useRecordPageModel({ ...options, composition: hasFailed ? undefined : experience.composition })
  const { t } = useTranslation('records-detail')
  const presentationModule = experience.requested
  const Presentation = hasFailed ? DefaultRecordPresentation : experience.Presentation ?? DefaultRecordPresentation
  const defaultModel = { ...model, tabs: model.uncomposedTabs, mainSections: undefined }

  return (
    <DisplayRecordProvider value={{ recordType: model.recordType, recordId: model.recordId, title: model.title }}>
      <ExperienceNotice diagnostics={[...experience.diagnostics, ...model.compositionDiagnostics, ...(hasFailed ? [{ code: 'presentationUnavailable' as const, identity: presentationModule }] : [])]} />
      <ErrorBoundary
        onError={() => setFailed({ key: failureKey, provider: experience.Presentation, detail: experience.detail })}
        resetKeys={[model.recordType, model.recordId, presentationModule, Presentation, experience.detail]}
        fallback={() => (
          <>
            <Alert role="alert">{t('composition.presentationFailed', { provider: presentationModule })}</Alert>
            <DefaultRecordPresentation model={defaultModel} defaultTab={defaultTab} className={className} />
          </>
        )}
      >
        <Presentation model={model} defaultTab={defaultTab} className={className} />
      </ErrorBoundary>
    </DisplayRecordProvider>
  )
}
