import type { PermissionDefinition, PermissionGrant, RecordScope, Role } from '../api'
import { permissionModule } from '../lib/permissions'

export type MatrixValue = Record<string, RecordScope | null>

export const MATRIX_SCOPES: RecordScope[] = ['own', 'team', 'organization']

export interface MatrixGroup {
  module: string
  definitions: PermissionDefinition[]
}

export function groupCatalog(catalog: PermissionDefinition[]): MatrixGroup[] {
  const groups: MatrixGroup[] = []
  for (const definition of catalog) {
    const module = permissionModule(definition.permission, definition.record_scoped)
    let group = groups.find((item) => item.module === module)
    if (!group) {
      group = { module, definitions: [] }
      groups.push(group)
    }
    group.definitions.push(definition)
  }
  return groups
}

export function matrixFromRole(role: Role | null | undefined): MatrixValue {
  const value: MatrixValue = {}
  for (const grant of role?.permissions ?? []) value[grant.permission] = grant.scope
  return value
}

export function grantsFromMatrix(value: MatrixValue): PermissionGrant[] {
  return Object.entries(value).flatMap(([permission, scope]) => (scope ? [{ permission, scope }] : []))
}
