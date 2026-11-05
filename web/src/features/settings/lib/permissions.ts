import type { ParseKeys } from 'i18next'
import i18n from '@/i18n'

export const USERS_MANAGE = 'users.manage'
export const SETTINGS_MANAGE = 'settings.manage'
export const AUDIT_READ = 'audit.read'
export { SYSTEM_MANAGE } from '@/kernel/permissions'
import { SYSTEM_MANAGE } from '@/kernel/permissions'
export const PRIVACY_MANAGE = 'privacy.manage'
export const AUTOMATIONS_MANAGE = 'automations.manage'
export const ASSIGNMENT_MANAGE = 'assignment.manage'
export const ORGANIZATION_EXPORT = 'organization.export'
export const EMAILS_READ = 'emails.read'
export const EMAILS_SEND = 'emails.send'
export const SEQUENCES_MANAGE = 'sequences.manage'
export const RECORDS_SHARE = 'records.share'

type SettingsKey = ParseKeys<'settings'>

const actionLabels: Record<string, SettingsKey> = {
  read: 'permissions.actions.read',
  create: 'permissions.actions.create',
  update: 'permissions.actions.update',
  delete: 'permissions.actions.delete',
  export: 'permissions.actions.export',
  archive: 'permissions.actions.archive',
  manage: 'permissions.actions.manage',
  send: 'permissions.actions.send',
  enroll: 'permissions.actions.enroll',
}

const administrativeLabels: Record<string, SettingsKey> = {
  [SETTINGS_MANAGE]: 'permissions.administrative.settingsManage',
  [USERS_MANAGE]: 'permissions.administrative.usersManage',
  [AUDIT_READ]: 'permissions.administrative.auditRead',
  [SYSTEM_MANAGE]: 'permissions.administrative.systemManage',
  [PRIVACY_MANAGE]: 'permissions.administrative.privacyManage',
  [AUTOMATIONS_MANAGE]: 'permissions.administrative.automationsManage',
  [ASSIGNMENT_MANAGE]: 'permissions.administrative.assignmentManage',
  [ORGANIZATION_EXPORT]: 'permissions.administrative.organizationExport',
  [SEQUENCES_MANAGE]: 'permissions.administrative.sequencesManage',
  [RECORDS_SHARE]: 'permissions.administrative.recordsShare',
  'lifecycle.manage': 'permissions.administrative.lifecycleManage',
  'scoring.manage': 'permissions.administrative.scoringManage',
}

export const ADMINISTRATION_MODULE = 'administration'

export interface PermissionLabel {
  key?: SettingsKey
  fallback: string
}

export function permissionModule(permission: string, recordScoped: boolean): string {
  if (!recordScoped) return ADMINISTRATION_MODULE
  return permission.split('.')[0] ?? permission
}

export function moduleLabel(module: string): PermissionLabel {
  if (module === ADMINISTRATION_MODULE) return { key: 'permissions.modules.administration', fallback: module }
  const key = `permissions.modules.${module}`
  return { key: i18n.exists(key, { ns: 'settings' }) ? (key as SettingsKey) : undefined, fallback: module }
}

export function permissionLabel(permission: string, recordScoped: boolean, label?: string | null): PermissionLabel {
  if (!recordScoped) {
    const key = administrativeLabels[permission]
    if (key || label?.trim()) return { key, fallback: label?.trim() || permission }
    return { fallback: derivedAdministrativeLabel(permission) }
  }
  const action = permission.split('.').slice(1).join('.')
  return { key: actionLabels[action], fallback: label?.trim() || action || permission }
}

function derivedAdministrativeLabel(permission: string): string {
  const [module = permission, ...rest] = permission.split('.')
  const action = rest.join('.')
  if (!action) return permission
  const actionKey = actionLabels[action]
  const label = moduleLabel(module)
  const moduleName = label.key ? i18n.t(label.key, { ns: 'settings' }) : module.replaceAll('_', ' ')
  const actionName = actionKey ? i18n.t(actionKey, { ns: 'settings' }) : action.replaceAll(/[._]/g, ' ')
  return `${moduleName}: ${actionName}`
}
