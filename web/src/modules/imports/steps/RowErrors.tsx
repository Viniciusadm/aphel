import { useTranslation } from 'react-i18next'
import { describeError, type ImportRowReport } from '../mapping'

export interface RowErrorsProps {
  rows: ImportRowReport[]
}

export function RowErrors({ rows }: RowErrorsProps) {
  const { t } = useTranslation('imports')
  if (rows.length === 0) return null
  return (
    <ul className="grid max-h-80 gap-2 overflow-y-auto">
      {rows.map((row) => (
        <li key={row.row_index} className="grid gap-1 rounded-md border px-3 py-2 text-sm">
          <span className="font-medium">{t('validation.lineLabel', { line: row.row_index })}</span>
          <ul className="grid gap-0.5 text-muted-foreground">
            {row.errors.map((error, index) => (
              <li key={index} className="break-words">
                {describeError(error)}
              </li>
            ))}
          </ul>
        </li>
      ))}
    </ul>
  )
}
