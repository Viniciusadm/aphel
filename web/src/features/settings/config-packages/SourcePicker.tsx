import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { FileJson, LayoutTemplate, Upload } from 'lucide-react'
import { useId, useRef, useState, type DragEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { cn } from '@/lib/utils'
import { usePackageTemplates, type PackageSource } from './api'
import { usePackageLabels } from './lib/labels'
import { readPackageFile, type PackageFileError } from './lib/package-file'

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
  const inputId = useId()
  const inputRef = useRef<HTMLInputElement>(null)
  const [dragging, setDragging] = useState(false)
  const [error, setError] = useState<PackageFileError | null>(null)
  const [fileName, setFileName] = useState<string | null>(null)

  const handleFile = async (file: File | undefined) => {
    if (!file) return
    setFileName(file.name)
    setError(null)
    const result = await readPackageFile(file)
    if (!result.ok) {
      setError(result.error)
      return
    }
    onChoose({ package: result.package })
  }

  const drop = (event: DragEvent<HTMLDivElement>) => {
    event.preventDefault()
    setDragging(false)
    if (!disabled) void handleFile(event.dataTransfer.files[0])
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('install.uploadTitle')}</CardTitle>
        <CardDescription>{t('install.uploadDescription')}</CardDescription>
      </CardHeader>
      <CardContent className="grid gap-3">
        <div
          className={cn(
            'flex flex-col items-center gap-3 rounded-lg border border-dashed p-6 text-center transition-colors',
            dragging && 'border-primary bg-accent',
          )}
          onDragOver={(event) => {
            event.preventDefault()
            setDragging(true)
          }}
          onDragLeave={() => setDragging(false)}
          onDrop={drop}
        >
          <FileJson className="size-8 text-muted-foreground" aria-hidden="true" />
          <p className="text-sm text-muted-foreground">{t('install.dropHint')}</p>
          <input
            ref={inputRef}
            id={inputId}
            type="file"
            accept=".json,application/json"
            className="sr-only"
            disabled={disabled}
            aria-invalid={error ? true : undefined}
            aria-describedby={error ? `${inputId}-error` : undefined}
            onChange={(event) => {
              void handleFile(event.target.files?.[0])
              event.target.value = ''
            }}
          />
          <Button type="button" variant="outline" disabled={disabled} onClick={() => inputRef.current?.click()}>
            <Upload aria-hidden="true" />
            {t('install.chooseFile')}
          </Button>
          <label htmlFor={inputId} className="sr-only">
            {t('install.fileLabel')}
          </label>
          {fileName && <p className="max-w-full truncate text-xs text-muted-foreground">{fileName}</p>}
        </div>
        <div aria-live="assertive">
          {error && (
            <SharedFieldError id={`${inputId}-error`}>
              {t(`install.fileErrors.${error}`)}
            </SharedFieldError>
          )}
        </div>
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
