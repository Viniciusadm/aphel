import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { AlertTriangle } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Skeleton } from '@/components/ui/skeleton'
import type { EmailPreview } from '../types'
import { HtmlFrame } from './HtmlFrame'

interface PreviewPaneProps {
  preview: EmailPreview | undefined
  pending: boolean
  error: boolean
}

export function PreviewPane({ preview, pending, error }: PreviewPaneProps) {
  const { t } = useTranslation('email')
  if (pending) {
    return (
      <div className="grid gap-2" aria-busy="true">
        <Skeleton className="h-6 w-2/3" />
        <Skeleton className="h-40 w-full" />
      </div>
    )
  }
  if (error || !preview) return <SharedFieldError>{t('preview.error')}</SharedFieldError>
  return (
    <div className="grid gap-3">
      {preview.missing_variables.length > 0 && (
        <div className="flex gap-2 rounded-md border border-warning-solid/40 bg-warning-solid/10 px-3 py-2 text-sm">
          <AlertTriangle className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
          <div className="grid gap-1">
            <p>{t('preview.missingVariables')}</p>
            <p className="font-mono text-xs break-all">{preview.missing_variables.map((key) => `{{${key}}}`).join(', ')}</p>
          </div>
        </div>
      )}
      <div className="grid gap-1">
        <span className="text-xs text-muted-foreground">{t('fields.subject')}</span>
        <p className="text-sm font-medium break-words">{preview.subject || t('thread.noSubject')}</p>
      </div>
      <HtmlFrame html={preview.html} title={t('preview.frameTitle')} />
    </div>
  )
}
