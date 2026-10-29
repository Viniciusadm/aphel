import type { FieldAccess, FieldPermissionEntry, FieldPermissionField, FieldPermissionMatrix } from './api'

export type MatrixDraft = Record<string, FieldAccess>

export function cellKey(fieldId: string, roleId: string): string {
  return `${fieldId}:${roleId}`
}

export function matrixDraft(matrix: FieldPermissionMatrix): MatrixDraft {
  const draft: MatrixDraft = {}
  for (const field of matrix.fields) {
    for (const cell of field.access) draft[cellKey(field.id, cell.role_id)] = cell.access
  }
  return draft
}

export function isConfigurable(field: FieldPermissionField): boolean {
  return !field.is_locked
}

export function changedCells(initial: MatrixDraft, draft: MatrixDraft): string[] {
  return Object.keys(draft).filter((key) => (initial[key] ?? 'editable') !== draft[key])
}

export function draftEntries(matrix: FieldPermissionMatrix, draft: MatrixDraft): FieldPermissionEntry[] {
  const configurableRoles = matrix.roles.filter((role) => role.is_configurable)
  return matrix.fields
    .filter(isConfigurable)
    .flatMap((field) =>
      configurableRoles.flatMap((role) => {
        const access = draft[cellKey(field.id, role.id)] ?? 'editable'
        return access === 'editable' ? [] : [{ field_id: field.id, role_id: role.id, access }]
      }),
    )
}

export function applyToColumn(
  draft: MatrixDraft,
  fields: FieldPermissionField[],
  roleId: string,
  access: FieldAccess,
): MatrixDraft {
  const next = { ...draft }
  for (const field of fields) {
    if (isConfigurable(field)) next[cellKey(field.id, roleId)] = access
  }
  return next
}
