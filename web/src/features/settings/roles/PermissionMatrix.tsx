import { Fragment } from 'react'
import { useTranslation } from 'react-i18next'
import type { PermissionDefinition, RecordScope } from '../api'
import { moduleLabel, permissionLabel } from '../lib/permissions'
import { useLabelText } from '../lib/use-label-text'
import { MATRIX_SCOPES, groupCatalog, type MatrixValue } from './matrix'

interface PermissionMatrixProps {
  catalog: PermissionDefinition[]
  value: MatrixValue
  readOnly?: boolean
  onChange: (value: MatrixValue) => void
}

export function PermissionMatrix({ catalog, value, readOnly = false, onChange }: PermissionMatrixProps) {
  const { t } = useTranslation('settings')
  const labelText = useLabelText()
  const groups = groupCatalog(catalog)
  const scopeLabel = (scope: RecordScope | null) => t(`permissions.columns.${scope ?? 'none'}`)

  const set = (permission: string, scope: RecordScope | null) => onChange({ ...value, [permission]: scope })

  return (
    <div className="relative overflow-x-auto rounded-lg border">
      <table className="w-full text-sm">
        <caption className="sr-only">{t('roles.editor.matrixCaption')}</caption>
        <thead className="bg-muted/50 text-muted-foreground">
          <tr>
            <th scope="col" className="px-3 py-2 text-left font-medium">
              {t('permissions.columns.permission')}
            </th>
            {[null, ...MATRIX_SCOPES].map((scope) => (
              <th key={scope ?? 'none'} scope="col" className="px-3 py-2 text-center font-medium">
                {scopeLabel(scope)}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {groups.map((group) => (
            <Fragment key={group.module}>
              <tr className="border-t bg-muted/20">
                <th scope="colgroup" colSpan={5} className="px-3 py-2 text-left text-xs font-semibold tracking-wide uppercase">
                  {labelText(moduleLabel(group.module))}
                </th>
              </tr>
              {group.definitions.map((definition) => {
                const rowLabel = `${labelText(moduleLabel(group.module))} · ${labelText(
                  permissionLabel(definition.permission, definition.record_scoped, definition.label),
                )}`
                const current = value[definition.permission] ?? null
                return (
                  <tr key={definition.permission} className="border-t">
                    <th scope="row" className="px-3 py-2 text-left font-normal">
                      {labelText(permissionLabel(definition.permission, definition.record_scoped, definition.label))}
                    {definition.description?.trim() && <div className="mt-1 text-xs text-muted-foreground">{definition.description}</div>}
                    </th>
                    {[null, ...MATRIX_SCOPES].map((scope) => {
                      const applicable = definition.record_scoped || scope === null || scope === 'organization'
                      return (
                        <td key={scope ?? 'none'} className="px-3 py-2 text-center">
                          {applicable ? (
                            <input
                              type="radio"
                              className="size-4 accent-primary"
                              name={`permission-${definition.permission}`}
                              aria-label={`${rowLabel}: ${
                                !definition.record_scoped && scope === 'organization'
                                  ? t('permissions.columns.granted')
                                  : scopeLabel(scope)
                              }`}
                              checked={current === scope}
                              disabled={readOnly}
                              onChange={() => set(definition.permission, scope)}
                            />
                          ) : (
                            <span className="text-muted-foreground" aria-label={t('permissions.notApplicable')}>
                              —
                            </span>
                          )}
                        </td>
                      )
                    })}
                  </tr>
                )
              })}
            </Fragment>
          ))}
        </tbody>
      </table>
    </div>
  )
}
