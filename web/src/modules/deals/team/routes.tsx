import type { RouteObject } from 'react-router'

export const DEAL_MEMBER_ROLES_SETTINGS_PATH = 'deal-team-roles'

export const dealMemberRolesSettingsRoute: RouteObject = {
  path: DEAL_MEMBER_ROLES_SETTINGS_PATH,
  lazy: async () => ({ Component: (await import('./RolesSettingsPage')).RolesSettingsPage }),
}
