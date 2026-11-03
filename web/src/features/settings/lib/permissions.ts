import type { ParseKeys } from 'i18next'

export const USERS_MANAGE = 'users.manage'
export const SETTINGS_MANAGE = 'settings.manage'
export const AUDIT_READ = 'audit.read'
export const SYSTEM_MANAGE = 'system.manage'
export const PRIVACY_MANAGE = 'privacy.manage'
export const AUTOMATIONS_MANAGE = 'automations.manage'
export const LEAD_DISTRIBUTION_MANAGE = 'lead_distribution.manage'
export const ORGANIZATION_EXPORT = 'organization.export'
export const EMAILS_READ = 'emails.read'
export const EMAILS_SEND = 'emails.send'
export const SEQUENCES_MANAGE = 'sequences.manage'
export const RECORDS_SHARE = 'records.share'

type SettingsKey = ParseKeys<'settings'>

const moduleLabels: Record<string, SettingsKey> = {
  people: 'permissions.modules.people',
  organizations: 'permissions.modules.organizations',
  deals: 'permissions.modules.deals',
  activities: 'permissions.modules.activities',
  notes: 'permissions.modules.notes',
  emails: 'permissions.modules.emails',
  sequences: 'permissions.modules.sequences',
  scheduling: 'permissions.modules.scheduling',
  goals: 'permissions.modules.goals',
  products: 'permissions.modules.products',
  quotes: 'permissions.modules.quotes',
}

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
  [LEAD_DISTRIBUTION_MANAGE]: 'permissions.administrative.leadDistributionManage',
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
  return { key: moduleLabels[module], fallback: module }
}

export function permissionLabel(permission: string, recordScoped: boolean, label?: string | null): PermissionLabel {
  if (!recordScoped) return { key: administrativeLabels[permission], fallback: label?.trim() || permission }
  const action = permission.split('.').slice(1).join('.')
  return { key: actionLabels[action], fallback: label?.trim() || action || permission }
}
