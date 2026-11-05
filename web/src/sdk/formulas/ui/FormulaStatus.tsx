import { AlertCircle, CheckCircle2, Loader2 } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { FormulaCatalog, FormulaPreview } from './api'
import { formulaErrorMessage } from './messages'

interface FormulaStatusProps {
  id: string
  preview: FormulaPreview | undefined
  expression: string
  validating: boolean
  failed: boolean
  catalog: FormulaCatalog | undefined
}

export function FormulaStatus({ id, preview, expression, validating, failed, catalog }: FormulaStatusProps) {
  const { t } = useTranslation('formulas')
  const fieldLabels = new Map(catalog?.fields.map((field) => [field.key, field.label]) ?? [])

  let status
  if (validating) {
    status = (
      <span className="inline-flex items-center gap-1.5 text-muted-foreground">
        <Loader2 className="size-4 animate-spin" aria-hidden="true" />
        {t('editor.validating')}
      </span>
    )
  } else if (failed) {
    status = <span className="text-muted-foreground">{t('editor.checkFailed')}</span>
  } else if (preview?.valid) {
    status = (
      <span className="inline-flex flex-wrap items-center gap-1.5 text-success">
        <CheckCircle2 className="size-4" aria-hidden="true" />
        {t('editor.valid')}
        {preview.kind && (
          <span className="text-muted-foreground">· {t('editor.inferredKind', { kind: t(`kinds.${preview.kind}`) })}</span>
        )}
      </span>
    )
  } else if (preview) {
    status = (
      <span className="inline-flex items-center gap-1.5 text-destructive">
        <AlertCircle className="size-4" aria-hidden="true" />
        {t('editor.invalid')}
      </span>
    )
  }

  const errors = preview && !preview.valid ? preview.errors : []
  const dependencies = preview?.valid ? preview.dependencies : undefined
  const hasDependencies =
    dependencies && (dependencies.fields.length > 0 || dependencies.aggregates.length > 0 || dependencies.uses_today)

  return (
    <div id={id} className="grid min-w-0 gap-2 text-sm" aria-live="polite">
      {status && <p>{status}</p>}
      {errors.length > 0 && (
        <ul className="grid gap-1" aria-label={t('editor.errorsLabel')}>
          {errors.map((error, index) => (
            <li
              key={`${error.code}-${error.start}-${index}`}
              className="flex min-w-0 flex-wrap gap-x-2 rounded-md border border-destructive/30 bg-destructive/5 px-2 py-1.5 text-destructive"
            >
              <span className="shrink-0 font-mono text-xs leading-5 font-medium">
                {t('editor.position', { position: error.start + 1 })}
              </span>
              <span className="min-w-0 break-words">{formulaErrorMessage(error, t, expression)}</span>
            </li>
          ))}
        </ul>
      )}
      {dependencies && (
        <div className="grid gap-1.5 rounded-md border bg-muted/30 px-3 py-2">
          <p className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">{t('dependencies.title')}</p>
          {!hasDependencies && <p className="text-muted-foreground">{t('dependencies.none')}</p>}
          {dependencies.fields.length > 0 && (
            <p className="flex flex-wrap items-center gap-1">
              <span className="text-muted-foreground">{t('dependencies.fields')}:</span>
              {dependencies.fields.map((key) => (
                <code key={key} className="rounded bg-muted px-1.5 py-0.5 text-xs" title={fieldLabels.get(key)}>
                  {key}
                </code>
              ))}
            </p>
          )}
          {dependencies.aggregates.length > 0 && (
            <p className="flex flex-wrap items-center gap-1">
              <span className="text-muted-foreground">{t('dependencies.aggregates')}:</span>
              {dependencies.aggregates.map((aggregate) => {
                const text = `${aggregate.function}(${aggregate.relation}${aggregate.field ? `.${aggregate.field}` : ''})`
                return (
                  <code key={text} className="rounded bg-muted px-1.5 py-0.5 text-xs">
                    {text}
                  </code>
                )
              })}
            </p>
          )}
          {dependencies.uses_today && <p className="text-muted-foreground">{t('dependencies.today')}</p>}
        </div>
      )}
    </div>
  )
}
