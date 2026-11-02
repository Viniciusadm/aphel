import { Fragment } from 'react'
import { useTranslation } from 'react-i18next'
import type { PermissionDefinition, RecordScope } from '../api'
import { moduleLabel, permissionLabel } from '../lib/permissions'
import { useCustomObjectsAdmin } from '@/modules/objects/ui/api'
import { useLabelText } from '../lib/use-label-text'
import { MATRIX_SCOPES, groupCatalog, type MatrixValue } from './matrix'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'

interface PermissionMatrixProps {
  catalog: PermissionDefinition[]
  value: MatrixValue
  readOnly?: boolean
  onChange: (value: MatrixValue) => void
}

export function PermissionMatrix({ catalog, value, readOnly = false, onChange }: PermissionMatrixProps) {
  const { t } = useTranslation('settings')
  const labelText = useLabelText()
  const objects = useCustomObjectsAdmin()
  const internal = new Set((objects.data ?? []).filter((object) => object.is_internal).map((object) => object.record_type))
  const groups = groupCatalog(catalog).filter((group) => !internal.has(group.module))
  const scopeLabel = (scope: RecordScope | null) => t(`permissions.columns.${scope ?? 'none'}`)

  const set = (permission: string, scope: RecordScope | null) => onChange({ ...value, [permission]: scope })

  return (
    <Table>
      <caption className="sr-only">{t('roles.editor.matrixCaption')}</caption>
      <TableHeader>
        <TableRow>
          <TableHead scope="col" className="text-left">
            {t('permissions.columns.permission')}
          </TableHead>
          {[null, ...MATRIX_SCOPES].map((scope) => (
            <TableHead key={scope ?? 'none'} scope="col" className="text-center">
              {scopeLabel(scope)}
            </TableHead>
          ))}
        </TableRow>
      </TableHeader>
      <TableBody>
        {groups.map((group) => (
          <Fragment key={group.module}>
            <TableRow className="bg-muted/20">
              <TableHead scope="colgroup" colSpan={5} className="text-left text-xs font-semibold tracking-wide uppercase">
                {labelText(moduleLabel(group.module))}
              </TableHead>
            </TableRow>
            {group.definitions.map((definition) => {
              const rowLabel = `${labelText(moduleLabel(group.module))} · ${labelText(
                permissionLabel(definition.permission, definition.record_scoped, definition.label),
              )}`
              const current = value[definition.permission] ?? null
              return (
                <TableRow key={definition.permission}>
                  <TableHead scope="row" className="text-left font-normal">
                    {labelText(permissionLabel(definition.permission, definition.record_scoped, definition.label))}
                  {definition.description?.trim() && <div className="mt-1 text-xs text-muted-foreground">{definition.description}</div>}
                  </TableHead>
                  {[null, ...MATRIX_SCOPES].map((scope) => {
                    const applicable = definition.record_scoped || scope === null || scope === 'organization'
                    return (
                      <TableCell key={scope ?? 'none'} className="text-center">
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
                      </TableCell>
                    )
                  })}
                </TableRow>
              )
            })}
          </Fragment>
        ))}
      </TableBody>
    </Table>
  )
}
