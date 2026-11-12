import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ErrorBoundary } from '@/components/feedback/ErrorBoundary'
import { Alert } from '@/components/ui/alert'
import { RECORD_PRESENTATION_CONTRACT, type RecordPresentationProps } from '@/kernel/modules/record-presentation'
import { useSurface } from '@/kernel/modules/ui-plugin-context'
import { DisplayRecordProvider } from '../display/display-record'
import { DefaultRecordPresentation } from './DefaultRecordPresentation'
import { useRecordPageModel, type RecordPageModelOptions } from './use-record-page-model'

export interface RecordPageLayoutProps extends RecordPageModelOptions {
  defaultTab?: string
  className?: string
}

export function RecordPageLayout({ defaultTab, className, ...options }: RecordPageLayoutProps) {
  const surface = useSurface<RecordPresentationProps, { recordTypes?: readonly string[] }>(RECORD_PRESENTATION_CONTRACT)
  const supported = surface.Component && (!surface.Component.recordTypes || surface.Component.recordTypes.includes(options.recordType))
  const [failedPlugin, setFailedPlugin] = useState<string | null>(null)
  const plugin = surface.plugin
  const hasFailed = failedPlugin !== null && failedPlugin === plugin
  const Presentation = !hasFailed && supported ? surface.Component! : DefaultRecordPresentation
  const model = useRecordPageModel(options)
  const { t } = useTranslation('records-detail')

  return (
    <DisplayRecordProvider value={{ recordType: model.recordType, recordId: model.recordId, title: model.title }}>
      <ErrorBoundary
        onError={() => setFailedPlugin(plugin)}
        resetKeys={[model.recordType, model.recordId, Presentation]}
        fallback={() => (
          <>
            <Alert role="alert">{t('composition.presentationFailed', { provider: plugin ?? '' })}</Alert>
            <DefaultRecordPresentation model={model} defaultTab={defaultTab} className={className} />
          </>
        )}
      >
        <Presentation model={model} defaultTab={defaultTab} className={className} />
      </ErrorBoundary>
      {model.editDialog}
    </DisplayRecordProvider>
  )
}
