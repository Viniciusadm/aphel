import { Loader2 } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { RecordFieldInput } from '@/components/fields'
import { useRecordFormatting } from '@/sdk'
import type { FormulaPreview } from '@/sdk'
import { formatFormulaValue, type FormulaFormatOptions } from '@/modules/formulas/ui/format'
import { Field } from '@/components/ui/field'

interface FormulaPreviewPanelProps {
  recordType: string
  recordId: string | null
  onRecordChange: (recordId: string | null) => void
  preview: FormulaPreview | undefined
  error: unknown
  loading: boolean
  format: FormulaFormatOptions
}

export function FormulaPreviewPanel({
  recordType,
  recordId,
  onRecordChange,
  preview,
  error,
  loading,
  format,
}: FormulaPreviewPanelProps) {
  const { t } = useTranslation('formulas')
  const formatting = useRecordFormatting()
  const pickerId = 'formula-preview-record'
  const hintId = `${pickerId}-hint`

  let result
  if (!recordId) {
    result = <span className="text-muted-foreground">{t('preview.pickRecord')}</span>
  } else if (isApiError(error) && error.code === 'record_not_found') {
    result = <span className="text-destructive">{t('preview.recordNotFound')}</span>
  } else if (loading && !preview) {
    result = <Loader2 className="size-4 animate-spin text-muted-foreground" aria-label={t('editor.validating')} />
  } else if (!preview || !preview.valid || !preview.evaluated) {
    result = <span className="text-muted-foreground">{t('preview.notEvaluated')}</span>
  } else {
    const text = formatFormulaValue(preview.value, format, formatting, t)
    result = text ? (
      <span className="font-medium break-words tabular-nums">{text}</span>
    ) : (
      <span className="text-muted-foreground">{t('preview.empty')}</span>
    )
  }

  return (
    <section className="grid min-w-0 gap-3 rounded-md border p-3" aria-labelledby="formula-preview-title">
      <h3 id="formula-preview-title" className="text-sm font-medium">
        {t('preview.title')}
      </h3>
      <div className="grid min-w-0 gap-3 sm:grid-cols-2">
        <Field label={t('preview.record')} className="min-w-0 content-start">
          {(fieldControl) => <>
            <RecordFieldInput
            id={fieldControl.id}
            value={recordId}
            onChange={onRecordChange}
            targetRecordType={recordType}
            aria-describedby={hintId}
          />
          <p id={hintId} className="text-xs text-muted-foreground">
            {t('preview.recordHint')}
          </p>
          </>}
        </Field>
        <div className="grid min-w-0 content-start gap-2">
          <p className="text-sm leading-none font-medium">{t('preview.value')}</p>
          <div className="flex min-h-9 min-w-0 items-center gap-2 rounded-md border bg-muted/40 px-3 py-1.5 text-sm" aria-live="polite">
            {result}
            {loading && preview && <Loader2 className="size-3.5 shrink-0 animate-spin text-muted-foreground" aria-hidden="true" />}
          </div>
        </div>
      </div>
    </section>
  )
}
