import { LayoutTemplate } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import {usePackageTemplates, type PackageSource} from '@/sdk'
import {usePackageLabels} from '@/sdk'
import { PackageFileDrop } from '@/modules/config_packages/ui/PackageFileDrop'

interface SourcePickerProps {
  planning: boolean
  onChoose: (source: PackageSource) => void
}

export function SourcePicker({ planning, onChoose }: SourcePickerProps) {
  const { t } = useTranslation('config-packages')
  return (
    <div className="grid gap-4" aria-busy={planning}>
      <p className="text-sm text-muted-foreground" role="status" aria-live="polite">
        {planning ? t('install.planning') : ''}
      </p>
      <UploadSource disabled={planning} onChoose={onChoose} />
      <TemplateSource disabled={planning} onChoose={onChoose} />
    </div>
  )
}

interface SourceProps {
  disabled: boolean
  onChoose: (source: PackageSource) => void
}

function UploadSource({ disabled, onChoose }: SourceProps) {
  const { t } = useTranslation('config-packages')
  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('install.uploadTitle')}</CardTitle>
        <CardDescription>{t('install.uploadDescription')}</CardDescription>
      </CardHeader>
      <CardContent>
        <PackageFileDrop
          disabled={disabled}
          hint={t('install.dropHint')}
          chooseLabel={t('install.chooseFile')}
          fileLabel={t('install.fileLabel')}
          errorText={(error) => t(`install.fileErrors.${error}`)}
          onChoose={(pkg) => onChoose({ package: pkg })}
        />
      </CardContent>
    </Card>
  )
}

function TemplateSource({ disabled, onChoose }: SourceProps) {
  const { t } = useTranslation('config-packages')
  const labels = usePackageLabels()
  const templates = usePackageTemplates()

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('install.templatesTitle')}</CardTitle>
        <CardDescription>{t('install.templatesDescription')}</CardDescription>
      </CardHeader>
      <CardContent>
        {templates.isPending ? (
          <div className="grid gap-3 sm:grid-cols-2" aria-busy="true">
            <Skeleton className="h-32 w-full" />
            <Skeleton className="h-32 w-full" />
          </div>
        ) : templates.isError ? (
          <ErrorState onRetry={() => void templates.refetch()} />
        ) : templates.data.length === 0 ? (
          <EmptyState icon={LayoutTemplate} title={t('install.templatesEmpty')} />
        ) : (
          <ul className="grid gap-3 sm:grid-cols-2" aria-label={t('install.templatesTitle')}>
            {templates.data.map((template) => (
              <li key={template.key} className="flex min-w-0 flex-col gap-3 rounded-lg border bg-card p-4">
                <div className="grid gap-1">
                  <h3 className="font-medium break-words">{template.name}</h3>
                  {template.description && (
                    <p className="text-sm text-muted-foreground break-words">{template.description}</p>
                  )}
                  {template.author && (
                    <p className="text-xs text-muted-foreground">{t('install.author', { author: template.author })}</p>
                  )}
                </div>
                <ul className="flex flex-wrap gap-1.5" aria-label={t('install.templateSections', { name: template.name })}>
                  {template.sections.map((section) => (
                    <li key={section}>
                      <Badge variant="secondary">{labels.section(section)}</Badge>
                    </li>
                  ))}
                </ul>
                <Button
                  className="mt-auto self-start"
                  size="sm"
                  disabled={disabled}
                  aria-label={t('install.useTemplateLabel', { name: template.name })}
                  onClick={() => onChoose({ template: template.key })}
                >
                  {t('install.useTemplate')}
                </Button>
              </li>
            ))}
          </ul>
        )}
      </CardContent>
    </Card>
  )
}
